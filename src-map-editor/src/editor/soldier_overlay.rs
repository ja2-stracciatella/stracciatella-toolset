//! What the soldiers mode draws over the map: the selected soldier and the tile under the cursor
//! (`CONFIRMMOVE1`, `FIRSTPOINTERS1` and `BADMARKER1` of the original editor), the patrol points
//! of the selected soldier (`DisplayWayPoints`) and the slot, the name and the patrol warnings of
//! all soldiers (`RenderMercStrings`)
//!
//! The texts are on the screen camera like the room numbers, so they stay sharp at every zoom.

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};

use crate::{
    map::{GridNo, Level, PlacementId, TacticalMap},
    render::{draw_list::VisibleTeams, world::MapWorld, RenderOptions},
    states::{AppState, UiFont},
    systems::{
        cursor::CursorInfo,
        pixel_view::{PixelView, SCREEN_LAYER},
    },
};

use super::{
    commands::CurrentMode,
    input::{area_outline, floor_outline, raised, ROOF_RAISE},
    schedules::ScheduleRow,
    soldier_events::tile_center,
    soldiers::{
        is_shown, is_sittable, placement, soldier_at, soldier_mode, soldier_slot, SoldierSelection,
        SoldierTool,
    },
};

/// Size of the texts in logical pixels
const FONT_SIZE: f32 = 11.;
/// Distance of the lines of a soldier in logical pixels
const LINE_HEIGHT: f32 = 11.;
/// How far above its tile the texts of a soldier start, in world pixels
const SOLDIER_HEIGHT: f32 = 45.;

/// The outline of a tile, raised onto the roof for the roof level
pub fn tile_outline(map: &TacticalMap, grid_no: GridNo, level: Level) -> Vec<(Vec2, Vec2)> {
    raised(area_outline(&[grid_no], map), level)
}

/// The outline of the tile under the cursor on a floor, see `floor_outline`
pub fn cursor_outline(map: &TacticalMap, grid_no: GridNo, floor: Level) -> Vec<(Vec2, Vec2)> {
    floor_outline(area_outline(&[grid_no], map), floor)
}

/// Outlines the selected soldier in yellow and the tile under the cursor on the floor: for
/// placing green where a click places a soldier and red where no soldier can stand, for
/// selecting green or red where the selected soldier is dragged to and faintly a soldier that a
/// click selects. The brush that erases is drawn by `draw_brush_cursor`.
pub fn draw_soldier_markers(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    selection: Res<SoldierSelection>,
    map: Res<TacticalMap>,
    world: Res<MapWorld>,
) {
    let Some(mode) = soldier_mode(&mode) else {
        return;
    };
    let mut lines = |lines: Vec<(Vec2, Vec2)>, color: Srgba| {
        for (from, to) in lines {
            gizmos.line_2d(from, to, color);
        }
    };
    let selected = selection.selected.and_then(|id| placement(&map, id));
    if let Some(selected) = selected {
        lines(
            tile_outline(&map, selected.grid_no, selected.level),
            Srgba::rgb(1., 0.85, 0.),
        );
    }
    let Some(tile) = cursor.tile.map(GridNo::from) else {
        return;
    };
    let floor = options.floor;
    let sittable = |level| {
        if is_sittable(&map, &world, tile, level, true) {
            Srgba::GREEN
        } else {
            Srgba::RED
        }
    };
    match mode.tool {
        SoldierTool::Place if mode.erase => {}
        SoldierTool::Place => lines(cursor_outline(&map, tile, floor), sittable(floor)),
        SoldierTool::Select if selection.dragging() => {
            if let Some(selected) = selected.filter(|selected| selected.grid_no != tile) {
                let color = sittable(selected.level);
                lines(cursor_outline(&map, tile, floor), color);
            }
        }
        SoldierTool::Select => {
            let hovered = soldier_at(&map, tile, &options.layers.soldier_teams, floor)
                .filter(|&hovered| Some(hovered) != selection.selected);
            if hovered.is_some() {
                lines(
                    cursor_outline(&map, tile, floor),
                    Srgba::new(1., 1., 1., 0.35),
                );
            }
        }
    }
}

/// A text over the map
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayLabel {
    pub text: String,
    pub color: Color,
    /// Where the text is, in world coordinates
    pub anchor: Vec2,
    /// The line below the anchor, for the texts of a soldier
    pub line: u8,
}

/// The patrol point labels in the colours of the original: red for soldiers on the ground, blue
/// for those on roofs
fn patrol_color(level: Level) -> Color {
    match level {
        Level::Ground => Color::srgb(1., 0.45, 0.45),
        Level::Roof => Color::srgb(0.45, 0.65, 1.),
    }
}

/// Where the texts of a tile are, raised onto the roof for the roof level
fn label_anchor(map: &TacticalMap, grid_no: GridNo, level: Level) -> Vec2 {
    let raise = if level == Level::Roof { ROOF_RAISE } else { 0. };
    tile_center(map, grid_no) + Vec2::new(0., raise)
}

/// The texts of the soldiers mode: the patrol points of the selected soldier and the slot, the
/// name and the patrol warnings of the shown soldiers
pub fn soldier_labels(
    map: &TacticalMap,
    selected: Option<PlacementId>,
    visible: &VisibleTeams,
    schedule: Option<&[ScheduleRow; 4]>,
) -> Vec<OverlayLabel> {
    let mut labels = vec![];
    for soldier in map.soldiers.iter().filter(|p| is_shown(p, visible)) {
        let anchor =
            label_anchor(map, soldier.grid_no, soldier.level) + Vec2::new(0., SOLDIER_HEIGHT);
        let mut line = 0;
        let mut add = |text: String, color: Color| {
            labels.push(OverlayLabel {
                text,
                color,
                anchor,
                line,
            });
            line += 1;
        };
        let name = soldier
            .detailed
            .as_ref()
            .filter(|detailed| detailed.profile.is_some() || !detailed.name.is_empty())
            .map(|detailed| detailed.name.clone())
            .filter(|name| !name.is_empty());
        if let Some(name) = name {
            add(name, Color::WHITE);
        }
        if let Some(slot) = soldier_slot(map, soldier.id) {
            add(format!("Slot #{}", slot), Color::srgb(0.75, 0.75, 0.75));
        }
        // Patrol orders need patrol points, and patrol points need patrol orders
        let patrol_orders = soldier.orders.uses_patrol_points();
        match (patrol_orders, soldier.patrol.is_empty()) {
            (true, true) => add(
                "Patrol orders with no waypoints".to_owned(),
                Color::srgb(1., 0.2, 0.2),
            ),
            (false, false) => add(
                "Waypoints with no patrol orders".to_owned(),
                Color::srgb(1., 0.2, 0.2),
            ),
            _ => {}
        }
    }
    if let Some(selected) = selected.and_then(|id| placement(map, id)) {
        for (index, &grid_no) in selected.patrol.iter().enumerate() {
            labels.push(OverlayLabel {
                text: (index + 1).to_string(),
                color: patrol_color(selected.level),
                // Brought down a touch, like the original
                anchor: label_anchor(map, grid_no, selected.level) - Vec2::new(0., 5.),
                line: 0,
            });
        }
    }
    // The tiles of the schedule (`RenderCurrentSchedule`)
    for (slot, row) in schedule.into_iter().flatten().enumerate() {
        for (point, data) in [('A', row.data1), ('B', row.data2)] {
            let Some(grid_no) = data.and_then(|d| GridNo::new(usize::from(d))) else {
                continue;
            };
            labels.push(OverlayLabel {
                text: format!("{}{}", slot + 1, point),
                color: Color::srgb(0.94, 0.85, 0.55),
                anchor: label_anchor(map, grid_no, Level::Ground) - Vec2::new(0., 5.),
                line: 0,
            });
        }
    }
    labels
}

/// The texts that are shown
#[derive(Resource, Default)]
pub struct SoldierLabels {
    shown: Vec<(OverlayLabel, Entity)>,
}

/// A text of [`SoldierLabels`]
#[derive(Component)]
pub struct SoldierLabel {
    anchor: Vec2,
    line: u8,
}

pub fn reset_soldier_labels(mut labels: ResMut<SoldierLabels>) {
    *labels = SoldierLabels::default();
}

/// Spawns the texts when they change and moves them with the view
pub fn update_soldier_labels(
    mut commands: Commands,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    map: Res<TacticalMap>,
    selection: Res<SoldierSelection>,
    view: Res<PixelView>,
    window: Single<&Window, With<PrimaryWindow>>,
    font: Res<UiFont>,
    mut labels: ResMut<SoldierLabels>,
    mut transforms: Query<(&SoldierLabel, &mut Transform)>,
) {
    let changed = mode.is_changed()
        || options.is_changed()
        || map.is_changed()
        || selection.is_changed()
        || labels.is_added();
    let shown = soldier_mode(&mode).is_some() && options.layers.soldiers;
    // 1 unit of the screen camera is 1 physical pixel, above the map
    let line_height = LINE_HEIGHT * window.scale_factor();
    let translation = |anchor: Vec2, line: u8| {
        (view.world_to_screen(anchor) - Vec2::new(0., f32::from(line) * line_height)).extend(11.)
    };
    if changed {
        let wanted = if shown {
            let schedule = selection.schedule.draft.as_ref().map(|draft| &draft.rows);
            let visible = &options.layers.soldier_teams;
            soldier_labels(&map, selection.selected, visible, schedule)
        } else {
            Vec::new()
        };
        let unchanged = labels.shown.len() == wanted.len()
            && labels
                .shown
                .iter()
                .zip(&wanted)
                .all(|((shown, _), w)| shown == w);
        if !unchanged {
            for (_, entity) in labels.shown.drain(..) {
                commands.entity(entity).despawn();
            }
            let text_font = font.text_font(FONT_SIZE * window.scale_factor());
            for label in wanted {
                let entity = commands
                    .spawn((
                        SoldierLabel {
                            anchor: label.anchor,
                            line: label.line,
                        },
                        Text2d::new(label.text.clone()),
                        text_font.clone(),
                        TextColor(label.color),
                        RenderLayers::layer(SCREEN_LAYER),
                        // New labels are not in `transforms` yet, so they start at their place
                        Transform::from_translation(translation(label.anchor, label.line)),
                        DespawnOnExit(AppState::Map),
                    ))
                    .id();
                labels.shown.push((label, entity));
            }
        }
    }
    for (label, mut transform) in &mut transforms {
        let translation = translation(label.anchor, label.line);
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        document::tests::grid,
        soldiers::tests::{mode, SoldierFixture},
    };
    use crate::map::Orders;

    #[test]
    fn soldiers_have_their_slot_and_patrol_warnings() {
        let mut fixture = SoldierFixture::new();
        let enemies = mode(r#"{"mode":"soldiers","defaults":{"orders":"pointPatrol"}}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &enemies, grid(30, 30), Level::Ground))
            .unwrap();
        let visible = VisibleTeams::default();
        let texts = |fixture: &SoldierFixture, selected| -> Vec<(String, u8)> {
            soldier_labels(fixture.map(), selected, &visible, None)
                .into_iter()
                .map(|label| (label.text, label.line))
                .collect()
        };
        assert_eq!(
            texts(&fixture, None),
            [
                ("Slot #20".to_owned(), 0),
                ("Patrol orders with no waypoints".to_owned(), 1)
            ]
        );
        fixture.edit(|context, _| {
            context.set_patrol_point(id, 2, grid(35, 35));
            context.change_soldier(id, |p| {
                p.orders = Orders::Stationary;
                let mut detailed = crate::editor::soldiers::static_detailed_placement(p);
                detailed.name = "Kingpin".to_owned();
                p.detailed = Some(detailed);
            });
        });
        let labels = texts(&fixture, Some(id));
        assert_eq!(
            labels,
            [
                ("Kingpin".to_owned(), 0),
                ("Slot #20".to_owned(), 1),
                ("Waypoints with no patrol orders".to_owned(), 2),
                ("1".to_owned(), 0),
                ("2".to_owned(), 0),
            ]
        );
        // Hidden teams have no texts
        let hidden = VisibleTeams {
            enemy: false,
            ..visible
        };
        assert!(soldier_labels(fixture.map(), None, &hidden, None).is_empty());
        // The tiles of the schedule
        let mut rows = [ScheduleRow::default(); 4];
        rows[1].data1 = Some(grid(40, 40).index() as u16);
        rows[1].data2 = Some(grid(41, 40).index() as u16);
        let labels = soldier_labels(fixture.map(), None, &hidden, Some(&rows));
        let texts: Vec<_> = labels.iter().map(|label| label.text.as_str()).collect();
        assert_eq!(texts, ["2A", "2B"]);
    }
}
