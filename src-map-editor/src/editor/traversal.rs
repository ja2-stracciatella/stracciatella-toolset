//! The traversal mode: exit grids and entry points (the exit grids and entry points of the Map
//! Info tab of the original editor, `DRAW_MODE_EXITGRID` and `SpecifyEntryPoint`)
//!
//! The `place` tool paints exit grids with the brush like the original, with the destination of
//! the mode, they replace the exit grids of the tiles they are painted on. Erasing removes them.
//! A right click copies the values of the clicked exit grid into the mode of the host (`queried`
//! of the `traversal` event). Placing an entry point sets it with a left click, the mode picks
//! which one. Entry points are moved into the area the game can show
//! (`ValidateEntryPointGridNo`).
//!
//! The `select` tool selects the entry point or the exit grid of the clicked tile, clicking it
//! again selects the next one of the tile. Dragging the selection moves it, dragging an exit grid
//! while Ctrl (Cmd on macOS) was held on the press copies it, it replaces the exit grid of the
//! target tile. Delete removes the selected exit grid or clears the selected entry point, the
//! game then falls back to another entry point. Esc deselects. Every change can be undone. The
//! host edits the selection with `set_traversal`, see `traversal_events.rs`.
//!
//! The edge points depend on the entry points and the movement costs, they are generated again
//! after the edits that can move them, see `traversal_events.rs`.

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    components::map::JA2_MAP_TILES,
    map::{EntryPoint, ExitGrid, GridNo, Level, SectorId, TacticalMap},
    render::{movement_costs::validate_entry_point_grid_no, RenderOptions},
    systems::cursor::CursorInfo,
};

use super::{
    brush::{Brush, BrushArea},
    commands::CurrentMode,
    document::EditContext,
    mode::EditorMode,
    soldiers::edit_once,
    MapEdit,
};

/// The levels below the surface that a sector can have
pub const MAX_SECTOR_LEVEL: u8 = 3;

/// What a click in the traversal mode does
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraversalTool {
    /// Selects, moves, copies and deletes exit grids and entry points
    #[default]
    Select,
    /// Paints exit grids or sets an entry point
    Place,
}

/// What the place tool places
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraversalPlace {
    /// Paints exit grids with the brush, or erases them
    #[default]
    Exit,
    /// Sets the entry point of the mode
    Entry,
}

/// The traversal mode
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TraversalMode {
    pub tool: TraversalTool,
    pub place: TraversalPlace,
    /// Removes exit grids instead
    pub erase: bool,
    /// The brush of the exit grids, square sizes and areas
    pub brush: Brush,
    /// Where new exit grids lead (`gExitGrid`)
    pub dest_sector: SectorId,
    pub dest_grid_no: u16,
    /// The entry point a click sets
    pub entry: EntryPoint,
}

impl Default for TraversalMode {
    fn default() -> Self {
        Self {
            tool: TraversalTool::Select,
            place: TraversalPlace::Exit,
            erase: false,
            brush: Brush::default(),
            dest_sector: SectorId { x: 1, y: 1, z: 0 },
            dest_grid_no: 0,
            entry: EntryPoint::North,
        }
    }
}

impl TraversalMode {
    /// The brush of the exit grids, `None` for selecting and entry points, which use clicks
    pub fn brush(&self) -> Option<Brush> {
        match (self.tool, self.place, self.brush) {
            (
                TraversalTool::Place,
                TraversalPlace::Exit,
                brush @ (Brush::Square { .. } | Brush::Area),
            ) => Some(brush),
            (TraversalTool::Place, TraversalPlace::Exit, _) => Some(Brush::default()),
            _ => None,
        }
    }

    /// The exit grid that is painted on a tile
    pub fn exit_grid(&self, grid_no: GridNo) -> ExitGrid {
        clamped_exit_grid(grid_no, self.dest_sector, self.dest_grid_no)
    }

    /// Paints or erases exit grids on the tiles of the brush
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea) {
        if self.brush().is_none() {
            return;
        }
        if self.erase {
            context.erase_exit_grids(area);
        } else {
            context.paint_exit_grids(area, self);
        }
    }
}

/// An exit grid with the destination clamped like the original clamps the fields
/// (`ExtractAndUpdateMapInfo`)
pub fn clamped_exit_grid(grid_no: GridNo, dest_sector: SectorId, dest_grid_no: u16) -> ExitGrid {
    let last = JA2_MAP_TILES - 1;
    ExitGrid {
        grid_no,
        dest_grid_no: GridNo::new(usize::from(dest_grid_no).min(last)).unwrap(),
        dest_sector: SectorId {
            x: dest_sector.x.clamp(1, 16),
            y: dest_sector.y.clamp(1, 16),
            z: dest_sector.z.min(MAX_SECTOR_LEVEL),
        },
    }
}

/// The exit grid of a tile
pub fn exit_grid_at(map: &TacticalMap, grid_no: GridNo) -> Option<&ExitGrid> {
    map.exit_grids.iter().find(|grid| grid.grid_no == grid_no)
}

/// What the select tool selects: an entry point or the exit grid of a tile
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalItem {
    Entry(EntryPoint),
    Exit(GridNo),
}

impl TraversalItem {
    /// The tile of the item, `None` if it is gone
    pub fn grid_no(self, map: &TacticalMap) -> Option<GridNo> {
        match self {
            TraversalItem::Entry(point) => map.info.entry(point),
            TraversalItem::Exit(grid_no) => exit_grid_at(map, grid_no).map(|grid| grid.grid_no),
        }
    }
}

/// The entry points of a tile in their order, then its exit grid
pub fn items_on(map: &TacticalMap, tile: GridNo) -> Vec<TraversalItem> {
    let entries = EntryPoint::ALL
        .into_iter()
        .filter(|&point| map.info.entry(point) == Some(tile))
        .map(TraversalItem::Entry);
    let exit = exit_grid_at(map, tile).map(|grid| TraversalItem::Exit(grid.grid_no));
    entries.chain(exit).collect()
}

impl EditContext<'_> {
    /// Puts the exit grid of the mode on the tiles of the brush, replacing their exit grids
    /// (`AddExitGridToWorld`), only recorded if this changes anything
    pub fn paint_exit_grids(&mut self, area: &BrushArea, mode: &TraversalMode) {
        let wanted: Vec<ExitGrid> = area.tiles().map(|g| mode.exit_grid(g)).collect();
        if wanted
            .iter()
            .all(|grid| exit_grid_at(self.map, grid.grid_no) == Some(grid))
        {
            return;
        }
        let exit_grids = self.modify_exit_grids();
        exit_grids.retain(|grid| !area.tiles.contains(&grid.grid_no));
        exit_grids.extend(wanted);
    }

    /// Removes the exit grids of the tiles of the brush (`RemoveExitGridFromWorld`)
    pub fn erase_exit_grids(&mut self, area: &BrushArea) {
        let on_area = |grid: &ExitGrid| area.tiles.contains(&grid.grid_no);
        if !self.map.exit_grids.iter().any(on_area) {
            return;
        }
        self.modify_exit_grids().retain(|grid| !on_area(grid));
    }

    /// Moves the exit grid of a tile to another tile, or copies it there, replacing the exit
    /// grid of that tile, returns the tile, `None` if there is no exit grid or it is already there
    pub fn move_exit_grid(&mut self, from: GridNo, to: GridNo, copy: bool) -> Option<GridNo> {
        if from == to {
            return None;
        }
        let mut grid = *exit_grid_at(self.map, from)?;
        grid.grid_no = to;
        let exit_grids = self.modify_exit_grids();
        exit_grids.retain(|g| g.grid_no != to && (copy || g.grid_no != from));
        exit_grids.push(grid);
        Some(to)
    }

    /// Removes the exit grid of a tile (`RemoveExitGridFromWorld`)
    pub fn delete_exit_grid(&mut self, grid_no: GridNo) {
        if exit_grid_at(self.map, grid_no).is_some() {
            self.modify_exit_grids()
                .retain(|grid| grid.grid_no != grid_no);
        }
    }

    /// Changes the destination of the exit grid of a tile, clamped like painted ones, only
    /// recorded if this changes it
    pub fn change_exit_grid(&mut self, grid_no: GridNo, dest_sector: SectorId, dest_grid_no: u16) {
        let Some(index) = self
            .map
            .exit_grids
            .iter()
            .position(|g| g.grid_no == grid_no)
        else {
            return;
        };
        let changed = clamped_exit_grid(grid_no, dest_sector, dest_grid_no);
        if self.map.exit_grids[index] != changed {
            self.modify_exit_grids()[index] = changed;
        }
    }

    /// Sets or clears an entry point, a tile is moved to where the game can show it
    /// (`SpecifyEntryPoint`), only recorded if this changes it
    pub fn set_entry_point(&mut self, point: EntryPoint, grid_no: Option<GridNo>) {
        let restricted_scroll = self.map.info.restricted_scroll;
        let grid_no =
            grid_no.map(|grid_no| validate_entry_point_grid_no(grid_no, restricted_scroll));
        if self.map.info.entry(point) != grid_no {
            *self.modify_map_info().entry_mut(point) = grid_no;
        }
    }
}

/// A drag of the selection in progress
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TraversalDrag {
    /// Where the mouse was pressed
    from: GridNo,
    /// Whether Ctrl or Cmd was held on the press, which copies exit grids instead of moving them
    copy: bool,
    /// Whether the item was selected before the press, releasing on the same tile then selects
    /// the next item of the tile
    was_selected: bool,
}

/// The selected entry point or exit grid, and the exit grid that was right clicked last
#[derive(Resource, Clone, Debug, Default)]
pub struct TraversalSelection {
    pub selected: Option<TraversalItem>,
    /// The exit grid that was right clicked last, its values go to the mode of the host
    pub queried: Option<ExitGrid>,
    drag: Option<TraversalDrag>,
}

impl TraversalSelection {
    /// Drops the selection when the item is gone, e.g. after undo
    pub fn validate(&mut self, map: &TacticalMap) {
        if self
            .selected
            .is_some_and(|item| item.grid_no(map).is_none())
        {
            self.selected = None;
            self.drag = None;
        }
    }

    /// Drops the selection and the drag
    pub fn clear(&mut self) {
        self.selected = None;
        self.drag = None;
    }

    /// Whether the selection is being dragged
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Selects the first item of a tile, or keeps the selected one if it is there, and starts
    /// dragging it, or clears the selection if the tile has none
    pub fn press(&mut self, map: &TacticalMap, tile: GridNo, copy: bool) {
        let on_tile = items_on(map, tile);
        let was_selected = self.selected.is_some_and(|item| on_tile.contains(&item));
        if !was_selected {
            let Some(&first) = on_tile.first() else {
                self.clear();
                return;
            };
            self.selected = Some(first);
        }
        self.drag = Some(TraversalDrag {
            from: tile,
            copy,
            was_selected,
        });
    }

    /// Ends the drag at a tile, moving the selection there or copying the exit grid there and
    /// selecting the copy. Releasing where the selection already was selects the next item of
    /// the tile.
    pub fn release(&mut self, context: &mut EditContext, to: GridNo) {
        let (Some(drag), Some(selected)) = (self.drag.take(), self.selected) else {
            return;
        };
        if to != drag.from {
            match selected {
                TraversalItem::Entry(point) => context.set_entry_point(point, Some(to)),
                TraversalItem::Exit(from) => {
                    if let Some(moved) = context.move_exit_grid(from, to, drag.copy) {
                        self.selected = Some(TraversalItem::Exit(moved));
                    }
                }
            }
            return;
        }
        if drag.was_selected {
            let on_tile = items_on(context.map, to);
            if let Some(position) = on_tile.iter().position(|&item| item == selected) {
                self.selected = Some(on_tile[(position + 1) % on_tile.len()]);
            }
        }
    }

    /// Removes the selected exit grid or clears the selected entry point
    pub fn delete(&mut self, context: &mut EditContext) {
        match self.selected.take() {
            Some(TraversalItem::Entry(point)) => context.set_entry_point(point, None),
            Some(TraversalItem::Exit(grid_no)) => context.delete_exit_grid(grid_no),
            None => {}
        }
        self.drag = None;
    }

    /// A right click: the exit grid of the tile goes to the host
    pub fn query(&mut self, map: &TacticalMap, grid_no: GridNo) {
        if let Some(grid) = exit_grid_at(map, grid_no) {
            self.queried = Some(*grid);
        }
    }
}

pub fn reset_traversal_selection(mut selection: ResMut<TraversalSelection>) {
    *selection = TraversalSelection::default();
}

/// The traversal mode, `None` for the others
pub fn traversal_mode(mode: &CurrentMode) -> Option<&TraversalMode> {
    match &mode.0 {
        EditorMode::Traversal(mode) => Some(mode),
        _ => None,
    }
}

/// The mouse and the keys of the traversal mode, painting exit grids is done by `input::paint`
pub fn edit_traversal(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    mut selection: ResMut<TraversalSelection>,
    mut edit: MapEdit,
) {
    // Exit grids and entry points are only edited on the ground floor
    let mode = traversal_mode(&mode).filter(|_| options.floor == Level::Ground);
    let Some(mode) = mode else {
        // Leaving the traversal mode or the ground deselects
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        return;
    };
    let selection = &mut *selection;
    let tile = cursor.tile.map(GridNo::from);
    if mode.tool != TraversalTool::Select {
        // Only the select tool has a selection
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        let Some(tile) = tile else {
            return;
        };
        match mode.place {
            TraversalPlace::Exit if mouse.just_pressed(MouseButton::Right) => {
                selection.query(&edit.map, tile);
            }
            TraversalPlace::Entry if mouse.just_pressed(MouseButton::Left) => {
                let point = mode.entry;
                edit_once(&mut edit, |context| {
                    context.set_entry_point(point, Some(tile))
                });
            }
            _ => {}
        }
        return;
    }
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
                Some(to) => edit_once(&mut edit, |context| selection.release(context, to)),
                None => selection.drag = None,
            }
        }
    }
    if selection.drag.is_none() && keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace]) {
        edit_once(&mut edit, |context| selection.delete(context));
    }
    if keys.just_pressed(KeyCode::Escape) {
        selection.clear();
    }
    selection.validate(&edit.map);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        editor::document::tests::{grid, Fixture},
    };

    pub(crate) fn mode(json: &str) -> TraversalMode {
        match serde_json::from_str(json).unwrap() {
            EditorMode::Traversal(mode) => mode,
            _ => panic!("not the traversal mode"),
        }
    }

    /// Edits the map as one undo command
    pub(crate) fn edit(fixture: &mut Fixture, change: impl FnOnce(&mut EditContext)) {
        fixture.document.undo.begin();
        let mut context = fixture.context();
        change(&mut context);
        context.finish();
        fixture.document.undo.end();
    }

    pub(crate) fn undo(fixture: &mut Fixture) {
        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
    }

    fn square(size: u8, x: u8, y: u8) -> BrushArea {
        Brush::Square { size }.region(None, GridPosition::new(x, y))
    }

    #[test]
    fn modes_are_read_from_json() {
        let read = mode(
            r#"{"mode":"traversal","tool":"place","place":"exit","erase":true,
                "brush":{"kind":"square","size":3},
                "destSector":{"x":11,"y":10,"z":1},"destGridNo":12345,"entry":"isolated"}"#,
        );
        assert_eq!(
            read,
            TraversalMode {
                tool: TraversalTool::Place,
                place: TraversalPlace::Exit,
                erase: true,
                brush: Brush::Square { size: 3 },
                dest_sector: SectorId { x: 11, y: 10, z: 1 },
                dest_grid_no: 12345,
                entry: EntryPoint::Isolated,
            }
        );
        assert_eq!(read.brush(), Some(Brush::Square { size: 3 }));
        let default = mode(r#"{"mode":"traversal"}"#);
        assert_eq!(default, TraversalMode::default());
        // Selecting and entry points don't use the brush, exit grids no circles or lines
        assert_eq!(default.brush(), None);
        let entry = mode(r#"{"mode":"traversal","tool":"place","place":"entry","entry":"center"}"#);
        assert_eq!((entry.brush(), entry.entry), (None, EntryPoint::Center));
        let circle =
            mode(r#"{"mode":"traversal","tool":"place","brush":{"kind":"circle","size":5}}"#);
        assert_eq!(circle.brush(), Some(Brush::default()));
        let area = mode(r#"{"mode":"traversal","tool":"place","brush":{"kind":"area"}}"#);
        assert_eq!(area.brush(), Some(Brush::Area));
    }

    #[test]
    fn destinations_are_clamped() {
        let mode = TraversalMode {
            dest_sector: SectorId { x: 0, y: 20, z: 9 },
            dest_grid_no: 30000,
            ..Default::default()
        };
        let exit = mode.exit_grid(grid(1, 1));
        assert_eq!(exit.dest_sector, SectorId { x: 1, y: 16, z: 3 });
        assert_eq!(exit.dest_grid_no.index(), 25599);
    }

    #[test]
    fn exit_grids_are_painted_replaced_and_erased() {
        let mut fixture = Fixture::new();
        let mut paint = mode(
            r#"{"mode":"traversal","tool":"place","destSector":{"x":9,"y":1,"z":1},"destGridNo":500}"#,
        );
        edit(&mut fixture, |context| {
            paint.apply(context, &square(3, 50, 50))
        });
        assert_eq!(fixture.map.exit_grids.len(), 9);
        assert!(fixture.document.traversal_dirty && fixture.document.edge_points_dirty);
        let exit = exit_grid_at(&fixture.map, grid(49, 51)).unwrap();
        assert_eq!(exit.dest_sector, SectorId { x: 9, y: 1, z: 1 });
        assert_eq!(exit.dest_grid_no.index(), 500);

        // Painting the same again changes nothing
        edit(&mut fixture, |context| {
            paint.apply(context, &square(3, 50, 50))
        });
        assert_eq!(fixture.document.undo.depth(), 1);

        // Another destination replaces the grids of the tiles it covers
        paint.dest_grid_no = 600;
        edit(&mut fixture, |context| {
            paint.apply(context, &square(1, 50, 50))
        });
        assert_eq!(fixture.map.exit_grids.len(), 9);
        assert_eq!(
            exit_grid_at(&fixture.map, grid(50, 50))
                .unwrap()
                .dest_grid_no
                .index(),
            600
        );

        paint.erase = true;
        edit(&mut fixture, |context| {
            paint.apply(context, &square(1, 49, 49))
        });
        assert_eq!(fixture.map.exit_grids.len(), 8);
        assert!(exit_grid_at(&fixture.map, grid(49, 49)).is_none());
        // Erasing where there are none changes nothing
        edit(&mut fixture, |context| {
            paint.apply(context, &square(1, 10, 10))
        });
        assert_eq!(fixture.document.undo.depth(), 3);

        undo(&mut fixture);
        assert_eq!(fixture.map.exit_grids.len(), 9);
        undo(&mut fixture);
        assert_eq!(
            exit_grid_at(&fixture.map, grid(50, 50))
                .unwrap()
                .dest_grid_no
                .index(),
            500
        );
        undo(&mut fixture);
        assert!(fixture.map.exit_grids.is_empty());

        // Selecting and placing entry points don't paint
        for json in [
            r#"{"mode":"traversal"}"#,
            r#"{"mode":"traversal","tool":"place","place":"entry"}"#,
        ] {
            edit(&mut fixture, |context| {
                mode(json).apply(context, &square(1, 50, 50))
            });
        }
        assert!(fixture.map.exit_grids.is_empty());
    }

    #[test]
    fn entry_points_are_moved_onto_the_screen() {
        let mut fixture = Fixture::new();
        // In the middle of the map nothing moves
        edit(&mut fixture, |context| {
            context.set_entry_point(EntryPoint::North, Some(grid(80, 80)))
        });
        assert_eq!(fixture.map.info.north_entry, Some(grid(80, 80)));
        assert!(fixture.document.edge_points_dirty);
        assert!(!fixture.document.traversal_dirty);
        // The top rows of the screen can't have entry points
        edit(&mut fixture, |context| {
            context.set_entry_point(EntryPoint::East, Some(grid(45, 40)))
        });
        assert_eq!(fixture.map.info.east_entry, Some(grid(48, 43)));
        // So can't the bottom ones
        edit(&mut fixture, |context| {
            context.set_entry_point(EntryPoint::South, Some(grid(155, 155)))
        });
        let south = fixture.map.info.south_entry.unwrap().position();
        assert!(u16::from(south.x) + u16::from(south.y) < 300, "{south:?}");
        // The same tile again changes nothing
        let depth = fixture.document.undo.depth();
        edit(&mut fixture, |context| {
            context.set_entry_point(EntryPoint::North, Some(grid(80, 80)))
        });
        assert_eq!(fixture.document.undo.depth(), depth);

        edit(&mut fixture, |context| {
            context.set_entry_point(EntryPoint::North, None)
        });
        assert_eq!(fixture.map.info.north_entry, None);
        undo(&mut fixture);
        assert_eq!(fixture.map.info.north_entry, Some(grid(80, 80)));
        // Undo only restores the entry points
        fixture.map.info.editor_smoothing = crate::map::EditorSmoothing::Caves;
        undo(&mut fixture);
        assert_eq!(fixture.map.info.south_entry, None);
        assert_eq!(
            fixture.map.info.editor_smoothing,
            crate::map::EditorSmoothing::Caves
        );
    }

    #[test]
    fn tile_edits_doors_and_the_map_type_move_the_edge_points() {
        let mut fixture = Fixture::new();
        edit(&mut fixture, |context| {
            context.set_room(grid(10, 10), crate::map::RoomNo::new(1))
        });
        assert!(fixture.document.edge_points_dirty);
        fixture.document.edge_points_dirty = false;
        edit(&mut fixture, |context| {
            context.set_smoothing(crate::map::EditorSmoothing::Basement)
        });
        assert!(fixture.document.edge_points_dirty);
        fixture.document.edge_points_dirty = false;
        edit(&mut fixture, |context| {
            context.set_door(
                grid(10, 10),
                Some(crate::editor::doors::DoorLock {
                    lock_id: 0,
                    trap: None,
                    locked: true,
                }),
            )
        });
        assert!(fixture.document.edge_points_dirty);
        fixture.document.edge_points_dirty = false;
        undo(&mut fixture);
        assert!(fixture.document.edge_points_dirty);
    }

    /// A map with exit grids to `dest_grid_no` on the tiles
    fn with_exit_grids(fixture: &mut Fixture, tiles: &[(u8, u8, u16)]) {
        for &(x, y, dest_grid_no) in tiles {
            let paint = TraversalMode {
                tool: TraversalTool::Place,
                dest_grid_no,
                ..Default::default()
            };
            edit(fixture, |context| paint.apply(context, &square(1, x, y)));
        }
    }

    fn press_and_release(
        fixture: &mut Fixture,
        selection: &mut TraversalSelection,
        from: GridNo,
        to: GridNo,
        copy: bool,
    ) {
        selection.press(&fixture.map, from, copy);
        edit(fixture, |context| selection.release(context, to));
    }

    #[test]
    fn pressing_selects_entry_points_and_exit_grids() {
        let mut fixture = Fixture::new();
        with_exit_grids(&mut fixture, &[(80, 80, 1)]);
        fixture.map.info.north_entry = Some(grid(80, 80));
        fixture.map.info.center = Some(grid(80, 80));
        assert_eq!(
            items_on(&fixture.map, grid(80, 80)),
            vec![
                TraversalItem::Entry(EntryPoint::North),
                TraversalItem::Entry(EntryPoint::Center),
                TraversalItem::Exit(grid(80, 80)),
            ]
        );
        let mut selection = TraversalSelection::default();
        let tile = grid(80, 80);
        press_and_release(&mut fixture, &mut selection, tile, tile, false);
        assert_eq!(
            selection.selected,
            Some(TraversalItem::Entry(EntryPoint::North))
        );
        // Clicking it again selects the next item of the tile, wrapping around
        for expected in [
            TraversalItem::Entry(EntryPoint::Center),
            TraversalItem::Exit(tile),
            TraversalItem::Entry(EntryPoint::North),
        ] {
            press_and_release(&mut fixture, &mut selection, tile, tile, false);
            assert_eq!(selection.selected, Some(expected));
        }
        // An empty tile deselects
        selection.press(&fixture.map, grid(10, 10), false);
        assert_eq!(selection.selected, None);
        assert!(!selection.dragging());
        assert_eq!(fixture.document.undo.depth(), 1);
    }

    #[test]
    fn dragging_moves_entry_points_and_moves_or_copies_exit_grids() {
        let mut fixture = Fixture::new();
        with_exit_grids(&mut fixture, &[(50, 50, 1), (60, 60, 2)]);
        fixture.map.info.west_entry = Some(grid(80, 80));
        let mut selection = TraversalSelection::default();

        // Moving replaces the exit grid of the target tile
        press_and_release(
            &mut fixture,
            &mut selection,
            grid(50, 50),
            grid(60, 60),
            false,
        );
        assert_eq!(selection.selected, Some(TraversalItem::Exit(grid(60, 60))));
        assert!(exit_grid_at(&fixture.map, grid(50, 50)).is_none());
        let moved = exit_grid_at(&fixture.map, grid(60, 60)).unwrap();
        assert_eq!(moved.dest_grid_no.index(), 1);
        assert_eq!(fixture.map.exit_grids.len(), 1);

        // Copying keeps the original
        press_and_release(
            &mut fixture,
            &mut selection,
            grid(60, 60),
            grid(70, 70),
            true,
        );
        assert_eq!(selection.selected, Some(TraversalItem::Exit(grid(70, 70))));
        assert_eq!(fixture.map.exit_grids.len(), 2);
        assert_eq!(
            exit_grid_at(&fixture.map, grid(70, 70))
                .unwrap()
                .dest_grid_no
                .index(),
            1
        );

        // Entry points are moved, also with Ctrl
        press_and_release(
            &mut fixture,
            &mut selection,
            grid(80, 80),
            grid(90, 90),
            true,
        );
        assert_eq!(
            selection.selected,
            Some(TraversalItem::Entry(EntryPoint::West))
        );
        assert_eq!(fixture.map.info.west_entry, Some(grid(90, 90)));
        assert!(fixture.document.edge_points_dirty);

        // Undo restores everything
        undo(&mut fixture);
        assert_eq!(fixture.map.info.west_entry, Some(grid(80, 80)));
        undo(&mut fixture);
        assert!(exit_grid_at(&fixture.map, grid(70, 70)).is_none());
        undo(&mut fixture);
        assert_eq!(fixture.map.exit_grids.len(), 2);
        assert_eq!(
            exit_grid_at(&fixture.map, grid(60, 60))
                .unwrap()
                .dest_grid_no
                .index(),
            2
        );
        // The selection is dropped when the undone exit grid is gone
        selection.validate(&fixture.map);
        assert_eq!(
            selection.selected,
            Some(TraversalItem::Entry(EntryPoint::West))
        );
        selection.selected = Some(TraversalItem::Exit(grid(70, 70)));
        selection.validate(&fixture.map);
        assert_eq!(selection.selected, None);
    }

    #[test]
    fn delete_removes_exit_grids_and_clears_entry_points() {
        let mut fixture = Fixture::new();
        with_exit_grids(&mut fixture, &[(50, 50, 1)]);
        fixture.map.info.isolated = Some(grid(80, 80));
        let mut selection = TraversalSelection::default();

        selection.press(&fixture.map, grid(50, 50), false);
        edit(&mut fixture, |context| selection.delete(context));
        assert!(fixture.map.exit_grids.is_empty());
        assert_eq!(selection.selected, None);

        selection.press(&fixture.map, grid(80, 80), false);
        edit(&mut fixture, |context| selection.delete(context));
        assert_eq!(fixture.map.info.isolated, None);

        // Without a selection nothing is deleted
        let depth = fixture.document.undo.depth();
        edit(&mut fixture, |context| selection.delete(context));
        assert_eq!(fixture.document.undo.depth(), depth);

        undo(&mut fixture);
        assert_eq!(fixture.map.info.isolated, Some(grid(80, 80)));
        undo(&mut fixture);
        assert_eq!(fixture.map.exit_grids.len(), 1);
    }

    #[test]
    fn the_selected_exit_grid_can_be_edited() {
        let mut fixture = Fixture::new();
        with_exit_grids(&mut fixture, &[(50, 50, 1)]);
        let sector = SectorId { x: 20, y: 3, z: 7 };
        edit(&mut fixture, |context| {
            context.change_exit_grid(grid(50, 50), sector, 30000)
        });
        let changed = exit_grid_at(&fixture.map, grid(50, 50)).unwrap();
        assert_eq!(changed.dest_sector, SectorId { x: 16, y: 3, z: 3 });
        assert_eq!(changed.dest_grid_no.index(), 25599);
        // The same again and tiles without exit grids change nothing
        let depth = fixture.document.undo.depth();
        edit(&mut fixture, |context| {
            context.change_exit_grid(grid(50, 50), sector, 30000);
            context.change_exit_grid(grid(10, 10), sector, 5);
        });
        assert_eq!(fixture.document.undo.depth(), depth);
        undo(&mut fixture);
        assert_eq!(
            exit_grid_at(&fixture.map, grid(50, 50))
                .unwrap()
                .dest_grid_no
                .index(),
            1
        );

        let mut selection = TraversalSelection::default();
        selection.query(&fixture.map, grid(50, 50));
        assert_eq!(selection.queried.unwrap().grid_no, grid(50, 50));
        // Tiles without exit grids keep the last one
        selection.query(&fixture.map, grid(1, 1));
        assert_eq!(selection.queried.unwrap().grid_no, grid(50, 50));
    }
}
