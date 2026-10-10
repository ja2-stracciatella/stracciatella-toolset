//! Copying and moving buildings (`CopyBuilding`, `PasteBuilding` and `MoveBuilding` of
//! `EditorBuildings.cc`)
//!
//! The layout of a building is the tiles with its floors and roofs, the tiles east and south of
//! them, which hold the walls on those sides, and the tiles north and west of them with walls
//! towards the building. Pasting replaces the tiles at the target by the tiles of the layout.

use std::collections::HashSet;

use crate::{
    components::map::GridPosition,
    map::{GridNo, MapTile, TacticalMap},
    tileset::Tile,
};

use super::document::{wall_node, EditContext};

/// Whether a tile has a floor or a flat or slant roof (`BuildingAtGridNo`)
pub fn building_at(tile: &MapTile) -> bool {
    tile.land.iter().any(|t| t.is_floor())
        || tile
            .roofs
            .iter()
            .any(|t| t.building.is_some_and(|b| b.piece.is_roof()))
}

/// The tiles of the building at a tile, `None` if there is none (`CopyBuilding`)
pub fn building_layout(map: &TacticalMap, anchor: GridNo) -> Option<Vec<GridNo>> {
    if !building_at(&map.tiles[anchor]) {
        return None;
    }
    let vertical = |grid_no| {
        wall_node(&map.tiles[grid_no], |t: &Tile| {
            t.wall_orientation().is_vertical()
        })
    };
    let horizontal = |grid_no| {
        wall_node(&map.tiles[grid_no], |t: &Tile| {
            t.wall_orientation().is_horizontal()
        })
    };
    let mut layout = vec![anchor];
    let mut visited = HashSet::from([anchor]);
    let mut stack = vec![anchor];
    // `BuildLayout`
    while let Some(from) = stack.pop() {
        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
            let Some(to) = from.neighbour(dx, dy) else {
                continue;
            };
            if visited.contains(&to) {
                continue;
            }
            // Outside of the building, only the tiles with its walls belong to it
            let belongs = building_at(&map.tiles[to])
                || match (dx, dy) {
                    (1, 0) | (0, 1) => building_at(&map.tiles[from]),
                    (-1, 0) => vertical(to).is_some(),
                    _ => horizontal(to).is_some(),
                };
            if belongs {
                visited.insert(to);
                layout.push(to);
                stack.push(to);
            }
        }
    }
    Some(layout)
}

/// The tile `dx` columns and `dy` rows from a tile
fn shifted(grid_no: GridNo, (dx, dy): (i32, i32)) -> Option<GridNo> {
    let position = grid_no.position();
    GridPosition::from_xy(i32::from(position.x) + dx, i32::from(position.y) + dy).map(GridNo::from)
}

/// The columns and rows from one tile to another
pub fn drag_offset(from: GridNo, to: GridNo) -> (i32, i32) {
    let (from, to) = (from.position(), to.position());
    (
        i32::from(to.x) - i32::from(from.x),
        i32::from(to.y) - i32::from(from.y),
    )
}

/// The tiles of a layout moved by an offset, those that would leave the map are left out
pub fn shifted_layout(layout: &[GridNo], offset: (i32, i32)) -> Vec<GridNo> {
    layout
        .iter()
        .filter_map(|&grid_no| shifted(grid_no, offset))
        .collect()
}

/// Removes everything from a tile, the land becomes the background (`DeleteStuffFromMapTile`)
///
/// The original leaves the tile without any land.
fn clear_tile(context: &mut EditContext, grid_no: GridNo) {
    let tile = context.modify(grid_no);
    tile.land.clear();
    tile.objects.clear();
    tile.structs.clear();
    tile.shadows.clear();
    tile.roofs.clear();
    tile.on_roofs.clear();
    tile.room = None;
    tile.cave = false;
    if let Some(background) = context.background_tile() {
        context.add_land_to_head(grid_no, background);
    }
}

/// Copies the layers of a tile onto another (`PasteMapElementToNewMapElement`), without its
/// room number
fn copy_tile(context: &mut EditContext, from: GridNo, to: GridNo) {
    let source = context.map.tiles[from].clone();
    let target = context.modify(to);
    target.land = source.land;
    target.objects = source.objects;
    target.structs = source.structs;
    target.shadows = source.shadows;
    target.roofs = source.roofs;
    target.on_roofs = source.on_roofs;
    target.cave = source.cave;
    target.room = None;
}

/// Copies the building at `anchor` to `target`, and removes it from where it was if `moving`
/// (`PasteBuilding` and `MoveBuilding`)
///
/// The tiles are processed in the order that never overwrites a tile before it is copied.
/// Tiles that would leave the map are left out.
pub fn paste_building(context: &mut EditContext, anchor: GridNo, target: GridNo, moving: bool) {
    let Some(mut layout) = building_layout(context.map, anchor) else {
        return;
    };
    let offset = drag_offset(anchor, target);
    if offset == (0, 0) {
        return;
    }
    // `SortBuildingLayout`: towards smaller grid numbers, the smallest tiles go first
    let towards_start = offset.1 * 160 + offset.0 < 0;
    layout.sort_by_key(|grid_no| grid_no.index());
    if !towards_start {
        layout.reverse();
    }
    for from in layout {
        let Some(to) = shifted(from, offset) else {
            continue;
        };
        copy_tile(context, from, to);
        if moving {
            clear_tile(context, from);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{
            building::tests::{add_room, building_fixture, rect},
            document::tests::{grid, Fixture},
        },
        map::RoomNo,
        tileset::TileIndex,
    };

    /// The ids of the layers of a tile, without the land below the floors
    fn layers(fixture: &Fixture, grid_no: GridNo) -> Vec<Vec<TileIndex>> {
        let tile = &fixture.map.tiles[grid_no];
        let ids = |tiles: &[std::sync::Arc<Tile>]| tiles.iter().map(|t| t.id).collect();
        let floors: Vec<_> = tile.land.iter().filter(|t| t.is_floor()).cloned().collect();
        vec![
            ids(&floors),
            ids(&tile.structs),
            ids(&tile.shadows),
            ids(&tile.roofs),
        ]
    }

    #[test]
    fn layouts_have_the_walls_around_the_floors() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        assert!(building_layout(&fixture.map, grid(60, 60)).is_none());
        let mut layout = building_layout(&fixture.map, grid(51, 51)).unwrap();
        layout.sort();
        // The floors, the tiles east and south of them, and those north and west with walls
        assert_eq!(layout.len(), 9 + 3 + 3 + 3 + 3);
        assert!(layout.contains(&grid(53, 50)));
        assert!(layout.contains(&grid(50, 53)));
        assert!(layout.contains(&grid(51, 49)));
        assert!(layout.contains(&grid(49, 52)));
        assert!(!layout.contains(&grid(53, 53)));
        assert!(!layout.contains(&grid(49, 49)));
    }

    #[test]
    fn copies_keep_the_original() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let mut context = fixture.context();
        context.set_room(grid(51, 51), RoomNo::new(4));
        context.finish();
        let layout = building_layout(&fixture.map, grid(51, 51)).unwrap();
        let before: Vec<_> = layout.iter().map(|&g| layers(&fixture, g)).collect();

        let mut context = fixture.context();
        paste_building(&mut context, grid(51, 51), grid(61, 51), false);
        context.finish();
        for (&grid_no, before) in layout.iter().zip(&before) {
            assert_eq!(&layers(&fixture, grid_no), before);
            let target = grid_no.neighbour(10, 0).unwrap();
            assert_eq!(&layers(&fixture, target), before);
        }
        // Room numbers are not copied
        assert_eq!(fixture.map.tiles[grid(61, 51)].room, None);
        assert_eq!(fixture.map.tiles[grid(51, 51)].room, RoomNo::new(4));
        assert_eq!(fixture.world.tiles[grid(61, 49)].structures.len(), 1);
    }

    #[test]
    fn moves_overlapping_the_building_leave_the_ground() {
        for (dx, dy) in [(2, 1), (-1, -2)] {
            let mut fixture = building_fixture();
            add_room(&mut fixture, rect(50, 50, 52, 52));
            let layout = building_layout(&fixture.map, grid(51, 51)).unwrap();
            let before: Vec<_> = layout.iter().map(|&g| layers(&fixture, g)).collect();
            let target = grid(51, 51).neighbour(dx, dy).unwrap();
            let mut context = fixture.context();
            paste_building(&mut context, grid(51, 51), target, true);
            context.finish();
            for (&grid_no, before) in layout.iter().zip(&before) {
                let moved = grid_no.neighbour(dx, dy).unwrap();
                assert_eq!(&layers(&fixture, moved), before, "{dx} {dy}");
            }
            // Where the building was, there is only ground
            let shifted = shifted_layout(&layout, (dx, dy));
            for &grid_no in layout.iter().filter(|g| !shifted.contains(g)) {
                assert!(layers(&fixture, grid_no).iter().all(Vec::is_empty));
                assert_eq!(fixture.land(grid_no).len(), 1);
            }
            assert_eq!(fixture.document.undo.depth(), 1);
        }
    }
}
