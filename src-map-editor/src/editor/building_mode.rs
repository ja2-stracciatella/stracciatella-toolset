//! The buildings mode (`TASK_BUILDINGS` of the original editor): placing single building tiles
//! (`PasteSingleWallCommon` of `Edit_Sys.cc`), erasing them (`EraseMapTile`) and the building
//! tools

use std::sync::Arc;

use bevy::input::mouse::MouseButton;
use serde::Deserialize;

use crate::{
    map::{EditorSmoothing, GridNo, RoomNo},
    random::Random,
    tileset::{BuildingPiece, Tile, TileIndex, Tileset, WallPiece},
};

use super::{
    brush::{Brush, BrushArea},
    building::{
        add_building_section, is_flat_roof, remove_building_section, replace_with_new_roof,
        Materials, TileRect,
    },
    caves::{add_cave_section, remove_cave_section},
    document::EditContext,
    doors::request_door_editing,
    mode::SelectionEntry,
    placement::SelectionList,
    smart::{
        erase_smart, paste_smart_broken_wall, paste_smart_door, paste_smart_wall,
        paste_smart_window, SmartBrokenWall,
    },
    walls::{
        erase_horizontal_wall, erase_vertical_wall, is_basement, is_door, is_door_shadow, is_wall,
        valid_decal_placement,
    },
};

/// The tools of the buildings mode, the buttons of the original's buildings taskbar
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BuildingTool {
    /// Single tiles of the walls, doors, windows, roofs, broken walls, decals and floors
    /// palettes, each in the layer of its piece. Decals only go onto walls and fences, broken
    /// walls replace the wall of their alignment. Erasing clears the layers of the selected
    /// kinds.
    #[default]
    Tiles,
    /// Places what the smart value says: a wall that picks its piece from the walls around it,
    /// doors, windows, broken walls or a hole that replace the walls of the tile, or a roof
    /// style that replaces the roof of the clicked building. Erasing restores doors, windows and
    /// broken walls and removes plain walls.
    Smart,
    /// A new room of the room palette, or an extension of the building it touches. Erasing saws
    /// the room out of the buildings and closes them with new walls.
    Room,
    /// Selects the clicked building, clicking the ground clears the selection. Delete removes
    /// the selected building, dragging it moves it and Ctrl+dragging (Cmd on macOS) copies it. The mouse is
    /// handled by `selection.rs`, not by the strokes of the brush.
    Select,
    /// Gives the tiles of the brush the room number, erasing removes it (`PasteRoomNumber`)
    RoomNumber,
    /// Fills the area with rock on cave maps, the right button carves a cave out of it
    Caves,
    /// Asks the host to edit the lock and trap of the clicked door
    DoorLocks,
}

impl BuildingTool {
    /// The tools that place single tiles of their palette
    fn is_selection(self) -> bool {
        self == BuildingTool::Tiles
    }

    /// Whether the erase flag makes the tool remove what it places
    pub fn can_erase(self) -> bool {
        self.is_selection()
            || matches!(
                self,
                BuildingTool::Room | BuildingTool::RoomNumber | BuildingTool::Smart
            )
    }

    /// Whether the preview shows what a click does, for the tools on single tiles
    pub fn previews(self) -> bool {
        self.is_selection() || self.is_smart()
    }

    fn is_smart(self) -> bool {
        self == BuildingTool::Smart
    }

    /// Whether the tool does something with a mouse button, the right button is only used by
    /// the caves. Selecting buildings doesn't paint with any.
    pub fn uses_button(self, button: MouseButton) -> bool {
        if self == BuildingTool::Select {
            return false;
        }
        match button {
            MouseButton::Left => true,
            MouseButton::Right => self == BuildingTool::Caves,
            _ => false,
        }
    }
}

/// The value of the smart method (`gubWallUIValue` and the others), as the host sends it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SmartValue {
    /// A tile of the smart palettes, whose piece tells what to place. Roofs replace the roof of
    /// the building in their style.
    Tile {
        #[serde(rename = "tileId")]
        tile_id: TileIndex,
    },
    /// A hole in the wall
    Hole,
}

/// What the smart method places
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartPlacement {
    /// A wall of a style
    Wall(u8),
    /// A door of a style, open or closed
    Door {
        style: u8,
        open: bool,
    },
    /// A window of a style
    Window(u8),
    BrokenWall(SmartBrokenWall),
    /// The roof of the clicked building, in a style
    Roof(u8),
}

impl SmartValue {
    /// What the value places, `None` for tiles that aren't of the smart palettes
    pub fn placement(self, tileset: &Tileset) -> Option<SmartPlacement> {
        let tile_id = match self {
            SmartValue::Hole => return Some(SmartPlacement::BrokenWall(SmartBrokenWall::Hole)),
            SmartValue::Tile { tile_id } => tile_id,
        };
        let building = tileset.tile(tile_id)?.building?;
        // The windows and broken walls are wall pieces too, so they go first
        match building.piece {
            BuildingPiece::Door { open } => Some(SmartPlacement::Door {
                style: building.style,
                open,
            }),
            BuildingPiece::Wall(WallPiece::Window { style }) => Some(SmartPlacement::Window(style)),
            BuildingPiece::Wall(
                piece @ (WallPiece::Broken { .. } | WallPiece::Cracked | WallPiece::Smudged),
            ) => Some(SmartPlacement::BrokenWall(SmartBrokenWall::Piece(piece))),
            piece if piece.is_wall() => Some(SmartPlacement::Wall(building.style)),
            piece if piece.is_flat_roof() => Some(SmartPlacement::Roof(building.style)),
            _ => None,
        }
    }
}

/// The tiles of the palettes of the buildings mode and how often each is picked
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildingSelections {
    /// The walls, floors and roofs of new rooms
    pub room: Vec<SelectionEntry>,
    /// The single tiles of the tiles tool, of any of its palettes
    pub tiles: Vec<SelectionEntry>,
}

/// The buildings mode, set by the host with `set_editor_mode`
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BuildingMode {
    pub tool: BuildingTool,
    /// Removes what the tool places instead
    pub erase: bool,
    pub brush: Brush,
    pub selections: BuildingSelections,
    pub smart: Option<SmartValue>,
    /// The room the room number tool paints, 0 is no room
    pub room_number: u8,
}

/// The selection list of some entries
fn selection_list(tileset: &Tileset, entries: &[SelectionEntry]) -> SelectionList {
    SelectionList::new(tileset, entries.iter().map(|e| (e.tile_id, e.count)))
}

impl BuildingMode {
    /// The entries of the palette of a tool
    fn entries(&self, tool: BuildingTool) -> &[SelectionEntry] {
        let selections = &self.selections;
        match tool {
            BuildingTool::Tiles => &selections.tiles,
            _ => &selections.room,
        }
    }

    /// A style of the flat roofs of the tiles selection
    pub fn roof_style(&self, context: &mut EditContext) -> Option<u8> {
        let list = selection_list(context.tileset, &self.selections.tiles);
        let tile = list.pick_where(&mut context.document.random, is_flat_roof)?;
        tile.building.map(|b| b.style)
    }

    /// The materials of new rooms, from the room palette
    pub fn materials(&self, tileset: &Tileset) -> Materials {
        Materials::new(selection_list(tileset, &self.selections.room))
    }

    /// Whether new rooms get a slant roof, which is 8 tiles wide
    fn builds_slant_roofs(&self, tileset: &Tileset, smoothing: EditorSmoothing) -> bool {
        self.tool == BuildingTool::Room
            && !self.erase
            && smoothing != EditorSmoothing::Basement
            && self.materials(tileset).has_slant_roof_only()
    }

    /// Whether the tiles tool places only floors, which take any brush
    fn places_floors_only(&self, tileset: &Tileset) -> bool {
        let tiles = &self.selections.tiles;
        self.tool == BuildingTool::Tiles
            && !tiles.is_empty()
            && tiles
                .iter()
                .all(|e| tileset.tile(e.tile_id).is_some_and(|t| t.is_floor()))
    }

    /// The kinds of tiles of the tiles selection, whose layers erasing clears
    fn selected_kinds(&self, tileset: &Tileset) -> Vec<BuildingPiece> {
        let mut kinds: Vec<BuildingPiece> = vec![];
        for entry in &self.selections.tiles {
            let piece = tileset.tile(entry.tile_id).and_then(|t| t.building);
            if let Some(building) = piece {
                if !kinds.contains(&building.piece) {
                    kinds.push(building.piece);
                }
            }
        }
        kinds
    }

    /// The brush, restricted like `EnsureSelectionType`: erasing and floors take any brush, the
    /// room tools take the rectangle brushes also when erasing, slant roofs a line 8 tiles wide
    /// and everything else the clicked tile
    pub fn brush(&self, tileset: &Tileset, smoothing: EditorSmoothing) -> Brush {
        let single = Brush::Square { size: 1 };
        if self.builds_slant_roofs(tileset, smoothing) {
            return Brush::Line { width: 8 };
        }
        let erase = self.erase && self.tool.can_erase();
        match (self.tool, self.brush) {
            (_, Brush::Fill) => single,
            (BuildingTool::Tiles, brush) if self.places_floors_only(tileset) => brush,
            (BuildingTool::RoomNumber, brush) => brush,
            (BuildingTool::Room | BuildingTool::Caves, Brush::Circle { size }) => {
                Brush::Square { size }
            }
            (BuildingTool::Room | BuildingTool::Caves, brush) => brush,
            (_, brush) if erase => brush,
            _ => single,
        }
    }

    /// The tiles a click places on the tiles of the brush, for the tools that place single
    /// tiles of their palette. The preview runs this with a copy of the random numbers.
    pub fn plan_tiles(
        &self,
        random: &mut Random,
        tileset: &Tileset,
        area: &BrushArea,
    ) -> Vec<(GridNo, Arc<Tile>)> {
        if !self.tool.is_selection() {
            return vec![];
        }
        let list = selection_list(tileset, self.entries(self.tool));
        area.tiles()
            .filter_map(|grid_no| Some((grid_no, list.pick(random)?.clone())))
            .collect()
    }

    /// Applies the mode to the tiles of the brush, with the left or the right button
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea, button: MouseButton) {
        if !self.tool.uses_button(button) {
            return;
        }
        if button == MouseButton::Right {
            self.apply_right(context, area);
            return;
        }
        let tileset = context.tileset;
        if self.erase && self.tool.can_erase() {
            if self.tool == BuildingTool::Room {
                if let Some(rect) = TileRect::around(area) {
                    remove_building_section(context, rect, &self.materials(tileset));
                }
                return;
            }
            if self.tool.is_selection() {
                let kinds = self.selected_kinds(tileset);
                for grid_no in area.tiles() {
                    erase_tiles(context, grid_no, &kinds);
                }
                return;
            }
            let materials = self.tool.is_smart().then(|| self.materials(tileset));
            for grid_no in area.tiles() {
                match &materials {
                    Some(materials) => erase_smart(context, grid_no, materials),
                    None => erase(context, grid_no, self.tool),
                }
            }
            return;
        }
        match self.tool {
            BuildingTool::Tiles => {
                let plan = self.plan_tiles(&mut context.document.random, tileset, area);
                for (grid_no, tile) in plan {
                    let Some(building) = tile.building else {
                        continue;
                    };
                    match building.piece {
                        // Decals only go onto walls and fences
                        BuildingPiece::WallDecal | BuildingPiece::Switch
                            if !valid_decal_placement(context, grid_no) =>
                        {
                            continue;
                        }
                        BuildingPiece::Decoration
                        | BuildingPiece::Wall(
                            WallPiece::Broken { .. } | WallPiece::Cracked | WallPiece::Smudged,
                        ) => {
                            // The broken wall replaces the wall of its alignment and its shadows
                            if tile.wall_orientation().is_horizontal() {
                                erase_horizontal_wall(context, Some(grid_no));
                            } else if tile.wall_orientation().is_vertical() {
                                erase_vertical_wall(context, Some(grid_no));
                            }
                        }
                        _ => {}
                    }
                    paste_single(context, grid_no, tile);
                }
            }
            BuildingTool::Room => {
                if let Some(rect) = TileRect::around(area) {
                    add_building_section(context, rect, &self.materials(tileset));
                }
            }
            BuildingTool::Select => {}
            BuildingTool::Caves => {
                if let Some(rect) = TileRect::around(area) {
                    add_cave_section(context, rect);
                }
            }
            BuildingTool::DoorLocks => {
                if let Some(first) = area.first() {
                    request_door_editing(context, first);
                }
            }
            BuildingTool::RoomNumber => {
                for grid_no in area.tiles() {
                    context.set_room(grid_no, RoomNo::new(self.room_number));
                }
            }
            BuildingTool::Smart => {
                let placement = self.smart.and_then(|smart| smart.placement(tileset));
                let (Some(first), Some(placement)) = (area.first(), placement) else {
                    return;
                };
                let materials = self.materials(tileset);
                match placement {
                    SmartPlacement::Wall(style) => {
                        paste_smart_wall(context, first, style, &materials);
                    }
                    SmartPlacement::Door { style, open } => {
                        paste_smart_door(context, first, style, open);
                    }
                    SmartPlacement::Window(window) => {
                        paste_smart_window(context, first, window, &materials);
                    }
                    SmartPlacement::BrokenWall(broken) => {
                        paste_smart_broken_wall(context, first, broken, &materials);
                    }
                    SmartPlacement::Roof(style) => replace_with_new_roof(context, first, style),
                }
            }
        }
    }

    /// The right button: the caves carve caves
    fn apply_right(&self, context: &mut EditContext, area: &BrushArea) {
        if self.tool == BuildingTool::Caves {
            if let Some(rect) = TileRect::around(area) {
                remove_cave_section(context, rect);
            }
        }
    }
}

/// Places a single tile of a palette of the buildings mode in the layer it belongs to
/// (`PasteSingleWallCommon`)
///
/// Walls, doors and decorations replace the wall of their alignment, roofs the flat roof and
/// floors the floor of their type. Structures that don't fit are left out.
pub fn paste_single(context: &mut EditContext, grid_no: GridNo, tile: Arc<Tile>) {
    let Some(building) = tile.building else {
        return;
    };
    let basement = is_basement(context);
    match building.piece {
        BuildingPiece::OnRoof => context.add_on_roof_to_tail(grid_no, tile),
        // A-frames belong to the roofs
        BuildingPiece::Wall(WallPiece::AFrame { .. }) => context.add_roof_to_tail(grid_no, tile),
        BuildingPiece::Wall(piece) if piece.is_shadow() => {
            if !basement {
                context.add_exclusive_shadow(grid_no, tile);
            }
        }
        BuildingPiece::Door { .. } | BuildingPiece::DoorFrame => {
            if let (Some(shadow), false) = (tile.door_shadow(), basement) {
                context.add_exclusive_shadow(grid_no, shadow.clone());
            }
            context.add_wall_to_struct_layer(grid_no, tile, true);
        }
        BuildingPiece::Wall(_) | BuildingPiece::Decoration => {
            context.add_wall_to_struct_layer(grid_no, tile, true);
        }
        BuildingPiece::Roof(_) | BuildingPiece::SlantRoof(_) => {
            context.remove_roofs_where(grid_no, is_flat_roof);
            context.add_roof_to_tail(grid_no, tile);
        }
        BuildingPiece::Floor => {
            if let Some(index) = context.find_type_in_land(grid_no, &tile.type_name) {
                context.remove_land(grid_no, index);
            }
            context.add_land_to_head(grid_no, tile);
        }
        BuildingPiece::WallDecal => {
            context.remove_structs_where(grid_no, |t| {
                t.building
                    .is_some_and(|b| b.piece == BuildingPiece::WallDecal)
            });
            if context.struct_fits(grid_no, &tile) {
                context.add_struct_to_tail(grid_no, tile);
            }
        }
        BuildingPiece::Switch => {
            if context.struct_fits(grid_no, &tile) {
                context.add_struct_to_tail(grid_no, tile);
            }
        }
        // Furniture is placed with the objects of the terrain mode
        BuildingPiece::DoorShadow | BuildingPiece::Furniture | BuildingPiece::GenericFurniture => {}
    }
}

fn is_decoration(tile: &Tile) -> bool {
    tile.building.is_some_and(|b| b.piece.is_decoration())
}

/// Removes what a tool places from a tile (`EraseMapTile`)
pub fn erase(context: &mut EditContext, grid_no: GridNo, tool: BuildingTool) {
    // Erasing rooms saws them out of the buildings instead, the tiles tool erases by the kinds
    // of its selection
    if tool == BuildingTool::RoomNumber {
        context.set_room(grid_no, None);
    }
}

fn erase_roofs(context: &mut EditContext, grid_no: GridNo) {
    context.remove_roofs_where(grid_no, |_| true);
    context.remove_on_roofs_where(grid_no, |_| true);
}

/// Removes the layers of some kinds of building tiles from a tile (`EraseMapTile`): walls, doors
/// and windows the walls and doors with their shadows, roof pieces the roofs and the tiles on
/// them, floors the floors and decals and decorations the decorations
pub fn erase_tiles(context: &mut EditContext, grid_no: GridNo, kinds: &[BuildingPiece]) {
    let is_roof = |piece: BuildingPiece| {
        matches!(
            piece,
            BuildingPiece::Roof(_)
                | BuildingPiece::SlantRoof(_)
                | BuildingPiece::OnRoof
                | BuildingPiece::Wall(WallPiece::AFrame { .. })
        )
    };
    let walls = kinds
        .iter()
        .any(|&piece| !is_roof(piece) && (piece.is_wall() || piece.is_door()));
    let roofs = kinds.iter().any(|&piece| is_roof(piece));
    let floors = kinds.contains(&BuildingPiece::Floor);
    let decorations = kinds.iter().any(|piece| {
        matches!(
            piece,
            BuildingPiece::WallDecal | BuildingPiece::Switch | BuildingPiece::Decoration
        )
    });
    if walls {
        context.remove_structs_where(grid_no, |tile| is_wall(tile) || is_door(tile));
        context.remove_shadows_where(grid_no, |tile| is_wall(tile) || is_door_shadow(tile));
    }
    if roofs {
        erase_roofs(context, grid_no);
    }
    if floors {
        context.remove_land_where(grid_no, Tile::is_floor);
    }
    if decorations {
        context.remove_structs_where(grid_no, is_decoration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        editor::{
            building::tests::{add_room, building_fixture, rect, walls},
            document::tests::{grid, Fixture},
            mode::EditorMode,
        },
        tileset::TileIndex,
    };

    fn mode(json: &str) -> BuildingMode {
        match serde_json::from_str(json).unwrap() {
            EditorMode::Buildings(mode) => *mode,
            other => panic!("not the buildings mode: {other:?}"),
        }
    }

    fn id(fixture: &Fixture, name: &str, sub_index: u16) -> TileIndex {
        fixture.tileset.tile_of(name, sub_index).unwrap()
    }

    fn entry(tile_id: TileIndex) -> SelectionEntry {
        SelectionEntry { tile_id, count: 1 }
    }

    fn apply(fixture: &mut Fixture, mode: &BuildingMode, x: u8, y: u8) {
        let brush = mode.brush(&fixture.tileset, EditorSmoothing::Normal);
        let area = brush.region(None, GridPosition::new(x, y));
        let mut context = fixture.context();
        mode.apply(&mut context, &area, MouseButton::Left);
        context.finish();
    }

    fn structs(fixture: &Fixture, x: u8, y: u8) -> Vec<(String, u16)> {
        fixture.map.tiles[grid(x, y)]
            .structs
            .iter()
            .map(|t| (t.type_name.clone(), t.sub_index))
            .collect()
    }

    #[test]
    fn modes_are_read_from_json() {
        let mode = mode(
            r#"{"mode":"buildings","tool":"tiles","erase":true,
                "brush":{"kind":"area"},
                "selections":{"room":[{"tileId":3,"count":2}],"tiles":[{"tileId":4,"count":1}]}}"#,
        );
        assert_eq!(mode.tool, BuildingTool::Tiles);
        assert!(mode.erase);
        assert_eq!(mode.brush, Brush::Area);
        assert_eq!(
            mode.selections.room,
            [SelectionEntry {
                tile_id: 3,
                count: 2
            }]
        );
        assert_eq!(mode.selections.tiles, [entry(4)]);
    }

    #[test]
    fn brushes_are_restricted_like_the_original() {
        let fixture = building_fixture();
        let tileset = &fixture.tileset;
        let brush = |json: &str, smoothing| mode(json).brush(tileset, smoothing);
        let normal = EditorSmoothing::Normal;
        let single = Brush::Square { size: 1 };
        assert_eq!(
            brush(
                r#"{"mode":"buildings","tool":"tiles","brush":{"kind":"area"}}"#,
                normal
            ),
            single
        );
        // Erasing takes any brush
        assert_eq!(
            brush(
                r#"{"mode":"buildings","tool":"tiles","erase":true,"brush":{"kind":"area"}}"#,
                normal
            ),
            Brush::Area
        );
        // Only floors take any brush
        let tiles = |names: &[(&str, u16)]| {
            let entries: Vec<String> = names
                .iter()
                .map(|&(name, sub_index)| {
                    format!(
                        r#"{{"tileId":{},"count":1}}"#,
                        id(&fixture, name, sub_index)
                    )
                })
                .collect();
            format!(
                r#"{{"mode":"buildings","tool":"tiles","brush":{{"kind":"circle","size":3}},
                    "selections":{{"tiles":[{}]}}}}"#,
                entries.join(",")
            )
        };
        assert_eq!(
            brush(&tiles(&[("FLOOR1", 1), ("FLOOR2", 3)]), normal),
            Brush::Circle { size: 3 }
        );
        assert_eq!(brush(&tiles(&[("WALL1", 1)]), normal), single);
        assert_eq!(
            brush(&tiles(&[("WALL1", 1), ("FLOOR1", 1)]), normal),
            single
        );
        assert_eq!(brush(&tiles(&[]), normal), single);
        assert_eq!(
            brush(
                r#"{"mode":"buildings","tool":"room","brush":{"kind":"circle","size":3}}"#,
                normal
            ),
            Brush::Square { size: 3 }
        );
        // Erasing rooms saws them, which keeps the rectangle brushes
        assert_eq!(
            brush(
                r#"{"mode":"buildings","tool":"room","erase":true,"brush":{"kind":"circle","size":3}}"#,
                normal
            ),
            Brush::Square { size: 3 }
        );
        assert_eq!(
            brush(
                r#"{"mode":"buildings","tool":"select","erase":true,"brush":{"kind":"area"}}"#,
                normal
            ),
            single
        );
        // A room with a slant roof is a line 8 tiles wide, but not in basements
        let slant = format!(
            r#"{{"mode":"buildings","tool":"room","brush":{{"kind":"area"}},
                "selections":{{"room":[{{"tileId":{},"count":1}}]}}}}"#,
            id(&fixture, "SROOF1", 1)
        );
        assert_eq!(brush(&slant, normal), Brush::Line { width: 8 });
        assert_eq!(brush(&slant, EditorSmoothing::Basement), Brush::Area);
    }

    #[test]
    fn walls_and_doors_replace_the_wall_of_their_alignment() {
        let mut fixture = building_fixture();
        let mut walls_mode = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        // Inside left, then outside left: the second replaces the first
        walls_mode.selections.tiles = vec![entry(id(&fixture, "WALL1", 10))];
        apply(&mut fixture, &walls_mode, 50, 50);
        walls_mode.selections.tiles = vec![entry(id(&fixture, "WALL1", 4))];
        apply(&mut fixture, &walls_mode, 50, 50);
        // A vertical wall goes next to it
        walls_mode.selections.tiles = vec![entry(id(&fixture, "WALL1", 1))];
        apply(&mut fixture, &walls_mode, 50, 50);
        let mut placed = structs(&fixture, 50, 50);
        placed.sort();
        assert_eq!(placed, [("WALL1".to_owned(), 1), ("WALL1".to_owned(), 4)]);
        assert_eq!(fixture.world.tiles[grid(50, 50)].structures.len(), 2);

        // A closed door replaces the vertical wall and adds its shadow
        let mut doors = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        doors.selections.tiles = vec![entry(id(&fixture, "DOOR1", 1))];
        apply(&mut fixture, &doors, 50, 50);
        let mut placed = structs(&fixture, 50, 50);
        placed.sort();
        assert_eq!(placed, [("DOOR1".to_owned(), 1), ("WALL1".to_owned(), 4)]);
        let shadows: Vec<&str> = fixture.map.tiles[grid(50, 50)]
            .shadows
            .iter()
            .map(|t| t.type_name.as_str())
            .collect();
        assert_eq!(shadows, ["DOORSH1"]);

        // Erasing walls also removes the doors and their shadows
        walls_mode.erase = true;
        apply(&mut fixture, &walls_mode, 50, 50);
        let tile = &fixture.map.tiles[grid(50, 50)];
        assert!(tile.structs.is_empty() && tile.shadows.is_empty());
    }

    #[test]
    fn tiles_go_to_their_layers() {
        let mut fixture = building_fixture();
        let mut roofs = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        for (name, sub_index) in [("ROOF1", 1), ("ROOF2", 3), ("WALL1", 32), ("ONROOF1", 1)] {
            roofs.selections.tiles = vec![entry(id(&fixture, name, sub_index))];
            apply(&mut fixture, &roofs, 50, 50);
        }
        let tile = &fixture.map.tiles[grid(50, 50)];
        let names = |tiles: &[Arc<Tile>]| {
            tiles
                .iter()
                .map(|t| format!("{} {}", t.type_name, t.sub_index))
                .collect::<Vec<_>>()
        };
        // The second roof replaced the first, the A-frame is on the roof layer
        assert_eq!(names(&tile.roofs), ["ROOF2 3", "WALL1 32"]);
        assert_eq!(names(&tile.on_roofs), ["ONROOF1 1"]);

        let mut floors =
            mode(r#"{"mode":"buildings","tool":"tiles","brush":{"kind":"square","size":3}}"#);
        floors.selections.tiles = vec![entry(id(&fixture, "FLOOR2", 3))];
        apply(&mut fixture, &floors, 50, 50);
        floors.selections.tiles = vec![entry(id(&fixture, "FLOOR2", 5))];
        apply(&mut fixture, &floors, 50, 50);
        assert_eq!(fixture.land(grid(49, 51)), [("TEXTURE1", 1), ("FLOOR2", 5)]);

        // Wall shadows go to the shadows
        let mut walls_mode = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        walls_mode.selections.tiles = vec![entry(id(&fixture, "WALL1", 30))];
        apply(&mut fixture, &walls_mode, 61, 61);
        apply(&mut fixture, &walls_mode, 61, 61);
        assert_eq!(fixture.map.tiles[grid(61, 61)].shadows.len(), 1);
        assert!(fixture.map.tiles[grid(61, 61)].structs.is_empty());
    }

    #[test]
    fn mixed_selections_place_every_kind() {
        let mut fixture = building_fixture();
        let mut tiles = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        tiles.selections.tiles = vec![
            entry(id(&fixture, "WALL1", 1)),
            entry(id(&fixture, "FLOOR2", 3)),
        ];
        // The random pick decides the tile of each click, so click until both are placed
        for _ in 0..50 {
            apply(&mut fixture, &tiles, 50, 50);
        }
        assert_eq!(structs(&fixture, 50, 50), [("WALL1".to_owned(), 1)]);
        assert!(fixture.land(grid(50, 50)).contains(&("FLOOR2", 3)));
    }

    #[test]
    fn erasing_clears_the_layers_of_the_selected_kinds() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let tile = |fixture: &Fixture| {
            let tile = &fixture.map.tiles[grid(52, 51)];
            (
                !tile.structs.is_empty(),
                tile.land.iter().any(|t| t.is_floor()),
                !tile.roofs.is_empty(),
            )
        };
        assert_eq!(tile(&fixture), (true, true, true));
        let mut erase = mode(r#"{"mode":"buildings","tool":"tiles","erase":true}"#);
        // Nothing selected erases nothing
        apply(&mut fixture, &erase, 52, 51);
        assert_eq!(tile(&fixture), (true, true, true));
        // Floors leave the walls and roofs
        erase.selections.tiles = vec![entry(id(&fixture, "FLOOR1", 1))];
        apply(&mut fixture, &erase, 52, 51);
        assert_eq!(tile(&fixture), (true, false, true));
        erase.selections.tiles = vec![entry(id(&fixture, "ROOF1", 1))];
        apply(&mut fixture, &erase, 52, 51);
        assert_eq!(tile(&fixture), (true, false, false));
        erase.selections.tiles = vec![entry(id(&fixture, "WALL1", 1))];
        apply(&mut fixture, &erase, 52, 51);
        assert_eq!(tile(&fixture), (false, false, false));
    }

    #[test]
    fn decals_need_a_wall() {
        let mut fixture = building_fixture();
        let mut decals = mode(r#"{"mode":"buildings","tool":"tiles"}"#);
        decals.selections.tiles = vec![entry(id(&fixture, "WALLDECAL1", 1))];
        apply(&mut fixture, &decals, 50, 50);
        assert!(structs(&fixture, 50, 50).is_empty());
        // The left fence of the structure tileset
        let fence = fixture
            .tileset
            .tile(id(&fixture, "FENCESTRUCT", 1))
            .unwrap()
            .clone();
        let mut context = fixture.context();
        context.add_struct_to_head(grid(50, 50), fence);
        context.finish();
        apply(&mut fixture, &decals, 50, 50);
        decals.selections.tiles = vec![entry(id(&fixture, "WALLDECAL2", 1))];
        apply(&mut fixture, &decals, 50, 50);
        // The second decal replaced the first, at the tail
        assert_eq!(
            structs(&fixture, 50, 50),
            [("FENCESTRUCT".to_owned(), 1), ("WALLDECAL2".to_owned(), 1)]
        );
    }

    #[test]
    fn whole_building_tools_use_the_room_palette() {
        let mut fixture = building_fixture();
        let mut room =
            mode(r#"{"mode":"buildings","tool":"room","brush":{"kind":"square","size":3}}"#);
        room.selections.room = ["WALL2", "FLOOR1", "ROOF1"]
            .iter()
            .map(|name| entry(id(&fixture, name, 1)))
            .collect();
        apply(&mut fixture, &room, 51, 51);
        assert_eq!(walls(&fixture, 51, 49).len(), 1);
        let wall = &fixture.map.tiles[grid(51, 49)].structs[0];
        assert_eq!(wall.type_name, "WALL2");

        // Smart roofs replace the roof of the whole building
        let mut new_roof = mode(r#"{"mode":"buildings","tool":"smart"}"#);
        new_roof.smart = Some(SmartValue::Tile {
            tile_id: id(&fixture, "ROOF4", 10),
        });
        apply(&mut fixture, &new_roof, 50, 52);
        assert!((50..=52).all(|x| fixture.map.tiles[grid(x, 50)].roofs[0].type_name == "ROOF4"));
    }

    #[test]
    fn smart_values_are_tiles_of_the_smart_palettes() {
        let fixture = building_fixture();
        let tileset = &fixture.tileset;
        let placement = |name, sub_index| {
            let json = format!(
                r#"{{"mode":"buildings","tool":"smart",
                    "smart":{{"kind":"tile","tileId":{}}}}}"#,
                id(&fixture, name, sub_index)
            );
            let mode = mode(&json);
            assert_eq!(mode.tool, BuildingTool::Smart);
            mode.smart.unwrap().placement(tileset)
        };
        assert_eq!(placement("WALL3", 1), Some(SmartPlacement::Wall(2)));
        assert_eq!(
            placement("DOOR2", 5),
            Some(SmartPlacement::Door {
                style: 1,
                open: true
            })
        );
        assert_eq!(placement("WALL1", 46), Some(SmartPlacement::Window(1)));
        assert_eq!(
            placement("WALL1", 65),
            Some(SmartPlacement::BrokenWall(SmartBrokenWall::Piece(
                WallPiece::Smudged
            )))
        );
        let hole = mode(r#"{"mode":"buildings","smart":{"kind":"hole"}}"#);
        assert_eq!(
            hole.smart.unwrap().placement(tileset),
            Some(SmartPlacement::BrokenWall(SmartBrokenWall::Hole))
        );
        assert_eq!(placement("ROOF4", 10), Some(SmartPlacement::Roof(3)));
        // Tiles of other palettes are no values
        assert_eq!(placement("FLOOR1", 1), None);
    }

    #[test]
    fn smart_places_by_the_piece_and_erases_with_the_brush() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let right = |fixture: &mut Fixture, mode: &BuildingMode, x, y| {
            let area = Brush::default().region(None, GridPosition::new(x, y));
            let mut context = fixture.context();
            mode.apply(&mut context, &area, MouseButton::Right);
            context.finish();
        };
        let mut smart = mode(r#"{"mode":"buildings","tool":"smart","brush":{"kind":"area"}}"#);
        assert_eq!(
            smart.brush(&fixture.tileset, EditorSmoothing::Normal),
            Brush::Square { size: 1 }
        );
        smart.smart = Some(SmartValue::Tile {
            tile_id: id(&fixture, "DOOR1", 1),
        });
        apply(&mut fixture, &smart, 52, 51);
        assert_eq!(structs(&fixture, 52, 51), [("DOOR1".to_owned(), 1)]);
        // The right button does nothing
        right(&mut fixture, &smart, 52, 51);
        assert_eq!(structs(&fixture, 52, 51), [("DOOR1".to_owned(), 1)]);
        assert!(!BuildingTool::Smart.uses_button(MouseButton::Right));
        assert!(!BuildingTool::Room.uses_button(MouseButton::Right));
        assert!(BuildingTool::Caves.uses_button(MouseButton::Right));

        // Erasing takes the brush, restores the door and removes the plain walls
        smart.erase = true;
        smart.brush = Brush::Square { size: 1 };
        apply(&mut fixture, &smart, 52, 51);
        assert_eq!(walls(&fixture, 52, 51).len(), 1);
        assert_eq!(structs(&fixture, 52, 51)[0].0, "WALL1");
        apply(&mut fixture, &smart, 52, 51);
        assert!(structs(&fixture, 52, 51).is_empty());
    }

    #[test]
    fn room_numbers_take_any_brush() {
        let mut fixture = building_fixture();
        let mut rooms = mode(
            r#"{"mode":"buildings","tool":"roomNumber","roomNumber":7,
                "brush":{"kind":"square","size":3}}"#,
        );
        apply(&mut fixture, &rooms, 50, 50);
        let room = |fixture: &Fixture, x, y| fixture.map.tiles[grid(x, y)].room.map(|r| r.number());
        assert_eq!(room(&fixture, 49, 51), Some(7));
        assert_eq!(room(&fixture, 52, 50), None);
        rooms.brush = Brush::Square { size: 1 };
        rooms.erase = true;
        apply(&mut fixture, &rooms, 50, 50);
        assert_eq!(room(&fixture, 50, 50), None);
        assert_eq!(room(&fixture, 51, 51), Some(7));
        // Room 0 is no room
        rooms.erase = false;
        rooms.room_number = 0;
        apply(&mut fixture, &rooms, 51, 51);
        assert_eq!(room(&fixture, 51, 51), None);
    }

    #[test]
    fn caves_are_filled_and_carved() {
        let mut fixture = Fixture::new();
        let caves =
            mode(r#"{"mode":"buildings","tool":"caves","brush":{"kind":"square","size":5}}"#);
        assert_eq!(
            caves.brush(&fixture.tileset, EditorSmoothing::Caves),
            Brush::Square { size: 5 }
        );
        apply(&mut fixture, &caves, 50, 50);
        assert!(!structs(&fixture, 50, 50).is_empty());
        let area = Brush::Square { size: 3 }.region(None, GridPosition::new(50, 50));
        let mut context = fixture.context();
        caves.apply(&mut context, &area, MouseButton::Right);
        context.finish();
        assert!(structs(&fixture, 50, 50).is_empty());
        assert!(!structs(&fixture, 48, 48).is_empty());
    }

    #[test]
    fn erasing_rooms_saws_them() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        // Furniture is placed with the objects of the terrain mode
        let furniture = fixture
            .tileset
            .tile(id(&fixture, "ISTRUCT1", 1))
            .unwrap()
            .clone();
        let mut context = fixture.context();
        context.add_struct_to_head(grid(52, 51), furniture);
        context.finish();
        assert!(structs(&fixture, 52, 51).contains(&("ISTRUCT1".to_owned(), 1)));
        let erase =
            mode(r#"{"mode":"buildings","tool":"room","erase":true,"brush":{"kind":"area"}}"#);
        let brush = erase.brush(&fixture.tileset, EditorSmoothing::Normal);
        let area = brush.region(Some(GridPosition::new(52, 50)), GridPosition::new(52, 52));
        let mut context = fixture.context();
        erase.apply(&mut context, &area, MouseButton::Left);
        context.finish();
        for y in 50..=52 {
            let tile = &fixture.map.tiles[grid(52, y)];
            assert!(!tile.land.iter().any(|tile| tile.is_floor()), "{y}");
            assert!(tile.roofs.is_empty(), "{y}");
            // The new right wall of the rest
            assert!(
                walls(&fixture, 51, y)
                    .iter()
                    .any(|&(_, horizontal)| !horizontal),
                "{y}"
            );
        }
        assert!(structs(&fixture, 52, 51).is_empty());
    }
}
