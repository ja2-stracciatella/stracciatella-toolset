//! The smart method of the buildings mode (`SmartMethod.cc`): walls that pick their piece from
//! the walls around them, and doors, windows and broken walls that replace the walls of a tile
//!
//! The original computes the sub indices of the doors, windows and broken walls from the wall
//! orientation, which has been wrong since the orientations became bit flags. Here the tiles are
//! looked up by their building piece and orientation.

use std::sync::Arc;

use crate::{
    map::GridNo,
    tileset::{BuildingPiece, Tile, WallPiece},
};

use super::{
    building::Materials,
    document::{EditContext, WallNode},
    walls::{
        at, build_wall_piece, change_vertical_wall, erase_horizontal_wall, erase_vertical_wall,
        erase_walls, has_floor, horizontal_wall, horizontal_wall_type, restore_walls,
        vertical_wall, vertical_wall_type, wall_type, WallSide,
    },
};

/// Number of wall styles, `FIRSTWALL` to `LASTWALL`
const WALL_STYLES: usize = 4;

/// What the smart broken walls method places
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartBrokenWall {
    /// A broken, cracked or smudged wall piece
    Piece(WallPiece),
    /// Removes the wall
    Hole,
}

/// Places a wall piece of a style for a floor tile that may be off the map
fn build(context: &mut EditContext, grid_no: Option<GridNo>, side: WallSide, style: u8) {
    if let Some(grid_no) = grid_no {
        build_wall_piece(context, grid_no, side, style);
    }
}

/// Places a wall of a style, vertical or horizontal depending on the walls around it
/// (`PasteSmartWall`)
///
/// The rules of the original, in this order:
/// - A: a tile with a vertical and a horizontal wall stays.
/// - B: on a horizontal wall of the style, a vertical piece is added if the wall above continues
///   down to it.
/// - C: on a vertical wall of the style, a horizontal piece is added if the wall to the left
///   continues to it.
/// - D and E: the walls of the style around the tile decide between a vertical and a horizontal
///   piece, the corners are fixed up.
/// - F: a wall of the style below or to the right is connected with two pieces.
///
/// Whether a piece is seen from inside depends on the floors next to it.
pub fn paste_smart_wall(context: &mut EditContext, g: GridNo, style: u8, materials: &Materials) {
    let w = Some(style);
    let vwall = |context: &EditContext, dx, dy| vertical_wall(context, at(g, dx, dy)).is_some();
    let hwall = |context: &EditContext, dx, dy| horizontal_wall(context, at(g, dx, dy)).is_some();
    let floor = |context: &EditContext, dx, dy| has_floor(context, at(g, dx, dy));

    // A
    if vwall(context, 0, 0) && hwall(context, 0, 0) {
        return;
    }
    // B
    if let Some(horizontal) = horizontal_wall_type(context, Some(g), materials) {
        if Some(horizontal) != w {
            return;
        }
        let inside = floor(context, 1, 0);
        let side = if inside {
            WallSide::ExteriorRight
        } else {
            WallSide::InteriorRight
        };
        if vertical_wall_type(context, at(g, 0, -1), materials) == w {
            build_wall_piece(context, g, side, style);
            return;
        }
        if horizontal_wall_type(context, at(g, 0, -1), materials) == w {
            build_wall_piece(context, g, side, style);
            if !hwall(context, 1, -1) {
                let piece = if inside {
                    WallPiece::InteriorExtended
                } else {
                    WallPiece::ExteriorExtended
                };
                change_vertical_wall(context, Some(g), piece);
            }
        }
        return;
    }
    // C
    if let Some(vertical) = vertical_wall_type(context, Some(g), materials) {
        if Some(vertical) == w && horizontal_wall_type(context, at(g, -1, 0), materials) == w {
            let side = if floor(context, 0, 1) {
                WallSide::ExteriorBottom
            } else {
                WallSide::InteriorBottom
            };
            build_wall_piece(context, g, side, style);
        }
        return;
    }
    // D: the walls around the tile weigh towards vertical or horizontal pieces
    let mut vertical_weights = [0u16; WALL_STYLES];
    let mut horizontal_weights = [0u16; WALL_STYLES];
    // Both walls of the tiles to the left and above count
    let neighbours = [
        (at(g, -1, 0), &mut horizontal_weights),
        (at(g, 0, -1), &mut vertical_weights),
    ];
    for (neighbour, weights) in neighbours {
        for style in [
            vertical_wall_type(context, neighbour, materials),
            horizontal_wall_type(context, neighbour, materials),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(weight) = weights.get_mut(usize::from(style)) {
                *weight += 1;
            }
        }
    }
    if let Some(style) = horizontal_wall_type(context, at(g, 1, 0), materials) {
        if let Some(weight) = horizontal_weights.get_mut(usize::from(style)) {
            *weight += 1;
        }
    }
    if let Some(style) = vertical_wall_type(context, at(g, 0, 1), materials) {
        if let Some(weight) = vertical_weights.get_mut(usize::from(style)) {
            *weight += 1;
        }
    }
    // E: the weights of the style decide
    let index = usize::from(style);
    let vertical_weight = vertical_weights.get(index).copied().unwrap_or(0);
    let horizontal_weight = horizontal_weights.get(index).copied().unwrap_or(0);
    if vertical_weight > 0 || horizontal_weight > 0 {
        if vertical_weight >= horizontal_weight {
            let inside = floor(context, 1, 0);
            let (side, extended) = if inside {
                (WallSide::ExteriorRight, WallPiece::InteriorExtended)
            } else {
                (WallSide::InteriorRight, WallPiece::ExteriorExtended)
            };
            build_wall_piece(context, g, side, style);
            // A new top right corner is covered by an extended piece
            if hwall(context, 0, -1) && !hwall(context, 1, -1) && !vwall(context, 0, -1) {
                change_vertical_wall(context, Some(g), extended);
            } else if hwall(context, 0, -1) && !hwall(context, -1, -1) && !vwall(context, -1, -1) {
                change_vertical_wall(context, Some(g), extended);
                erase_horizontal_wall(context, at(g, 0, -1));
            }
        } else {
            if vwall(context, -1, 0) && !vwall(context, -1, -1) && !hwall(context, -1, -1) {
                erase_vertical_wall(context, at(g, -1, 0));
            }
            let inside = floor(context, 0, 1);
            let (side, extended, bottom_end) = if inside {
                (
                    WallSide::ExteriorBottom,
                    WallPiece::InteriorExtended,
                    WallPiece::InteriorBottomEnd,
                )
            } else {
                (
                    WallSide::InteriorBottom,
                    WallPiece::ExteriorExtended,
                    WallPiece::ExteriorBottomEnd,
                )
            };
            build_wall_piece(context, g, side, style);
            if vwall(context, 0, 1) {
                change_vertical_wall(context, at(g, 0, 1), extended);
            }
            if vwall(context, -1, 1) && !vwall(context, -1, 0) {
                change_vertical_wall(context, at(g, -1, 1), extended);
            } else if vwall(context, -1, 0) && !vwall(context, -1, 1) && context.has_floor(g) {
                change_vertical_wall(context, at(g, -1, 0), bottom_end);
            }
        }
        return;
    }
    // F: connect to a wall of the style below or to the right
    if horizontal_wall_type(context, at(g, 0, 1), materials) == w {
        if !hwall(context, -1, 1) {
            erase_horizontal_wall(context, at(g, 0, 1));
        }
        let inside = floor(context, 1, 0);
        let side = if inside {
            WallSide::ExteriorRight
        } else {
            WallSide::InteriorRight
        };
        build(context, at(g, 0, 1), side, style);
        build_wall_piece(context, g, side, style);
        // Overrides the smoothing of the pieces
        let piece = match (inside, !vwall(context, 0, 2), floor(context, 1, 2)) {
            (true, true, true) => WallPiece::InteriorBottomEnd,
            (true, _, _) => WallPiece::InteriorRight,
            (false, true, false) => WallPiece::ExteriorBottomEnd,
            (false, _, _) => WallPiece::ExteriorRight,
        };
        change_vertical_wall(context, at(g, 0, 1), piece);
        return;
    }
    if vertical_wall_type(context, at(g, 1, 0), materials) == w {
        let inside = floor(context, 0, 1);
        let (side, extended, bottom_end) = if inside {
            (
                WallSide::ExteriorBottom,
                WallPiece::InteriorExtended,
                WallPiece::InteriorBottomEnd,
            )
        } else {
            (
                WallSide::InteriorBottom,
                WallPiece::ExteriorExtended,
                WallPiece::ExteriorBottomEnd,
            )
        };
        build(context, at(g, 1, 0), side, style);
        build_wall_piece(context, g, side, style);
        if !vwall(context, 1, -1) {
            erase_vertical_wall(context, at(g, 1, 0));
            change_vertical_wall(context, at(g, 1, 1), extended);
        }
        if !vwall(context, 1, 1) {
            let piece = if !hwall(context, 1, -1) && !vwall(context, 1, -1) && hwall(context, 2, -1)
            {
                extended
            } else {
                bottom_end
            };
            change_vertical_wall(context, at(g, 1, 0), piece);
        }
    }
}

/// The tile of a building piece of a style with the orientation of a wall
fn matching_tile(
    context: &EditContext,
    style: u8,
    piece: BuildingPiece,
    wall: &WallNode,
) -> Option<Arc<Tile>> {
    let orientation = wall.tile.wall_orientation();
    context
        .tileset
        .building_tiles(style, piece)
        .iter()
        .find(|tile| tile.wall_orientation() == orientation)
        .cloned()
}

/// Replaces the vertical and the horizontal wall of a tile by what `replacement` returns for
/// them, `None` removes the wall
fn replace_walls(
    context: &mut EditContext,
    grid_no: GridNo,
    replacement: impl Fn(&mut EditContext, &WallNode) -> Option<Option<Arc<Tile>>>,
) {
    for vertical in [true, false] {
        let wall = if vertical {
            context.vertical_wall(grid_no)
        } else {
            context.horizontal_wall(grid_no)
        };
        let Some(wall) = wall else {
            continue;
        };
        match replacement(context, &wall) {
            Some(Some(tile)) => context.replace_struct(grid_no, wall.index, tile),
            Some(None) => context.remove_struct(grid_no, wall.index),
            None => {}
        }
    }
}

/// Replaces the walls of a tile by doors of a style with the same orientation (`PasteSmartDoor`)
pub fn paste_smart_door(context: &mut EditContext, grid_no: GridNo, style: u8, open: bool) {
    replace_walls(context, grid_no, |context, wall| {
        matching_tile(context, style, BuildingPiece::Door { open }, wall).map(Some)
    });
}

/// Replaces the walls of a tile by windows of their wall style (`PasteSmartWindow`)
pub fn paste_smart_window(
    context: &mut EditContext,
    grid_no: GridNo,
    window: u8,
    materials: &Materials,
) {
    replace_walls(context, grid_no, |context, wall| {
        let style = wall_type(context, wall, grid_no, materials)?;
        let piece = BuildingPiece::Wall(WallPiece::Window { style: window });
        matching_tile(context, style, piece, wall).map(Some)
    });
}

/// Replaces the walls of a tile by broken, cracked or smudged walls of their style, or removes
/// them for a hole (`PasteSmartBrokenWall`)
pub fn paste_smart_broken_wall(
    context: &mut EditContext,
    grid_no: GridNo,
    broken: SmartBrokenWall,
    materials: &Materials,
) {
    replace_walls(context, grid_no, |context, wall| match broken {
        SmartBrokenWall::Hole => Some(None),
        SmartBrokenWall::Piece(piece) => {
            let style = wall_type(context, wall, grid_no, materials)?;
            matching_tile(context, style, BuildingPiece::Wall(piece), wall).map(Some)
        }
    });
}

/// Whether a wall or door is something the smart method places instead of a plain wall
fn replaces_wall(wall: &WallNode) -> bool {
    wall.tile.building.is_some_and(|b| {
        matches!(
            b.piece,
            BuildingPiece::Door { .. }
                | BuildingPiece::Wall(
                    WallPiece::Window { .. }
                        | WallPiece::Broken { .. }
                        | WallPiece::Cracked
                        | WallPiece::Smudged
                )
        )
    })
}

/// Erasing with the smart method: doors, windows and broken walls become plain walls again,
/// plain walls are removed and tiles without walls stay as they are
pub fn erase_smart(context: &mut EditContext, grid_no: GridNo, materials: &Materials) {
    let vertical = context.vertical_wall(grid_no);
    let horizontal = context.horizontal_wall(grid_no);
    if vertical.iter().chain(&horizontal).any(replaces_wall) {
        restore_walls(context, grid_no, materials);
    } else if vertical.is_some() || horizontal.is_some() {
        erase_walls(context, grid_no);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{
            building::tests::{add_room, building_fixture, materials, rect, walls},
            building_mode::paste_single,
            document::tests::{grid, Fixture},
        },
        tileset::Orientation,
    };

    fn place(fixture: &mut Fixture, x: u8, y: u8, name: &str, sub_index: u16) {
        let mut context = fixture.context();
        let tile = context.tile(name, sub_index).unwrap();
        paste_single(&mut context, grid(x, y), tile);
        context.finish();
    }

    fn smart_wall(fixture: &mut Fixture, x: u8, y: u8, style: u8) {
        let materials = materials(fixture, &[]);
        let mut context = fixture.context();
        paste_smart_wall(&mut context, grid(x, y), style, &materials);
        context.finish();
    }

    /// `(type, sub index, orientation)` of the structs of a tile
    fn structs(fixture: &Fixture, x: u8, y: u8) -> Vec<(String, u16, Orientation)> {
        let mut structs: Vec<_> = fixture.map.tiles[grid(x, y)]
            .structs
            .iter()
            .map(|t| (t.type_name.clone(), t.sub_index, t.wall_orientation()))
            .collect();
        structs.sort_by_key(|(name, sub_index, _)| (name.clone(), *sub_index));
        structs
    }

    #[test]
    fn smart_walls_continue_the_walls_of_their_style() {
        let mut fixture = building_fixture();
        // A horizontal wall continues horizontally
        place(&mut fixture, 60, 60, "WALL1", 10);
        smart_wall(&mut fixture, 61, 60, 0);
        assert_eq!(walls(&fixture, 61, 60), [(WallPiece::ExteriorLeft, true)]);
        // A vertical wall continues vertically
        place(&mut fixture, 70, 60, "WALL1", 7);
        smart_wall(&mut fixture, 70, 61, 0);
        assert_eq!(walls(&fixture, 70, 61), [(WallPiece::ExteriorRight, false)]);
        // Inside of a building, the pieces are seen from inside
        let mut context = fixture.context();
        let floor = context.tile("FLOOR1", 1).unwrap();
        context.add_land_to_head(grid(71, 62), floor);
        context.finish();
        smart_wall(&mut fixture, 70, 62, 0);
        let (piece, horizontal) = walls(&fixture, 70, 62)[0];
        assert!(!horizontal);
        assert!(
            matches!(
                piece,
                WallPiece::InteriorRight | WallPiece::InteriorExtended
            ),
            "{piece:?}"
        );

        // Walls of other styles don't count
        place(&mut fixture, 80, 60, "WALL2", 10);
        smart_wall(&mut fixture, 81, 60, 0);
        assert!(walls(&fixture, 81, 60).is_empty());
        // A tile with both walls stays
        place(&mut fixture, 90, 60, "WALL1", 10);
        place(&mut fixture, 90, 60, "WALL1", 7);
        smart_wall(&mut fixture, 90, 60, 0);
        assert_eq!(walls(&fixture, 90, 60).len(), 2);
    }

    #[test]
    fn smart_walls_connect_to_walls_below() {
        let mut fixture = building_fixture();
        // A horizontal wall below: two vertical pieces lead down to it
        place(&mut fixture, 60, 61, "WALL1", 10);
        smart_wall(&mut fixture, 60, 60, 0);
        assert!(walls(&fixture, 60, 60).iter().any(|&(_, h)| !h));
        assert!(walls(&fixture, 60, 61).iter().any(|&(_, h)| !h));
    }

    #[test]
    fn smart_doors_windows_and_broken_walls_keep_the_orientation() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let materials = materials(&fixture, &[]);
        let right = (52, 51);
        let top = (51, 49);
        assert_eq!(
            structs(&fixture, right.0, right.1)[0].2,
            Orientation::OUTSIDE_RIGHT
        );
        assert_eq!(
            structs(&fixture, top.0, top.1)[0].2,
            Orientation::INSIDE_LEFT
        );

        let mut context = fixture.context();
        paste_smart_door(&mut context, grid(right.0, right.1), 1, false);
        paste_smart_door(&mut context, grid(top.0, top.1), 2, true);
        context.finish();
        assert_eq!(
            structs(&fixture, right.0, right.1),
            [("DOOR2".to_owned(), 1, Orientation::OUTSIDE_RIGHT)]
        );
        assert_eq!(
            structs(&fixture, top.0, top.1),
            [("DOOR3".to_owned(), 20, Orientation::INSIDE_LEFT)]
        );

        // Windows take the style of the nearest wall for doors
        let mut context = fixture.context();
        paste_smart_window(&mut context, grid(right.0, right.1), 2, &materials);
        paste_smart_window(&mut context, grid(50, 49), 1, &materials);
        context.finish();
        assert_eq!(
            structs(&fixture, right.0, right.1),
            [("WALL1".to_owned(), 47, Orientation::OUTSIDE_RIGHT)]
        );
        assert_eq!(
            structs(&fixture, 50, 49),
            [("WALL1".to_owned(), 37, Orientation::INSIDE_LEFT)]
        );

        let mut context = fixture.context();
        let cracked = SmartBrokenWall::Piece(WallPiece::Cracked);
        paste_smart_broken_wall(&mut context, grid(right.0, right.1), cracked, &materials);
        let broken = SmartBrokenWall::Piece(WallPiece::Broken { variant: 1 });
        paste_smart_broken_wall(&mut context, grid(52, 50), broken, &materials);
        paste_smart_broken_wall(
            &mut context,
            grid(52, 49),
            SmartBrokenWall::Hole,
            &materials,
        );
        context.finish();
        assert_eq!(
            structs(&fixture, right.0, right.1),
            [("WALL1".to_owned(), 63, Orientation::OUTSIDE_RIGHT)]
        );
        assert_eq!(
            structs(&fixture, 52, 50),
            [("WALL1".to_owned(), 51, Orientation::OUTSIDE_RIGHT)]
        );
        assert!(structs(&fixture, 52, 49).is_empty());
    }

    #[test]
    fn restoring_replaces_doors_by_walls() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let materials = materials(&fixture, &[]);
        let mut context = fixture.context();
        paste_smart_door(&mut context, grid(52, 51), 0, false);
        restore_walls(&mut context, grid(52, 51), &materials);
        context.finish();
        let structs = structs(&fixture, 52, 51);
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].0, "WALL1");
        assert!(structs[0].2.is_vertical());
    }

    #[test]
    fn erasing_restores_doors_and_removes_walls() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let materials = materials(&fixture, &[]);
        let mut context = fixture.context();
        paste_smart_door(&mut context, grid(52, 51), 0, false);
        erase_smart(&mut context, grid(52, 51), &materials);
        context.finish();
        // The door went back to a wall
        let restored = structs(&fixture, 52, 51);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].0, "WALL1");
        assert!(restored[0].2.is_vertical());

        let mut context = fixture.context();
        erase_smart(&mut context, grid(52, 51), &materials);
        // Next to the walls of the room, but without walls itself
        erase_smart(&mut context, grid(53, 51), &materials);
        context.finish();
        assert!(structs(&fixture, 52, 51).is_empty());
        assert!(structs(&fixture, 53, 51).is_empty());
    }
}
