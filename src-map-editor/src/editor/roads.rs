//! Placing roads (`Road_Smoothing.cc`)
//!
//! Roads are made of single tile pieces (`ROADPIECES`). A road macro places the pieces of one of
//! the 32 road images of `FIRSTROAD` around the clicked tile. The sub index of a piece tells its
//! macro and where it is relative to the clicked tile, so erasing removes whole macros.

use std::sync::Arc;

use crate::{
    components::map::GridPosition,
    map::{GridNo, TacticalMap},
    tileset::{Tile, Tileset},
};

use super::document::EditContext;

// The road macros, in the order of the `FIRSTROAD` images
const L1: u8 = 0;
const R1: u8 = 1;
const B1: u8 = 2;
const T1: u8 = 3;
const TTR: u8 = 4;
const BTR: u8 = 5;
const LBL: u8 = 6;
const RBL: u8 = 7;
const LTL: u8 = 8;
const RTL: u8 = 9;
const RBR: u8 = 10;
const LBR: u8 = 11;
const L2: u8 = 12;
const R2: u8 = 13;
const L3: u8 = 14;
const R3: u8 = 15;
const B2: u8 = 16;
const T2: u8 = 17;
const B3: u8 = 18;
const T3: u8 = 19;
const BI: u8 = 20;
const LI: u8 = 21;
const TI: u8 = 22;
const RI: u8 = 23;
const L4: u8 = 24;
const R4: u8 = 25;
const B4: u8 = 26;
const T4: u8 = 27;
const RE: u8 = 28;
const LE: u8 = 29;
const BE: u8 = 30;
const TE: u8 = 31;

/// The pieces of the road macros (`gRoadMacros`): the macro and the grid number offset from the
/// clicked tile. The index of a piece is its sub index in `ROADPIECES` minus 1.
#[rustfmt::skip]
const ROAD_MACROS: [(u8, i16); 312] = [
    (L1, -2), (L1, -162), (L1, -322), (L1, -1), (L1, -161), (L1, -321), (L1, 0), (L1, -160), (L1, -320),
    (R1, -2), (R1, -162), (R1, -322), (R1, -1), (R1, -161), (R1, -321), (R1, 0), (R1, -160), (R1, -320),
    (B1, -2), (B1, -162), (B1, -322), (B1, -1), (B1, -161), (B1, -321), (B1, 0), (B1, -160), (B1, -320),
    (T1, -2), (T1, -162), (T1, -322), (T1, -1), (T1, -161), (T1, -321), (T1, 0), (T1, -160), (T1, -320),
    (TTR, -4), (TTR, -164), (TTR, -324), (TTR, -3), (TTR, -163), (TTR, -323), (TTR, -2), (TTR, -162), (TTR, -1), (TTR, -161), (TTR, 0),
    (BTR, -5), (BTR, -165), (BTR, -325), (BTR, -4), (BTR, -164), (BTR, -324), (BTR, -3), (BTR, -163), (BTR, -323), (BTR, -2), (BTR, -162), (BTR, -322), (BTR, -1), (BTR, -161), (BTR, -321), (BTR, 0), (BTR, -160),
    (LBL, -322), (LBL, -482), (LBL, -642), (LBL, -161), (LBL, -321), (LBL, -481), (LBL, -641), (LBL, 0), (LBL, -160), (LBL, -320), (LBL, -480), (LBL, -640),
    (RBL, -162), (RBL, -322), (RBL, -482), (RBL, -642), (RBL, -802), (RBL, -1), (RBL, -161), (RBL, -321), (RBL, -481), (RBL, -641), (RBL, -801), (RBL, 0), (RBL, -160), (RBL, -320), (RBL, -480), (RBL, -640), (RBL, -800),
    (LTL, -2), (LTL, -162), (LTL, -322), (LTL, -1), (LTL, -161), (LTL, -321), (LTL, -481), (LTL, 0), (LTL, -160), (LTL, -320), (LTL, -480), (LTL, -640),
    (RTL, -2), (RTL, -162), (RTL, -322), (RTL, -482), (RTL, -642), (RTL, -802), (RTL, -1), (RTL, -161), (RTL, -321), (RTL, -481), (RTL, -641), (RTL, -801), (RTL, 0), (RTL, -160), (RTL, -320), (RTL, -480), (RTL, -640), (RTL, -800),
    (RBR, 159), (RBR, -1), (RBR, -161), (RBR, -321), (RBR, -481), (RBR, -641), (RBR, 0), (RBR, -160), (RBR, -320), (RBR, -480), (RBR, -640), (RBR, -159), (RBR, -319), (RBR, -479), (RBR, -639),
    (LBR, 158), (LBR, -2), (LBR, -162), (LBR, -322), (LBR, -482), (LBR, -642), (LBR, 159), (LBR, -1), (LBR, -161), (LBR, -321), (LBR, -481), (LBR, -641), (LBR, 160), (LBR, 0), (LBR, -160), (LBR, -320), (LBR, -480), (LBR, -640),
    (L2, -2), (L2, -162), (L2, -322), (L2, -1), (L2, -161), (L2, -321), (L2, 0), (L2, -160), (L2, -320),
    (R2, -2), (R2, -162), (R2, -322), (R2, -1), (R2, -161), (R2, -321), (R2, 0), (R2, -160), (R2, -320),
    (L3, -2), (L3, -162), (L3, -322), (L3, -1), (L3, -161), (L3, -321), (L3, 0), (L3, -160), (L3, -320),
    (R3, -2), (R3, -162), (R3, -322), (R3, -1), (R3, -161), (R3, -321), (R3, 0), (R3, -160), (R3, -320),
    (B2, -2), (B2, -162), (B2, -322), (B2, -1), (B2, -161), (B2, -321), (B2, 0), (B2, -160), (B2, -320),
    (T2, -2), (T2, -162), (T2, -322), (T2, -1), (T2, -161), (T2, -321), (T2, 0), (T2, -160), (T2, -320),
    (B3, -2), (B3, -162), (B3, -322), (B3, -1), (B3, -161), (B3, -321), (B3, 0), (B3, -160), (B3, -320),
    (T3, -2), (T3, -162), (T3, -322), (T3, -1), (T3, -161), (T3, -321), (T3, 0), (T3, -160), (T3, -320),
    (BI, -2), (BI, -162), (BI, -322), (BI, -1), (BI, -161), (BI, -321), (BI, 0), (BI, -160), (BI, -320),
    (LI, -2), (LI, -162), (LI, -322), (LI, -1), (LI, -161), (LI, -321), (LI, 0), (LI, -160), (LI, -320),
    (TI, -2), (TI, -162), (TI, -322), (TI, -1), (TI, -161), (TI, -321), (TI, 0), (TI, -160), (TI, -320),
    (RI, -2), (RI, -162), (RI, -322), (RI, -1), (RI, -161), (RI, -321), (RI, 0), (RI, -160), (RI, -320),
    (L4, -2), (L4, -162), (L4, -322), (L4, -1), (L4, -161), (L4, -321), (L4, 0), (L4, -160), (L4, -320),
    (R4, -2), (R4, -162), (R4, -322), (R4, -1), (R4, -161), (R4, -321), (R4, 0), (R4, -160), (R4, -320),
    (B4, -2), (B4, -162), (B4, -322), (B4, -1), (B4, -161), (B4, -321), (B4, 0), (B4, -160), (B4, -320),
    (T4, -2), (T4, -162), (T4, -322), (T4, -1), (T4, -161), (T4, -321), (T4, 0), (T4, -160), (T4, -320),
    (RE, 0), (RE, -160), (RE, -320),
    (LE, 0), (LE, -160), (LE, -320),
    (BE, -2), (BE, -1), (BE, 0),
    (TE, -2), (TE, -1), (TE, 0),
];

/// Column and row offset of a grid number offset, the game adds them as one number
fn offset_to_xy(offset: i16) -> (i32, i32) {
    let offset = i32::from(offset);
    let dy = (offset + 80).div_euclid(160);
    (offset - dy * 160, dy)
}

/// The pieces of a road macro placed at a position and the tiles they go to, pieces off the map
/// are left out
fn pieces_at(tileset: &Tileset, x: i32, y: i32, macro_id: u16) -> Vec<(GridNo, Arc<Tile>)> {
    ROAD_MACROS
        .iter()
        .enumerate()
        .filter(|&(_, &(id, _))| u16::from(id) == macro_id)
        .filter_map(|(index, &(_, offset))| {
            let (dx, dy) = offset_to_xy(offset);
            let target = GridPosition::from_xy(x + dx, y + dy)?;
            let piece = tileset.tile_of("ROADPIECES", index as u16 + 1)?;
            Some((GridNo::from(target), tileset.tile(piece)?.clone()))
        })
        .collect()
}

/// The pieces of a road macro around a tile and the tiles they go to
///
/// Pieces that would be off the map or wrap into another row are left out.
pub fn road_macro_pieces(
    tileset: &Tileset,
    grid_no: GridNo,
    macro_id: u16,
) -> Vec<(GridNo, Arc<Tile>)> {
    let position = grid_no.position();
    pieces_at(
        tileset,
        i32::from(position.x),
        i32::from(position.y),
        macro_id,
    )
}

/// All pieces of the road macro a piece on a tile belongs to, where the macro would have put them
///
/// The clicked tile of the macro may be off the map. A piece outside the table only stands for
/// itself.
fn macro_of_piece(
    tileset: &Tileset,
    grid_no: GridNo,
    piece: &Arc<Tile>,
) -> Vec<(GridNo, Arc<Tile>)> {
    let entry = usize::from(piece.sub_index)
        .checked_sub(1)
        .and_then(|index| ROAD_MACROS.get(index));
    let Some(&(macro_id, offset)) = entry else {
        return vec![(grid_no, piece.clone())];
    };
    let position = grid_no.position();
    let (dx, dy) = offset_to_xy(offset);
    pieces_at(
        tileset,
        i32::from(position.x) - dx,
        i32::from(position.y) - dy,
        u16::from(macro_id),
    )
}

/// The road pieces that erasing a tile removes: the pieces of the macros its road pieces belong
/// to that are still on the map
pub fn erased_road_pieces(
    map: &TacticalMap,
    tileset: &Tileset,
    grid_no: GridNo,
) -> Vec<(GridNo, Arc<Tile>)> {
    let mut erased: Vec<(GridNo, Arc<Tile>)> = vec![];
    let pieces = map.tiles[grid_no]
        .objects
        .iter()
        .filter(|t| is_road_piece(t));
    for piece in pieces {
        for (target, tile) in macro_of_piece(tileset, grid_no, piece) {
            let on_map = map.tiles[target].objects.iter().any(|t| t.id == tile.id);
            let known = erased.iter().any(|(g, t)| *g == target && t.id == tile.id);
            if on_map && !known {
                erased.push((target, tile));
            }
        }
    }
    erased
}

/// Places the pieces of a road macro around a tile (`PlaceRoadMacroAtGridNo`)
///
/// Unlike the original, the road pieces are removed from the tiles the pieces go to instead of
/// the tiles with the index of the piece as grid number, and pieces that would be off the map or
/// wrap into another row are left out.
pub fn place_road_macro(context: &mut EditContext, grid_no: GridNo, macro_id: u16) {
    for (target, piece) in road_macro_pieces(context.tileset, grid_no, macro_id) {
        context.remove_objects_where(target, is_road_piece);
        context.add_object_to_head(target, piece);
    }
}

/// Whether a tile of the objects is a road piece, which placing and erasing roads removes
pub fn is_road_piece(tile: &Tile) -> bool {
    tile.type_name == "ROADPIECES"
}

/// Removes the roads on a tile (`EraseMapTile` for roads)
///
/// Unlike the original, which only removes the road pieces of the tile, this removes all pieces
/// of their road macros, so no partial roads are left behind. Pieces another macro replaced
/// stay.
pub fn erase_roads(context: &mut EditContext, grid_no: GridNo) {
    for (target, piece) in erased_road_pieces(context.map, context.tileset, grid_no) {
        context.remove_objects_where(target, |t| t.id == piece.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        file_formats::ja2_tile_types::{NUM_TILES_PER_TYPE, ROADPIECES},
        tileset::NUM_ROAD_MACROS,
    };

    /// Sub indices of the road pieces of a tile
    fn pieces(fixture: &Fixture, grid_no: GridNo) -> Vec<u16> {
        fixture.map.tiles[grid_no]
            .objects
            .iter()
            .filter(|tile| tile.type_name == "ROADPIECES")
            .map(|tile| tile.sub_index)
            .collect()
    }

    fn all_pieces(fixture: &Fixture) -> usize {
        GridNo::all().map(|g| pieces(fixture, g).len()).sum()
    }

    #[test]
    fn the_table_matches_the_road_pieces() {
        assert!(ROAD_MACROS.len() <= usize::from(NUM_TILES_PER_TYPE[ROADPIECES]));
        // The pieces of each macro are together, in the order of the macros
        let ids: Vec<u16> = ROAD_MACROS.iter().map(|&(id, _)| u16::from(id)).collect();
        assert!(ids.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(ids.last(), Some(&(NUM_ROAD_MACROS - 1)));
        assert_eq!(offset_to_xy(-2), (-2, 0));
        assert_eq!(offset_to_xy(-162), (-2, -1));
        assert_eq!(offset_to_xy(159), (-1, 1));
        assert_eq!(offset_to_xy(-159), (1, -1));
        assert_eq!(offset_to_xy(-802), (-2, -5));
    }

    #[test]
    fn a_macro_places_its_pieces_around_the_tile() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        let rock = context.tile("ROCKS", 1).unwrap();
        context.add_object_to_head(grid(49, 49), rock);
        place_road_macro(&mut context, grid(50, 50), u16::from(L1));
        // Placing it again replaces the pieces
        place_road_macro(&mut context, grid(50, 50), u16::from(L1));
        context.finish();
        // L1 is the first macro: -2, -162, -322, -1, -161, -321, 0, -160, -320
        assert_eq!(pieces(&fixture, grid(48, 50)), [1]);
        assert_eq!(pieces(&fixture, grid(48, 49)), [2]);
        assert_eq!(pieces(&fixture, grid(48, 48)), [3]);
        assert_eq!(pieces(&fixture, grid(50, 50)), [7]);
        assert_eq!(pieces(&fixture, grid(50, 48)), [9]);
        // Other objects stay
        assert_eq!(fixture.map.tiles[grid(49, 49)].objects.len(), 2);
        assert_eq!(pieces(&fixture, grid(49, 49)), [5]);
        assert_eq!(all_pieces(&fixture), 9);
        // Pieces of other macros follow the pieces of the ones before them
        let mut context = fixture.context();
        place_road_macro(&mut context, grid(80, 80), u16::from(R1));
        context.finish();
        assert_eq!(pieces(&fixture, grid(78, 80)), [10]);
    }

    #[test]
    fn pieces_off_the_map_are_left_out() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        place_road_macro(&mut context, grid(1, 0), u16::from(L1));
        context.finish();
        // Only the row of the tile, and not the piece that would wrap into the row before
        assert_eq!(pieces(&fixture, grid(0, 0)), [4]);
        assert_eq!(pieces(&fixture, grid(1, 0)), [7]);
        assert!(pieces(&fixture, grid(159, 0)).is_empty());
        assert_eq!(all_pieces(&fixture), 2);

        fixture.document.undo.end();
        let mut context = fixture.context();
        erase_roads(&mut context, grid(1, 0));
        context.finish();
        assert_eq!(all_pieces(&fixture), 0);
        assert_eq!(fixture.document.undo.depth(), 2);
    }

    #[test]
    fn erasing_removes_the_whole_macro() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        let rock = context.tile("ROCKS", 1).unwrap();
        context.add_object_to_head(grid(49, 49), rock);
        place_road_macro(&mut context, grid(50, 50), u16::from(L1));
        erase_roads(&mut context, grid(48, 49));
        context.finish();
        assert_eq!(all_pieces(&fixture), 0);
        assert_eq!(fixture.map.tiles[grid(49, 49)].objects.len(), 1);
    }

    #[test]
    fn erasing_keeps_overlapping_macros() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        place_road_macro(&mut context, grid(50, 50), u16::from(L1));
        // Replaces the pieces of L1 in the column 50
        place_road_macro(&mut context, grid(52, 50), u16::from(R1));
        context.finish();
        assert_eq!(all_pieces(&fixture), 15);

        let mut context = fixture.context();
        erase_roads(&mut context, grid(48, 49));
        context.finish();
        assert_eq!(all_pieces(&fixture), 9);
        assert_eq!(pieces(&fixture, grid(50, 50)), [10]);

        // Erasing where R1 replaced L1 removes R1
        let mut context = fixture.context();
        place_road_macro(&mut context, grid(50, 50), u16::from(L1));
        place_road_macro(&mut context, grid(52, 50), u16::from(R1));
        erase_roads(&mut context, grid(50, 50));
        context.finish();
        assert_eq!(all_pieces(&fixture), 6);
        assert!(pieces(&fixture, grid(52, 50)).is_empty());
    }
}
