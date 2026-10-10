//! Renders the map into region textures
//!
//! The map is drawn by replaying the game's draw list for every pixel (see `render/`), which is
//! too expensive to do every frame. Instead, one bake camera per region renders a quad with the
//! replay material into the region's texture when the draws of the region changed. The world
//! camera then draws the regions, a handful of sprites.
//!
//! The regions have an even size and integer centers, so their texels land exactly on world
//! pixels and the baked map stays pixel-perfect. Each region has its own draws and bins, the
//! sprites, palettes and animations are shared (`ReplayScene`).

use bevy::{
    asset::RenderAssetUsages,
    camera::{visibility::RenderLayers, RenderTarget},
    image::ImageSampler,
    prelude::*,
    render::{render_resource::TextureFormat, storage::ShaderBuffer},
};

use crate::{
    components::map::game_rect_to_world,
    render::{
        atlas::{RegionScene, DRAW_WORDS},
        palette::pack_rgba,
        regions::{map_bounds, RegionLayout},
        replay_material::{
            PendingReplayMaterials, ReadyReplayMaterials, ReplayMaterial, ReplayParams,
        },
        ReplayAnimation, ReplayScene,
    },
    systems::MapSystems,
    CLEAR_COLOR,
};

/// Render layer of the region quads, only seen by the bake cameras
pub const BAKE_LAYER: usize = 2;

/// How many frames the bake cameras render once the materials are ready. More than one, in case
/// a pipeline only becomes ready a frame later.
const BAKE_FRAMES: u32 = 2;

/// A region texture and the part of the map it shows
#[derive(Debug, Clone)]
struct Region {
    /// Game pixel of the top left texel
    game_origin: IVec2,
    quad: Entity,
    /// Version of the region scene whose buffers were uploaded, `None` before the first upload
    version: Option<u64>,
    buffers: Option<RegionBuffers>,
    /// Current material, `None` before the first scene
    material: Option<Handle<ReplayMaterial>>,
    /// The render world has not yet reported the material as prepared
    unconfirmed: bool,
    /// Needs to be rendered again
    dirty: bool,
    /// The bake camera currently renders it
    baking: bool,
}

#[derive(Debug, Clone)]
struct RegionBuffers {
    draws: Handle<ShaderBuffer>,
    bins: Handle<ShaderBuffer>,
    bins_origin: IVec2,
    bins_count: UVec2,
    bin_indices_offset: u32,
    animated: bool,
}

#[derive(Debug, Clone)]
struct SharedBuffers {
    version: u64,
    atlas: Handle<ShaderBuffer>,
    palettes: Handle<ShaderBuffer>,
    sprites: Handle<ShaderBuffer>,
    animations_offset: u32,
}

/// State of the bake
#[derive(Resource, Debug, Default)]
pub struct MapBake {
    regions: Vec<Region>,
    frames_left: u32,
    /// Generation of materials that the dirty regions wait for
    waiting_for: Option<u32>,
    shared: Option<SharedBuffers>,
    animation_frames: Option<Handle<ShaderBuffer>>,
}

impl MapBake {
    /// Whether nothing waits to be baked or is being baked
    pub fn is_idle(&self) -> bool {
        self.waiting_for.is_none() && self.frames_left == 0
    }
}

/// Camera rendering the quad of one region into its texture
#[derive(Component)]
struct BakeCamera(usize);

pub struct BakePlugin;

impl Plugin for BakePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapBake>()
            .add_systems(Startup, setup)
            // Run in every state, so the cameras are also switched off while the next map loads
            .add_systems(
                Update,
                (update_replay_materials, update_bake_cameras)
                    .chain()
                    .in_set(MapSystems::View),
            );
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bake: ResMut<MapBake>,
) {
    let layout = RegionLayout::default();
    let mesh = meshes.add(Rectangle::from_size(layout.size.as_vec2()));
    for (index, game_origin) in layout.origins().enumerate() {
        let mut image = Image::new_target_texture(
            layout.size.x,
            layout.size.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        );
        // Regions are drawn at exactly 1 texel = 1 world pixel
        image.sampler = ImageSampler::nearest();
        image.data = None;
        image.copy_on_resize = false;
        let image = images.add(image);

        let world_rect = game_rect_to_world(game_origin, game_origin + layout.size.as_ivec2());
        let center = world_rect.center();
        commands.spawn((
            Camera2d,
            BakeCamera(index),
            Camera {
                // Before the world camera, so that it sees the new bake in the same frame
                order: -2,
                is_active: false,
                clear_color: ClearColorConfig::Custom(CLEAR_COLOR),
                ..default()
            },
            RenderTarget::Image(image.clone().into()),
            Msaa::Off,
            RenderLayers::layer(BAKE_LAYER),
            Transform::from_translation(center.extend(0.)),
        ));
        // Seen by the world camera on the default render layer 0
        commands.spawn((
            Sprite::from_image(image),
            Transform::from_translation(center.extend(0.)),
        ));
        // The material is added once there is a scene
        let quad = commands
            .spawn((
                Mesh2d(mesh.clone()),
                RenderLayers::layer(BAKE_LAYER),
                Transform::from_translation(center.extend(0.)),
            ))
            .id();
        bake.regions.push(Region {
            game_origin,
            quad,
            version: None,
            buffers: None,
            material: None,
            unconfirmed: false,
            dirty: false,
            baking: false,
        });
    }
}

/// A storage buffer with the given words, padded with zeros to at least `min_words`
///
/// WebGPU requires the buffer of a runtime-sized array to hold at least one element, so
/// `min_words` has to be the size of an element of the shader's array.
pub(crate) fn storage_buffer(
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

/// Creates new buffers and materials for the parts of the scene that changed
///
/// Regions whose draws changed are baked again. New shared data (sprites, palettes, animation
/// frames) only replaces the materials, draws that were baked before look the same with it. Only
/// the animated regions are baked again when the animation frames change.
fn update_replay_materials(
    mut commands: Commands,
    scene: Option<Res<ReplayScene>>,
    animation: Option<Res<ReplayAnimation>>,
    mut bake: ResMut<MapBake>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut materials: ResMut<Assets<ReplayMaterial>>,
    mut pending: ResMut<PendingReplayMaterials>,
) {
    let (Some(scene), Some(animation)) = (scene, animation) else {
        return;
    };
    let animation_changed = animation.is_changed();
    if !scene.is_changed() && !animation_changed {
        return;
    }
    let bake = &mut *bake;

    let shared_changed = bake
        .shared
        .as_ref()
        .is_none_or(|shared| shared.version != scene.shared_version);
    if shared_changed {
        warn_about_draws_outside(scene.bounds);
        let shared = &scene.shared;
        bake.shared = Some(SharedBuffers {
            version: scene.shared_version,
            atlas: storage_buffer(&mut buffers, &shared.atlas, 1),
            palettes: storage_buffer(&mut buffers, &shared.palettes, 1),
            sprites: storage_buffer(&mut buffers, &shared.sprites, 1),
            animations_offset: shared.animations_offset,
        });
    }
    let frames_changed = animation_changed || bake.animation_frames.is_none();
    if frames_changed {
        bake.animation_frames = Some(storage_buffer(&mut buffers, &animation.frames, 1));
    }
    let (Some(shared), Some(animation_frames)) = (&bake.shared, &bake.animation_frames) else {
        return;
    };

    let clear_color = pack_rgba(CLEAR_COLOR.to_srgba().to_u8_array_no_alpha());
    let empty = RegionScene::default();
    let mut new_materials = false;
    for (index, region) in bake.regions.iter_mut().enumerate() {
        // Without a map, there are no regions in the scene
        let (version, region_scene) = scene
            .regions
            .get(index)
            .map_or((0, &empty), |r| (r.version, &r.scene));
        let region_changed = region.version != Some(version);
        if region_changed {
            region.version = Some(version);
            region.buffers = Some(RegionBuffers {
                draws: storage_buffer(&mut buffers, &region_scene.draws, DRAW_WORDS),
                bins: storage_buffer(&mut buffers, &region_scene.bins, 1),
                bins_origin: region_scene.bins_origin,
                bins_count: region_scene.bins_count,
                bin_indices_offset: region_scene.bin_indices_offset,
                animated: region_scene.animated,
            });
            region.dirty = true;
        }
        let Some(region_buffers) = &region.buffers else {
            continue;
        };
        if animation_changed && region_buffers.animated {
            region.dirty = true;
        }
        if !region_changed && !shared_changed && !frames_changed {
            continue;
        }
        let material = materials.add(ReplayMaterial {
            params: ReplayParams {
                region_origin: region.game_origin,
                bins_origin: region_buffers.bins_origin,
                bins_count: region_buffers.bins_count,
                bin_indices_offset: region_buffers.bin_indices_offset,
                animations_offset: shared.animations_offset,
                clear_color,
            },
            draws: region_buffers.draws.clone(),
            bins: region_buffers.bins.clone(),
            atlas: shared.atlas.clone(),
            palettes: shared.palettes.clone(),
            sprites: shared.sprites.clone(),
            animation_frames: animation_frames.clone(),
        });
        commands
            .entity(region.quad)
            .insert(MeshMaterial2d(material.clone()));
        region.material = Some(material);
        region.unconfirmed = true;
        new_materials = true;
    }
    if !new_materials {
        return;
    }

    // Wait for all materials that the render world has not reported yet, including the ones of
    // a generation that was replaced before it was ready
    pending.generation = pending.generation.wrapping_add(1);
    pending.materials = bake
        .regions
        .iter()
        .filter(|region| region.unconfirmed)
        .filter_map(|region| region.material.as_ref().map(|material| material.id()))
        .collect();
    bake.waiting_for = Some(pending.generation);
}

fn update_bake_cameras(
    mut bake: ResMut<MapBake>,
    ready: Res<ReadyReplayMaterials>,
    mut cameras: Query<(&mut Camera, &BakeCamera)>,
) {
    let bake = &mut *bake;
    if bake.frames_left > 0 {
        bake.frames_left -= 1;
        if bake.frames_left == 0 {
            for region in &mut bake.regions {
                region.baking = false;
            }
        }
    }
    // Start baking once the materials are ready, a running bake is finished first
    if let Some(generation) = bake.waiting_for {
        if bake.frames_left == 0 && ready.generation() == generation {
            bake.waiting_for = None;
            let mut baked = 0;
            for region in &mut bake.regions {
                region.unconfirmed = false;
                region.baking = region.dirty;
                region.dirty = false;
                baked += usize::from(region.baking);
            }
            if baked > 0 {
                bake.frames_left = BAKE_FRAMES;
                debug!("Baking {} regions", baked);
            }
        }
    }
    for (mut camera, BakeCamera(index)) in &mut cameras {
        let active = bake.frames_left > 0 && bake.regions[*index].baking;
        if camera.is_active != active {
            camera.is_active = active;
        }
    }
}

/// Warns about draws outside of the baked area, they would be cut off
fn warn_about_draws_outside(draw_bounds: IRect) {
    let bounds = map_bounds();
    if !draw_bounds.is_empty() && bounds.union(draw_bounds) != bounds {
        warn!(
            "The map reaches outside of the baked area and is cut off: {:?} is not within {:?}",
            draw_bounds, bounds
        );
    }
}
