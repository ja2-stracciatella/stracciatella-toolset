//! Replays the draw list for every pixel that the world camera shows
//!
//! The map is drawn by replaying the game's draw list (see `render/`). One quad covering
//! `map_bounds()` has the replay material of the whole map (`ReplayScene::draws`) with the
//! `VIEW_REPLAY` variant of the shader, which computes the game pixel from the world position.
//! That is too expensive to do every frame, so the world camera only renders when the view or the
//! map changed (`MapRedraw`, see `pixel_view.rs`).
//!
//! Buffers and materials are never modified: every change creates new ones. A new material
//! replaces the one of the quad once the render world has prepared it, so the camera can render
//! the old one meanwhile.

use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    components::map::{game_rect_to_world, map_bounds, world_to_game},
    render::{
        atlas::{PackedDraws, DRAW_WORDS},
        palette::pack_rgba,
        replay_material::{
            storage_buffer, PendingReplayMaterials, ReadyReplayMaterials, ReplayMaterial,
            ReplayParams,
        },
        RendererIdle, ReplayAnimation, ReplayScene,
    },
    systems::{pixel_view::MapRedraw, MapSystems},
    CLEAR_COLOR,
};

/// The buffers of the `PackedDraws` of a scene
#[derive(Debug, Clone)]
struct SceneBuffers {
    draws: Handle<ShaderBuffer>,
    bins: Handle<ShaderBuffer>,
    bins_origin: IVec2,
    bins_count: UVec2,
    bin_indices_offset: u32,
}

impl SceneBuffers {
    fn new(scene: &PackedDraws, buffers: &mut Assets<ShaderBuffer>) -> Self {
        Self {
            draws: storage_buffer(buffers, &scene.draws, DRAW_WORDS),
            bins: storage_buffer(buffers, &scene.bins, 1),
            bins_origin: scene.bins_origin,
            bins_count: scene.bins_count,
            bin_indices_offset: scene.bin_indices_offset,
        }
    }
}

#[derive(Debug, Clone)]
struct SharedBuffers {
    version: u64,
    atlas: Handle<ShaderBuffer>,
    palettes: Handle<ShaderBuffer>,
    sprites: Handle<ShaderBuffer>,
    animations_offset: u32,
}

/// The buffers of the sprites, palettes and animations of a scene
#[derive(Debug, Default)]
struct SharedReplayBuffers {
    shared: Option<SharedBuffers>,
    animation_frames: Option<Handle<ShaderBuffer>>,
}

/// Which buffers `SharedReplayBuffers::update` replaced
#[derive(Debug, Clone, Copy)]
struct SharedChanges {
    shared: bool,
    frames: bool,
}

impl SharedReplayBuffers {
    /// Creates new buffers for what changed in the scene and the animation
    fn update(
        &mut self,
        scene: &ReplayScene,
        animation: &ReplayAnimation,
        animation_changed: bool,
        buffers: &mut Assets<ShaderBuffer>,
    ) -> SharedChanges {
        let shared_changed = self
            .shared
            .as_ref()
            .is_none_or(|shared| shared.version != scene.shared_version);
        if shared_changed {
            warn_about_draws_outside(scene.bounds);
            let shared = &scene.shared;
            self.shared = Some(SharedBuffers {
                version: scene.shared_version,
                atlas: storage_buffer(buffers, &shared.atlas, 1),
                palettes: storage_buffer(buffers, &shared.palettes, 1),
                sprites: storage_buffer(buffers, &shared.sprites, 1),
                animations_offset: shared.animations_offset,
            });
        }
        let frames_changed = animation_changed || self.animation_frames.is_none();
        if frames_changed {
            self.animation_frames = Some(storage_buffer(buffers, &animation.frames, 1));
        }
        SharedChanges {
            shared: shared_changed,
            frames: frames_changed,
        }
    }

    /// The material drawn by the world camera, `origin` is the game pixel at the world origin.
    /// `None` before the first update.
    fn material(&self, origin: IVec2, scene: &SceneBuffers) -> Option<ReplayMaterial> {
        let (Some(shared), Some(animation_frames)) = (&self.shared, &self.animation_frames) else {
            return None;
        };
        Some(ReplayMaterial {
            params: ReplayParams {
                origin,
                bins_origin: scene.bins_origin,
                bins_count: scene.bins_count,
                bin_indices_offset: scene.bin_indices_offset,
                animations_offset: shared.animations_offset,
                clear_color: pack_rgba(CLEAR_COLOR.to_srgba().to_u8_array_no_alpha()),
            },
            draws: scene.draws.clone(),
            bins: scene.bins.clone(),
            atlas: shared.atlas.clone(),
            palettes: shared.palettes.clone(),
            sprites: shared.sprites.clone(),
            animation_frames: animation_frames.clone(),
            view: true,
        })
    }
}

/// State of the quad and its material
#[derive(Resource, Debug, Default)]
struct ViewReplay {
    quad: Option<Entity>,
    buffers: SharedReplayBuffers,
    /// Version of the draws whose buffers were uploaded
    version: Option<u64>,
    draws: Option<SceneBuffers>,
    /// The material of the quad
    material: Option<Handle<ReplayMaterial>>,
    /// The material that replaces it once it is prepared, with its generation
    next: Option<(Handle<ReplayMaterial>, u32)>,
}

pub struct ViewReplayPlugin;

impl Plugin for ViewReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewReplay>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (update_material, swap_material)
                    .chain()
                    .in_set(MapSystems::View),
            );
    }
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut replay: ResMut<ViewReplay>) {
    let bounds = map_bounds();
    let rect = game_rect_to_world(bounds.min, bounds.max);
    // Seen by the world camera on the default render layer 0, the material is added once there
    // is a scene
    let quad = commands
        .spawn((
            Mesh2d(meshes.add(Rectangle::from_size(rect.size()))),
            Transform::from_translation(rect.center().extend(0.)),
        ))
        .id();
    replay.quad = Some(quad);
}

/// Creates new buffers and a new material for what changed
fn update_material(
    scene: Option<Res<ReplayScene>>,
    animation: Option<Res<ReplayAnimation>>,
    mut replay: ResMut<ViewReplay>,
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
    let replay = &mut *replay;
    let changes = replay
        .buffers
        .update(&scene, &animation, animation_changed, &mut buffers);
    let draws_changed = replay.version != Some(scene.draws_version);
    if draws_changed {
        replay.version = Some(scene.draws_version);
        replay.draws = Some(SceneBuffers::new(&scene.draws, &mut buffers));
    }
    if !draws_changed && !changes.shared && !changes.frames {
        return;
    }
    let Some(draws) = &replay.draws else {
        return;
    };
    // The game pixel at the world origin
    let origin = world_to_game(Vec2::ZERO).as_ivec2();
    let Some(material) = replay.buffers.material(origin, draws) else {
        return;
    };
    let material = materials.add(material);
    pending.generation = pending.generation.wrapping_add(1);
    pending.materials = vec![material.id()];
    replay.next = Some((material, pending.generation));
}

/// Puts the new material on the quad once it is prepared
///
/// The first material goes on the quad right away, so that its pipeline is compiled while the
/// render world prepares it.
fn swap_material(
    mut commands: Commands,
    mut replay: ResMut<ViewReplay>,
    ready: Res<ReadyReplayMaterials>,
    mut redraw: ResMut<MapRedraw>,
    mut idle: ResMut<RendererIdle>,
) {
    let replay = &mut *replay;
    let Some(quad) = replay.quad else {
        return;
    };
    if let Some((material, generation)) = &replay.next {
        let first = replay.material.is_none();
        let prepared = ready.generation() == *generation;
        let on_quad = replay.material.as_ref() == Some(material);
        if (first || prepared) && !on_quad {
            commands
                .entity(quad)
                .insert(MeshMaterial2d(material.clone()));
            replay.material = Some(material.clone());
            redraw.changed = true;
        }
        if prepared {
            replay.next = None;
        }
    }
    idle.set_if_neq(RendererIdle(replay.next.is_none()));
}

/// Warns about draws outside of the drawn area, they would be cut off
fn warn_about_draws_outside(draw_bounds: IRect) {
    let bounds = map_bounds();
    if !draw_bounds.is_empty() && bounds.union(draw_bounds) != bounds {
        warn!(
            "The map reaches outside of the drawn area and is cut off: {:?} is not within {:?}",
            draw_bounds, bounds
        );
    }
}
