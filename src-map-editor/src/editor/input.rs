//! Painting with the mouse and the brush cursor
//!
//! A stroke lasts from pressing a mouse button until it is released or the cursor leaves the
//! viewer, and it is undone as a whole. All modes paint with the left button, some building
//! tools also do something with the right one. The brush is applied once to every tile the cursor
//! enters, not every frame like in the original editor. The line and area brushes are applied once
//! when the button is released, leaving the viewer cancels them. On the roof floor the modes that
//! only edit the ground don't paint.
//!
//! The cursor is drawn on the floor: on the roof it is raised to the roofs, with lines down to
//! the tiles on the ground.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    components::map::{game_to_world, GridPosition, JA2_TILE_HEIGHT, JA2_TILE_WIDTH},
    map::{GridNo, Level, TacticalMap},
    render::{soldiers::ROOF_HEIGHT_ADJUSTMENT, RenderOptions},
    systems::cursor::CursorInfo,
    tileset::Tileset,
};

use super::{
    brush::{tiles_between, Brush},
    commands::CurrentMode,
    mode::EditorMode,
    MapEdit,
};

/// The stroke in progress
#[derive(Resource, Clone, Debug, Default)]
pub struct EditorStroke {
    pub active: bool,
    /// Where the stroke started
    pub anchor: Option<GridPosition>,
    /// The last tile the brush was on, `None` while the cursor is off the map
    pub last: Option<GridPosition>,
    /// The button that started the stroke
    pub button: Option<MouseButton>,
}

/// The mode and its brush, `None` while only viewing and for the modes that don't edit the floor
fn editing_mode<'a>(
    mode: &'a CurrentMode,
    edit: &MapEdit,
    floor: Level,
) -> Option<(&'a EditorMode, Brush)> {
    if floor == Level::Roof && mode.0.is_ground_only() {
        return None;
    }
    let brush = mode.0.brush(edit.tileset(), edit.smoothing())?;
    Some((&mode.0, brush))
}

pub fn paint(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    mut stroke: ResMut<EditorStroke>,
    mut edit: MapEdit,
) {
    let floor = options.floor;
    let mode = editing_mode(&mode, &edit, floor);
    let in_viewer = cursor.world_position.is_some();
    let button = stroke.button.unwrap_or(MouseButton::Left);
    let pressed = mouse.pressed(button);

    if stroke.active {
        let Some((mode, brush)) = mode.filter(|_| pressed && in_viewer) else {
            // The stroke ends
            if let (Some((mode, brush)), false, Some(anchor), Some(last)) =
                (mode, pressed, stroke.anchor, stroke.last)
            {
                if brush.applies_on_release() {
                    let area = brush.region(Some(anchor), last);
                    edit.edit(|context| mode.apply(context, &area, button, &options));
                }
            }
            *stroke = EditorStroke::default();
            edit.document().undo.end();
            return;
        };
        let Some(tile) = cursor.tile else {
            if !brush.applies_on_release() {
                // Painting starts again where the cursor comes back
                stroke.last = None;
            }
            return;
        };
        if stroke.last == Some(tile) {
            return;
        }
        if !brush.applies_on_release() {
            let tiles = match stroke.last {
                Some(last) => tiles_between(last, tile),
                None => vec![tile],
            };
            edit.edit(|context| {
                for tile in tiles {
                    mode.apply(context, &brush.region(None, tile), button, &options);
                }
            });
        }
        stroke.last = Some(tile);
        return;
    }

    let (Some((mode, brush)), Some(tile)) = (mode, cursor.tile) else {
        return;
    };
    let Some(button) = [MouseButton::Left, MouseButton::Right]
        .into_iter()
        .find(|&button| mouse.just_pressed(button) && mode.uses_button(button))
    else {
        return;
    };
    *stroke = EditorStroke {
        active: true,
        anchor: Some(tile),
        last: Some(tile),
        button: Some(button),
    };
    edit.document().undo.begin();
    if !brush.applies_on_release() {
        edit.edit(|context| mode.apply(context, &brush.region(None, tile), button, &options));
    }
}

/// The edges around a set of tiles in world space
///
/// Each tile is a diamond, raised by its terrain height like the renderer draws it. Its edges are
/// drawn where the neighbouring tile is not in the set or lies at another height, so that only
/// the outline of the brush is drawn, whatever its shape.
pub(super) fn area_outline(tiles: &[GridNo], map: &TacticalMap) -> Vec<(Vec2, Vec2)> {
    let set: HashSet<GridNo> = tiles.iter().copied().collect();
    let size = IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT);
    let top = IVec2::new(size.x / 2, 0);
    let right = IVec2::new(size.x, size.y / 2);
    let bottom = IVec2::new(size.x / 2, size.y);
    let left = IVec2::new(0, size.y / 2);
    // The neighbour across each edge of the diamond
    let edges = [
        ((0, -1), top, right),
        ((1, 0), right, bottom),
        ((0, 1), bottom, left),
        ((-1, 0), left, top),
    ];
    let mut lines = vec![];
    for &grid_no in tiles {
        let position = grid_no.position();
        let height = map.tiles[grid_no].height;
        let raised = position.tile_box() - IVec2::new(0, i32::from(height));
        let corner = |offset: IVec2| game_to_world(raised + offset);
        for ((dx, dy), from, to) in edges {
            let neighbour =
                GridPosition::from_xy(i32::from(position.x) + dx, i32::from(position.y) + dy)
                    .map(GridNo::from);
            let joined =
                neighbour.is_some_and(|n| set.contains(&n) && map.tiles[n].height == height);
            if !joined {
                lines.push((corner(from), corner(to)));
            }
        }
    }
    lines
}

/// How much higher the roof level is drawn, in world pixels (`SECOND_LEVEL_Z_OFFSET`)
pub const ROOF_RAISE: f32 = ROOF_HEIGHT_ADJUSTMENT as f32;

/// Lines raised onto the roof for the roof level
pub fn raised(lines: Vec<(Vec2, Vec2)>, level: Level) -> Vec<(Vec2, Vec2)> {
    if level == Level::Ground {
        return lines;
    }
    let raise = Vec2::new(0., ROOF_RAISE);
    lines
        .into_iter()
        .map(|(from, to)| (from + raise, to + raise))
        .collect()
}

/// The outline of a cursor on a floor: unchanged on the ground, on the roof raised with
/// vertical lines from its leftmost, rightmost, top and bottom corners down to the ground
pub fn floor_outline(lines: Vec<(Vec2, Vec2)>, floor: Level) -> Vec<(Vec2, Vec2)> {
    if floor == Level::Ground {
        return lines;
    }
    let corners: Vec<Vec2> = lines.iter().flat_map(|&(from, to)| [from, to]).collect();
    let extreme = |better: fn(Vec2, Vec2) -> bool| {
        corners
            .iter()
            .copied()
            .reduce(|best, corner| if better(corner, best) { corner } else { best })
    };
    let mut drops: Vec<Vec2> = vec![];
    for corner in [
        extreme(|a, b| a.x < b.x),
        extreme(|a, b| a.x > b.x),
        extreme(|a, b| a.y > b.y),
        extreme(|a, b| a.y < b.y),
    ]
    .into_iter()
    .flatten()
    {
        if !drops.contains(&corner) {
            drops.push(corner);
        }
    }
    let raise = Vec2::new(0., ROOF_RAISE);
    let mut outline = raised(lines, floor);
    outline.extend(drops.into_iter().map(|corner| (corner + raise, corner)));
    outline
}

/// The colour of a cursor of a mode that doesn't edit the floor
pub const INACTIVE_CURSOR: Srgba = Srgba::new(0.6, 0.6, 0.6, 0.8);

/// Outlines the tiles the brush covers, red while erasing, grey on the roof for the modes that
/// only edit the ground
pub fn draw_brush_cursor(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    stroke: Res<EditorStroke>,
    tileset: Res<Tileset>,
    map: Res<TacticalMap>,
) {
    let Some(tile) = cursor.tile else {
        return;
    };
    // The selection draws the buildings instead
    if mode.0.selects_buildings() {
        return;
    }
    let Some(brush) = mode.0.brush(&tileset, map.info.editor_smoothing) else {
        return;
    };
    let floor = options.floor;
    let color = if mode.0.erases() {
        Srgba::RED
    } else if floor == Level::Roof && mode.0.is_ground_only() {
        INACTIVE_CURSOR
    } else {
        Srgba::GREEN
    };
    let anchor = stroke.anchor.filter(|_| stroke.active);
    let outline = area_outline(&brush.region(anchor, tile).tiles, &map);
    for (from, to) in floor_outline(outline, floor) {
        gizmos.line_2d(from, to, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::brush::Brush, tileset::TilesetId};

    fn outline(brush: Brush) -> Vec<(Vec2, Vec2)> {
        let map = TacticalMap::empty(TilesetId::new(0));
        area_outline(&brush.region(None, GridPosition::new(10, 10)).tiles, &map)
    }

    #[test]
    fn outlines_go_around_the_tiles() {
        let single = outline(Brush::Square { size: 1 });
        assert_eq!(single.len(), 4);
        // A closed diamond, 40×20
        for (i, (_, to)) in single.iter().enumerate() {
            assert_eq!(*to, single[(i + 1) % 4].0);
        }
        assert_eq!(single[1].0 - single[0].0, Vec2::new(20., -10.));

        let square = outline(Brush::Square { size: 3 });
        assert_eq!(square.len(), 12);
        // No edge inside the square: every edge is on the diamond around the 3×3 tiles
        let area = Brush::Square { size: 3 }.region(None, GridPosition::new(10, 10));
        let center = game_to_world(
            GridPosition::new(10, 10).tile_box() + IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT) / 2,
        );
        for (from, to) in &square {
            let middle = (*from + *to) / 2. - center;
            // On the diamond with the half sizes 60 and 30
            assert_eq!(middle.x.abs() + 2. * middle.y.abs(), 60.);
        }
        assert_eq!(area.tiles.len(), 9);

        // Every tile of a circle is on the outline or surrounded
        let circle = outline(Brush::Circle { size: 5 });
        assert_eq!(circle.len(), 5 * 4);

        // Raised tiles are outlined where they are drawn, apart from their neighbours
        let mut map = TacticalMap::empty(TilesetId::new(0));
        map.tiles[GridNo::from(GridPosition::new(10, 10))].height = 30;
        let area = Brush::Square { size: 3 }.region(None, GridPosition::new(10, 10));
        let raised = area_outline(&area.tiles, &map);
        assert_eq!(raised.len(), 12 + 4 + 4);
        let flat = area_outline(&area.tiles[..1], &map);
        let top = game_to_world(GridPosition::new(9, 9).tile_box() + IVec2::new(20, 0));
        assert_eq!(flat[0].0, top);
        let center = GridPosition::new(10, 10).tile_box() + IVec2::new(20, -30);
        assert!(raised
            .iter()
            .any(|&(from, _)| from == game_to_world(center)));
    }

    #[test]
    fn floor_outlines_are_raised_onto_the_roof() {
        let single = outline(Brush::Square { size: 1 });
        assert_eq!(floor_outline(single.clone(), Level::Ground), single);

        let roof = floor_outline(single.clone(), Level::Roof);
        assert_eq!(roof.len(), 4 + 4);
        let raise = Vec2::new(0., ROOF_RAISE);
        for (line, (from, to)) in single.iter().zip(&roof) {
            assert_eq!((line.0 + raise, line.1 + raise), (*from, *to));
        }
        // Vertical lines from every corner of the raised tile down to the ground
        let mut drops: Vec<(Vec2, Vec2)> = roof[4..].to_vec();
        let mut corners: Vec<Vec2> = single.iter().map(|&(from, _)| from).collect();
        drops.sort_by(|a, b| a.1.x.total_cmp(&b.1.x).then(a.1.y.total_cmp(&b.1.y)));
        corners.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        for ((from, to), corner) in drops.iter().zip(&corners) {
            assert_eq!(*to, *corner);
            assert_eq!(*from, *corner + raise);
        }

        // Larger areas only get the lines at their outermost corners
        let square = floor_outline(outline(Brush::Square { size: 3 }), Level::Roof);
        assert_eq!(square.len(), 12 + 4);
    }
}
