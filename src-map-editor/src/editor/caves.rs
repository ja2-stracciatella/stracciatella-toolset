//! Caves (`AddCaveSectionToWorld` and `RemoveCaveSectionFromWorld` of `NewSmooth.cc`)
//!
//! Cave maps are rock, the caves are carved out of it. The rock is made of the pieces of the
//! first two wall types of the cave tilesets, and the piece of a tile follows which of its 8
//! neighbours are rock.

use std::sync::Arc;

use crate::{
    map::GridNo,
    tileset::{StructureFlags, Tile},
};

use super::{
    building::TileRect,
    document::EditContext,
    walls::{at, is_wall},
};

/// The rock piece of each perimeter value (`GetCaveTileIndexFromPerimeterValue`): the tile
/// type, the first sub index and the number of variants. `None` is a lone piece, which is
/// removed.
///
/// The perimeter value has a bit for each neighbour that is rock:
///
/// ```text
///  16 | 1 | 32
///  ---+---+---
///   8 |   | 2
///  ---+---+---
///  128| 4 | 64
/// ```
#[rustfmt::skip]
const CAVE_PIECES: [Option<(&str, u16, u16)>; 256] = [
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 5, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 9, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 11, 1)), Some(("WALL1", 12, 1)), Some(("WALL1", 13, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 5, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 16, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 17, 1)), Some(("WALL1", 12, 1)), Some(("WALL1", 18, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 21, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 22, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 11, 1)), Some(("WALL1", 12, 1)), Some(("WALL1", 23, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 21, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 24, 3)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 17, 1)), Some(("WALL1", 12, 1)), Some(("WALL1", 27, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 30, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 9, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 11, 1)), Some(("WALL1", 31, 1)), Some(("WALL1", 32, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 30, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 16, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 17, 1)), Some(("WALL1", 31, 1)), Some(("WALL1", 33, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 34, 3)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 22, 1)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 11, 1)), Some(("WALL1", 31, 1)), Some(("WALL1", 37, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 34, 3)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 24, 3)),
    Some(("WALL1", 10, 1)), Some(("WALL1", 17, 1)), Some(("WALL1", 31, 1)), Some(("WALL1", 38, 2)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 5, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 9, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 42, 1)), Some(("WALL1", 43, 1)), Some(("WALL1", 44, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 5, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 16, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 45, 3)), Some(("WALL1", 43, 1)), Some(("WALL1", 48, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 21, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 22, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 42, 1)), Some(("WALL1", 43, 1)), Some(("WALL1", 49, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 4, 1)), Some(("WALL1", 21, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 24, 3)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 45, 3)), Some(("WALL1", 43, 1)), Some(("WALL1", 50, 2)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 30, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 9, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 42, 1)), Some(("WALL1", 52, 3)), Some(("WALL1", 55, 1)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 1, 1)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 30, 1)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 16, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 45, 3)), Some(("WALL1", 52, 3)), Some(("WALL1", 56, 2)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 34, 3)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 6, 1)), Some(("WALL1", 7, 2)), Some(("WALL1", 22, 1)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 42, 1)), Some(("WALL1", 52, 3)), Some(("WALL1", 58, 2)),
    None, Some(("WALL2", 1, 4)), Some(("WALL2", 5, 4)), Some(("WALL1", 19, 2)),
    Some(("WALL2", 9, 4)), Some(("WALL1", 2, 2)), Some(("WALL1", 28, 2)), Some(("WALL1", 34, 3)),
    Some(("WALL2", 13, 4)), Some(("WALL1", 14, 2)), Some(("WALL1", 7, 2)), Some(("WALL1", 24, 3)),
    Some(("WALL1", 40, 2)), Some(("WALL1", 45, 3)), Some(("WALL1", 52, 3)), Some(("WALL1", 60, 6)),
];
/// The perimeter value of a tile surrounded by rock
const SURROUNDED: u8 = 0xff;

/// Whether a tile is rock (`CaveAtGridNo`): off the map, a cave wall structure, or a wall the
/// editor placed as rock
pub fn cave_at(context: &EditContext, grid_no: Option<GridNo>) -> bool {
    let Some(grid_no) = grid_no else {
        return true;
    };
    let tile = &context.map.tiles[grid_no];
    tile.structs.iter().any(|t| {
        t.structure
            .as_ref()
            .is_some_and(|s| s.flags.contains(StructureFlags::CAVEWALL))
            || (tile.cave && is_wall(t))
    })
}

/// Which of the 8 neighbours of a tile are rock (`CalcNewCavePerimeterValue`)
fn perimeter_value(context: &EditContext, grid_no: GridNo) -> u8 {
    const NEIGHBOURS: [(i32, i32, u8); 8] = [
        (0, -1, 0x01),
        (1, 0, 0x02),
        (0, 1, 0x04),
        (-1, 0, 0x08),
        (-1, -1, 0x10),
        (1, -1, 0x20),
        (1, 1, 0x40),
        (-1, 1, 0x80),
    ];
    NEIGHBOURS
        .iter()
        .filter(|&&(dx, dy, _)| cave_at(context, at(grid_no, dx, dy)))
        .map(|&(_, _, bit)| bit)
        .sum()
}

/// A random variant of the rock piece of a perimeter value, `None` for a lone piece or if the
/// tileset lacks it
fn cave_piece(context: &mut EditContext, perimeter: u8) -> Option<Arc<Tile>> {
    let (type_name, first, variants) = CAVE_PIECES[usize::from(perimeter)]?;
    let sub_index = first + context.random(u32::from(variants)) as u16;
    context.tile(type_name, sub_index)
}

/// Removes the rock of a tile
fn remove_cave(context: &mut EditContext, grid_no: GridNo) {
    context.remove_structs_where(grid_no, is_wall);
    if context.map.tiles[grid_no].cave {
        context.modify(grid_no).cave = false;
    }
}

/// Replaces the rock of a tile by a piece (`AddCave`)
fn add_cave(context: &mut EditContext, grid_no: GridNo, tile: Arc<Tile>) {
    remove_cave(context, grid_no);
    if context.add_wall_to_struct_layer(grid_no, tile, true) {
        context.modify(grid_no).cave = true;
    }
}

/// Gives the rock of the area and the ring around it the pieces that fit their neighbours, lone
/// pieces are removed
fn smooth_caves(context: &mut EditContext, rect: TileRect) {
    for (_, _, grid_no) in rect.grown().tiles() {
        if !cave_at(context, Some(grid_no)) {
            continue;
        }
        let perimeter = perimeter_value(context, grid_no);
        match cave_piece(context, perimeter) {
            Some(tile) => add_cave(context, grid_no, tile),
            None => remove_cave(context, grid_no),
        }
    }
}

/// Fills an area with rock (`AddCaveSectionToWorld`)
pub fn add_cave_section(context: &mut EditContext, rect: TileRect) {
    for (_, _, grid_no) in rect.tiles() {
        if let Some(tile) = cave_piece(context, SURROUNDED) {
            add_cave(context, grid_no, tile);
        }
    }
    smooth_caves(context, rect);
}

/// Carves a cave out of the rock (`RemoveCaveSectionFromWorld`)
pub fn remove_cave_section(context: &mut EditContext, rect: TileRect) {
    for (_, _, grid_no) in rect.tiles() {
        remove_cave(context, grid_no);
    }
    smooth_caves(context, rect);
}

/// Marks the rock pieces of a cave map, which the map file doesn't
/// (`AnalyseCaveMapForStructureInfo`): the pieces surrounded by rock, `WALL1` 60 to 65
pub fn mark_caves(map: &mut crate::map::TacticalMap) {
    for tile in map.tiles.values_mut() {
        tile.cave = tile
            .structs
            .iter()
            .any(|t| t.type_name == "WALL1" && (60..=65).contains(&t.sub_index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        building::tests::rect,
        document::tests::{grid, Fixture},
    };

    fn rock(fixture: &Fixture, x: u8, y: u8) -> Vec<(&str, u16)> {
        fixture.map.tiles[grid(x, y)]
            .structs
            .iter()
            .map(|t| (t.type_name.as_str(), t.sub_index))
            .collect()
    }

    #[test]
    fn every_perimeter_has_a_piece_or_is_lone() {
        for (perimeter, piece) in CAVE_PIECES.iter().enumerate() {
            // Without neighbours at the sides, a piece is lone
            assert_eq!(piece.is_none(), perimeter & 0x0f == 0, "{perimeter:#x}");
        }
        assert_eq!(CAVE_PIECES[0xff], Some(("WALL1", 60, 6)));
        assert_eq!(CAVE_PIECES[0x46], Some(("WALL1", 28, 2)));
    }

    #[test]
    fn rock_is_smoothed_by_its_neighbours() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        add_cave_section(&mut context, rect(50, 50, 54, 54));
        // A lone piece is removed again
        add_cave_section(&mut context, rect(70, 70, 70, 70));
        for (_, _, grid_no) in rect(50, 50, 54, 54).tiles() {
            assert!(cave_at(&context, Some(grid_no)));
        }
        assert!(!cave_at(&context, Some(grid(55, 52))));
        // Off the map is rock
        assert!(cave_at(&context, None));
        context.finish();
        let center = rock(&fixture, 52, 52);
        assert_eq!(center.len(), 1);
        assert!(center[0].0 == "WALL1" && (60..=65).contains(&center[0].1));
        // East, south and south east are rock
        let corner = rock(&fixture, 50, 50);
        assert!(
            corner == [("WALL1", 28)] || corner == [("WALL1", 29)],
            "{corner:?}"
        );
        assert!(rock(&fixture, 70, 70).is_empty());
        assert!(!fixture.map.tiles[grid(70, 70)].cave);

        // Carving the middle smooths the rock around it
        let mut context = fixture.context();
        remove_cave_section(&mut context, rect(51, 51, 53, 53));
        context.finish();
        assert!(rock(&fixture, 52, 52).is_empty());
        assert!(!fixture.map.tiles[grid(52, 52)].cave);
        // Only east and south are rock now
        assert_eq!(rock(&fixture, 50, 50), [("WALL1", 4)]);
    }

    #[test]
    fn loaded_rock_is_marked() {
        let mut fixture = Fixture::new();
        let rock_tile = fixture
            .tileset
            .tile(fixture.tileset.tile_of("WALL1", 62).unwrap())
            .unwrap()
            .clone();
        let wall = fixture
            .tileset
            .tile(fixture.tileset.tile_of("WALL1", 3).unwrap())
            .unwrap()
            .clone();
        fixture.map.tiles[grid(5, 5)].structs.push(rock_tile);
        fixture.map.tiles[grid(6, 5)].structs.push(wall);
        mark_caves(&mut fixture.map);
        assert!(fixture.map.tiles[grid(5, 5)].cave);
        assert!(!fixture.map.tiles[grid(6, 5)].cave);
        // The pieces around rock count as rock once they are marked
        let context = fixture.context();
        assert!(cave_at(&context, Some(grid(5, 5))));
        assert!(!cave_at(&context, Some(grid(6, 5))));
    }
}
