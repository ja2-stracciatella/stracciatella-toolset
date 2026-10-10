//! The editing mode chosen in the host, and applying it to the map

use std::sync::Arc;

use bevy::input::mouse::MouseButton;
use serde::Deserialize;

use crate::{
    map::{EditorSmoothing, GridNo, Level},
    random::Random,
    render::RenderOptions,
    tileset::{Tile, TileIndex, Tileset, NUM_ROAD_MACROS},
};

use super::{
    brush::{density_test, Brush, BrushArea},
    building_mode::{BuildingMode, BuildingTool},
    cliffs::{erase_cliffs, place_cliff, raise_world_land},
    document::EditContext,
    ground::{
        choose_weighted_texture, erase_ground, paste_texture_common, terrain_fill, GroundSelection,
    },
    items::{ItemMode, ItemTool},
    lights::{LightMode, LightTool},
    placement::{erase_debris, erase_structures, paste_tile, SelectionList},
    roads::{erase_roads, place_road_macro},
    soldiers::{SoldierMode, SoldierTool},
    traversal::TraversalMode,
};

/// The mode, set by the host with `set_editor_mode`
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum EditorMode {
    /// Only viewing, the mouse doesn't edit
    #[default]
    View,
    /// Painting ground textures or placing roads
    Terrain(TerrainMode),
    Objects(ObjectsMode),
    Buildings(Box<BuildingMode>),
    /// Placing, erasing and editing soldiers, see `soldiers.rs`
    Soldiers(SoldierMode),
    /// Placing, erasing and editing world items, see `items.rs`
    Items(ItemMode),
    /// Placing, erasing and editing lights, see `lights.rs`
    Lights(LightMode),
    /// Painting exit grids and setting entry points, see `traversal.rs`
    Traversal(TraversalMode),
}

impl EditorMode {
    /// The brush of the modes that edit, `None` for viewing
    pub fn brush(&self, tileset: &Tileset, smoothing: EditorSmoothing) -> Option<Brush> {
        match self {
            EditorMode::View => None,
            EditorMode::Terrain(mode) => Some(mode.brush()),
            EditorMode::Objects(mode) => Some(mode.brush()),
            EditorMode::Buildings(mode) => Some(mode.brush(tileset, smoothing)),
            EditorMode::Traversal(mode) => mode.brush(),
            EditorMode::Lights(mode) => mode.brush(),
            EditorMode::Soldiers(mode) => mode.brush(),
            EditorMode::Items(mode) => mode.brush(),
        }
    }

    /// Whether the mode removes what it places
    pub fn erases(&self) -> bool {
        match self {
            EditorMode::View => false,
            EditorMode::Terrain(mode) => mode.erase,
            EditorMode::Objects(mode) => mode.erase,
            EditorMode::Buildings(mode) => mode.erase && mode.tool.can_erase(),
            EditorMode::Traversal(mode) => mode.erase && mode.brush().is_some(),
            EditorMode::Lights(mode) => mode.erase && mode.tool == LightTool::Place,
            EditorMode::Soldiers(mode) => mode.erase && mode.tool == SoldierTool::Place,
            EditorMode::Items(mode) => mode.erase && mode.tool == ItemTool::Place,
        }
    }

    /// Whether a stroke with a mouse button edits, all modes paint with the left button
    pub fn uses_button(&self, button: MouseButton) -> bool {
        match self {
            EditorMode::View => false,
            EditorMode::Terrain(_) | EditorMode::Objects(_) => button == MouseButton::Left,
            EditorMode::Buildings(mode) => mode.tool.uses_button(button),
            // Only painting exit grids uses the brush, the right button queries them, see
            // `traversal.rs`
            EditorMode::Traversal(mode) => mode.brush().is_some() && button == MouseButton::Left,
            // Only erasing paints, see `lights.rs`, `soldiers.rs` and `items.rs`
            EditorMode::Lights(mode) => mode.brush().is_some() && button == MouseButton::Left,
            EditorMode::Soldiers(mode) => mode.brush().is_some() && button == MouseButton::Left,
            EditorMode::Items(mode) => mode.brush().is_some() && button == MouseButton::Left,
        }
    }

    /// Whether the mouse selects buildings instead of painting, see `selection.rs`
    pub fn selects_buildings(&self) -> bool {
        matches!(self, EditorMode::Buildings(mode) if mode.tool == BuildingTool::Select)
    }

    /// Whether the mode only edits the ground: terrain, roads, objects, lights and the
    /// traversal points have no roof level, so the mode edits nothing on the roof floor
    pub fn is_ground_only(&self) -> bool {
        matches!(
            self,
            EditorMode::Terrain(_)
                | EditorMode::Objects(_)
                | EditorMode::Lights(_)
                | EditorMode::Traversal(_)
        )
    }

    /// Applies the mode to the tiles of the brush, with the button of the stroke, on the floor
    /// of the options
    pub fn apply(
        &self,
        context: &mut EditContext,
        area: &BrushArea,
        button: MouseButton,
        options: &RenderOptions,
    ) {
        let floor = options.floor;
        let left = button == MouseButton::Left;
        if floor == Level::Roof && self.is_ground_only() {
            return;
        }
        match self {
            EditorMode::View => {}
            EditorMode::Terrain(mode) if left => mode.apply(context, area),
            EditorMode::Objects(mode) if left => mode.apply(context, area),
            EditorMode::Buildings(mode) => mode.apply(context, area, button),
            EditorMode::Traversal(mode) if left => mode.apply(context, area),
            EditorMode::Lights(mode) if left => mode.apply(context, area),
            EditorMode::Soldiers(mode) if left => {
                mode.apply(context, area, &options.layers.soldier_teams, floor)
            }
            EditorMode::Items(mode) if left => mode.apply(context, area, floor),
            EditorMode::Terrain(_)
            | EditorMode::Objects(_)
            | EditorMode::Traversal(_)
            | EditorMode::Lights(_)
            | EditorMode::Soldiers(_)
            | EditorMode::Items(_) => {}
        }
    }
}

/// The ground textures, as the host sends them
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct GroundTextures {
    /// Name of the tile type of the texture that is painted while no texture has a weight
    pub selected: Option<String>,
    pub weights: Vec<TextureWeight>,
}

/// A ground texture and how often it is painted
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureWeight {
    /// Name of the tile type of the texture
    pub type_name: String,
    pub weight: u8,
}

/// A tile of a palette and how often it is picked
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionEntry {
    pub tile_id: TileIndex,
    pub count: u32,
}

/// What a click in the terrain mode does
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerrainTool {
    /// Paints or erases ground textures with the brush
    #[default]
    Ground,
    /// Places road macros on the clicked tile, or erases roads
    Roads,
    /// Places cliff pieces on the clicked tiles, or erases cliffs, and raises the land behind them
    Cliffs,
}

/// The terrain mode, painting ground textures and placing roads and cliffs (`TASK_TERRAIN` of the
/// original editor)
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct TerrainMode {
    pub tool: TerrainTool,
    /// Removes the ground textures, roads or cliffs instead
    pub erase: bool,
    /// The brush of the ground tool, the only one of the terrain modes that fills
    pub brush: Brush,
    /// Chance of a placement on each tile of the brush, in percent, ground textures ignore it
    pub density: u8,
    pub ground: GroundTextures,
    /// The road macro of the roads tool
    pub road: u16,
    /// The sub index of the `FIRSTCLIFF` piece of the cliffs tool
    pub cliff: u16,
}

impl TerrainMode {
    fn ground(&self, context: &EditContext) -> GroundSelection {
        let weights = self.ground.weights.iter();
        let weights = weights.map(|w| (w.type_name.clone(), w.weight));
        GroundSelection::new(context, self.ground.selected.clone(), weights)
    }

    /// The brush, roads and cliffs only work on the clicked tile
    pub fn brush(&self) -> Brush {
        match self.tool {
            TerrainTool::Ground => self.brush,
            TerrainTool::Roads | TerrainTool::Cliffs => Brush::Square { size: 1 },
        }
    }

    /// Applies the tool to the tiles of the brush
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea) {
        match self.tool {
            TerrainTool::Ground => self.apply_ground(context, area),
            TerrainTool::Roads => self.apply_roads(context, area),
            TerrainTool::Cliffs => self.apply_cliffs(context, area),
        }
    }

    /// Paints or erases the ground textures on the tiles of the brush, fills from the first tile
    /// (`EraseMapTile`)
    fn apply_ground(&self, context: &mut EditContext, area: &BrushArea) {
        if self.erase {
            for grid_no in area.tiles() {
                erase_ground(context, grid_no);
            }
            return;
        }

        let ground = self.ground(context);
        if self.brush == Brush::Fill {
            if let (Some(first), Some(paste)) = (
                area.first(),
                choose_weighted_texture(&mut context.document.random, &ground),
            ) {
                terrain_fill(context, first, paste);
            }
            return;
        }
        for grid_no in area.tiles() {
            let Some(paste) = choose_weighted_texture(&mut context.document.random, &ground) else {
                return;
            };
            paste_texture_common(context, grid_no, paste);
        }
    }
}

impl TerrainMode {
    /// Places the road macro on the first tile of the brush or erases the roads of its tiles
    fn apply_roads(&self, context: &mut EditContext, area: &BrushArea) {
        if self.erase {
            for grid_no in area.tiles() {
                erase_roads(context, grid_no);
            }
            return;
        }
        if let Some(first) = area.first().filter(|_| self.road < NUM_ROAD_MACROS) {
            place_road_macro(context, first, self.road);
        }
    }

    /// Places the cliff piece on the tiles of the brush or erases their cliffs, then works out the
    /// height of the land again
    fn apply_cliffs(&self, context: &mut EditContext, area: &BrushArea) {
        for grid_no in area.tiles() {
            if self.erase {
                erase_cliffs(context, grid_no);
            } else {
                place_cliff(context, grid_no, self.cliff);
            }
        }
        raise_world_land(context);
    }
}

/// The objects mode, placing debris, trees, rocks, other junk and furniture (part of
/// `TASK_TERRAIN` of the original editor)
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct ObjectsMode {
    /// Removes debris and structures instead
    pub erase: bool,
    pub brush: Brush,
    /// Chance of a placement on each tile of the brush, in percent
    pub density: u8,
    /// The selected debris, trees, rocks, other junk and furniture
    pub objects: Vec<SelectionEntry>,
}

impl ObjectsMode {
    /// The brush, objects don't fill
    pub fn brush(&self) -> Brush {
        match self.brush {
            Brush::Fill => Brush::Square { size: 1 },
            brush => brush,
        }
    }

    /// The debris and structures a click places on the tiles of the brush, with the density
    /// test and the random picks of the objects palette
    ///
    /// The preview runs this with a copy of the random numbers of the document, so it shows what
    /// the click places.
    pub fn plan_objects(
        &self,
        random: &mut Random,
        tileset: &Tileset,
        area: &BrushArea,
    ) -> Vec<(GridNo, Arc<Tile>)> {
        let entries = self.objects.iter().map(|e| (e.tile_id, e.count));
        let list = SelectionList::new(tileset, entries);
        if list.is_empty() {
            return vec![];
        }
        // Single tiles always get a placement
        let every_tile = area.is_single();
        area.tiles()
            .filter_map(|grid_no| {
                if !every_tile && !density_test(random, self.density) {
                    return None;
                }
                Some((grid_no, list.pick(random)?.clone()))
            })
            .collect()
    }

    /// Places or erases objects on the tiles of the brush (`DrawObjectsBasedOnSelectionRegion`
    /// and `EraseMapTile`)
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea) {
        if self.erase {
            for grid_no in area.tiles() {
                erase_debris(context, grid_no);
                erase_structures(context, grid_no);
            }
            return;
        }
        let plan = self.plan_objects(&mut context.document.random, context.tileset, area);
        for (grid_no, tile) in plan {
            paste_tile(context, grid_no, tile);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        editor::document::tests::{grid, Fixture},
        file_formats::ja2_tile_types::{DEBRISROCKS, FIRSTFULLSTRUCT, FIRSTISTRUCT},
        tileset::tests::structure_tileset,
    };

    fn mode(json: &str) -> EditorMode {
        serde_json::from_str(json).unwrap()
    }

    fn terrain(json: &str) -> TerrainMode {
        match mode(json) {
            EditorMode::Terrain(mode) => mode,
            _ => panic!("not the terrain mode"),
        }
    }

    fn objects(json: &str) -> ObjectsMode {
        match mode(json) {
            EditorMode::Objects(mode) => mode,
            _ => panic!("not the objects mode"),
        }
    }

    #[test]
    fn modes_are_read_from_json() {
        assert_eq!(mode(r#"{"mode":"view"}"#), EditorMode::View);
        let terrain_mode = terrain(
            r#"{"mode":"terrain","erase":false,
                "brush":{"kind":"line","width":3},"density":20,
                "ground":{"selected":"TEXTURE3","weights":[{"typeName":"TEXTURE2","weight":3}]}}"#,
        );
        assert_eq!(terrain_mode.brush, Brush::Line { width: 3 });
        assert_eq!(terrain_mode.density, 20);
        assert_eq!(terrain_mode.ground.selected.as_deref(), Some("TEXTURE3"));
        assert_eq!(
            terrain_mode.ground.weights,
            [TextureWeight {
                type_name: "TEXTURE2".to_owned(),
                weight: 3
            }]
        );
        let objects_mode = objects(
            r#"{"mode":"objects","brush":{"kind":"line","width":3},"density":20,
                "objects":[{"tileId":7,"count":2}]}"#,
        );
        assert_eq!(objects_mode.brush(), Brush::Line { width: 3 });
        assert_eq!(objects_mode.density, 20);
        assert_eq!(
            objects_mode.objects,
            [SelectionEntry {
                tile_id: 7,
                count: 2
            }]
        );
        assert_eq!(terrain_mode.tool, TerrainTool::Ground);
        let roads = terrain(r#"{"mode":"terrain","tool":"roads","erase":true,"road":4}"#);
        assert_eq!(roads.tool, TerrainTool::Roads);
        assert!(roads.erase);
        assert_eq!(roads.road, 4);
        let cliffs = terrain(r#"{"mode":"terrain","tool":"cliffs","cliff":3}"#);
        assert_eq!(cliffs.tool, TerrainTool::Cliffs);
        assert_eq!(cliffs.cliff, 3);
        // Only the terrain fills
        let fixture = Fixture::new();
        let brush = |json| mode(json).brush(&fixture.tileset, EditorSmoothing::Normal);
        let single = Some(Brush::Square { size: 1 });
        assert_eq!(
            brush(r#"{"mode":"terrain","brush":{"kind":"fill"}}"#),
            Some(Brush::Fill)
        );
        assert_eq!(
            brush(r#"{"mode":"objects","brush":{"kind":"fill"}}"#),
            single
        );
        // Roads and cliffs ignore the brush
        assert_eq!(
            brush(r#"{"mode":"terrain","tool":"roads","brush":{"kind":"fill"}}"#),
            single
        );
        assert_eq!(
            brush(r#"{"mode":"terrain","tool":"cliffs","brush":{"kind":"fill"}}"#),
            single
        );
    }

    #[test]
    fn the_cliffs_tool_places_and_erases_cliffs_and_raises_the_land() {
        let mut fixture = Fixture::new();
        let column = [grid(50, 49), grid(50, 50), grid(50, 51)];
        let place = terrain(r#"{"mode":"terrain","tool":"cliffs","cliff":2}"#);
        let mut context = fixture.context();
        for g in column {
            place.apply(&mut context, &place.brush().region(None, g.position()));
        }
        context.finish();
        let cliffs = &fixture.map.tiles[grid(50, 50)].structs;
        assert_eq!(cliffs.len(), 1);
        assert_eq!(
            (cliffs[0].type_name.as_str(), cliffs[0].sub_index),
            ("FIRSTCLIFF", 2)
        );
        // The land before the cliff in the row is raised
        assert_eq!(fixture.map.tiles[grid(40, 50)].height, 80);
        assert_eq!(fixture.map.tiles[grid(60, 50)].height, 0);

        let erase = terrain(r#"{"mode":"terrain","tool":"cliffs","erase":true,"cliff":2}"#);
        let mut context = fixture.context();
        for g in column {
            erase.apply(&mut context, &erase.brush().region(None, g.position()));
        }
        context.finish();
        assert!(fixture.map.tiles[grid(50, 50)].structs.is_empty());
        assert!(fixture.map.tiles.values().all(|t| t.height == 0));
    }

    fn debris_count(fixture: &Fixture, area: &BrushArea) -> usize {
        area.tiles()
            .filter(|&g| !fixture.map.tiles[g].objects.is_empty())
            .count()
    }

    #[test]
    fn density_applies_to_larger_brushes_only() {
        let mut fixture = Fixture::new();
        let rock = fixture.tileset.index(DEBRISROCKS, 1).unwrap();
        let mut mode = objects(r#"{"mode":"objects","density":0}"#);
        mode.objects = vec![SelectionEntry {
            tile_id: rock,
            count: 1,
        }];
        let center = GridPosition::new(50, 50);

        let mut context = fixture.context();
        let single = mode.brush().region(None, center);
        mode.apply(&mut context, &single);
        context.finish();
        assert_eq!(debris_count(&fixture, &single), 1);

        mode.brush = Brush::Square { size: 7 };
        let region = mode.brush().region(None, GridPosition::new(80, 80));
        let mut context = fixture.context();
        mode.apply(&mut context, &region);
        context.finish();
        assert_eq!(debris_count(&fixture, &region), 0);

        mode.density = 100;
        let mut context = fixture.context();
        mode.apply(&mut context, &region);
        context.finish();
        assert_eq!(debris_count(&fixture, &region), 49);

        mode.erase = true;
        let mut context = fixture.context();
        mode.apply(&mut context, &region);
        context.finish();
        assert_eq!(debris_count(&fixture, &region), 0);
    }

    #[test]
    fn ground_ignores_the_density() {
        let mut fixture = Fixture::new();
        let mut mode = terrain(
            r#"{"mode":"terrain","brush":{"kind":"square","size":3},"density":0,
                "ground":{"selected":"TEXTURE2","weights":[]}}"#,
        );
        let mut context = fixture.context();
        mode.apply(
            &mut context,
            &mode.brush.region(None, GridPosition::new(50, 50)),
        );
        context.finish();
        assert_eq!(fixture.land(grid(49, 49)).last().unwrap().0, "TEXTURE2");
        assert_eq!(fixture.land(grid(50, 50)).last().unwrap().0, "TEXTURE2");
        // Fills from the clicked tile
        mode.brush = Brush::Fill;
        mode.ground.selected = Some("TEXTURE1".to_owned());
        let mut context = fixture.context();
        mode.apply(
            &mut context,
            &mode.brush.region(None, GridPosition::new(50, 50)),
        );
        context.finish();
        assert_eq!(fixture.land(grid(50, 50)).last().unwrap().0, "TEXTURE1");

        // Nothing is painted without a texture
        mode.brush = Brush::Square { size: 3 };
        mode.ground.selected = None;
        let mut context = fixture.context();
        mode.apply(
            &mut context,
            &mode.brush.region(None, GridPosition::new(80, 80)),
        );
        assert!(context.finish().is_empty());
    }

    #[test]
    fn objects_place_debris_and_structures() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        let rock = fixture.tileset.index(DEBRISROCKS, 1).unwrap();
        let tree = fixture.tileset.index(FIRSTFULLSTRUCT, 1).unwrap();
        let mut mode = objects(r#"{"mode":"objects"}"#);
        mode.objects = vec![
            SelectionEntry {
                tile_id: rock,
                count: 1,
            },
            SelectionEntry {
                tile_id: tree,
                count: 1,
            },
        ];
        let positions: Vec<_> = (0..40).map(|x| GridPosition::new(10 + x * 2, 50)).collect();
        let tiles: Vec<GridNo> = positions.iter().map(|&p| GridNo::from(p)).collect();
        let mut context = fixture.context();
        for &p in &positions {
            mode.apply(&mut context, &Brush::default().region(None, p));
        }
        context.finish();
        let with_debris = tiles
            .iter()
            .filter(|&&g| !fixture.map.tiles[g].objects.is_empty());
        let with_trees = tiles
            .iter()
            .filter(|&&g| !fixture.map.tiles[g].structs.is_empty());
        let (debris, trees) = (with_debris.count(), with_trees.count());
        assert!(debris > 0 && trees > 0, "{debris} debris, {trees} trees");
        assert_eq!(debris + trees, tiles.len());

        // Erasing removes both
        mode.erase = true;
        let mut context = fixture.context();
        for &p in &positions {
            mode.apply(&mut context, &Brush::default().region(None, p));
        }
        context.finish();
        assert!(tiles
            .iter()
            .all(|&g| fixture.map.tiles[g].objects.is_empty()
                && fixture.map.tiles[g].structs.is_empty()));
    }

    #[test]
    fn objects_place_and_erase_furniture() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        let furniture = fixture.tileset.index(FIRSTISTRUCT, 2).unwrap();
        let mut mode = objects(r#"{"mode":"objects"}"#);
        mode.objects = vec![SelectionEntry {
            tile_id: furniture,
            count: 1,
        }];
        let area = Brush::default().region(None, GridPosition::new(60, 60));
        let mut context = fixture.context();
        mode.apply(&mut context, &area);
        context.finish();
        let structs = |fixture: &Fixture| -> Vec<(String, u16)> {
            fixture.map.tiles[grid(60, 60)]
                .structs
                .iter()
                .map(|t| (t.type_name.clone(), t.sub_index))
                .collect()
        };
        assert_eq!(structs(&fixture), [("ISTRUCT1".to_owned(), 2)]);

        mode.erase = true;
        let mut context = fixture.context();
        mode.apply(&mut context, &area);
        context.finish();
        assert!(structs(&fixture).is_empty());
    }

    #[test]
    fn the_plan_matches_what_is_placed() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        let rock = fixture.tileset.index(DEBRISROCKS, 1).unwrap();
        let tree = fixture.tileset.index(FIRSTFULLSTRUCT, 1).unwrap();
        let mut mode = objects(
            r#"{"mode":"objects","brush":{"kind":"circle","size":9},
                "density":30}"#,
        );
        mode.objects = vec![
            SelectionEntry {
                tile_id: rock,
                count: 3,
            },
            SelectionEntry {
                tile_id: tree,
                count: 1,
            },
        ];
        let area = mode.brush().region(None, GridPosition::new(60, 60));
        let mut random = fixture.document.random.clone();
        let plan = mode.plan_objects(&mut random, &fixture.tileset, &area);
        assert!(!plan.is_empty() && plan.len() < area.tiles.len());

        let mut context = fixture.context();
        mode.apply(&mut context, &area);
        context.finish();
        let placed: Vec<(GridNo, TileIndex)> = area
            .tiles()
            .flat_map(|g| {
                let tile = &fixture.map.tiles[g];
                tile.objects
                    .iter()
                    .chain(&tile.structs)
                    .map(move |t| (g, t.id))
            })
            .collect();
        let planned: Vec<(GridNo, TileIndex)> = plan.iter().map(|(g, t)| (*g, t.id)).collect();
        assert!(placed.iter().all(|p| planned.contains(p)));
        // Debris always fits, structures that overlap others of the plan are left out
        let planned_debris = planned.iter().filter(|&&(_, id)| id == rock);
        assert!(planned_debris.clone().count() > 0);
        assert!(planned_debris.into_iter().all(|p| placed.contains(p)));
        // The document went through the same random numbers
        assert_eq!(
            fixture.document.random.clone().next(1000),
            random.next(1000)
        );
    }
}
