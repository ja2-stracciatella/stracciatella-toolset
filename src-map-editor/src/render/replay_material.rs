//! The material that replays the draw list for every pixel of a region, see
//! `shaders/ja2_replay.wesl`
//!
//! The region quads are only drawn by the bake cameras, which must not run before the material
//! is ready on the GPU, or they would clear the region. Buffers and materials are therefore never
//! modified: every change creates new ones, and the render world reports when all materials of
//! the newest generation are prepared and no pipeline is being compiled.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use bevy::{
    prelude::*,
    render::{
        erased_render_asset::ErasedRenderAssets,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{AsBindGroup, PipelineCache, ShaderType},
        storage::ShaderBuffer,
        Render, RenderApp, RenderSystems,
    },
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin, PreparedMaterial2d},
};

/// Path of the embedded replay shader, see `embed_shaders` in `lib.rs`
const SHADER_PATH: &str = "embedded://ja2_map_editor/shaders/ja2_replay.wesl";

/// Uniform values of the replay shader (`ReplayParams`)
#[derive(ShaderType, Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplayParams {
    pub region_origin: IVec2,
    pub bins_origin: IVec2,
    pub bins_count: UVec2,
    pub bin_indices_offset: u32,
    pub animations_offset: u32,
    pub clear_color: u32,
}

/// See `render/atlas.rs` for the contents of the buffers
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
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
}

impl Material2d for ReplayMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
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

/// The material of the radar map render (`systems/radar.rs`), checked like the region materials
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
