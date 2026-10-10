//! Placing and changing the walls of buildings (`BuildWallPiece` of `NewSmooth.cc` and
//! `Smoothing_Utils.cc`)
//!
//! A wall belongs to the edge of a tile: the horizontal walls stored on a tile are on the edge to
//! the next row, the vertical ones on the edge to the next column. So the walls on the top and
//! left edges of a tile are stored on the tiles above and left of it. The pieces are named from
//! the point of view of a floor tile that places them, e.g. `InteriorTop` is the wall above a
//! floor tile, seen from inside the building.

use std::sync::Arc;

use crate::{
    map::{EditorSmoothing, GridNo},
    render::movement_costs::grid_no_on_visible_world_tile,
    tileset::{BuildingPiece, Orientation, StructureFlags, Tile, TileCategory, WallPiece},
};

use super::{
    building::Materials,
    document::{EditContext, WallNode},
    smart::paste_smart_wall,
};

/// Where a floor tile places a wall (`EXTERIOR_TOP` to `INTERIOR_RIGHT`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallSide {
    ExteriorTop,
    ExteriorBottom,
    ExteriorLeft,
    ExteriorRight,
    InteriorTop,
    InteriorBottom,
    InteriorLeft,
    InteriorRight,
}

/// Searches farther than this for the wall and roof types of a building
const MAX_SEARCH_RADIUS: i32 = 32;

/// Whether the map is a basement, whose buildings are carved out of rock (`gfBasement`)
pub fn is_basement(context: &EditContext) -> bool {
    context.map.info.editor_smoothing == EditorSmoothing::Basement
}

/// The tile `dx` columns and `dy` rows away, `None` off the map
pub fn at(grid_no: GridNo, dx: i32, dy: i32) -> Option<GridNo> {
    grid_no.neighbour(dx, dy)
}

/// [`EditContext::vertical_wall`] of a tile that may be off the map
pub fn vertical_wall(context: &EditContext, grid_no: Option<GridNo>) -> Option<WallNode> {
    context.vertical_wall(grid_no?)
}

/// [`EditContext::horizontal_wall`] of a tile that may be off the map
pub fn horizontal_wall(context: &EditContext, grid_no: Option<GridNo>) -> Option<WallNode> {
    context.horizontal_wall(grid_no?)
}

/// [`EditContext::has_floor`] of a tile that may be off the map
pub fn has_floor(context: &EditContext, grid_no: Option<GridNo>) -> bool {
    grid_no.is_some_and(|grid_no| context.has_floor(grid_no))
}

/// Whether a tile is a piece of a wall, including the windows, A-frames and wall shadows
/// (`FIRSTWALL` to `LASTWALL`)
pub fn is_wall(tile: &Tile) -> bool {
    tile.building.is_some_and(|b| b.piece.is_wall())
}

/// Whether a tile is a door in any state (`FIRSTDOOR` to `LASTDOOR`)
pub fn is_door(tile: &Tile) -> bool {
    tile.building.is_some_and(|b| b.piece.is_door())
}

/// Whether a tile is the shadow of a door (`FIRSTDOORSHADOW` to `LASTDOORSHADOW`)
pub fn is_door_shadow(tile: &Tile) -> bool {
    tile.building
        .is_some_and(|b| b.piece == BuildingPiece::DoorShadow)
}

/// A random variant of a wall piece of a style (`PickAWallPiece`)
pub fn pick_wall_piece(
    context: &mut EditContext,
    style: u8,
    piece: WallPiece,
) -> Option<Arc<Tile>> {
    let tiles = context
        .tileset
        .building_tiles(style, BuildingPiece::Wall(piece));
    if tiles.is_empty() {
        return None;
    }
    let index = context.random(tiles.len() as u32) as usize;
    Some(tiles[index].clone())
}

/// Replaces a wall by a variant of a piece of a style (`ReplaceStructIndex`)
fn replace_wall(
    context: &mut EditContext,
    grid_no: GridNo,
    wall: WallNode,
    style: u8,
    piece: WallPiece,
) {
    if let Some(tile) = pick_wall_piece(context, style, piece) {
        context.replace_struct(grid_no, wall.index, tile);
    }
}

/// Adds a wall shadow of a style, unless the tile has it
fn add_wall_shadow(
    context: &mut EditContext,
    grid_no: Option<GridNo>,
    style: u8,
    piece: WallPiece,
) {
    let Some(grid_no) = grid_no else {
        return;
    };
    if let Some(tile) = pick_wall_piece(context, style, piece) {
        context.add_exclusive_shadow(grid_no, tile);
    }
}

/// Places a wall piece of a style for the floor tile `grid_no` and fixes up the corners it makes
/// (`BuildWallPiece`)
///
/// The top and left pieces go to the tiles above and left of the floor tile.
pub fn build_wall_piece(context: &mut EditContext, grid_no: GridNo, side: WallSide, style: u8) {
    let basement = is_basement(context);
    let target = match side {
        WallSide::ExteriorTop | WallSide::InteriorTop => at(grid_no, 0, -1),
        WallSide::ExteriorLeft | WallSide::InteriorLeft => at(grid_no, -1, 0),
        _ => Some(grid_no),
    };
    let Some(g) = target else {
        return;
    };
    let vwall = |context: &EditContext, dx, dy| vertical_wall(context, at(g, dx, dy));
    let hwall = |context: &EditContext, dx, dy| horizontal_wall(context, at(g, dx, dy));

    let class = match side {
        WallSide::ExteriorTop => {
            // A new bottom left corner contributes to the bottom shadow
            if !basement && vwall(context, -1, 0).is_some() && vwall(context, -1, 1).is_none() {
                add_wall_shadow(
                    context,
                    at(g, -1, 0),
                    style,
                    WallPiece::InteriorBottomEndShadow,
                );
            }
            // A new corner to the right, the vertical piece becomes a bottom end
            if let Some(wall) = vwall(context, 0, 0) {
                replace_wall(context, g, wall, style, WallPiece::ExteriorBottomEnd);
            }
            if !basement {
                // All exterior left walls have shadows
                add_wall_shadow(context, Some(g), style, WallPiece::ExteriorLeftShadow);
            }
            WallPiece::ExteriorLeft
        }
        WallSide::ExteriorBottom => {
            if let Some(wall) = vwall(context, -1, 1).filter(|_| vwall(context, -1, 0).is_none()) {
                if let Some(target) = at(g, -1, 1) {
                    replace_wall(context, target, wall, style, WallPiece::InteriorExtended);
                }
            }
            WallPiece::InteriorLeft
        }
        WallSide::ExteriorLeft => {
            // A new corner, the piece becomes an exterior bottom end
            let mut class = if hwall(context, 0, 0).is_some() {
                WallPiece::ExteriorBottomEnd
            } else {
                WallPiece::ExteriorRight
            };
            if hwall(context, 1, -1).is_some() {
                class = if class == WallPiece::ExteriorBottomEnd {
                    WallPiece::ExteriorExtendedBottomEnd
                } else {
                    WallPiece::ExteriorExtended
                };
            }
            class
        }
        WallSide::ExteriorRight => {
            if hwall(context, 1, -1).is_some() && hwall(context, 0, -1).is_none() {
                WallPiece::InteriorExtended
            } else if hwall(context, 0, 0).is_some() && vwall(context, 0, 1).is_none() {
                WallPiece::InteriorBottomEnd
            } else {
                WallPiece::InteriorRight
            }
        }
        WallSide::InteriorTop => {
            // A lower left corner, the piece becomes an extended piece
            if let (Some(wall), Some(target)) = (vwall(context, -1, 1), at(g, -1, 1)) {
                replace_wall(context, target, wall, style, WallPiece::InteriorExtended);
            }
            if let Some(wall) = vwall(context, 0, 0) {
                replace_wall(context, g, wall, style, WallPiece::InteriorBottomEnd);
            }
            WallPiece::InteriorLeft
        }
        WallSide::InteriorBottom => {
            // A new corner to the right, the vertical piece becomes a bottom end
            if let Some(wall) = vwall(context, 0, 0) {
                replace_wall(context, g, wall, style, WallPiece::ExteriorBottomEnd);
            }
            if let Some(wall) = vwall(context, -1, 1).filter(|_| vwall(context, -1, 0).is_none()) {
                if let Some(target) = at(g, -1, 1) {
                    replace_wall(context, target, wall, style, WallPiece::ExteriorExtended);
                }
            }
            if !basement {
                add_wall_shadow(context, Some(g), style, WallPiece::ExteriorLeftShadow);
            }
            WallPiece::ExteriorLeft
        }
        WallSide::InteriorLeft => {
            let mut class = if hwall(context, 0, 0).is_some() {
                WallPiece::InteriorBottomEnd
            } else {
                WallPiece::InteriorRight
            };
            if !basement && hwall(context, 1, 0).is_some() {
                add_wall_shadow(context, Some(g), style, WallPiece::InteriorBottomEndShadow);
            }
            if hwall(context, 1, -1).is_some() {
                class = if class == WallPiece::InteriorBottomEnd {
                    WallPiece::InteriorExtendedBottomEnd
                } else {
                    WallPiece::InteriorExtended
                };
            }
            class
        }
        WallSide::InteriorRight => {
            let mut class = if hwall(context, 0, 0).is_some() {
                WallPiece::ExteriorBottomEnd
            } else {
                WallPiece::ExteriorRight
            };
            if hwall(context, 1, -1).is_some() {
                class = if class == WallPiece::ExteriorBottomEnd {
                    WallPiece::ExteriorExtendedBottomEnd
                } else {
                    WallPiece::ExteriorExtended
                };
            }
            if !basement
                && hwall(context, 1, 0).is_some()
                && hwall(context, 0, 0).is_none()
                && !has_floor(context, at(g, 0, 1))
            {
                add_wall_shadow(context, Some(g), style, WallPiece::InteriorBottomEndShadow);
            }
            class
        }
    };
    if let Some(tile) = pick_wall_piece(context, style, class) {
        context.add_wall_to_struct_layer(g, tile, false);
    }
}

/// Changes the vertical wall of a tile to another piece of its style, doors stay
/// (`ChangeVerticalWall`)
pub fn change_vertical_wall(context: &mut EditContext, grid_no: Option<GridNo>, piece: WallPiece) {
    let Some(grid_no) = grid_no else {
        return;
    };
    let Some(wall) = context.vertical_wall(grid_no) else {
        return;
    };
    let Some(building) = wall.tile.building.filter(|b| b.piece.is_wall()) else {
        return;
    };
    replace_wall(context, grid_no, wall, building.style, piece);
}

/// Removes a wall and the wall shadows of its tile
fn erase_wall(context: &mut EditContext, grid_no: GridNo, wall: WallNode) {
    context.remove_struct(grid_no, wall.index);
    context.remove_shadows_where(grid_no, is_wall);
}

/// Removes the horizontal wall of a tile with the wall shadows (`EraseHorizontalWall`)
pub fn erase_horizontal_wall(context: &mut EditContext, grid_no: Option<GridNo>) {
    if let Some(grid_no) = grid_no {
        if let Some(wall) = context.horizontal_wall(grid_no) {
            erase_wall(context, grid_no, wall);
        }
    }
}

/// Removes the vertical wall of a tile with the wall shadows (`EraseVerticalWall`)
pub fn erase_vertical_wall(context: &mut EditContext, grid_no: Option<GridNo>) {
    if let Some(grid_no) = grid_no {
        if let Some(wall) = context.vertical_wall(grid_no) {
            erase_wall(context, grid_no, wall);
        }
    }
}

/// The visible tiles on the ring `radius` tiles around a tile, from the bottom row to the top
/// one if `bottom_first`
fn ring(
    context: &EditContext,
    grid_no: GridNo,
    radius: i32,
    bottom_first: bool,
) -> impl Iterator<Item = GridNo> + use<> {
    let rows: Vec<i32> = if bottom_first {
        (-radius..=radius).rev().collect()
    } else {
        (-radius..=radius).collect()
    };
    let restricted_scroll = context.world.restricted_scroll;
    rows.into_iter()
        .flat_map(move |dy| (-radius..=radius).map(move |dx| (dx, dy)))
        .filter(move |&(dx, dy)| dx.abs() == radius || dy.abs() == radius)
        .filter_map(move |(dx, dy)| at(grid_no, dx, dy))
        .filter(move |&g| grid_no_on_visible_world_tile(g, restricted_scroll))
}

/// The style of the nearest wall (`SearchForWallType`), in basements a wall of the materials
///
/// The search starts at the bottom row of each ring, because another building can be one tile
/// north, but not one tile south.
pub fn search_for_wall_type(
    context: &mut EditContext,
    grid_no: GridNo,
    materials: &Materials,
) -> Option<u8> {
    if is_basement(context) {
        return Some(materials.wall(context).unwrap_or(0));
    }
    (0..MAX_SEARCH_RADIUS).find_map(|radius| {
        ring(context, grid_no, radius, true).find_map(|g| {
            context
                .structs(g)
                .iter()
                .find_map(|tile| tile.building.filter(|b| b.piece.is_wall()))
                .map(|b| b.style)
        })
    })
}

/// The style of the nearest flat roof (`SearchForRoofType`)
pub fn search_for_roof_type(context: &EditContext, grid_no: GridNo) -> Option<u8> {
    (0..MAX_SEARCH_RADIUS).find_map(|radius| {
        ring(context, grid_no, radius, false).find_map(|g| {
            context
                .roofs(g)
                .iter()
                .find_map(|tile| tile.building.filter(|b| b.piece.is_flat_roof()))
                .map(|b| b.style)
        })
    })
}

/// The style of a wall, doors take the style of the nearest wall (`GetWallType`)
pub fn wall_type(
    context: &mut EditContext,
    wall: &WallNode,
    grid_no: GridNo,
    materials: &Materials,
) -> Option<u8> {
    match wall.tile.building {
        Some(building) if building.piece.is_wall() => Some(building.style),
        _ => search_for_wall_type(context, grid_no, materials),
    }
}

/// The style of the vertical wall of a tile, `None` if it has none (`GetVerticalWallType`)
pub fn vertical_wall_type(
    context: &mut EditContext,
    grid_no: Option<GridNo>,
    materials: &Materials,
) -> Option<u8> {
    let grid_no = grid_no?;
    let wall = context.vertical_wall(grid_no)?;
    wall_type(context, &wall, grid_no, materials)
}

/// The style of the horizontal wall of a tile, `None` if it has none
/// (`GetHorizontalWallType`)
pub fn horizontal_wall_type(
    context: &mut EditContext,
    grid_no: Option<GridNo>,
    materials: &Materials,
) -> Option<u8> {
    let grid_no = grid_no?;
    let wall = context.horizontal_wall(grid_no)?;
    wall_type(context, &wall, grid_no, materials)
}

/// Replaces the walls of a tile by plain walls of their style and alignment (`RestoreWalls`)
///
/// Without walls, a smart wall of the style of an adjacent wall is placed.
pub fn restore_walls(context: &mut EditContext, grid_no: GridNo, materials: &Materials) {
    let mut done = false;
    if let Some(wall) = context.horizontal_wall(grid_no) {
        let style = wall_type(context, &wall, grid_no, materials);
        let orientation = wall.tile.wall_orientation();
        erase_wall(context, grid_no, wall);
        let side = match orientation {
            Orientation::OUTSIDE_LEFT => Some(WallSide::InteriorBottom),
            Orientation::INSIDE_LEFT => Some(WallSide::ExteriorBottom),
            _ => None,
        };
        if let (Some(side), Some(style)) = (side, style) {
            build_wall_piece(context, grid_no, side, style);
        }
        done = true;
    }
    if let Some(wall) = context.vertical_wall(grid_no) {
        let style = wall_type(context, &wall, grid_no, materials);
        let orientation = wall.tile.wall_orientation();
        erase_wall(context, grid_no, wall);
        let side = match orientation {
            Orientation::OUTSIDE_RIGHT => Some(WallSide::InteriorRight),
            Orientation::INSIDE_RIGHT => Some(WallSide::ExteriorRight),
            _ => None,
        };
        if let (Some(side), Some(style)) = (side, style) {
            build_wall_piece(context, grid_no, side, style);
        }
        done = true;
    }
    if done {
        return;
    }
    // Nothing to restore, place a smart wall of the style of an adjacent wall instead
    let wall = horizontal_wall(context, at(grid_no, -1, 0))
        .or_else(|| horizontal_wall(context, at(grid_no, 1, 0)))
        .or_else(|| vertical_wall(context, at(grid_no, 0, -1)))
        .or_else(|| vertical_wall(context, at(grid_no, 0, 1)));
    let Some(wall) = wall else {
        return;
    };
    if let Some(style) = wall_type(context, &wall, grid_no, materials) {
        paste_smart_wall(context, grid_no, style, materials);
    }
}

/// Removes all structs of a tile, the wall and door shadows and the debris (`EraseWalls`)
pub fn erase_walls(context: &mut EditContext, grid_no: GridNo) {
    context.remove_structs_where(grid_no, |_| true);
    context.remove_shadows_where(grid_no, |tile| is_wall(tile) || is_door_shadow(tile));
    context.remove_objects_where(grid_no, |tile| tile.category == Some(TileCategory::Debris));
}

/// Whether a fence of a tile has the alignment
fn has_fence(
    context: &EditContext,
    grid_no: GridNo,
    aligned: impl Fn(Orientation) -> bool,
) -> bool {
    context.structs(grid_no).iter().any(|tile| {
        tile.structure.as_ref().is_some_and(|structure| {
            structure.flags.intersects(StructureFlags::ANYFENCE)
                && aligned(structure.wall_orientation)
        })
    })
}

/// Whether a decal can be placed on a tile: it needs a wall or fence (`ValidDecalPlacement`)
pub fn valid_decal_placement(context: &EditContext, grid_no: GridNo) -> bool {
    context.vertical_wall(grid_no).is_some()
        || context.horizontal_wall(grid_no).is_some()
        || has_fence(context, grid_no, Orientation::is_vertical)
        || has_fence(context, grid_no, Orientation::is_horizontal)
}
