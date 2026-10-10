//! What the lights mode draws over the map: a ring on every light (`GOODRING1` of the original
//! editor) in the colour of its time like the labels of the original (prime time orange, night
//! red, always yellow), the selected light and the tile under the cursor, which depends on the
//! tool
//!
//! The rings show all lights, the lighting of the scene only the ones that are on at the time of
//! day of the render options.

use bevy::prelude::*;

use crate::{
    light::LightTemplates,
    map::{light_template_name, GridNo, Level, LightTime, TacticalMap},
    render::RenderOptions,
    systems::cursor::CursorInfo,
};

use super::{
    commands::CurrentMode,
    input::INACTIVE_CURSOR,
    item_overlay::ring,
    lights::{light_mode, map_light, LightSelection, LightTool},
    soldier_events::tile_center,
    soldier_overlay::{cursor_outline, tile_outline},
};

/// The colour of the ring of a light
pub fn time_color(time: LightTime) -> Srgba {
    match time {
        LightTime::PrimeTime => Srgba::rgb(1., 0.55, 0.),
        LightTime::Night => Srgba::rgb(1., 0.15, 0.15),
        LightTime::Always => Srgba::rgb(1., 0.95, 0.2),
    }
}

/// A ring around the middle of a tile, twice as large as the ring of the lights
pub fn outer_ring(map: &TacticalMap, grid_no: GridNo) -> Vec<(Vec2, Vec2)> {
    let center = tile_center(map, grid_no);
    ring(map, grid_no)
        .into_iter()
        .map(|(from, to)| (center + (from - center) * 2., center + (to - center) * 2.))
        .collect()
}

/// Draws the rings of the lights, the selected light in white and the tile under the cursor:
/// for placing green where a click places a light, red where the template of the radius is
/// missing, for selecting green where the selected light is dragged to and faintly a tile with
/// lights that a click selects. On the roof floor, where lights aren't edited, the cursor is
/// grey. The brush that erases is drawn by `draw_brush_cursor`.
pub fn draw_light_markers(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    selection: Res<LightSelection>,
    map: Res<TacticalMap>,
    templates: Option<Res<LightTemplates>>,
) {
    let Some(mode) = light_mode(&mode) else {
        return;
    };
    let mut lines = |lines: Vec<(Vec2, Vec2)>, color: Srgba| {
        for (from, to) in lines {
            gizmos.line_2d(from, to, color);
        }
    };
    for light in &map.lights {
        lines(
            ring(&map, GridNo::from(light.position)),
            time_color(light.time),
        );
    }
    let selected = selection
        .selected
        .and_then(|id| map_light(&map, id))
        .map(|light| GridNo::from(light.position));
    if let Some(grid_no) = selected {
        lines(outer_ring(&map, grid_no), Srgba::WHITE);
        lines(tile_outline(&map, grid_no, Level::Ground), Srgba::WHITE);
    }
    let Some(tile) = cursor.tile.map(GridNo::from) else {
        return;
    };
    if options.floor == Level::Roof {
        if mode.tool == LightTool::Select || !mode.erase {
            lines(cursor_outline(&map, tile, Level::Roof), INACTIVE_CURSOR);
        }
        return;
    }
    match mode.tool {
        LightTool::Place if mode.erase => {}
        LightTool::Place => {
            let template = light_template_name(mode.radius);
            let can_place = templates.is_some_and(|templates| templates.get(&template).is_some());
            let color = if can_place { Srgba::GREEN } else { Srgba::RED };
            lines(tile_outline(&map, tile, Level::Ground), color);
            if can_place {
                lines(ring(&map, tile), time_color(mode.time));
            }
        }
        LightTool::Select if selection.dragging() => {
            if selected.is_some_and(|selected| selected != tile) {
                lines(outer_ring(&map, tile), Srgba::GREEN);
                lines(tile_outline(&map, tile, Level::Ground), Srgba::GREEN);
            }
        }
        LightTool::Select => {
            let position = tile.position();
            let has_lights = map.lights.iter().any(|light| light.position == position);
            if has_lights && selected != Some(tile) {
                lines(
                    tile_outline(&map, tile, Level::Ground),
                    Srgba::new(1., 1., 1., 0.35),
                );
            }
        }
    }
}
