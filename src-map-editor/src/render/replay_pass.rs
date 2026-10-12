//! Replays the draw list into a texture, see `shaders/ja2_replay.wesl`
//!
//! The replay is a render pass of its own in the root render graph, before the cameras: a
//! fullscreen triangle with the replay shader is drawn into a viewport of the target texture. It
//! needs no camera, because a camera would get intermediate textures and a depth texture of the
//! size of its target from Bevy, which for the world texture would be several times its size in
//! GPU memory.
//!
//! The bindings must not be drawn before their buffers are on the GPU, or the map would vanish
//! while they are uploaded. Buffers are therefore never modified: every change creates new ones,
//! and the render world reports when all buffers of the newest generation are prepared and the
//! pipeline is compiled. Until then, the pass keeps drawing the old bindings
//! (`systems/view_replay.rs`).

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{schedule::camera_driver, FullscreenShader},
    prelude::*,
    render::{
        diagnostic::RecordDiagnostics,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{storage_buffer_read_only_sized, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, LoadOp,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, ShaderType, StoreOp, TextureFormat,
            UniformBuffer,
        },
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue},
        storage::{GpuShaderBuffer, ShaderBuffer},
        texture::GpuImage,
        Render, RenderApp, RenderStartup, RenderSystems,
    },
};

use crate::render::palette::pack_rgba;

/// Path of the embedded replay shader, see `embed_shaders` in `lib.rs`
const SHADER_PATH: &str = "embedded://ja2_map_editor/shaders/ja2_replay.wesl";

/// Format of the textures that the pass draws into
pub const REPLAY_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// Uniform values of the replay shader (`ReplayParams`)
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplayParams {
    pub origin: IVec2,
    pub bins_origin: IVec2,
    pub bins_count: UVec2,
    pub bin_indices_offset: u32,
    pub animations_offset: u32,
    pub clear_color: u32,
}

/// The buffers that the replay shader reads, see `render/atlas.rs` for their contents
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayBindings {
    pub bins_origin: IVec2,
    pub bins_count: UVec2,
    pub bin_indices_offset: u32,
    pub animations_offset: u32,
    pub draws: Handle<ShaderBuffer>,
    pub bins: Handle<ShaderBuffer>,
    pub atlas: Handle<ShaderBuffer>,
    pub palettes: Handle<ShaderBuffer>,
    pub sprites: Handle<ShaderBuffer>,
    pub animation_frames: Handle<ShaderBuffer>,
}

impl ReplayBindings {
    /// The buffers in the order of their bindings
    fn buffers(&self) -> [&Handle<ShaderBuffer>; 6] {
        [
            &self.draws,
            &self.bins,
            &self.atlas,
            &self.palettes,
            &self.sprites,
            &self.animation_frames,
        ]
    }

    fn buffer_ids(&self) -> Vec<AssetId<ShaderBuffer>> {
        self.buffers().into_iter().map(Handle::id).collect()
    }
}

/// A replay into a texture
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReplayPass {
    pub target: Handle<Image>,
    /// Top left texel and size of the drawn part of the target. The rest of the target is
    /// cleared.
    pub viewport_position: UVec2,
    pub viewport_size: UVec2,
    /// Game pixel of the top left texel of the target
    pub origin: IVec2,
    pub clear_color: Color,
    /// Without bindings, the target is only cleared
    pub bindings: Option<ReplayBindings>,
    /// Whether the pass is drawn in this frame
    pub active: bool,
}

/// The replays that the render world draws
#[derive(Resource, ExtractResource, Clone, Debug, Default)]
#[extract_app(RenderApp)]
pub struct ReplayPasses {
    /// The world texture of the view (`systems/pixel_view.rs`)
    pub world: ReplayPass,
    /// The texture of the radar map that is being rendered (`systems/radar.rs`)
    pub radar: Option<ReplayPass>,
}

/// Buffers of the newest generation of bindings, the render world checks whether they are ready
#[derive(Clone, Debug, Default)]
pub struct PendingBindings {
    pub generation: u32,
    buffers: Vec<AssetId<ShaderBuffer>>,
}

impl PendingBindings {
    /// Waits for `bindings` instead, returns their generation
    pub fn wait_for(&mut self, bindings: &ReplayBindings) -> u32 {
        self.generation = self.generation.wrapping_add(1);
        self.buffers = bindings.buffer_ids();
        self.generation
    }
}

#[derive(Resource, ExtractResource, Clone, Debug, Default)]
#[extract_app(RenderApp)]
pub struct PendingReplayBindings {
    pub world: PendingBindings,
    pub radar: PendingBindings,
}

/// Newest generations of bindings that the render world has prepared, shared with it
#[derive(Resource, Clone, Debug, Default)]
pub struct ReadyReplayBindings {
    world: Arc<AtomicU32>,
    radar: Arc<AtomicU32>,
}

impl ReadyReplayBindings {
    pub fn world(&self) -> u32 {
        self.world.load(Ordering::Acquire)
    }

    pub fn radar(&self) -> u32 {
        self.radar.load(Ordering::Acquire)
    }
}

/// The world texture that the render world drew into last, shared with it. A new world texture is
/// only shown once it was drawn into.
#[derive(Resource, Clone, Debug, Default)]
pub struct RenderedWorldTexture(Arc<Mutex<Option<AssetId<Image>>>>);

impl RenderedWorldTexture {
    pub fn get(&self) -> Option<AssetId<Image>> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set(&self, target: AssetId<Image>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(target);
    }
}

/// A storage buffer with the given words, padded with zeros to at least `min_words`
///
/// WebGPU requires the buffer of a runtime-sized array to hold at least one element, so
/// `min_words` has to be the size of an element of the shader's array.
pub fn storage_buffer(
    buffers: &mut Assets<ShaderBuffer>,
    words: &[u32],
    min_words: usize,
) -> Handle<ShaderBuffer> {
    let mut words = words.to_vec();
    let min_words = min_words.max(1);
    if words.len() < min_words {
        words.resize(min_words, 0);
    }
    buffers.add(ShaderBuffer::new(words, RenderAssetUsages::RENDER_WORLD))
}

pub struct ReplayPassPlugin;

impl Plugin for ReplayPassPlugin {
    fn build(&self, app: &mut App) {
        let ready = ReadyReplayBindings::default();
        let rendered = RenderedWorldTexture::default();
        app.add_plugins((
            ExtractResourcePlugin::<ReplayPasses>::default(),
            ExtractResourcePlugin::<PendingReplayBindings>::default(),
        ))
        .init_resource::<ReplayPasses>()
        .init_resource::<PendingReplayBindings>()
        .insert_resource(ready.clone())
        .insert_resource(rendered.clone());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(ready)
            .insert_resource(rendered)
            .init_resource::<ReplayBindGroups>()
            .add_systems(RenderStartup, init_replay_pipeline)
            .add_systems(
                Render,
                (
                    prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    report_ready_bindings.in_set(RenderSystems::Cleanup),
                ),
            )
            // Before the cameras, which show the world texture
            .add_systems(
                RenderGraph,
                replay_passes
                    .in_set(RenderGraphSystems::Render)
                    .before(camera_driver),
            );
    }
}

#[derive(Resource)]
struct ReplayPipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedRenderPipelineId,
}

fn init_replay_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let storage = || storage_buffer_read_only_sized(false, None);
    let layout = BindGroupLayoutDescriptor::new(
        "replay_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<ReplayParams>(false),
                storage(),
                storage(),
                storage(),
                storage(),
                storage(),
                storage(),
            ),
        ),
    );
    let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("replay_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: asset_server.load(SHADER_PATH),
            targets: vec![Some(ColorTargetState {
                format: REPLAY_TEXTURE_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(ReplayPipeline { layout, pipeline });
}

/// What a bind group was created from
#[derive(Debug, Clone, PartialEq)]
struct BindGroupKey {
    params: ReplayParams,
    buffers: Vec<AssetId<ShaderBuffer>>,
}

#[derive(Resource, Default)]
struct ReplayBindGroups {
    world: Option<(BindGroupKey, BindGroup)>,
    radar: Option<(BindGroupKey, BindGroup)>,
}

/// The bind group of an active pass, kept while its params and buffers stay the same. `None`
/// if the pass has no bindings or their buffers are not prepared yet.
fn update_bind_group(
    cached: &mut Option<(BindGroupKey, BindGroup)>,
    pass: &ReplayPass,
    pipeline: &ReplayPipeline,
    buffers: &RenderAssets<GpuShaderBuffer>,
    device: &RenderDevice,
    queue: &RenderQueue,
    pipeline_cache: &PipelineCache,
) {
    let Some(bindings) = &pass.bindings else {
        *cached = None;
        return;
    };
    let key = BindGroupKey {
        params: ReplayParams {
            origin: pass.origin,
            bins_origin: bindings.bins_origin,
            bins_count: bindings.bins_count,
            bin_indices_offset: bindings.bin_indices_offset,
            animations_offset: bindings.animations_offset,
            clear_color: pack_rgba(pass.clear_color.to_srgba().to_u8_array_no_alpha()),
        },
        buffers: bindings.buffer_ids(),
    };
    if cached.as_ref().is_some_and(|(cached, _)| *cached == key) {
        return;
    }
    *cached = None;
    let gpu_buffers: Option<Vec<_>> = key.buffers.iter().map(|id| buffers.get(*id)).collect();
    let Some(gpu_buffers) = gpu_buffers else {
        return;
    };
    let mut params = UniformBuffer::from(key.params);
    params.write_buffer(device, queue);
    let Some(params) = params.binding() else {
        return;
    };
    let bind_group = device.create_bind_group(
        "replay_bind_group",
        &pipeline_cache.get_bind_group_layout(&pipeline.layout),
        &BindGroupEntries::sequential((
            params,
            gpu_buffers[0].buffer.as_entire_binding(),
            gpu_buffers[1].buffer.as_entire_binding(),
            gpu_buffers[2].buffer.as_entire_binding(),
            gpu_buffers[3].buffer.as_entire_binding(),
            gpu_buffers[4].buffer.as_entire_binding(),
            gpu_buffers[5].buffer.as_entire_binding(),
        )),
    );
    *cached = Some((key, bind_group));
}

fn prepare_bind_groups(
    passes: Option<Res<ReplayPasses>>,
    pipeline: Res<ReplayPipeline>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    mut bind_groups: ResMut<ReplayBindGroups>,
) {
    let Some(passes) = passes else {
        return;
    };
    let bind_groups = &mut *bind_groups;
    let update = |cached: &mut Option<(BindGroupKey, BindGroup)>, pass: &ReplayPass| {
        update_bind_group(
            cached,
            pass,
            &pipeline,
            &buffers,
            &device,
            &queue,
            &pipeline_cache,
        );
    };
    if passes.world.active {
        update(&mut bind_groups.world, &passes.world);
    }
    match &passes.radar {
        Some(radar) if radar.active => update(&mut bind_groups.radar, radar),
        Some(_) => {}
        None => bind_groups.radar = None,
    }
}

/// Draws the active passes, returns whether `pass` was drawn
fn draw_pass(
    ctx: &mut RenderContext,
    name: &'static str,
    pass: &ReplayPass,
    bind_group: Option<&(BindGroupKey, BindGroup)>,
    pipeline_cache: &PipelineCache,
    pipeline: &ReplayPipeline,
    images: &RenderAssets<GpuImage>,
) -> bool {
    let Some(target) = images.get(&pass.target) else {
        return false;
    };
    // Bindings that can't be drawn yet keep the old contents, a pass without bindings only clears
    let draw = match &pass.bindings {
        Some(_) => match (
            bind_group,
            pipeline_cache.get_render_pipeline(pipeline.pipeline),
        ) {
            (Some((_, bind_group)), Some(render_pipeline)) => Some((bind_group, render_pipeline)),
            _ => return false,
        },
        None => None,
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let color_attachments = [Some(RenderPassColorAttachment {
        view: &target.texture_view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(pass.clear_color.to_linear().into()),
            store: StoreOp::Store,
        },
    })];
    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(name),
        color_attachments: &color_attachments,
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, name);
    if let Some((bind_group, render_pipeline)) = draw {
        let (position, size) = (pass.viewport_position, pass.viewport_size);
        render_pass.set_viewport(
            position.x as f32,
            position.y as f32,
            size.x as f32,
            size.y as f32,
            0.,
            1.,
        );
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
    pass_span.end(&mut render_pass);
    true
}

fn replay_passes(
    passes: Option<Res<ReplayPasses>>,
    bind_groups: Res<ReplayBindGroups>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<ReplayPipeline>,
    images: Res<RenderAssets<GpuImage>>,
    rendered: Res<RenderedWorldTexture>,
    mut ctx: RenderContext,
) {
    let Some(passes) = passes else {
        return;
    };
    let world = &passes.world;
    if world.active
        && draw_pass(
            &mut ctx,
            "replay_world",
            world,
            bind_groups.world.as_ref(),
            &pipeline_cache,
            &pipeline,
            &images,
        )
    {
        rendered.set(world.target.id());
    }
    if let Some(radar) = passes.radar.as_ref().filter(|radar| radar.active) {
        draw_pass(
            &mut ctx,
            "replay_radar",
            radar,
            bind_groups.radar.as_ref(),
            &pipeline_cache,
            &pipeline,
            &images,
        );
    }
}

/// Reports the generation of `pending` as ready once its buffers are prepared and the pipeline is
/// compiled
fn report_ready(
    pending: &PendingBindings,
    ready: &AtomicU32,
    buffers: &RenderAssets<GpuShaderBuffer>,
    pipeline_ready: bool,
) {
    if ready.load(Ordering::Acquire) == pending.generation {
        return;
    }
    if pipeline_ready && pending.buffers.iter().all(|id| buffers.get(*id).is_some()) {
        ready.store(pending.generation, Ordering::Release);
    }
}

fn report_ready_bindings(
    pending: Option<Res<PendingReplayBindings>>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<ReplayPipeline>,
    ready: Res<ReadyReplayBindings>,
) {
    let Some(pending) = pending else {
        return;
    };
    let pipeline_ready = pipeline_cache
        .get_render_pipeline(pipeline.pipeline)
        .is_some();
    report_ready(&pending.world, &ready.world, &buffers, pipeline_ready);
    report_ready(&pending.radar, &ready.radar, &buffers, pipeline_ready);
}
