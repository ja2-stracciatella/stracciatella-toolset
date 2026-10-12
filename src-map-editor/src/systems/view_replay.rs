//! Replays the draw list for every texel of the world texture
//!
//! The map is drawn by replaying the game's draw list (see `render/`). The world pass
//! (`ReplayPasses::world`) has the bindings of the whole map (`ReplayScene::draws`), the view
//! sets where it draws (`pixel_view.rs`). That is too expensive to do every frame, so the pass is
//! only drawn when the view or the map changed (`MapRedraw`).
//!
//! Buffers are never modified: every change creates new ones. New bindings replace the ones of
//! the pass once the render world has prepared them, so the pass can draw the old ones meanwhile.

use bevy::{prelude::*, render::storage::ShaderBuffer};

use crate::{
    components::map::map_bounds,
    render::{
        atlas::{PackedDraws, DRAW_WORDS},
        replay_pass::{
            storage_buffer, PendingReplayBindings, ReadyReplayBindings, ReplayBindings,
            ReplayPasses,
        },
        RendererIdle, ReplayAnimation, ReplayScene,
    },
    systems::{pixel_view::MapRedraw, MapSystems},
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

    /// The bindings of the world pass, `None` before the first update
    fn bindings(&self, scene: &SceneBuffers) -> Option<ReplayBindings> {
        let (Some(shared), Some(animation_frames)) = (&self.shared, &self.animation_frames) else {
            return None;
        };
        Some(ReplayBindings {
            bins_origin: scene.bins_origin,
            bins_count: scene.bins_count,
            bin_indices_offset: scene.bin_indices_offset,
            animations_offset: shared.animations_offset,
            draws: scene.draws.clone(),
            bins: scene.bins.clone(),
            atlas: shared.atlas.clone(),
            palettes: shared.palettes.clone(),
            sprites: shared.sprites.clone(),
            animation_frames: animation_frames.clone(),
        })
    }
}

/// State of the buffers and bindings of the world pass
#[derive(Resource, Debug, Default)]
struct ViewReplay {
    buffers: SharedReplayBuffers,
    /// Version of the draws whose buffers were uploaded
    version: Option<u64>,
    draws: Option<SceneBuffers>,
    /// The bindings that replace the ones of the pass once they are prepared, with their
    /// generation
    next: Option<(ReplayBindings, u32)>,
}

pub struct ViewReplayPlugin;

impl Plugin for ViewReplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewReplay>().add_systems(
            Update,
            (update_bindings, swap_bindings)
                .chain()
                .in_set(MapSystems::View),
        );
    }
}

/// Creates new buffers and new bindings for what changed
fn update_bindings(
    scene: Option<Res<ReplayScene>>,
    animation: Option<Res<ReplayAnimation>>,
    mut replay: ResMut<ViewReplay>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut pending: ResMut<PendingReplayBindings>,
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
    let Some(bindings) = replay.buffers.bindings(draws) else {
        return;
    };
    let generation = pending.world.wait_for(&bindings);
    replay.next = Some((bindings, generation));
}

/// Puts the new bindings into the world pass once they are prepared
fn swap_bindings(
    mut replay: ResMut<ViewReplay>,
    mut passes: ResMut<ReplayPasses>,
    ready: Res<ReadyReplayBindings>,
    mut redraw: ResMut<MapRedraw>,
    mut idle: ResMut<RendererIdle>,
) {
    let prepared = replay
        .next
        .as_ref()
        .is_some_and(|(_, generation)| ready.world() == *generation);
    if prepared {
        passes.world.bindings = replay.next.take().map(|(bindings, _)| bindings);
        redraw.changed = true;
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
