//! The material that replays the draw list for every pixel, see `shaders/ja2_replay.wesl`
//!
//! A material must not be drawn before it is ready on the GPU, or the map would vanish while its
//! buffers are uploaded. Buffers and materials are therefore never modified: every change creates
//! new ones, and the render world reports when all materials of the newest generation are
//! prepared and no pipeline is being compiled. Until then, the quad of the map keeps the old
//! material (`systems/view_replay.rs`).

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use bevy::{
    asset::RenderAssetUsages,
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    render::{
        erased_render_asset::ErasedRenderAssets,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            AsBindGroup, PipelineCache, RenderPipelineDescriptor, ShaderType,
            SpecializedMeshPipelineError,
        },
        storage::ShaderBuffer,
        Render, RenderApp, RenderSystems,
    },
    shader::{ShaderDefVal, ShaderRef},
    sprite_render::{
        Material2d, Material2dKey, Material2dPipeline, Material2dPlugin, PreparedMaterial2d,
    },
};

/// Path of the embedded replay shader, see `embed_shaders` in `lib.rs`
const SHADER_PATH: &str = "embedded://ja2_map_editor/shaders/ja2_replay.wesl";

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

/// Variant of the replay shader
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReplayMaterialKey {
    view: bool,
}

impl From<&ReplayMaterial> for ReplayMaterialKey {
    fn from(material: &ReplayMaterial) -> Self {
        Self {
            view: material.view,
        }
    }
}

/// See `render/atlas.rs` for the contents of the buffers
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ReplayMaterialKey)]
pub struct ReplayMaterial {
    #[uniform(0)]
    pub params: ReplayParams,
    #[storage(1, read_only)]
    pub draws: Handle<ShaderBuffer>,
    #[storage(2, read_only)]
    pub bins: Handle<ShaderBuffer>,
    #[storage(3, read_only)]
    pub atlas: Handle<ShaderBuffer>,
    #[storage(4, read_only)]
    pub palettes: Handle<ShaderBuffer>,
    #[storage(5, read_only)]
    pub sprites: Handle<ShaderBuffer>,
    #[storage(6, read_only)]
    pub animation_frames: Handle<ShaderBuffer>,
    /// Drawn by the world camera instead of into a texture: the game pixel comes from the world
    /// position, and `params.origin` is the game pixel at the world origin. Otherwise
    /// `params.origin` is the game pixel of the top left texel.
    pub view: bool,
}

impl Material2d for ReplayMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.push(ShaderDefVal::Bool(
                "VIEW_REPLAY".into(),
                key.bind_group_data.view,
            ));
        }
        Ok(())
    }
}

/// The materials of the newest generation, the render world checks whether they are ready
#[derive(Resource, ExtractResource, Clone, Debug, Default)]
#[extract_app(RenderApp)]
pub struct PendingReplayMaterials {
    pub generation: u32,
    pub materials: Vec<AssetId<ReplayMaterial>>,
}

/// Newest generation of materials that the render world has prepared, shared with it
#[derive(Resource, Clone, Debug, Default)]
pub struct ReadyReplayMaterials(Arc<AtomicU32>);

impl ReadyReplayMaterials {
    pub fn generation(&self) -> u32 {
        self.0.load(Ordering::Acquire)
    }
}

/// The material of the radar map render (`systems/radar.rs`), checked like the map materials
#[derive(Resource, ExtractResource, Clone, Debug, Default)]
#[extract_app(RenderApp)]
pub struct PendingRadarMaterial {
    pub generation: u32,
    pub material: Option<AssetId<ReplayMaterial>>,
}

/// Newest generation of the radar material that the render world has prepared
#[derive(Resource, Clone, Debug, Default)]
pub struct ReadyRadarMaterial(Arc<AtomicU32>);

impl ReadyRadarMaterial {
    pub fn generation(&self) -> u32 {
        self.0.load(Ordering::Acquire)
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

pub struct ReplayMaterialPlugin;

impl Plugin for ReplayMaterialPlugin {
    fn build(&self, app: &mut App) {
        let ready = ReadyReplayMaterials::default();
        let ready_radar = ReadyRadarMaterial::default();
        app.add_plugins((
            Material2dPlugin::<ReplayMaterial>::default(),
            ExtractResourcePlugin::<PendingReplayMaterials>::default(),
            ExtractResourcePlugin::<PendingRadarMaterial>::default(),
        ))
        .init_resource::<PendingReplayMaterials>()
        .init_resource::<PendingRadarMaterial>()
        .insert_resource(ready.clone())
        .insert_resource(ready_radar.clone());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(ready)
            .insert_resource(ready_radar)
            .add_systems(
                Render,
                (report_ready_materials, report_ready_radar_material)
                    .in_set(RenderSystems::Cleanup),
            );
    }
}

/// Whether the materials are prepared and no pipeline is being compiled
fn materials_ready<'a>(
    mut materials: impl Iterator<Item = &'a AssetId<ReplayMaterial>>,
    prepared: &ErasedRenderAssets<PreparedMaterial2d>,
    pipeline_cache: &PipelineCache,
) -> bool {
    materials.all(|id| prepared.get(*id).is_some())
        && pipeline_cache.waiting_pipelines().next().is_none()
}

fn report_ready_materials(
    pending: Option<Res<PendingReplayMaterials>>,
    prepared: Res<ErasedRenderAssets<PreparedMaterial2d>>,
    pipeline_cache: Res<PipelineCache>,
    ready: Res<ReadyReplayMaterials>,
) {
    let Some(pending) = pending else {
        return;
    };
    if ready.generation() == pending.generation {
        return;
    }
    if materials_ready(pending.materials.iter(), &prepared, &pipeline_cache) {
        ready.0.store(pending.generation, Ordering::Release);
    }
}

fn report_ready_radar_material(
    pending: Option<Res<PendingRadarMaterial>>,
    prepared: Res<ErasedRenderAssets<PreparedMaterial2d>>,
    pipeline_cache: Res<PipelineCache>,
    ready: Res<ReadyRadarMaterial>,
) {
    let Some(pending) = pending else {
        return;
    };
    if ready.generation() == pending.generation || pending.material.is_none() {
        return;
    }
    if materials_ready(pending.material.iter(), &prepared, &pipeline_cache) {
        ready.0.store(pending.generation, Ordering::Release);
    }
}
