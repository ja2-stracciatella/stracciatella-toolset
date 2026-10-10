//! Placing cliffs (`PasteBanks`) and raising the land behind them (`RaiseWorldLand`)
//!
//! A cliff piece is a `FIRSTCLIFF` structure with the `FIRSTCLIFFHANG` object of the same sub
//! index, the overhanging edge of the land above. The world adds the `FIRSTCLIFFSHADOW` buddy
//! shadow of the structure outdoors. The pieces are picked by hand like in the original editor,
//! there is no smoothing. The height of the tiles is worked out from the pieces of the whole map
//! afterwards.

use std::sync::Arc;

use crate::{
    components::map::{JA2_MAP_SIZE, JA2_MAP_TILES},
    map::{grid::out_of_bounds, EditorSmoothing, GridNo, TacticalMap},
    tileset::{Tile, Tileset},
};

use super::document::EditContext;

/// Type of the cliff structures
const CLIFF: &str = "FIRSTCLIFF";
/// Type of the overhanging edges of the cliffs, in the objects layer
const CLIFF_HANG: &str = "FIRSTCLIFFHANG";
/// Type of the shadows of the cliffs
const CLIFF_SHADOW: &str = "FIRSTCLIFFSHADOW";

/// Height of one cliff in pixels (`WORLD_CLIFF_HEIGHT`)
pub const WORLD_CLIFF_HEIGHT: i32 = 80;

// The cliff pieces where the land drops instead of rising (`LAND_DROP_1` to `LAND_DROP_5`)
const LAND_DROP_1: u16 = 1;
const LAND_DROP_2: u16 = 11;
const LAND_DROP_3: u16 = 12;
const LAND_DROP_4: u16 = 15;
const LAND_DROP_5: u16 = 8;

pub fn is_cliff(tile: &Tile) -> bool {
    tile.type_name == CLIFF
}

pub fn is_cliff_hang(tile: &Tile) -> bool {
    tile.type_name == CLIFF_HANG
}

pub fn is_cliff_shadow(tile: &Tile) -> bool {
    tile.type_name == CLIFF_SHADOW
}

/// The cliff structure and its overhanging edge that placing a cliff piece on a tile adds, the
/// edge is left out in basements (`PasteBanks`)
///
/// Nothing is placed if the piece is already the head of the structs.
pub fn planned_cliff(
    map: &TacticalMap,
    tileset: &Tileset,
    grid_no: GridNo,
    sub_index: u16,
) -> Option<(Arc<Tile>, Option<Arc<Tile>>)> {
    let tile = |type_name| {
        tileset
            .tile(tileset.tile_of(type_name, sub_index)?)
            .cloned()
    };
    let cliff = tile(CLIFF)?;
    if map.tiles[grid_no]
        .structs
        .first()
        .is_some_and(|head| head.id == cliff.id)
    {
        return None;
    }
    let basement = map.info.editor_smoothing == EditorSmoothing::Basement;
    let hang = tile(CLIFF_HANG).filter(|_| !basement);
    Some((cliff, hang))
}

/// Places a cliff piece on a tile, see [`planned_cliff`]
pub fn place_cliff(context: &mut EditContext, grid_no: GridNo, sub_index: u16) {
    let Some((cliff, hang)) = planned_cliff(context.map, context.tileset, grid_no, sub_index)
    else {
        return;
    };
    context.add_struct_to_head(grid_no, cliff);
    if let Some(hang) = hang {
        context.add_object_to_head(grid_no, hang);
    }
}

/// Removes the cliffs of a tile with their shadows and overhanging edges (`EraseMapTile` for
/// banks)
pub fn erase_cliffs(context: &mut EditContext, grid_no: GridNo) {
    context.remove_structs_where(grid_no, is_cliff);
    context.remove_shadows_where(grid_no, is_cliff_shadow);
    context.remove_objects_where(grid_no, is_cliff_hang);
}

/// Sets the height of the tiles from the cliffs of the map (`RaiseWorldLand`), only the tiles
/// whose height changes are modified
pub fn raise_world_land(context: &mut EditContext) {
    let heights = world_land_heights(|g| &context.map.tiles[g].structs);
    for grid_no in GridNo::all() {
        let height = heights[grid_no.index()];
        if context.map.tiles[grid_no].height != height {
            context.modify(grid_no).height = height;
        }
    }
}

/// The marks of the cliffs on the tiles (`MAPELEMENT_RAISE_LAND_START` and `..._END`)
#[derive(Clone, Copy, Default)]
struct Marks {
    start: bool,
    end: bool,
}

/// The height of each tile in grid number order, from the cliff structs of the tiles
/// (`RaiseWorldLand`)
///
/// Each row is scanned from its end: the pieces where the land rises mark the start of a raised
/// stretch, the ones where it drops mark its end, and the tiles in between are one cliff higher.
/// This is a port of the original with its quirks, so that the maps get the heights the original
/// editor gave them. Unlike the original, a drop at the start of a row doesn't hang the editor and
/// tiles off the map are left out.
fn world_land_heights<'a>(structs: impl Fn(GridNo) -> &'a [Arc<Tile>]) -> Vec<u8> {
    const ROWS: i32 = JA2_MAP_SIZE as i32;
    const MAX: i32 = JA2_MAP_TILES as i32;

    let mut marks = vec![Marks::default(); JA2_MAP_TILES];
    let mut heights = vec![0u8; JA2_MAP_TILES];
    let in_map = |g: i32| (0..MAX).contains(&g);

    // The sub index of the last piece of several tiles, the original also uses it for the single
    // tile pieces
    let mut sub_index = u16::MAX;
    let mut something_raised = false;
    for grid_no in GridNo::all() {
        let cnt = grid_no.index() as i32;
        for tile in structs(grid_no).iter().filter(|t| is_cliff(t)) {
            something_raised = true;
            if tile.covered_tiles.len() > 1 {
                for (part, &(x, y)) in tile.covered_tiles.iter().enumerate() {
                    sub_index = tile.sub_index;
                    let temp = cnt + i32::from(x) + i32::from(y) * ROWS;
                    if out_of_bounds(cnt, temp) || !in_map(temp) {
                        continue;
                    }
                    let temp = temp as usize;
                    if matches!(sub_index, LAND_DROP_1 | LAND_DROP_2 | LAND_DROP_4)
                        || (sub_index == LAND_DROP_3 && part <= 2)
                    {
                        marks[temp] = Marks {
                            start: false,
                            end: true,
                        };
                    } else if sub_index == LAND_DROP_5 && part == 4 {
                        marks[temp] = Marks {
                            start: false,
                            end: true,
                        };
                        if let Some(next) = marks.get_mut(temp + 1) {
                            next.start = true;
                        }
                    } else {
                        marks[temp].start = true;
                    }
                }
            } else if matches!(sub_index, LAND_DROP_1 | LAND_DROP_3) {
                marks[cnt as usize] = Marks {
                    start: false,
                    end: true,
                };
            } else {
                marks[cnt as usize].start = true;
            }
        }
    }
    if !something_raised {
        return heights;
    }

    // Rows with a drop but no rise start raised at their end
    let mut raise_set = false;
    for cnt in (0..MAX).rev() {
        if cnt % ROWS == ROWS - 1 {
            raise_set = false;
        }
        if marks[cnt as usize].start {
            raise_set = true;
        } else if marks[cnt as usize].end && !raise_set {
            let row_end = cnt + (ROWS - 1 - cnt % ROWS);
            marks[row_end as usize] = Marks {
                start: true,
                end: false,
            };
            if row_end - ROWS > 0 {
                marks[(row_end - ROWS) as usize] = Marks {
                    start: true,
                    end: false,
                };
            }
            raise_set = true;
        }
    }

    // A cliff along the lower or the right edge of the map, along the diagonal from x = 159,
    // y = 80 to x = 80, y = 0
    let mut raise_set = false;
    let mut about_to_raise = false;
    let mut cnt = MAX - (ROWS / 2) * (ROWS - 2) - 1;
    while cnt > ROWS - 1 {
        if about_to_raise {
            raise_set = true;
            about_to_raise = false;
        }
        let around = [cnt, cnt - 1, cnt + 1].map(|g| marks[g as usize]);
        if around.iter().any(|m| m.start) {
            about_to_raise = true;
            raise_set = false;
        } else if around.iter().any(|m| m.end) {
            raise_set = false;
        }
        if raise_set {
            marks[(cnt + (ROWS - 1 - cnt % ROWS)) as usize].start = true;
        }
        cnt -= ROWS + 1;
    }

    // Count the rises and drops of each row from its end
    let mut raises = 0;
    let mut start_raises = 0;
    let mut cnt = MAX - 1;
    while cnt >= 0 {
        let row_start = cnt % ROWS == 0;
        if row_start {
            raises = 0;
            start_raises = 0;
        }
        let mark = |g: i32| marks[g as usize];
        if mark(cnt).end {
            if cnt > 1 && !mark(cnt - 1).end && !mark(cnt - 2).end {
                raises -= 1;
            }
        } else if mark(cnt).start && cnt < MAX - 2 && !mark(cnt + 1).start && !mark(cnt + 2).start {
            raises += 1;
        }

        // More drops than rises, the row starts again one cliff higher
        if raises < 0 {
            if row_start {
                // The original starts the row again forever
                raises = 0;
            } else {
                start_raises += 1;
                cnt += ROWS - cnt % ROWS - 1;
                raises = start_raises;
                continue;
            }
        }
        heights[cnt as usize] = (raises * WORLD_CLIFF_HEIGHT) as u8;
        cnt -= 1;
    }

    // Tiles that differ from the tiles of the rows before and after them
    for cnt in (0..MAX).rev() {
        if cnt < MAX - ROWS && cnt > ROWS {
            let (cnt, above, below) = (cnt as usize, (cnt - ROWS) as usize, (cnt + ROWS) as usize);
            if heights[below] == heights[above] && heights[cnt] != heights[above] {
                heights[cnt] = heights[below];
            } else if heights[cnt] > heights[above]
                && heights[below] != heights[above]
                && heights[cnt] > heights[below]
            {
                heights[cnt] = heights[above].max(heights[below]);
            }
        }
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::document::tests::{grid, Fixture};

    fn types(tiles: &[Arc<Tile>]) -> Vec<(&str, u16)> {
        tiles
            .iter()
            .map(|t| (t.type_name.as_str(), t.sub_index))
            .collect()
    }

    /// A cliff piece covering the given tiles
    fn piece(fixture: &Fixture, sub_index: u16, covered: &[(i8, i8)]) -> Arc<Tile> {
        let index = fixture.tileset.tile_of(CLIFF, sub_index).unwrap();
        let mut tile = (**fixture.tileset.tile(index).unwrap()).clone();
        tile.covered_tiles = covered.to_vec();
        Arc::new(tile)
    }

    fn heights_of(structs: &[(GridNo, Arc<Tile>)]) -> Vec<u8> {
        let mut tiles = vec![Vec::new(); JA2_MAP_TILES];
        for (grid_no, tile) in structs {
            tiles[grid_no.index()].push(tile.clone());
        }
        world_land_heights(|g| &tiles[g.index()])
    }

    fn height(heights: &[u8], x: u8, y: u8) -> u8 {
        heights[grid(x, y).index()]
    }

    #[test]
    fn a_cliff_is_placed_with_its_hang() {
        let mut fixture = Fixture::new();
        let g = grid(50, 50);
        let mut context = fixture.context();
        place_cliff(&mut context, g, 3);
        // The same piece again changes nothing
        place_cliff(&mut context, g, 3);
        context.finish();
        assert_eq!(types(&fixture.map.tiles[g].structs), [(CLIFF, 3)]);
        assert_eq!(types(&fixture.map.tiles[g].objects), [(CLIFF_HANG, 3)]);

        let mut context = fixture.context();
        erase_cliffs(&mut context, g);
        context.finish();
        assert!(fixture.map.tiles[g].structs.is_empty());
        assert!(fixture.map.tiles[g].objects.is_empty());
        assert!(fixture.map.tiles[g].shadows.is_empty());
    }

    #[test]
    fn erasing_removes_the_cliff_shadows() {
        let mut fixture = Fixture::new();
        let g = grid(50, 50);
        let shadow = fixture.context().tile(CLIFF_SHADOW, 3).unwrap();
        fixture.map.tiles[g].shadows.push(shadow);
        let mut context = fixture.context();
        erase_cliffs(&mut context, g);
        context.finish();
        assert!(fixture.map.tiles[g].shadows.is_empty());
    }

    #[test]
    fn basements_have_no_hangs() {
        let mut fixture = Fixture::new();
        fixture.map.info.editor_smoothing = EditorSmoothing::Basement;
        let g = grid(50, 50);
        let mut context = fixture.context();
        place_cliff(&mut context, g, 3);
        context.finish();
        assert_eq!(types(&fixture.map.tiles[g].structs), [(CLIFF, 3)]);
        assert!(fixture.map.tiles[g].objects.is_empty());
    }

    #[test]
    fn the_land_is_flat_without_cliffs() {
        assert!(heights_of(&[]).iter().all(|&h| h == 0));
    }

    #[test]
    fn the_land_between_a_rise_and_a_drop_is_raised() {
        let fixture = Fixture::new();
        // Rows are scanned from their end: a rise at x = 60 and a drop at x = 40
        let mut structs = Vec::new();
        for y in 40..60 {
            structs.push((grid(60, y), piece(&fixture, 2, &[(0, 0), (0, 1)])));
            structs.push((grid(40, y), piece(&fixture, LAND_DROP_1, &[(0, 0), (0, 1)])));
        }
        let heights = heights_of(&structs);
        assert_eq!(height(&heights, 50, 50), WORLD_CLIFF_HEIGHT as u8);
        assert_eq!(height(&heights, 41, 50), WORLD_CLIFF_HEIGHT as u8);
        assert_eq!(height(&heights, 60, 50), WORLD_CLIFF_HEIGHT as u8);
        assert_eq!(height(&heights, 40, 50), 0);
        assert_eq!(height(&heights, 61, 50), 0);
        assert_eq!(height(&heights, 50, 30), 0);
        assert_eq!(height(&heights, 50, 70), 0);
    }

    #[test]
    fn a_drop_without_a_rise_raises_the_rest_of_the_row() {
        let fixture = Fixture::new();
        let structs: Vec<_> = (40..60)
            .map(|y| (grid(40, y), piece(&fixture, LAND_DROP_1, &[(0, 0), (0, 1)])))
            .collect();
        let heights = heights_of(&structs);
        assert_eq!(height(&heights, 159, 50), WORLD_CLIFF_HEIGHT as u8);
        assert_eq!(height(&heights, 41, 50), WORLD_CLIFF_HEIGHT as u8);
        assert_eq!(height(&heights, 39, 50), 0);
    }

    #[test]
    fn a_drop_at_the_start_of_a_row_ends() {
        let fixture = Fixture::new();
        let heights = heights_of(&[(grid(0, 50), piece(&fixture, LAND_DROP_1, &[(0, 0), (0, 1)]))]);
        assert_eq!(height(&heights, 0, 50), 0);
    }

    #[test]
    fn placing_and_erasing_cliffs_raises_the_land_with_undo() {
        let mut fixture = Fixture::new();
        // The test tileset has no tile locations, single tile pieces use the last sub index of
        // the pieces of several tiles, which is none here: they all rise
        let mut context = fixture.context();
        for y in 40..60 {
            place_cliff(&mut context, grid(60, y), 2);
        }
        raise_world_land(&mut context);
        context.finish();
        fixture.document.undo.end();
        assert_eq!(fixture.map.tiles[grid(60, 50)].height, 80);
        assert_eq!(fixture.map.tiles[grid(1, 50)].height, 80);
        // The count starts again on the first tile of a row
        assert_eq!(fixture.map.tiles[grid(0, 50)].height, 0);
        assert_eq!(fixture.map.tiles[grid(61, 50)].height, 0);

        let mut context = fixture.context();
        for y in 40..60 {
            erase_cliffs(&mut context, grid(60, y));
        }
        raise_world_land(&mut context);
        context.finish();
        fixture.document.undo.end();
        assert!(fixture.map.tiles.values().all(|t| t.height == 0));

        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert_eq!(fixture.map.tiles[grid(60, 50)].height, 80);
        assert_eq!(fixture.world.tiles[grid(60, 50)].height, 80);
    }
}
