//! Rooms and roofs of buildings (`NewSmooth.cc` and `EditorBuildings.cc`)
//!
//! A building is made of floors, the walls around them and roofs above them. Adding a room
//! extends the building it touches, erasing a room saws it out of the building and closes it with
//! new walls.
//! The roof pieces follow the walls or floors around them.

use std::sync::Arc;

use crate::{
    components::map::GridPosition,
    map::{EditorSmoothing, GridNo},
    render::movement_costs::grid_no_on_visible_world_tile,
    tileset::{BuildingPiece, RoofPiece, SlantPiece, SlantSide, Tile, WallPiece},
};

use super::{
    brush::BrushArea,
    document::EditContext,
    placement::SelectionList,
    walls::{
        at, build_wall_piece, erase_horizontal_wall, erase_vertical_wall, erase_walls, has_floor,
        horizontal_wall, is_basement, is_door, is_wall, search_for_roof_type, search_for_wall_type,
        vertical_wall, WallSide,
    },
};

/// Number of floor variants a room picks from (`FLOOR_VARIANTS`)
const FLOOR_VARIANTS: u16 = 8;

/// The walls, floors and roofs a new room is built of, picked from the selection of the room
/// palette by their counts (`GetRandomIndexByRange`)
#[derive(Clone, Debug, Default)]
pub struct Materials(SelectionList);

impl Materials {
    pub fn new(list: SelectionList) -> Self {
        Self(list)
    }

    fn pick(&self, context: &mut EditContext, piece: impl Fn(BuildingPiece) -> bool) -> Option<u8> {
        let tile = self.0.pick_where(&mut context.document.random, |tile| {
            tile.building.is_some_and(|b| piece(b.piece))
        })?;
        tile.building.map(|b| b.style)
    }

    /// The style of a wall type of the selection
    pub fn wall(&self, context: &mut EditContext) -> Option<u8> {
        self.pick(context, BuildingPiece::is_wall)
    }

    pub fn floor(&self, context: &mut EditContext) -> Option<u8> {
        self.pick(context, |piece| piece == BuildingPiece::Floor)
    }

    /// The style of a flat roof type of the selection
    pub fn roof(&self, context: &mut EditContext) -> Option<u8> {
        self.pick(context, BuildingPiece::is_flat_roof)
    }

    pub fn slant_roof(&self, context: &mut EditContext) -> Option<u8> {
        self.pick(context, |piece| {
            matches!(piece, BuildingPiece::SlantRoof(_))
        })
    }

    /// Whether new rooms get a slant roof: the selection has a slant roof, but no flat roof
    /// (`CheckForSlantRoofs`)
    pub fn has_slant_roof_only(&self) -> bool {
        let has = |piece: fn(BuildingPiece) -> bool| {
            self.0
                .contains(|tile| tile.building.is_some_and(|b| piece(b.piece)))
        };
        has(|piece| matches!(piece, BuildingPiece::SlantRoof(_)))
            && !has(BuildingPiece::is_flat_roof)
    }
}

/// The rectangle of tiles an area tool works on (`gSelectRegion`), the maximum is included
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl TileRect {
    /// The rectangle around the tiles of a brush, `None` without tiles
    pub fn around(area: &BrushArea) -> Option<Self> {
        let positions = || area.tiles().map(GridNo::position);
        Some(Self {
            left: i32::from(positions().map(|p| p.x).min()?),
            top: i32::from(positions().map(|p| p.y).min()?),
            right: i32::from(positions().map(|p| p.x).max()?),
            bottom: i32::from(positions().map(|p| p.y).max()?),
        })
    }

    /// The tiles row by row, like the original loops over them, without those off the map
    pub fn tiles(self) -> impl Iterator<Item = (i32, i32, GridNo)> {
        (self.top..=self.bottom).flat_map(move |y| {
            (self.left..=self.right).filter_map(move |x| {
                GridPosition::from_xy(x, y).map(|position| (x, y, GridNo::from(position)))
            })
        })
    }

    /// The rectangle one tile larger on every side
    pub fn grown(self) -> Self {
        Self {
            left: self.left - 1,
            top: self.top - 1,
            right: self.right + 1,
            bottom: self.bottom + 1,
        }
    }

    /// The tiles one tile outside of the rectangle, row by row
    pub fn ring(self) -> impl Iterator<Item = GridNo> {
        let grown = self.grown();
        grown
            .tiles()
            .filter(move |&(x, y, _)| {
                x == grown.left || x == grown.right || y == grown.top || y == grown.bottom
            })
            .map(|(_, _, grid_no)| grid_no)
    }
}

/// A random tile of a building piece of a style
fn pick_piece(context: &mut EditContext, style: u8, piece: BuildingPiece) -> Option<Arc<Tile>> {
    let tiles = context.tileset.building_tiles(style, piece);
    if tiles.is_empty() {
        return None;
    }
    let index = context.random(tiles.len() as u32) as usize;
    Some(tiles[index].clone())
}

/// Adds a random floor variant of a style to the head of the land
pub fn add_floor(context: &mut EditContext, grid_no: GridNo, style: u8) {
    let variants: Vec<Arc<Tile>> = context
        .tileset
        .building_tiles(style, BuildingPiece::Floor)
        .iter()
        .filter(|tile| tile.sub_index <= FLOOR_VARIANTS)
        .cloned()
        .collect();
    if variants.is_empty() {
        return;
    }
    let index = context.random(variants.len() as u32) as usize;
    context.add_land_to_head(grid_no, variants[index].clone());
}

/// Whether a tile is a flat or slant roof (`FIRSTROOF` to `LASTSLANTROOF`)
pub fn is_roof(tile: &Tile) -> bool {
    tile.building.is_some_and(|b| b.piece.is_roof())
}

/// Whether a tile is a flat roof (`FIRSTROOF` to `LASTROOF`)
pub fn is_flat_roof(tile: &Tile) -> bool {
    tile.building.is_some_and(|b| b.piece.is_flat_roof())
}

/// Removes the roofs, the tiles on them and the roof shadows of a tile (`EraseRoof`)
pub fn erase_roof(context: &mut EditContext, grid_no: GridNo) {
    context.remove_roofs_where(grid_no, |_| true);
    context.remove_on_roofs_where(grid_no, |_| true);
    context.remove_shadows_where(grid_no, is_roof);
}

/// Removes the floors of a tile (`EraseFloor`)
pub fn erase_floor(context: &mut EditContext, grid_no: GridNo) {
    context.remove_land_where(grid_no, Tile::is_floor);
}

/// Removes everything of a building from a tile (`EraseBuilding`)
pub fn erase_building(context: &mut EditContext, grid_no: GridNo) {
    erase_roof(context, grid_no);
    erase_floor(context, grid_no);
    erase_walls(context, grid_no);
    context.set_room(grid_no, None);
}

/// Removes the trees, rocks, structures, their shadows and the debris of a tile
/// (`BulldozeNature`)
pub fn bulldoze_nature(context: &mut EditContext, grid_no: GridNo) {
    let bulldozed = |tile: &Tile| tile.bulldozed;
    context.remove_structs_where(grid_no, bulldozed);
    context.remove_shadows_where(grid_no, bulldozed);
    context.remove_objects_where(grid_no, bulldozed);
}

/// The roof piece of a tile, by the sides of the building it is on: the top and left sides win,
/// a tile at no side is a center piece
fn roof_piece(top: bool, left: bool, bottom: bool, right: bool) -> RoofPiece {
    match (top, left, bottom, right) {
        (true, true, _, _) => RoofPiece::TopLeft,
        (true, _, _, true) => RoofPiece::TopRight,
        (_, true, true, _) => RoofPiece::BottomLeft,
        (_, _, true, true) => RoofPiece::BottomRight,
        (true, _, _, _) => RoofPiece::Top,
        (_, _, true, _) => RoofPiece::Bottom,
        (_, true, _, _) => RoofPiece::Left,
        (_, _, _, true) => RoofPiece::Right,
        _ => RoofPiece::Center,
    }
}

/// Replaces the roof of a tile by a piece of a style
fn set_roof(context: &mut EditContext, grid_no: GridNo, style: u8, piece: RoofPiece) {
    erase_roof(context, grid_no);
    if let Some(tile) = pick_piece(context, style, BuildingPiece::Roof(Some(piece))) {
        context.add_roof_to_head(grid_no, tile);
    }
}

/// Replaces the roof of a tile by the piece of a style that fits the walls around it
/// (`RebuildRoof`), nothing happens without a style
pub fn rebuild_roof(context: &mut EditContext, grid_no: GridNo, style: Option<u8>) {
    let Some(style) = style else {
        return;
    };
    let top = horizontal_wall(context, at(grid_no, 0, -1)).is_some();
    let left = vertical_wall(context, at(grid_no, -1, 0)).is_some();
    let bottom = context.horizontal_wall(grid_no).is_some();
    let right = context.vertical_wall(grid_no).is_some();
    set_roof(
        context,
        grid_no,
        style,
        roof_piece(top, left, bottom, right),
    );
}

/// Replaces the roof of a tile by the piece of a style that fits the floors around it
/// (`RebuildRoofUsingFloorInfo`)
pub fn rebuild_roof_using_floor_info(context: &mut EditContext, grid_no: GridNo, style: u8) {
    let edge = |dx, dy| !has_floor(context, at(grid_no, dx, dy));
    let (top, left, bottom, right) = (edge(0, -1), edge(-1, 0), edge(0, 1), edge(1, 0));
    set_roof(
        context,
        grid_no,
        style,
        roof_piece(top, left, bottom, right),
    );
}

/// Whether the edge of a floor tile that a wall piece goes to has a wall
fn has_wall(context: &EditContext, grid_no: GridNo, side: WallSide) -> bool {
    match side {
        WallSide::ExteriorTop | WallSide::InteriorTop => {
            horizontal_wall(context, at(grid_no, 0, -1)).is_some()
        }
        WallSide::ExteriorBottom | WallSide::InteriorBottom => {
            context.horizontal_wall(grid_no).is_some()
        }
        WallSide::ExteriorLeft | WallSide::InteriorLeft => {
            vertical_wall(context, at(grid_no, -1, 0)).is_some()
        }
        WallSide::ExteriorRight | WallSide::InteriorRight => {
            context.vertical_wall(grid_no).is_some()
        }
    }
}

/// The style of the floor of a tile
fn floor_style(context: &EditContext, grid_no: GridNo) -> Option<u8> {
    context
        .land(grid_no)
        .iter()
        .find_map(|tile| tile.building.filter(|b| b.piece == BuildingPiece::Floor))
        .map(|b| b.style)
}

/// Adds a room to the world, or extends the building it touches (`AddBuildingSectionToWorld`)
///
/// A new building bulldozes the nature of the area and takes its walls, floor and roof from the
/// materials, an extended one those of the building. The walls between the building and the new
/// part are removed. Without walls in the materials, only the floors are replaced. The original
/// never ends if the head of the land of a floor tile is not a floor, here the floor is looked
/// up in all of the land.
pub fn add_building_section(context: &mut EditContext, rect: TileRect, materials: &Materials) {
    let basement = is_basement(context);
    let tiles: Vec<(i32, i32, GridNo)> = rect.tiles().collect();
    let mut wall = materials.wall(context);
    let mut floor = materials.floor(context);
    if wall.is_none() {
        // Only floors, e.g. to give a kitchen another floor
        if let Some(floor) = floor {
            for &(_, _, grid_no) in &tiles {
                erase_floor(context, grid_no);
                add_floor(context, grid_no, floor);
            }
        }
        return;
    }

    // A floor in the area means that an existing building is extended
    let mut new_building = true;
    let mut roof = None;
    for &(_, _, grid_no) in &tiles {
        if !context.has_floor(grid_no) {
            continue;
        }
        new_building = false;
        floor = floor_style(context, grid_no);
        wall = search_for_wall_type(context, grid_no, materials);
        roof = search_for_roof_type(context, grid_no);
        if wall.is_some() && roof.is_some() && floor.is_some() {
            break;
        }
    }

    let mut slant_roof = None;
    if new_building {
        wall = materials.wall(context);
        floor = materials.floor(context);
        roof = materials.roof(context);
        if roof.is_none() {
            if let Some(slant) = materials.slant_roof(context) {
                if basement {
                    roof = Some(0);
                } else {
                    slant_roof = Some(slant);
                }
            }
        }
    }
    // The building's walls weren't found, use the materials
    let Some(wall) = wall.or_else(|| materials.wall(context)) else {
        return;
    };

    // Remove the walls between the building and the new part. The floor and roof of the tiles
    // whose walls are removed are rebuilt below. Outside of buildings the nature goes.
    for &(_, _, grid_no) in &tiles {
        if basement {
            erase_building(context, grid_no);
        } else if !new_building && context.has_floor(grid_no) {
            let open = |context: &EditContext, dx, dy| !has_floor(context, at(grid_no, dx, dy));
            let sides = [
                (open(context, 0, -1), at(grid_no, 0, -1), true),
                (open(context, 0, 1), Some(grid_no), true),
                (open(context, -1, 0), at(grid_no, -1, 0), false),
                (open(context, 1, 0), Some(grid_no), false),
            ];
            for (open, wall_tile, horizontal) in sides {
                if !open {
                    continue;
                }
                if horizontal {
                    erase_horizontal_wall(context, wall_tile);
                } else {
                    erase_vertical_wall(context, wall_tile);
                }
                erase_floor(context, grid_no);
                erase_roof(context, grid_no);
            }
        } else {
            bulldoze_nature(context, grid_no);
        }
    }

    // The walls around the tiles without floors, seen from inside. Top and bottom first.
    for &(x, y, grid_no) in &tiles {
        if context.has_floor(grid_no) {
            continue;
        }
        let sides = [
            (y == rect.top, (0, -1), WallSide::InteriorTop),
            (y == rect.bottom, (0, 1), WallSide::InteriorBottom),
            (x == rect.left, (-1, 0), WallSide::InteriorLeft),
            (x == rect.right, (1, 0), WallSide::InteriorRight),
        ];
        for (edge, (dx, dy), side) in sides {
            if edge
                && !has_wall(context, grid_no, side)
                && basement == has_floor(context, at(grid_no, dx, dy))
            {
                build_wall_piece(context, grid_no, side, wall);
            }
        }
    }

    if let Some(slant) = slant_roof {
        // Slant roofs are always 8 tiles wide or high
        let vertical = rect.bottom - rect.top != 7;
        build_slant_roof(context, rect, wall, slant, vertical);
    }

    // The tiles without floors get their roofs and floors
    for &(_, _, grid_no) in &tiles {
        if context.has_floor(grid_no) {
            continue;
        }
        if slant_roof.is_none() {
            rebuild_roof(context, grid_no, roof);
        }
        if let (Some(floor), false) = (floor, basement) {
            add_floor(context, grid_no, floor);
        }
    }
}

/// Places the slant roof of a room 8 tiles wide or high, with the A-frame walls at the side the
/// viewer looks at (`BuildSlantRoof`)
///
/// A vertical roof runs from the bottom to the top of the room, its pieces are 3, 4, 7 and 8
/// columns right of its left side. A horizontal one runs from the right to the left, 3, 4, 7 and
/// 8 rows below its top.
fn build_slant_roof(context: &mut EditContext, rect: TileRect, wall: u8, roof: u8, vertical: bool) {
    let tileset = context.tileset;
    let aframe = |vertical, left| {
        let piece = BuildingPiece::Wall(WallPiece::AFrame { vertical, left });
        tileset.building_tiles(wall, piece).first().cloned()
    };
    let slant = |thick, side| {
        let piece = BuildingPiece::SlantRoof(Some(SlantPiece { thick, side }));
        tileset.building_tiles(roof, piece).first().cloned()
    };
    let mut add = |x: i32, y: i32, tile: Option<Arc<Tile>>| {
        if let (Some(position), Some(tile)) = (GridPosition::from_xy(x, y), tile) {
            context.add_roof_to_head(GridNo::from(position), tile);
        }
    };
    if vertical {
        let (x, bottom) = (rect.left, rect.bottom);
        // The closest pieces: the A-frames and the extended roof pieces
        add(x + 4, bottom, aframe(true, true));
        add(x + 8, bottom, aframe(true, false));
        for y in (rect.top..=bottom).rev() {
            let thick = y == bottom || y == rect.top;
            add(x + 3, y, slant(thick, SlantSide::Left));
            add(x + 7, y, slant(thick, SlantSide::Right));
        }
    } else {
        let (right, y) = (rect.right, rect.top);
        add(right, y + 4, aframe(false, true));
        add(right, y + 8, aframe(false, false));
        for x in (rect.left..=right).rev() {
            let thick = x == right || x == rect.left;
            add(x, y + 3, slant(thick, SlantSide::Top));
            add(x, y + 7, slant(thick, SlantSide::Bottom));
        }
    }
}

/// Removes what a floor tile owns: its building and the walls on its top and left edges, which
/// are stored on the tiles above and left of it (`EraseFloorOwnedBuildingPieces`)
pub fn erase_floor_owned_pieces(context: &mut EditContext, grid_no: GridNo) {
    if !is_basement(context) && !context.has_floor(grid_no) {
        return;
    }
    erase_building(context, grid_no);
    let owned = [(at(grid_no, -1, 0), true), (at(grid_no, 0, -1), false)];
    for (neighbour, vertical) in owned {
        let Some(neighbour) = neighbour else {
            continue;
        };
        let index = context.structs(neighbour).iter().position(|tile| {
            let orientation = tile.wall_orientation();
            (is_wall(tile) || is_door(tile))
                && if vertical {
                    orientation.is_vertical()
                } else {
                    orientation.is_horizontal()
                }
        });
        if let Some(index) = index {
            context.remove_struct(neighbour, index);
            context.remove_shadows_where(neighbour, is_wall);
        }
    }
}

/// Saws a room out of the buildings (`RemoveBuildingSectionFromWorld`)
///
/// The area loses what its floor tiles own, the buildings around it get new walls where they
/// border it and their roofs are rebuilt along it. In basements the area is filled with rock.
pub fn remove_building_section(context: &mut EditContext, rect: TileRect, materials: &Materials) {
    let basement = is_basement(context);
    let tiles: Vec<(i32, i32, GridNo)> = rect.tiles().collect();
    for &(_, _, grid_no) in &tiles {
        erase_floor_owned_pieces(context, grid_no);
    }

    // New walls wherever the neighbours keep their floors. Top and bottom first.
    for &(x, y, grid_no) in &tiles {
        let sides = [
            (y == rect.top, (0, -1), WallSide::ExteriorTop),
            (y == rect.bottom, (0, 1), WallSide::ExteriorBottom),
            (x == rect.left, (-1, 0), WallSide::ExteriorLeft),
            (x == rect.right, (1, 0), WallSide::ExteriorRight),
        ];
        for (edge, (dx, dy), side) in sides {
            if !edge {
                continue;
            }
            let floor = has_floor(context, at(grid_no, dx, dy));
            if (basement && !floor) || (!basement && floor && !has_wall(context, grid_no, side)) {
                if let Some(style) = search_for_wall_type(context, grid_no, materials) {
                    build_wall_piece(context, grid_no, side, style);
                }
            }
        }
    }

    if basement {
        let floor = materials.floor(context).unwrap_or(0);
        for &(_, _, grid_no) in &tiles {
            add_floor(context, grid_no, floor);
        }
    }
    for grid_no in rect.ring() {
        if basement != context.has_floor(grid_no) {
            let style = search_for_roof_type(context, grid_no);
            rebuild_roof(context, grid_no, style);
        }
    }
}

/// The neighbours of a tile the flood fills go to, on the visible part of the map
fn fill_neighbours(context: &EditContext, grid_no: GridNo) -> Vec<GridNo> {
    let restricted_scroll = context.world.restricted_scroll;
    [(0, -1), (0, 1), (1, 0), (-1, 0)]
        .into_iter()
        .filter_map(|(dx, dy)| at(grid_no, dx, dy))
        .filter(|&g| grid_no_on_visible_world_tile(g, restricted_scroll))
        .collect()
}

/// Removes the building at a tile with its room numbers, going from tile to tile while there
/// are floors or roofs (`KillBuilding`)
///
/// In basements, the rock is restored.
pub fn kill_building(context: &mut EditContext, grid_no: GridNo) {
    let basement = is_basement(context);
    // The original recurses and rebuilds the roofs of basements after the neighbours
    enum Step {
        Visit(GridNo),
        Finish(GridNo),
    }
    let mut stack = vec![Step::Visit(grid_no)];
    while let Some(step) = stack.pop() {
        let grid_no = match step {
            Step::Visit(grid_no) => grid_no,
            Step::Finish(grid_no) => {
                let style = search_for_roof_type(context, grid_no);
                rebuild_roof(context, grid_no, style);
                continue;
            }
        };
        let mut found = false;
        if !basement {
            found |= context.remove_roofs_where(grid_no, |_| true);
        }
        found |= context.remove_land_where(grid_no, Tile::is_floor);
        erase_building(context, grid_no);
        if basement {
            stack.push(Step::Finish(grid_no));
        }
        if !found {
            continue;
        }
        // Visited in the order of the original: up, down, right, left
        for neighbour in fill_neighbours(context, grid_no).into_iter().rev() {
            stack.push(Step::Visit(neighbour));
        }
    }
}

/// Gives every tile with a room a center piece of a flat roof (the F7 key of the original in
/// basements), so that the rooms carved into the rock have a ceiling
pub fn roof_all_rooms(context: &mut EditContext, style: u8) {
    for grid_no in GridNo::all() {
        if context.map.tiles[grid_no].room.is_some() {
            set_roof(context, grid_no, style, RoofPiece::Center);
        }
    }
}

/// Replaces the roof of the building at a tile by a new style, going from floor to floor
/// (`ReplaceBuildingWithNewRoof`)
///
/// Only on normal maps and on floors.
pub fn replace_with_new_roof(context: &mut EditContext, grid_no: GridNo, style: u8) {
    if context.map.info.editor_smoothing != EditorSmoothing::Normal || !context.has_floor(grid_no) {
        return;
    }
    let mut visited = std::collections::HashSet::from([grid_no]);
    let mut stack = vec![grid_no];
    while let Some(grid_no) = stack.pop() {
        rebuild_roof_using_floor_info(context, grid_no, style);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let Some(neighbour) = at(grid_no, dx, dy) else {
                continue;
            };
            if context.has_floor(neighbour) && visited.insert(neighbour) {
                stack.push(neighbour);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        map::RoomNo,
        tileset::{tests::building_tileset, TileIndex, WallPiece},
    };

    /// The fixture with the building tileset
    pub(crate) fn building_fixture() -> Fixture {
        Fixture::with_tileset(building_tileset())
    }

    /// Materials of the given tiles, by type name and sub index
    pub(crate) fn materials(fixture: &Fixture, tiles: &[(&str, u16)]) -> Materials {
        let ids: Vec<(TileIndex, u32)> = tiles
            .iter()
            .map(|&(name, sub_index)| (fixture.tileset.tile_of(name, sub_index).unwrap(), 1))
            .collect();
        Materials::new(SelectionList::new(&fixture.tileset, ids))
    }

    pub(crate) fn rect(left: i32, top: i32, right: i32, bottom: i32) -> TileRect {
        TileRect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Adds a room of `WALL1`, `FLOOR1` and `ROOF1`
    pub(crate) fn add_room(fixture: &mut Fixture, rect: TileRect) {
        let materials = materials(fixture, &[("WALL1", 1), ("FLOOR1", 1), ("ROOF1", 1)]);
        let mut context = fixture.context();
        add_building_section(&mut context, rect, &materials);
        context.finish();
    }

    /// The wall pieces of the structs of a tile, with whether they are horizontal
    pub(crate) fn walls(fixture: &Fixture, x: u8, y: u8) -> Vec<(WallPiece, bool)> {
        fixture.map.tiles[grid(x, y)]
            .structs
            .iter()
            .filter_map(|tile| match tile.building?.piece {
                BuildingPiece::Wall(piece) => {
                    Some((piece, tile.wall_orientation().is_horizontal()))
                }
                _ => None,
            })
            .collect()
    }

    fn roof(fixture: &Fixture, x: u8, y: u8) -> Vec<(u8, BuildingPiece)> {
        fixture.map.tiles[grid(x, y)]
            .roofs
            .iter()
            .filter_map(|tile| tile.building.map(|b| (b.style, b.piece)))
            .collect()
    }

    fn shadows(fixture: &Fixture, x: u8, y: u8) -> Vec<BuildingPiece> {
        fixture.map.tiles[grid(x, y)]
            .shadows
            .iter()
            .filter_map(|tile| tile.building.map(|b| b.piece))
            .collect()
    }

    fn has_floor(fixture: &Fixture, x: u8, y: u8) -> bool {
        fixture.map.tiles[grid(x, y)]
            .land
            .iter()
            .any(|tile| tile.is_floor())
    }

    /// Whether nothing of a building is on a tile
    fn is_empty(fixture: &Fixture, x: u8, y: u8) -> bool {
        let tile = &fixture.map.tiles[grid(x, y)];
        tile.structs.is_empty()
            && tile.roofs.is_empty()
            && tile.shadows.is_empty()
            && !has_floor(fixture, x, y)
            && tile.room.is_none()
    }

    use WallPiece::*;

    #[test]
    fn a_new_room_gets_walls_roofs_and_floors() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        for y in 50..=52 {
            for x in 50..=52 {
                assert!(has_floor(&fixture, x, y), "{x} {y}");
            }
        }
        // The top walls are stored on the row above, the left walls on the column left
        for x in 50..=52 {
            assert_eq!(walls(&fixture, x, 49), [(InteriorLeft, true)]);
        }
        assert_eq!(walls(&fixture, 49, 50), [(InteriorExtended, false)]);
        assert_eq!(walls(&fixture, 49, 51), [(InteriorRight, false)]);
        assert_eq!(walls(&fixture, 49, 52), [(InteriorRight, false)]);
        assert_eq!(walls(&fixture, 52, 50), [(ExteriorRight, false)]);
        assert_eq!(walls(&fixture, 52, 51), [(ExteriorRight, false)]);
        assert_eq!(walls(&fixture, 50, 52), [(ExteriorLeft, true)]);
        let mut corner = walls(&fixture, 52, 52);
        corner.sort_by_key(|&(_, horizontal)| horizontal);
        assert_eq!(corner, [(ExteriorBottomEnd, false), (ExteriorLeft, true)]);
        assert!(walls(&fixture, 51, 51).is_empty());
        // The shadows of the bottom walls and of the bottom left corner
        assert_eq!(
            shadows(&fixture, 51, 52),
            [BuildingPiece::Wall(ExteriorLeftShadow)]
        );
        assert_eq!(
            shadows(&fixture, 49, 52),
            [BuildingPiece::Wall(InteriorBottomEndShadow)]
        );

        let piece = |piece| (0, BuildingPiece::Roof(Some(piece)));
        assert_eq!(roof(&fixture, 50, 50), [piece(RoofPiece::TopLeft)]);
        assert_eq!(roof(&fixture, 51, 50), [piece(RoofPiece::Top)]);
        assert_eq!(roof(&fixture, 52, 50), [piece(RoofPiece::TopRight)]);
        assert_eq!(roof(&fixture, 50, 51), [piece(RoofPiece::Left)]);
        assert_eq!(roof(&fixture, 51, 51), [piece(RoofPiece::Center)]);
        assert_eq!(roof(&fixture, 52, 51), [piece(RoofPiece::Right)]);
        assert_eq!(roof(&fixture, 50, 52), [piece(RoofPiece::BottomLeft)]);
        assert_eq!(roof(&fixture, 51, 52), [piece(RoofPiece::Bottom)]);
        assert_eq!(roof(&fixture, 52, 52), [piece(RoofPiece::BottomRight)]);
        assert!(roof(&fixture, 53, 51).is_empty());

        // The world has the structures of the walls
        let world = &fixture.world.tiles[grid(52, 52)];
        assert_eq!(world.structures.len(), 2);
    }

    #[test]
    fn new_rooms_bulldoze_nature() {
        let mut fixture = building_fixture();
        let mut context = fixture.context();
        let tree = context.tile("OFSTRUCT1", 1).unwrap();
        let rock = context.tile("ROCKS", 1).unwrap();
        let road = context.tile("ROADPIECES", 1).unwrap();
        context.add_struct_to_head(grid(51, 51), tree);
        context.add_object_to_head(grid(51, 51), rock);
        context.add_object_to_head(grid(51, 51), road);
        context.finish();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let tile = &fixture.map.tiles[grid(51, 51)];
        assert!(tile.structs.is_empty());
        let objects: Vec<&str> = tile.objects.iter().map(|t| t.type_name.as_str()).collect();
        assert_eq!(objects, ["ROADPIECES"]);
    }

    #[test]
    fn only_floors_replace_the_floors() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let materials = materials(&fixture, &[("FLOOR3", 1)]);
        let mut context = fixture.context();
        add_building_section(&mut context, rect(51, 51, 51, 51), &materials);
        context.finish();
        let floors: Vec<&str> = fixture.map.tiles[grid(51, 51)]
            .land
            .iter()
            .filter(|t| t.is_floor())
            .map(|t| t.type_name.as_str())
            .collect();
        assert_eq!(floors, ["FLOOR3"]);
        // The walls and roofs stay
        assert_eq!(walls(&fixture, 51, 49), [(InteriorLeft, true)]);
        assert_eq!(roof(&fixture, 51, 51).len(), 1);
    }

    #[test]
    fn extending_a_room_removes_the_shared_wall() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        // Overlaps the right column of the room, the style comes from the building
        let other = materials(&fixture, &[("WALL2", 1), ("FLOOR2", 1), ("ROOF2", 1)]);
        let mut context = fixture.context();
        add_building_section(&mut context, rect(52, 50, 55, 52), &other);
        context.finish();
        for y in 50..=52 {
            for x in 50..=55 {
                assert!(has_floor(&fixture, x, y), "{x} {y}");
            }
            // No walls inside of the building
            for x in 50..=54 {
                assert!(
                    walls(&fixture, x, y)
                        .iter()
                        .all(|&(_, horizontal)| horizontal),
                    "{x} {y}: {:?}",
                    walls(&fixture, x, y)
                );
            }
            assert!(walls(&fixture, 55, y)
                .iter()
                .any(|&(_, horizontal)| !horizontal));
        }
        for x in 50..=55 {
            assert_eq!(walls(&fixture, x, 49), [(InteriorLeft, true)], "{x}");
        }
        // The new part has the materials of the building
        let styles: Vec<u8> = fixture.map.tiles[grid(55, 50)]
            .structs
            .iter()
            .filter_map(|t| t.building.map(|b| b.style))
            .collect();
        assert_eq!(styles, [0]);
        assert_eq!(
            roof(&fixture, 55, 50),
            [(0, BuildingPiece::Roof(Some(RoofPiece::TopRight)))]
        );
        assert_eq!(
            roof(&fixture, 52, 51),
            [(0, BuildingPiece::Roof(Some(RoofPiece::Center)))]
        );
    }

    #[test]
    fn sawing_a_room_closes_the_building() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let materials = materials(&fixture, &[]);
        let mut context = fixture.context();
        context.set_room(grid(52, 51), RoomNo::new(3));
        remove_building_section(&mut context, rect(52, 50, 52, 52), &materials);
        context.finish();
        for y in 50..=52 {
            assert!(!has_floor(&fixture, 52, y));
            assert!(fixture.map.tiles[grid(52, y)].roofs.is_empty());
            // The new right wall of the rest
            assert!(
                walls(&fixture, 51, y)
                    .iter()
                    .any(|&(_, horizontal)| !horizontal),
                "{y}"
            );
        }
        assert_eq!(fixture.map.tiles[grid(52, 51)].room, None);
        assert!(walls(&fixture, 52, 49).is_empty());
        // The roof along the cut follows the new walls
        assert_eq!(
            roof(&fixture, 51, 51),
            [(0, BuildingPiece::Roof(Some(RoofPiece::Right)))]
        );

        // Sawing the rest removes everything
        let mut context = fixture.context();
        remove_building_section(&mut context, rect(50, 50, 51, 52), &materials);
        context.finish();
        for y in 49..=53 {
            for x in 49..=53 {
                assert!(is_empty(&fixture, x, y), "{x} {y}");
            }
        }
    }

    #[test]
    fn killing_a_building_restores_the_ground() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        add_room(&mut fixture, rect(60, 50, 61, 51));
        let mut context = fixture.context();
        context.set_room(grid(51, 51), RoomNo::new(1));
        kill_building(&mut context, grid(51, 51));
        context.finish();
        for y in 49..=53 {
            for x in 49..=53 {
                assert!(is_empty(&fixture, x, y), "{x} {y}");
                assert_eq!(fixture.land(grid(x, y)), [("TEXTURE1", 1)]);
            }
        }
        // Other buildings stay
        assert!(has_floor(&fixture, 60, 50));
    }

    #[test]
    fn new_roofs_follow_the_floors() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let mut context = fixture.context();
        replace_with_new_roof(&mut context, grid(51, 51), 2);
        // Nothing without a floor
        replace_with_new_roof(&mut context, grid(60, 60), 2);
        context.finish();
        let piece = |piece| (2, BuildingPiece::Roof(Some(piece)));
        assert_eq!(roof(&fixture, 50, 50), [piece(RoofPiece::TopLeft)]);
        assert_eq!(roof(&fixture, 52, 51), [piece(RoofPiece::Right)]);
        assert_eq!(roof(&fixture, 51, 51), [piece(RoofPiece::Center)]);
        assert!(fixture.map.tiles[grid(60, 60)].roofs.is_empty());
    }

    /// The slant roof pieces of a tile, as `(thick, side)`, and the A-frames
    fn slant(fixture: &Fixture, x: u8, y: u8) -> Vec<BuildingPiece> {
        fixture.map.tiles[grid(x, y)]
            .roofs
            .iter()
            .filter_map(|tile| tile.building.map(|b| b.piece))
            .collect()
    }

    fn slant_piece(thick: bool, side: SlantSide) -> BuildingPiece {
        BuildingPiece::SlantRoof(Some(SlantPiece { thick, side }))
    }

    #[test]
    fn slant_roofs_run_along_the_room() {
        let mut fixture = building_fixture();
        let materials = materials(&fixture, &[("WALL1", 1), ("FLOOR1", 1), ("SROOF1", 1)]);
        // 8 rows high: the roof runs from the right to the left
        let mut context = fixture.context();
        add_building_section(&mut context, rect(50, 50, 55, 57), &materials);
        context.finish();
        assert_eq!(slant(&fixture, 55, 53), [slant_piece(true, SlantSide::Top)]);
        assert_eq!(
            slant(&fixture, 52, 53),
            [slant_piece(false, SlantSide::Top)]
        );
        assert_eq!(
            slant(&fixture, 50, 57),
            [slant_piece(true, SlantSide::Bottom)]
        );
        let aframe = |vertical, left| BuildingPiece::Wall(WallPiece::AFrame { vertical, left });
        assert_eq!(slant(&fixture, 55, 54), [aframe(false, true)]);
        assert_eq!(slant(&fixture, 55, 58), [aframe(false, false)]);
        // No flat roofs, but floors
        assert!(slant(&fixture, 52, 51).is_empty());
        assert!(has_floor(&fixture, 52, 51));
        assert_eq!(walls(&fixture, 52, 49), [(InteriorLeft, true)]);

        // 8 columns wide: the roof runs from the bottom to the top
        let mut context = fixture.context();
        add_building_section(&mut context, rect(70, 50, 77, 54), &materials);
        context.finish();
        assert_eq!(
            slant(&fixture, 73, 54),
            [slant_piece(true, SlantSide::Left)]
        );
        assert_eq!(
            slant(&fixture, 73, 52),
            [slant_piece(false, SlantSide::Left)]
        );
        assert_eq!(
            slant(&fixture, 77, 50),
            [slant_piece(true, SlantSide::Right)]
        );
        assert_eq!(slant(&fixture, 74, 54), [aframe(true, true)]);
        assert_eq!(slant(&fixture, 78, 54), [aframe(true, false)]);
    }

    #[test]
    fn basements_get_flat_roofs_for_slant_roofs() {
        let mut fixture = building_fixture();
        fixture.map.info.editor_smoothing = EditorSmoothing::Basement;
        let materials = materials(&fixture, &[("WALL1", 1), ("FLOOR1", 1), ("SROOF1", 1)]);
        let mut context = fixture.context();
        add_building_section(&mut context, rect(50, 50, 55, 57), &materials);
        context.finish();
        assert!(slant(&fixture, 55, 53)
            .iter()
            .all(|piece| piece.is_flat_roof()));
        assert!(!slant(&fixture, 55, 53).is_empty());
    }

    #[test]
    fn basement_rooms_are_carved_out_of_rock() {
        let mut fixture = building_fixture();
        fixture.map.info.editor_smoothing = EditorSmoothing::Basement;
        // Rock is floor and roof
        let mut context = fixture.context();
        for (_, _, grid_no) in rect(45, 45, 60, 60).tiles() {
            add_floor(&mut context, grid_no, 0);
            set_roof(&mut context, grid_no, 0, RoofPiece::Center);
        }
        context.finish();
        let materials = materials(&fixture, &[("WALL2", 1), ("FLOOR1", 1), ("ROOF1", 1)]);
        let mut context = fixture.context();
        add_building_section(&mut context, rect(50, 50, 52, 52), &materials);
        context.finish();
        // The room has no floors, and walls towards the rock without shadows
        assert!(!has_floor(&fixture, 51, 51));
        assert_eq!(walls(&fixture, 51, 49), [(InteriorLeft, true)]);
        assert_eq!(walls(&fixture, 52, 51), [(ExteriorRight, false)]);
        assert!(shadows(&fixture, 51, 52).is_empty());
        // The walls take the style of the materials
        assert_eq!(
            fixture.map.tiles[grid(51, 49)].structs[0].type_name,
            "WALL2"
        );
        assert_eq!(
            roof(&fixture, 51, 51),
            [(0, BuildingPiece::Roof(Some(RoofPiece::Center)))]
        );

        // Sawing refills the rock
        let mut context = fixture.context();
        remove_building_section(&mut context, rect(50, 50, 52, 52), &materials);
        context.finish();
        assert!(has_floor(&fixture, 51, 51));
        assert!(walls(&fixture, 51, 49).is_empty());
    }

    #[test]
    fn all_rooms_get_roofs() {
        let mut fixture = building_fixture();
        let mut context = fixture.context();
        context.set_room(grid(5, 5), RoomNo::new(2));
        let old = context.tile("ROOF1", 3).unwrap();
        context.add_roof_to_head(grid(5, 5), old);
        roof_all_rooms(&mut context, 3);
        context.finish();
        assert_eq!(
            roof(&fixture, 5, 5),
            [(3, BuildingPiece::Roof(Some(RoofPiece::Center)))]
        );
        assert!(roof(&fixture, 6, 5).is_empty());
    }

    #[test]
    fn slant_roofs_are_picked_without_flat_roofs() {
        let fixture = building_fixture();
        assert!(materials(&fixture, &[("WALL1", 1), ("SROOF1", 1)]).has_slant_roof_only());
        assert!(!materials(&fixture, &[("SROOF1", 1), ("ROOF1", 1)]).has_slant_roof_only());
        assert!(!materials(&fixture, &[("WALL1", 1)]).has_slant_roof_only());
    }

    #[test]
    fn rectangles_have_rings() {
        let rect = rect(0, 5, 1, 5);
        let tiles: Vec<(i32, i32)> = rect.tiles().map(|(x, y, _)| (x, y)).collect();
        assert_eq!(tiles, [(0, 5), (1, 5)]);
        // Off the map is left out
        assert_eq!(rect.ring().count(), 3 + 3 + 1);
        let area = crate::editor::brush::Brush::Area
            .region(Some(GridPosition::new(3, 9)), GridPosition::new(1, 7));
        assert_eq!(TileRect::around(&area), Some(self::rect(1, 7, 3, 9)));
    }
}
