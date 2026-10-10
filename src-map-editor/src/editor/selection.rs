//! Selecting whole buildings with the `select` tool of the buildings mode
//!
//! Pressing the left button on a building selects it, pressing it on the ground clears the
//! selection. Dragging the building moves it (`MoveBuilding` of the original editor), dragging it
//! while Ctrl (Cmd on macOS) was held on the press copies it (`CopyBuilding` and `PasteBuilding`), and Delete
//! removes it (`KillBuilding`). Each of them is undone as a whole. Buildings aren't moved or
//! copied in basements, like in the original.

use bevy::prelude::*;

use crate::{
    map::{GridNo, TacticalMap},
    render::RenderOptions,
    systems::cursor::CursorInfo,
};

use super::{
    building::kill_building,
    commands::CurrentMode,
    document::EditContext,
    input::{area_outline, floor_outline},
    layout::{building_at, building_layout, drag_offset, paste_building, shifted_layout},
    walls::is_basement,
    MapEdit,
};

/// A drag of the selected building in progress
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drag {
    /// Where the mouse was pressed
    from: GridNo,
    /// Whether Ctrl or Cmd was held on the press, which copies the building instead of moving it
    copy: bool,
}

/// The selected building
#[derive(Resource, Clone, Debug, Default)]
pub struct BuildingSelection {
    /// Any tile of the selected building
    selected: Option<GridNo>,
    drag: Option<Drag>,
}

impl BuildingSelection {
    /// Selects the building at a tile and starts dragging it, or clears the selection if there
    /// is none
    fn press(&mut self, map: &TacticalMap, tile: GridNo, ctrl: bool) {
        if building_at(&map.tiles[tile]) {
            self.selected = Some(tile);
            self.drag = Some(Drag {
                from: tile,
                copy: ctrl,
            });
        } else {
            *self = Self::default();
        }
    }

    /// Ends the drag at a tile, moving or copying the building there and selecting it
    fn release(&mut self, context: &mut EditContext, to: GridNo) {
        let Some(Drag { from, copy }) = self.drag.take() else {
            return;
        };
        if to == from || is_basement(context) {
            return;
        }
        paste_building(context, from, to, !copy);
        self.selected = Some(to);
    }

    /// Removes the selected building
    fn delete(&mut self, context: &mut EditContext) {
        if let Some(selected) = self.selected.take() {
            kill_building(context, selected);
        }
        self.drag = None;
    }

    /// Drops the selection when the building is gone, e.g. after undo or edits with other modes
    fn validate(&mut self, map: &TacticalMap) {
        if self
            .selected
            .is_some_and(|selected| !building_at(&map.tiles[selected]))
        {
            *self = Self::default();
        }
    }
}

pub fn reset_selection(mut selection: ResMut<BuildingSelection>) {
    *selection = BuildingSelection::default();
}

pub fn select_buildings(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    mut selection: ResMut<BuildingSelection>,
    mut edit: MapEdit,
) {
    if !mode.0.selects_buildings() {
        if selection.selected.is_some() || selection.drag.is_some() {
            *selection = BuildingSelection::default();
        }
        return;
    }
    let tile = cursor.tile.map(GridNo::from);
    if let Some(tile) = tile.filter(|_| mouse.just_pressed(MouseButton::Left)) {
        // Cmd on macOS, where Ctrl+click is a right click
        let copy = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
        selection.press(&edit.map, tile, copy);
    }
    if selection.drag.is_some() {
        if cursor.world_position.is_none() {
            // Leaving the viewer cancels the drag, like the area brushes
            selection.drag = None;
        } else if !mouse.pressed(MouseButton::Left) {
            match tile {
                Some(to) => {
                    edit.document().undo.begin();
                    edit.edit(|context| selection.release(context, to));
                    edit.document().undo.end();
                }
                None => selection.drag = None,
            }
        }
    }
    if selection.drag.is_none() && keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace]) {
        edit.document().undo.begin();
        edit.edit(|context| selection.delete(context));
        edit.document().undo.end();
    }
    selection.validate(&edit.map);
}

/// Outlines the selected building in yellow, and on the floor where it is dragged to in green
/// and the building under the cursor faintly
pub fn draw_selection(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    selection: Res<BuildingSelection>,
    map: Res<TacticalMap>,
) {
    if !mode.0.selects_buildings() {
        return;
    }
    let mut lines = |lines: Vec<(Vec2, Vec2)>, color: Color| {
        for (from, to) in lines {
            gizmos.line_2d(from, to, color);
        }
    };
    let cursor_outline = |tiles: &[GridNo]| floor_outline(area_outline(tiles, &map), options.floor);
    let selected = selection
        .selected
        .and_then(|selected| building_layout(&map, selected));
    if let Some(layout) = &selected {
        lines(area_outline(layout, &map), Srgba::rgb(1., 0.85, 0.).into());
    }
    let cursor = cursor.tile.map(GridNo::from);
    match (selection.drag, cursor) {
        (Some(drag), Some(cursor)) => {
            let offset = drag_offset(drag.from, cursor);
            if let (Some(layout), false) = (&selected, offset == (0, 0)) {
                lines(
                    cursor_outline(&shifted_layout(layout, offset)),
                    Srgba::GREEN.into(),
                );
            }
        }
        (None, Some(cursor)) => {
            let hovered = selected
                .as_ref()
                .is_some_and(|layout| layout.contains(&cursor));
            if !hovered {
                if let Some(layout) = building_layout(&map, cursor) {
                    lines(cursor_outline(&layout), Srgba::new(1., 1., 1., 0.35).into());
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{
            building::tests::{add_room, building_fixture, rect, walls},
            document::tests::{grid, Fixture},
        },
        map::EditorSmoothing,
    };

    fn fixture() -> Fixture {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        fixture
    }

    fn release(fixture: &mut Fixture, selection: &mut BuildingSelection, to: GridNo) {
        fixture.document.undo.begin();
        let mut context = fixture.context();
        selection.release(&mut context, to);
        context.finish();
        fixture.document.undo.end();
    }

    fn has_floor(fixture: &Fixture, x: u8, y: u8) -> bool {
        fixture.map.tiles[grid(x, y)]
            .land
            .iter()
            .any(|t| t.is_floor())
    }

    #[test]
    fn pressing_selects_buildings_or_clears_the_selection() {
        let fixture = fixture();
        let mut selection = BuildingSelection::default();
        selection.press(&fixture.map, grid(51, 51), false);
        assert_eq!(selection.selected, Some(grid(51, 51)));
        assert_eq!(
            selection.drag,
            Some(Drag {
                from: grid(51, 51),
                copy: false
            })
        );
        selection.press(&fixture.map, grid(20, 20), false);
        assert_eq!(selection.selected, None);
        assert_eq!(selection.drag, None);
    }

    #[test]
    fn dragging_moves_the_building() {
        let mut fixture = fixture();
        let mut selection = BuildingSelection::default();
        selection.press(&fixture.map, grid(51, 51), false);
        release(&mut fixture, &mut selection, grid(51, 61));
        assert!(walls(&fixture, 51, 49).is_empty());
        assert!(!has_floor(&fixture, 51, 51));
        assert_eq!(walls(&fixture, 51, 59).len(), 1);
        assert!(has_floor(&fixture, 51, 61));
        assert_eq!(selection.selected, Some(grid(51, 61)));
        assert_eq!(selection.drag, None);
        // Releasing where it was pressed changes nothing
        let depth = fixture.document.undo.depth();
        selection.press(&fixture.map, grid(51, 61), false);
        release(&mut fixture, &mut selection, grid(51, 61));
        assert!(has_floor(&fixture, 51, 61));
        assert_eq!(fixture.document.undo.depth(), depth);
    }

    #[test]
    fn ctrl_dragging_copies_the_building() {
        let mut fixture = fixture();
        let mut selection = BuildingSelection::default();
        selection.press(&fixture.map, grid(51, 51), true);
        release(&mut fixture, &mut selection, grid(61, 51));
        assert_eq!(walls(&fixture, 61, 49).len(), 1);
        assert_eq!(walls(&fixture, 51, 49).len(), 1);
        assert!(has_floor(&fixture, 51, 51));
        assert!(has_floor(&fixture, 61, 51));
        assert_eq!(selection.selected, Some(grid(61, 51)));
    }

    #[test]
    fn buildings_are_not_dragged_in_basements() {
        let mut fixture = fixture();
        fixture.map.info.editor_smoothing = EditorSmoothing::Basement;
        let mut selection = BuildingSelection::default();
        selection.press(&fixture.map, grid(51, 51), false);
        release(&mut fixture, &mut selection, grid(51, 61));
        assert!(has_floor(&fixture, 51, 51));
        assert!(!has_floor(&fixture, 51, 61));
        assert_eq!(selection.selected, Some(grid(51, 51)));
    }

    #[test]
    fn delete_removes_the_selected_building() {
        let mut fixture = fixture();
        let mut selection = BuildingSelection::default();
        // Nothing selected
        let mut context = fixture.context();
        selection.delete(&mut context);
        context.finish();
        assert!(has_floor(&fixture, 51, 51));

        selection.press(&fixture.map, grid(51, 51), false);
        let mut context = fixture.context();
        selection.delete(&mut context);
        context.finish();
        assert!((50..=52).all(|x| (50..=52).all(|y| !has_floor(&fixture, x, y))));
        assert_eq!(selection.selected, None);
        assert_eq!(selection.drag, None);
    }

    #[test]
    fn the_selection_is_dropped_when_the_building_is_gone() {
        let mut fixture = fixture();
        let mut selection = BuildingSelection::default();
        selection.press(&fixture.map, grid(51, 51), false);
        release(&mut fixture, &mut selection, grid(51, 61));
        selection.validate(&fixture.map);
        assert_eq!(selection.selected, Some(grid(51, 61)));
        // Undoing the move
        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert!(has_floor(&fixture, 51, 51));
        selection.validate(&fixture.map);
        assert_eq!(selection.selected, None);
    }
}
