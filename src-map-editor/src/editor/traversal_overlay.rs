//! What the traversal mode draws over the map: the exit grids, the entry points with their
//! letters, and the edge points
//!
//! Exit grids get a cyan outline (`FIRSTPOINTERS8` of the original), the one under the cursor and
//! the selected one are labelled with their destination. Entry points get a ring in their own colour
//! and a letter (N, E, S, W, C, I). Edge points are small diamonds, the primary ones green and
//! the secondary ones orange, the inner row dimmer than the outer one. The labels are text on
//! the screen camera like the room numbers (`rooms_overlay.rs`).

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};

use crate::{
    map::{EdgePointList, EntryPoint, GridNo, Level, TacticalMap},
    render::RenderOptions,
    states::{AppState, UiFont},
    systems::{
        cursor::CursorInfo,
        pixel_view::{PixelView, SCREEN_LAYER},
    },
};

use super::{
    commands::CurrentMode,
    input::INACTIVE_CURSOR,
    item_overlay::ring,
    light_overlay::outer_ring,
    soldier_events::tile_center,
    soldier_overlay::{cursor_outline, tile_outline},
    traversal::{
        exit_grid_at, items_on, traversal_mode, TraversalItem, TraversalPlace, TraversalSelection,
        TraversalTool,
    },
};

/// Size of the labels in logical pixels
const FONT_SIZE: f32 = 12.;

const EXIT_GRID_COLOR: Srgba = Srgba::rgb(0., 0.9, 0.9);

/// The colour of an entry point
pub fn entry_color(point: EntryPoint) -> Srgba {
    match point {
        EntryPoint::North => Srgba::rgb(1., 0.3, 0.3),
        EntryPoint::East => Srgba::rgb(1., 0.85, 0.2),
        EntryPoint::South => Srgba::rgb(0.4, 0.6, 1.),
        EntryPoint::West => Srgba::rgb(1., 0.5, 1.),
        EntryPoint::Center => Srgba::WHITE,
        EntryPoint::Isolated => Srgba::rgb(0.7, 0.4, 1.),
    }
}

/// The letter of an entry point
pub fn entry_letter(point: EntryPoint) -> &'static str {
    match point {
        EntryPoint::North => "N",
        EntryPoint::East => "E",
        EntryPoint::South => "S",
        EntryPoint::West => "W",
        EntryPoint::Center => "C",
        EntryPoint::Isolated => "I",
    }
}

/// How much of the edge points that don't belong to the selected entry point is drawn
const UNRELATED_ALPHA: f32 = 0.15;

/// Whether the edge points of a side (in the order of `EdgePointSet::sides`: north, east, south,
/// west) and set belong to an entry point: a side's entry point uses both sets of its side, the
/// isolated one all secondary sets, and the center one none (it only seeds the reachability test)
fn related_edge_points(point: EntryPoint, side: usize, secondary: bool) -> bool {
    match point {
        EntryPoint::North => side == 0,
        EntryPoint::East => side == 1,
        EntryPoint::South => side == 2,
        EntryPoint::West => side == 3,
        EntryPoint::Isolated => secondary,
        EntryPoint::Center => false,
    }
}

/// A diamond of a fifth of the size of a tile in its middle
fn small_diamond(map: &TacticalMap, grid_no: GridNo) -> Vec<(Vec2, Vec2)> {
    let center = tile_center(map, grid_no);
    tile_outline(map, grid_no, Level::Ground)
        .into_iter()
        .map(|(from, to)| (center + (from - center) / 5., center + (to - center) / 5.))
        .collect()
}

/// Draws the exit grids, the entry points, the edge points (with the select tool, unless an exit
/// grid is selected, the ones of the selected entry point highlighted), the selection in white
/// and the tile under the cursor: for placing an entry point in its colour, for selecting green where the
/// selection is dragged to and faintly a tile that a click selects something on, grey on the
/// roof floor where nothing is edited. The brush of the exit grids is drawn by
/// `draw_brush_cursor`.
pub fn draw_traversal_markers(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    selection: Res<TraversalSelection>,
    map: Res<TacticalMap>,
) {
    let Some(mode) = traversal_mode(&mode) else {
        return;
    };
    let mut lines = |lines: Vec<(Vec2, Vec2)>, color: Srgba| {
        for (from, to) in lines {
            gizmos.line_2d(from, to, color);
        }
    };
    let cursor_tile = cursor.tile.map(GridNo::from);

    for grid in &map.exit_grids {
        lines(
            tile_outline(&map, grid.grid_no, Level::Ground),
            EXIT_GRID_COLOR,
        );
    }
    let placing_exit_grids =
        mode.tool == TraversalTool::Place && mode.place == TraversalPlace::Exit;
    if let Some(grid) = cursor_tile
        .filter(|_| placing_exit_grids)
        .and_then(|tile| exit_grid_at(&map, tile))
    {
        lines(ring(&map, grid.grid_no), Srgba::WHITE);
    }

    let show_edge_points = mode.tool == TraversalTool::Select
        && !matches!(selection.selected, Some(TraversalItem::Exit(_)));
    let selected_entry = match selection.selected {
        Some(TraversalItem::Entry(point)) => Some(point),
        _ => None,
    };
    let mut edge_points = |list: &EdgePointList, color: Srgba, side: usize, secondary: bool| {
        let alpha = match selected_entry {
            Some(point) if !related_edge_points(point, side, secondary) => UNRELATED_ALPHA,
            _ => 1.,
        };
        for (index, &grid_no) in list.grid_nos.iter().enumerate() {
            let inner = index >= usize::from(list.middle_index);
            let alpha = if inner { alpha * 0.45 } else { alpha };
            lines(small_diamond(&map, grid_no), color.with_alpha(alpha));
        }
    };
    if show_edge_points {
        for (side, list) in map.edge_points.primary.sides().into_iter().enumerate() {
            edge_points(list, Srgba::rgb(0.2, 1., 0.2), side, false);
        }
        if let Some(secondary) = &map.edge_points.secondary {
            for (side, list) in secondary.sides().into_iter().enumerate() {
                edge_points(list, Srgba::rgb(1., 0.55, 0.), side, true);
            }
        }
    }

    for point in EntryPoint::ALL {
        if let Some(grid_no) = map.info.entry(point) {
            let color = entry_color(point);
            lines(ring(&map, grid_no), color);
            lines(tile_outline(&map, grid_no, Level::Ground), color);
        }
    }
    let selected = selection.selected.and_then(|item| item.grid_no(&map));
    if let Some(grid_no) = selected {
        lines(outer_ring(&map, grid_no), Srgba::WHITE);
        lines(tile_outline(&map, grid_no, Level::Ground), Srgba::WHITE);
    }
    let Some(tile) = cursor_tile else {
        return;
    };
    if options.floor == Level::Roof {
        if !placing_exit_grids {
            lines(cursor_outline(&map, tile, Level::Roof), INACTIVE_CURSOR);
        }
        return;
    }
    match mode.tool {
        TraversalTool::Place if mode.place == TraversalPlace::Entry => {
            lines(
                tile_outline(&map, tile, Level::Ground),
                entry_color(mode.entry),
            );
        }
        TraversalTool::Place => {}
        TraversalTool::Select if selection.dragging() => {
            if selected.is_some_and(|selected| selected != tile) {
                lines(outer_ring(&map, tile), Srgba::GREEN);
                lines(tile_outline(&map, tile, Level::Ground), Srgba::GREEN);
            }
        }
        TraversalTool::Select => {
            if !items_on(&map, tile).is_empty() && selected != Some(tile) {
                lines(
                    tile_outline(&map, tile, Level::Ground),
                    Srgba::new(1., 1., 1., 0.35),
                );
            }
        }
    }
}

/// A label of the overlay
#[derive(Component)]
pub struct TraversalLabel {
    /// Where the label is drawn, in world coordinates
    position: Vec2,
}

/// The labels that are shown: the text and the tile of each
#[derive(Resource, Default)]
pub struct TraversalLabels {
    shown: Vec<(String, GridNo, Entity)>,
}

pub fn reset_traversal_labels(mut labels: ResMut<TraversalLabels>) {
    *labels = TraversalLabels::default();
}

/// Shows the letters of the entry points and the destination of the exit grids under the cursor
/// and of the selected one, and moves the labels with the view
#[allow(clippy::too_many_arguments)]
pub fn update_traversal_labels(
    mut commands: Commands,
    mode: Res<CurrentMode>,
    cursor: Res<CursorInfo>,
    selection: Res<TraversalSelection>,
    map: Res<TacticalMap>,
    view: Res<PixelView>,
    window: Single<&Window, With<PrimaryWindow>>,
    font: Res<UiFont>,
    mut labels: ResMut<TraversalLabels>,
    mut transforms: Query<(&TraversalLabel, &mut Transform)>,
) {
    let mut wanted: Vec<(String, GridNo, Color)> = vec![];
    if traversal_mode(&mode).is_some() {
        for point in EntryPoint::ALL {
            if let Some(grid_no) = map.info.entry(point) {
                wanted.push((
                    entry_letter(point).to_owned(),
                    grid_no,
                    entry_color(point).into(),
                ));
            }
        }
        let selected = match selection.selected {
            Some(TraversalItem::Exit(grid_no)) => Some(grid_no),
            _ => None,
        };
        let hovered = cursor
            .tile
            .map(GridNo::from)
            .filter(|&tile| Some(tile) != selected);
        for grid in [selected, hovered]
            .into_iter()
            .flatten()
            .filter_map(|tile| exit_grid_at(&map, tile))
        {
            wanted.push((
                format!("{} {}", grid.dest_sector, grid.dest_grid_no.index()),
                grid.grid_no,
                EXIT_GRID_COLOR.into(),
            ));
        }
    }
    let same = wanted.len() == labels.shown.len()
        && wanted
            .iter()
            .zip(&labels.shown)
            .all(|((text, grid_no, _), (shown, tile, _))| text == shown && grid_no == tile);
    if !same {
        for (_, _, entity) in labels.shown.drain(..) {
            commands.entity(entity).despawn();
        }
        let text_font = font.text_font(FONT_SIZE * window.scale_factor());
        for (text, grid_no, color) in wanted {
            let position = tile_center(&map, grid_no) + Vec2::new(0., 14.);
            let entity = commands
                .spawn((
                    TraversalLabel { position },
                    Text2d::new(text.clone()),
                    text_font.clone(),
                    TextColor(color),
                    RenderLayers::layer(SCREEN_LAYER),
                    // New labels are not in `transforms` yet, so they start at their place
                    Transform::from_translation(view.world_to_screen(position).extend(10.)),
                    DespawnOnExit(AppState::Map),
                ))
                .id();
            labels.shown.push((text, grid_no, entity));
        }
    }
    // 1 unit of the screen camera is 1 physical pixel, above the map
    for (label, mut transform) in &mut transforms {
        let translation = view.world_to_screen(label.position).extend(10.);
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_points_of_entry_points() {
        for secondary in [false, true] {
            for (side, point) in [
                EntryPoint::North,
                EntryPoint::East,
                EntryPoint::South,
                EntryPoint::West,
            ]
            .into_iter()
            .enumerate()
            {
                for other in 0..4 {
                    assert_eq!(
                        related_edge_points(point, other, secondary),
                        other == side,
                        "{point:?} side {other} secondary {secondary}"
                    );
                }
            }
            for side in 0..4 {
                assert_eq!(
                    related_edge_points(EntryPoint::Isolated, side, secondary),
                    secondary
                );
                assert!(!related_edge_points(EntryPoint::Center, side, secondary));
            }
        }
    }
}
