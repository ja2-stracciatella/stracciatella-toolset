//! Renders maps pixel-exact like the game, on the GPU
//!
//! The game renders in software: it blits 8 bit sprites through shaded palettes into a frame
//! buffer with a 16 bit z buffer, using blitters with different depth rules, in a fixed order.
//! The final colour of a pixel only depends on the ordered list of blits that cover it. So the
//! CPU builds the game's draw list (`draw_list.rs`), sorts it into bins (`bins.rs`) and packs it
//! with the sprites and palettes into storage buffers (`atlas.rs`). The replay shader then applies
//! the draws to each pixel independently (`replay_material.rs`, `shaders/ja2_replay.wesl`). The
//! result is baked into region textures (`systems/bake.rs`).
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
    host::report_editor_event,
    map::{GridNo, Level, MapLight, TacticalMap},
    render::{
        atlas::{current_animation_frames, RegionScene, SceneAtlas, SharedBuffers, SpriteSheets},
        draw_list::{Draw, DrawCache, DrawContext, DrawListOptions, LayerVisibility},
        lighting::{light_all, relight_area, Lighting, LightingMode, PreviewLight},
        movement_costs::MovementCosts,
        regions::RegionLayout,
        world::MapWorld,
    },
    states::{
        loading::{MapAssets, MapToLoad},
        AppState,
    },
    systems::{bake::MapBake, MapSystems},
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
#[cfg(test)]
mod reference;
pub mod regions;
pub mod replay_material;
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

/// Changes of the map that the scene has not caught up with yet
///
/// The scene is only updated while the bake is idle. Changes that arrive in the meantime, e.g.
/// while the mouse paints, are collected and handled together.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingSceneUpdate {
    /// Everything has to be rebuilt: a map was loaded or the render options changed
    pub full: bool,
    /// Tiles whose level nodes changed
    pub tiles: BTreeSet<GridNo>,
}

impl PendingSceneUpdate {
    /// A pending rebuild of everything
    pub fn full() -> Self {
        Self {
            full: true,
            tiles: BTreeSet::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.tiles.is_empty()
    }
}

/// A new version number for the parts of the scene, unique for the whole app, so that the bake
/// never mistakes new data for data it already has
pub fn next_scene_version() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// The data of one region of the scene
#[derive(Clone, Debug, Default)]
pub struct ReplayRegion {
    pub scene: RegionScene,
    /// Changes with every new `scene`, see `next_scene_version`
    pub version: u64,
}

/// The replay data of the current map, see `RegionLayout` for the regions
#[derive(Resource, Clone, Debug, Default)]
pub struct ReplayScene {
    /// The sprites, palettes and animations of all regions
    pub shared: SharedBuffers,
    /// Changes with every new `shared`, see `next_scene_version`
    pub shared_version: u64,
    /// One entry per region of `RegionLayout::default()`, empty without a map
    pub regions: Vec<ReplayRegion>,
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
    pub layout: RegionLayout,
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
        app.add_plugins(replay_material::ReplayMaterialPlugin)
            .init_resource::<RenderOptions>()
            .init_resource::<ReplayScene>()
            .init_resource::<ReplayAnimation>()
            .init_resource::<PendingSceneUpdate>()
            .init_resource::<PreviewLight>()
            .init_resource::<SceneSources>()
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

/// Brings the scene up to date with the map, once the bake is idle
fn update_scene(
    mut pending: ResMut<PendingSceneUpdate>,
    bake: Res<MapBake>,
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
    // Updating the scene while a bake waits for its materials would delay the bake again and
    // again, e.g. while painting
    if pending.is_empty() || !bake.is_idle() {
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
        && update.tiles.len() <= MAX_INCREMENTAL_TILES;
    if incremental {
        builder.update(&update.tiles, &mut scene, &mut animation);
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

    /// Lights the world and builds every region
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

        // Every draw goes to the regions it overlaps, in draw list order
        let sources = &mut *self.sources;
        let layout = &sources.layout;
        let draws = sources.draws.all();
        let mut region_draws: Vec<Vec<Draw>> = vec![Vec::new(); layout.len()];
        for draw in &draws {
            for region in layout.overlapping(draw.bounds) {
                region_draws[region].push(draw.clone());
            }
        }
        let regions = region_draws
            .iter()
            .enumerate()
            .map(|(index, draws)| ReplayRegion {
                scene: RegionScene::build(draws, layout.rect(index), &mut sources.atlas, sheets),
                version: next_scene_version(),
            })
            .collect();
        let (sprites, sprite_bytes) = sources.atlas.sprite_stats();
        info!(
            "Built {} draws of {} sprites, {} KiB of sprites",
            draws.len(),
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
            regions,
            bounds: sources.draws.bounds(),
        };
    }

    /// Updates the scene after the level nodes of `tiles` changed (`MapWorld::sync_tiles`)
    ///
    /// The movement costs and the lights are updated where they can have changed, then the
    /// draws of all tiles whose nodes or lights changed. Only the regions that the old or new
    /// draws of these tiles overlap are built again.
    fn update(
        &mut self,
        tiles: &BTreeSet<GridNo>,
        scene: &mut ReplayScene,
        animation: &mut ReplayAnimation,
    ) {
        let Some(lighting) = self.sources.lighting else {
            return;
        };
        let changed_costs = self.costs.recompile_area(self.world, self.map, tiles);
        let mut redraw = tiles.clone();
        redraw.extend(changed_costs);
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
        let mut dirty_regions = BTreeSet::new();
        for &grid_no in &redraw {
            if let Some(changed) = sources.draws.replace(grid_no, context.tile_draws(grid_no)) {
                dirty_regions.extend(sources.layout.overlapping(changed));
            }
        }
        if dirty_regions.is_empty() {
            return;
        }

        let atlas_version = sources.atlas.version;
        for &index in &dirty_regions {
            let rect = sources.layout.rect(index);
            let draws = sources.draws.ordered(rect);
            if let Some(region) = scene.regions.get_mut(index) {
                *region = ReplayRegion {
                    scene: RegionScene::build(&draws, rect, &mut sources.atlas, sheets),
                    version: next_scene_version(),
                };
            }
        }
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
        info!(
            "Updated {} tiles and {} of {} regions",
            redraw.len(),
            dirty_regions.len(),
            sources.layout.len()
        );
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
