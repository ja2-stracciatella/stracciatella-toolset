//! The edits and view changes of the scenarios, applied every frame

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    components::map::{game_rect_to_world, map_bounds, GridPosition, JA2_MAP_SIZE},
    editor::{
        commands::CurrentMode,
        lights::{LightMode, LightTool},
        mode::EditorMode,
        MapEdit,
    },
    map::{GridNo, LightTime},
    systems::{
        cursor::CursorInfo,
        pixel_view::{MapRedraw, PixelView},
    },
};

use super::{args::Scenario, report::Report, synthetic, verify::Verify, Bench, Synthetic};

/// How long each step of the zoom scenario lasts
const ZOOM_STEP_SECONDS: f32 = 2.;
/// Zoom and window size (relative to the requested size) of each step of the zoom scenario
const ZOOM_STEPS: [(f32, f32); 6] = [(1., 1.), (0.5, 1.), (2., 1.), (4., 1.), (1., 0.5), (1., 1.)];

/// How long each zoom of the pan scenario lasts
const PAN_ZOOM_SECONDS: f32 = 5.;
const PAN_ZOOMS: [f32; 4] = [0.5, 1., 2., 4.];
/// How far the pan scenario moves the view every frame, in physical pixels
const PAN_STEP: f32 = 16.;
/// Size of the loop of the pan scenario, relative to the size of the map
const PAN_LOOP: f32 = 0.4;

/// Row and columns that the cursor of the light preview scenario moves back and forth on,
/// through buildings, forests and yards
const LIGHT_PREVIEW_ROW: i32 = 80;
const LIGHT_PREVIEW_COLUMNS: std::ops::Range<i32> = 20..140;

/// The cursor tile of the light preview scenario in a frame
fn light_preview_tile(phase: u32) -> Option<GridPosition> {
    let span = LIGHT_PREVIEW_COLUMNS.len() as u32;
    let step = phase % (2 * span);
    let offset = if step < span {
        step
    } else {
        2 * span - 1 - step
    };
    GridPosition::from_xy(
        LIGHT_PREVIEW_COLUMNS.start + offset as i32,
        LIGHT_PREVIEW_ROW,
    )
}

/// `count` tiles on an even grid over the map
pub fn spread_tiles(count: usize) -> Vec<GridNo> {
    let side = (count as f32).sqrt().ceil().max(1.) as usize;
    let cell = f32::from(JA2_MAP_SIZE) / side as f32;
    let coordinate = |index: usize| ((index as f32 + 0.5) * cell) as i32;
    (0..count)
        .filter_map(|index| {
            GridPosition::from_xy(coordinate(index % side), coordinate(index / side))
                .map(GridNo::from)
        })
        .collect()
}

/// Applies the edits of the current scenario
pub fn drive(
    bench: Res<Bench>,
    report: Res<Report>,
    synthetic: Res<Synthetic>,
    mut edit: MapEdit,
    mut pixel_view: ResMut<PixelView>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    verify: Res<Verify>,
    mut redraw: ResMut<MapRedraw>,
    mut mode: ResMut<CurrentMode>,
    mut cursor: ResMut<CursorInfo>,
    mut phase: Local<u32>,
    mut pan_angle: Local<f32>,
) {
    if report.frame().is_none() {
        return;
    }
    // The view and the map stay as they are while they are compared with the CPU replay
    if verify.is_running() {
        return;
    }
    *phase += 1;
    let phase = *phase;
    match bench.scenario() {
        Scenario::Idle => {}
        Scenario::Zoom => {
            let step = (report.elapsed() / ZOOM_STEP_SECONDS) as usize % ZOOM_STEPS.len();
            let (zoom, size) = ZOOM_STEPS[step];
            if pixel_view.target_zoom != zoom {
                pixel_view.target_zoom = zoom;
                pixel_view.zoom = zoom;
            }
            let physical = (UVec2::new(bench.args.width, bench.args.height).as_vec2() * size)
                .round()
                .as_uvec2();
            if window.resolution.physical_size() != physical {
                window
                    .resolution
                    .set_physical_resolution(physical.x, physical.y);
            }
        }
        Scenario::Edits(count) => {
            let tiles = spread_tiles(count);
            edit.document().undo.begin();
            edit.edit(|context| {
                for &grid_no in &tiles {
                    synthetic::fill_tile(
                        context.modify(grid_no),
                        &synthetic.tiles,
                        grid_no.position(),
                        phase,
                    );
                }
            });
            edit.document().undo.end();
        }
        Scenario::Pan => {
            let step = (report.elapsed() / PAN_ZOOM_SECONDS) as usize % PAN_ZOOMS.len();
            let zoom = PAN_ZOOMS[step];
            if pixel_view.target_zoom != zoom {
                pixel_view.target_zoom = zoom;
                pixel_view.zoom = zoom;
            }
            // An ellipse around the middle of the map, at the same speed on the screen
            let bounds = map_bounds();
            let world = game_rect_to_world(bounds.min, bounds.max);
            let radii = world.half_size() * PAN_LOOP;
            *pan_angle += PAN_STEP / zoom / ((radii.x + radii.y) / 2.);
            let (sin, cos) = pan_angle.sin_cos();
            pixel_view.position = world.center() + radii * Vec2::new(cos, sin);
            // The view can't move where it is clamped to the map, e.g. at zoom 0.5. The world
            // pass is then still drawn every frame, like for a moving cursor.
            redraw.changed = true;
        }
        Scenario::LightPreview(radius) => {
            let light_mode = EditorMode::Lights(LightMode {
                tool: LightTool::Place,
                radius,
                // On at day as well
                time: LightTime::Always,
                ..default()
            });
            if mode.0 != light_mode {
                mode.0 = light_mode;
            }
            // Instead of the tile under the mouse
            cursor.tile = light_preview_tile(phase);
        }
        Scenario::Churn => {
            edit.document().undo.begin();
            edit.edit(|context| {
                for position in synthetic::positions() {
                    synthetic::fill_tile(
                        context.modify(position.into()),
                        &synthetic.tiles,
                        position,
                        phase,
                    );
                }
                // The soldiers move onto the free tiles of the new structures
                context.commit();
                *context.modify_lights() = synthetic::lights(&synthetic.light, phase);
                *context.modify_items() = synthetic::items(&synthetic.tiles, phase);
                let ids: Vec<_> = context.map.soldiers.iter().map(|s| s.id).collect();
                let targets = synthetic::spread(ids.len(), phase + 2);
                for (id, grid_no) in ids.into_iter().zip(targets) {
                    // Two soldiers can pick the same tile, then one stays
                    let _ = context.move_soldier(id, grid_no);
                }
            });
            edit.document().undo.end();
        }
    }
}
