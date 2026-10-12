//! Renders maps pixel-exact like the game, on the GPU
//!
//! The game renders in software: it blits 8 bit sprites through shaded palettes into a frame
//! buffer with a 16 bit z buffer, using blitters with different depth rules, in a fixed order.
//! The final colour of a pixel only depends on the ordered list of blits that cover it. So the
//! CPU builds the game's draw list (`draw_list.rs`), sorts it into bins (`bins.rs`) and packs it
//! with the sprites and palettes into storage buffers (`atlas.rs`). The replay shader then applies
//! the draws to each pixel independently (`replay_pass.rs`, `shaders/ja2_replay.wesl`) of the
//! world texture of the view (`systems/view_replay.rs`).
//!
//! The colours are computed with 8 bits per channel instead of the game's 16 bit frame buffer.

use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::map::map_bounds,
    host::report_editor_event,
    map::{GridNo, Level, MapLight, TacticalMap},
    render::{
        atlas::{
            current_animation_frames, PackedDraws, SceneAtlas, SharedBuffers, SpriteSheets,
            DRAW_WORDS,
        },
        draw_list::{DrawCache, DrawContext, DrawListOptions, LayerVisibility},
        lighting::{light_all, relight_area, Lighting, LightingMode, PreviewLight},
        movement_costs::MovementCosts,
        world::MapWorld,
    },
    states::{
        loading::{MapAssets, MapToLoad},
        AppState,
    },
    systems::MapSystems,
    take_requested_render_options,
    tileset::{StciSheet, Tileset},
};

pub mod atlas;
pub mod bins;
pub mod draw_list;
pub mod edge_points;
pub mod items;
pub mod lighting;
pub mod movement_costs;
pub mod palette;
pub mod quantize;
pub mod radar;
pub mod reachable;
#[cfg(any(test, feature = "bench"))]
mod reference;
pub mod replay_pass;
pub mod soldiers;
#[cfg(test)]
mod tests;
pub mod world;

/// How often animated tiles advance a frame (`ANIMATETILES` timer)
const ANIMATION_INTERVAL: Duration = Duration::from_millis(200);

/// What the viewer shows, set by the host with `set_render_options`
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct RenderOptions {
    pub layers: LayerVisibility,
    /// The floor that is shown and edited: the roofs and what is on them are only drawn on the
    /// roof, and only soldiers and items on the floor can be edited
    pub floor: Level,
    pub lighting: LightingMode,
}

/// The palettes of the soldiers of the loaded map, the soldiers of the merc nodes of `MapWorld`
/// refer to them by their index
///
/// The atlas keeps the palettes it packed by their index, so a palette never changes. Soldiers
/// with the same palette share it, other palettes are added.
#[derive(Resource, Clone, Debug, Default)]
pub struct MapSoldiers {
    pub palettes: Vec<[[u8; 3]; 256]>,
}

impl MapSoldiers {
    /// The index of a palette, it is added if it is new
    pub fn palette_id(&mut self, palette: &[[u8; 3]; 256]) -> u16 {
        let index = match self.palettes.iter().position(|known| known == palette) {
            Some(index) => index,
            None => {
                self.palettes.push(*palette);
                self.palettes.len() - 1
            }
        };
        index as u16
    }
}

/// Whether the renderer of the map is done with the current scene: its bindings are prepared and
/// shown. Set by `systems/view_replay.rs`.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RendererIdle(pub bool);

impl Default for RendererIdle {
    fn default() -> Self {
        Self(true)
    }
}

/// Changes of the map that the scene has not caught up with yet
///
/// The scene is only updated while the renderer is idle (`RendererIdle`). Changes that arrive in
/// the meantime, e.g. while the mouse paints, are collected and handled together.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingSceneUpdate {
    /// Everything has to be rebuilt: a map was loaded or the render options changed
    pub full: bool,
    /// Tiles whose level nodes changed
    pub tiles: BTreeSet<GridNo>,
    /// Tiles whose lights changed, but not their level nodes, so that their movement costs stay
    /// as they are
    pub relit: BTreeSet<GridNo>,
}

impl PendingSceneUpdate {
    /// A pending rebuild of everything
    pub fn full() -> Self {
        Self {
            full: true,
            ..default()
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.tiles.is_empty() && self.relit.is_empty()
    }
}

/// A new version number for the parts of the scene, unique for the whole app, so that the
/// renderer never mistakes new data for data it already has
pub fn next_scene_version() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// The replay data of the current map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReplayScene {
    /// The sprites, palettes and animations that the draws refer to
    pub shared: SharedBuffers,
    /// Changes with every new `shared`, see `next_scene_version`
    pub shared_version: u64,
    /// All draws with bins covering `map_bounds()`, empty without a map
    pub draws: PackedDraws,
    /// Changes with every new `draws`, see `next_scene_version`
    pub draws_version: u64,
    /// Whether the last change was a full rebuild rather than an update of some tiles
    pub full_build: bool,
    /// Area covered by the draws in game pixels
    pub bounds: IRect,
}

/// What the scene was built from, kept to update it
#[derive(Resource, Clone, Debug, Default)]
pub struct SceneSources {
    pub draws: DrawCache,
    pub atlas: SceneAtlas,
    /// `None` until the scene is built
    pub lighting: Option<Lighting>,
}

/// Current frames of the animations of the scene, changes without the scene
#[derive(Resource, Clone, Debug, Default)]
pub struct ReplayAnimation {
    /// Tile database index of each animation
    pub animations: Vec<u16>,
    pub frames: Vec<u32>,
}

#[derive(Resource)]
struct AnimationTimer(Timer);

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(replay_pass::ReplayPassPlugin)
            .init_resource::<RenderOptions>()
            .init_resource::<ReplayScene>()
            .init_resource::<ReplayAnimation>()
            .init_resource::<PendingSceneUpdate>()
            .init_resource::<PreviewLight>()
            .init_resource::<SceneSources>()
            .init_resource::<RendererIdle>()
            .insert_resource(AnimationTimer(Timer::new(
                ANIMATION_INTERVAL,
                TimerMode::Repeating,
            )))
            .add_systems(
                PreUpdate,
                (
                    apply_requested_options,
                    switch_floor.run_if(in_state(AppState::Map)),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    update_scene.run_if(
                        resource_exists::<MapWorld>.and_then(resource_exists::<TacticalMap>),
                    ),
                    animate_tiles.run_if(resource_exists::<Tileset>),
                )
                    .chain()
                    .in_set(MapSystems::Scene)
                    .run_if(in_state(AppState::Map)),
            );
    }
}

fn apply_requested_options(
    mut options: ResMut<RenderOptions>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    if let Some(requested) = take_requested_render_options() {
        if *options != requested {
            // The overlays are not part of the scene
            let redraw = options.lighting != requested.lighting
                || options.floor != requested.floor
                || options.layers.drawn() != requested.layers.drawn();
            *options = requested;
            pending.full |= redraw;
        }
    }
}

/// Tells the host that Page Up or Page Down switched the floor
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "floor")]
pub struct FloorEvent {
    pub map: String,
    pub floor: Level,
}

/// Page Up switches to the roof, Page Down to the ground
fn switch_floor(
    keys: Res<ButtonInput<KeyCode>>,
    map_to_load: Res<MapToLoad>,
    mut options: ResMut<RenderOptions>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    let floor = if keys.just_pressed(KeyCode::PageUp) {
        Level::Roof
    } else if keys.just_pressed(KeyCode::PageDown) {
        Level::Ground
    } else {
        return;
    };
    if options.floor == floor {
        return;
    }
    options.floor = floor;
    pending.full = true;
    report_editor_event(&FloorEvent {
        map: map_to_load.0.clone(),
        floor,
    });
}

/// The sheets and palettes that the draws of the loaded map refer to
pub(crate) fn sprite_sheets<'a>(
    tileset: &'a Tileset,
    map_assets: &MapAssets,
    sheets: &'a Assets<StciSheet>,
    soldiers: &MapSoldiers,
) -> SpriteSheets<'a> {
    SpriteSheets {
        tileset,
        surface_sheets: map_assets
            .surface_sheets
            .iter()
            .map(|sheet| sheets.get(sheet))
            .collect(),
        soldier_sheets: map_assets
            .soldier_surfaces
            .iter()
            .map(|sheet| sheets.get(sheet))
            .collect(),
        soldier_palettes: soldiers.palettes.clone(),
    }
}

/// Above this many changed tiles, the scene is rebuilt completely instead of updating it
const MAX_INCREMENTAL_TILES: usize = 4000;

/// Brings the scene up to date with the map, once the renderer is idle
fn update_scene(
    mut pending: ResMut<PendingSceneUpdate>,
    idle: Res<RendererIdle>,
    options: Res<RenderOptions>,
    mut world: ResMut<MapWorld>,
    tileset: Res<Tileset>,
    mut costs: ResMut<MovementCosts>,
    map: Res<TacticalMap>,
    preview: Res<PreviewLight>,
    soldiers: Res<MapSoldiers>,
    map_assets: Res<MapAssets>,
    sheets: Res<Assets<StciSheet>>,
    mut sources: ResMut<SceneSources>,
    mut scene: ResMut<ReplayScene>,
    mut animation: ResMut<ReplayAnimation>,
) {
    // Updating the scene while the renderer waits for its bindings would delay it again and
    // again, e.g. while painting
    if pending.is_empty() || !idle.0 {
        return;
    }
    let update = std::mem::take(&mut *pending);
    let sprite_sheets = sprite_sheets(&tileset, &map_assets, &sheets, &soldiers);
    let mut builder = SceneBuilder {
        world: &mut world,
        map: &map,
        preview: preview.0.as_ref(),
        costs: &mut costs,
        options: *options,
        tileset: &tileset,
        sheets: &sprite_sheets,
        sources: &mut sources,
    };
    let incremental = !update.full
        && builder.sources.lighting.is_some()
        && update.tiles.len() + update.relit.len() <= MAX_INCREMENTAL_TILES;
    if incremental {
        builder.update(&update.tiles, &update.relit, &mut scene, &mut animation);
    } else {
        if !update.tiles.is_empty() {
            *builder.costs = MovementCosts::compile(builder.world, &map);
        }
        builder.rebuild(&mut scene, &mut animation);
    }
}

/// Builds and updates the scene of the loaded map
struct SceneBuilder<'a> {
    world: &'a mut MapWorld,
    map: &'a TacticalMap,
    /// The light that the scene is lit with as if it was placed
    preview: Option<&'a MapLight>,
    costs: &'a mut MovementCosts,
    options: RenderOptions,
    tileset: &'a Tileset,
    sheets: &'a SpriteSheets<'a>,
    sources: &'a mut SceneSources,
}

impl SceneBuilder<'_> {
    fn draw_options(&self, lighting: &Lighting) -> DrawListOptions {
        DrawListOptions {
            layers: self.options.layers,
            floor: self.options.floor,
            shadows: lighting.shadows,
        }
    }

    /// Packs all draws, with bins covering the map
    fn pack_draws(&mut self) -> PackedDraws {
        let sources = &mut *self.sources;
        PackedDraws::build(
            &sources.draws.all(),
            map_bounds(),
            &mut sources.atlas,
            self.sheets,
        )
    }

    /// Lights the world and builds the scene
    fn rebuild(&mut self, scene: &mut ReplayScene, animation: &mut ReplayAnimation) {
        let lighting = light_all(
            self.world,
            self.map,
            self.preview,
            self.costs,
            self.options.lighting,
        );
        let sheets = self.sheets;
        self.sources.draws = DrawCache::build(&DrawContext {
            world: self.world,
            tileset: self.tileset,
            sprites: |key| sheets.geometry(key),
            options: self.draw_options(&lighting),
        });
        self.sources.atlas = SceneAtlas::new(lighting.light_color);
        self.sources.lighting = Some(lighting);

        let draws = self.pack_draws();
        let sources = &mut *self.sources;
        let (sprites, sprite_bytes) = sources.atlas.sprite_stats();
        info!(
            "Built {} draws of {} sprites, {} KiB of sprites",
            draws.draws.len() / DRAW_WORDS,
            sprites,
            sprite_bytes / 1024
        );

        let shared = sources.atlas.shared_buffers();
        *animation = ReplayAnimation {
            frames: current_animation_frames(&shared.animations, self.tileset),
            animations: shared.animations.clone(),
        };
        *scene = ReplayScene {
            shared,
            shared_version: next_scene_version(),
            draws,
            draws_version: next_scene_version(),
            full_build: true,
            bounds: sources.draws.bounds(),
        };
    }

    /// Updates the scene after the level nodes of `tiles` changed (`MapWorld::sync_tiles`) and
    /// the lights of `relit` changed
    ///
    /// The movement costs and the lights are updated where they can have changed, then the
    /// draws of all tiles whose nodes or lights changed. If any of them changed, all draws are
    /// packed again.
    fn update(
        &mut self,
        tiles: &BTreeSet<GridNo>,
        relit: &BTreeSet<GridNo>,
        scene: &mut ReplayScene,
        animation: &mut ReplayAnimation,
    ) {
        let Some(lighting) = self.sources.lighting else {
            return;
        };
        let mut redraw = tiles.clone();
        // The movement costs only depend on the level nodes
        if !tiles.is_empty() {
            redraw.extend(self.costs.recompile_area(self.world, self.map, tiles));
        }
        redraw.extend(relit);
        let relit = relight_area(
            self.world,
            self.map,
            self.preview,
            self.costs,
            self.options.lighting,
            &redraw,
        );
        redraw.extend(relit);

        let sheets = self.sheets;
        let context = DrawContext {
            world: &*self.world,
            tileset: self.tileset,
            sprites: |key| sheets.geometry(key),
            options: self.draw_options(&lighting),
        };
        let sources = &mut *self.sources;
        let mut changed = false;
        for &grid_no in &redraw {
            changed |= sources.draws.replace(grid_no, context.tile_draws(grid_no));
        }
        if !changed {
            return;
        }

        let atlas_version = sources.atlas.version;
        scene.draws = self.pack_draws();
        scene.draws_version = next_scene_version();
        let sources = &mut *self.sources;
        if sources.atlas.version != atlas_version {
            scene.shared = sources.atlas.shared_buffers();
            scene.shared_version = next_scene_version();
            if scene.shared.animations != animation.animations {
                *animation = ReplayAnimation {
                    frames: current_animation_frames(&scene.shared.animations, self.tileset),
                    animations: scene.shared.animations.clone(),
                };
            }
        }
        scene.bounds = sources.draws.bounds();
        scene.full_build = false;
        info!("Updated {} tiles", redraw.len());
    }
}

/// Advances the animated tiles, like `RenderWorld` does on the `ANIMATETILES` timer
fn animate_tiles(
    time: Res<Time>,
    mut timer: ResMut<AnimationTimer>,
    mut tileset: ResMut<Tileset>,
    mut animation: ResMut<ReplayAnimation>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }
    tileset.advance_animations();
    let frames = current_animation_frames(&animation.animations, &tileset);
    if frames != animation.frames {
        animation.frames = frames;
    }
}
