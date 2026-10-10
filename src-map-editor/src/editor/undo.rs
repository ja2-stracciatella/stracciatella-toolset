//! Undoing edits of the map
//!
//! Like the undo list of the original editor (`Editor_Undo.cc`), a command keeps the state of
//! every tile before the command changed it. A command is everything that happened between
//! pressing and releasing the mouse button, i.e. one stroke. The tiles hold `Arc`s of the tiles of
//! the tileset, so keeping copies of them is cheap.

use std::collections::{BTreeMap, VecDeque};

use crate::map::{
    AmbientLight, Door, EditorSmoothing, ExitGrid, GridNo, MapInfo, MapItem, MapLight, MapTile,
    Schedule, SoldierPlacement,
};

/// The lights, the light colour and the ambient light of a map
pub type LightState = (Vec<MapLight>, [u8; 3], Option<AmbientLight>);

/// How many commands can be undone (`MAX_UNDO_COMMAND_LENGTH` in the original editor)
pub const MAX_UNDO_COMMANDS: usize = 10;

/// What a command changed, to restore it
#[derive(Clone, Debug, Default)]
pub struct UndoCommand {
    /// The tiles before the command changed them
    pub tiles: BTreeMap<GridNo, MapTile>,
    /// The background texture before the command changed it
    pub background: Option<String>,
    /// The smoothing of the map before the command changed it
    pub smoothing: Option<EditorSmoothing>,
    /// The scroll restriction of the map before the command changed it
    pub restricted_scroll: Option<bool>,
    /// The door table before the command changed it
    pub doors: Option<Vec<Door>>,
    /// The soldier placements and the schedules before the command changed them
    pub soldiers: Option<(Vec<SoldierPlacement>, Vec<Schedule>)>,
    /// The world items before the command changed them
    pub items: Option<Vec<MapItem>>,
    /// The lights, the light colour and the ambient light before the command changed them
    pub lights: Option<LightState>,
    /// The exit grids before the command changed them
    pub exit_grids: Option<Vec<ExitGrid>>,
    /// The map info before the command changed its entry points, only they are restored
    pub entry_points: Option<MapInfo>,
}

impl UndoCommand {
    fn is_empty(&self) -> bool {
        self.tiles.is_empty()
            && self.background.is_none()
            && self.smoothing.is_none()
            && self.restricted_scroll.is_none()
            && self.doors.is_none()
            && self.soldiers.is_none()
            && self.items.is_none()
            && self.lights.is_none()
            && self.exit_grids.is_none()
            && self.entry_points.is_none()
    }
}

/// The commands that can be undone and redone, and the one that is being recorded
#[derive(Clone, Debug, Default)]
pub struct UndoStack {
    /// The oldest command first
    commands: VecDeque<UndoCommand>,
    /// The commands that restore what was undone, the last undone one last
    redo: VecDeque<UndoCommand>,
    current: Option<UndoCommand>,
    /// How often anything was recorded, to tell whether an edit changed something
    records: u64,
}

impl UndoStack {
    /// Starts recording a new command, ending the current one
    pub fn begin(&mut self) {
        self.end();
        self.current = Some(UndoCommand::default());
    }

    /// Ends recording the current command, commands that changed nothing are dropped
    pub fn end(&mut self) {
        let Some(command) = self.current.take() else {
            return;
        };
        if command.is_empty() {
            return;
        }
        // A new edit can't be followed by what was undone before
        self.redo.clear();
        self.push_undo(command);
    }

    /// Adds a command that can be undone, e.g. the one that undoes a redo, keeping what can be
    /// redone
    pub fn push_undo(&mut self, command: UndoCommand) {
        push_limited(&mut self.commands, command);
    }

    /// Adds a command that redoes an undone command
    pub fn push_redo(&mut self, command: UndoCommand) {
        push_limited(&mut self.redo, command);
    }

    /// Ends the current command and takes the last command that redoes an undone one, there is
    /// none if the current command changed something
    pub fn pop_redo(&mut self) -> Option<UndoCommand> {
        self.end();
        self.redo.pop_back()
    }

    /// Records a tile before it is changed, only the first state of a tile in a command is kept
    ///
    /// Starts a command if none is being recorded.
    pub fn record_tile(&mut self, grid_no: GridNo, tile: &MapTile) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .tiles
            .entry(grid_no)
            .or_insert_with(|| tile.clone());
    }

    /// Records the background texture before it is changed
    pub fn record_background(&mut self, background: &str) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .background
            .get_or_insert_with(|| background.to_owned());
    }

    /// Records the smoothing of the map before it is changed
    pub fn record_smoothing(&mut self, smoothing: EditorSmoothing) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .smoothing
            .get_or_insert(smoothing);
    }

    /// Records the scroll restriction of the map before it is changed
    pub fn record_restricted_scroll(&mut self, restricted_scroll: bool) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .restricted_scroll
            .get_or_insert(restricted_scroll);
    }

    /// Records the door table before it is changed
    pub fn record_doors(&mut self, doors: &[Door]) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .doors
            .get_or_insert_with(|| doors.to_vec());
    }

    /// Records the soldier placements and the schedules before they are changed
    pub fn record_soldiers(&mut self, soldiers: &[SoldierPlacement], schedules: &[Schedule]) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .soldiers
            .get_or_insert_with(|| (soldiers.to_vec(), schedules.to_vec()));
    }

    /// Records the world items before they are changed
    pub fn record_items(&mut self, items: &[MapItem]) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .items
            .get_or_insert_with(|| items.to_vec());
    }

    /// Records the lights, the light colour and the ambient light before they are changed
    pub fn record_lights(
        &mut self,
        lights: &[MapLight],
        color: [u8; 3],
        ambient: Option<AmbientLight>,
    ) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .lights
            .get_or_insert_with(|| (lights.to_vec(), color, ambient));
    }

    /// Records the exit grids before they are changed
    pub fn record_exit_grids(&mut self, exit_grids: &[ExitGrid]) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .exit_grids
            .get_or_insert_with(|| exit_grids.to_vec());
    }

    /// Records the map info before its entry points are changed
    pub fn record_entry_points(&mut self, info: &MapInfo) {
        self.records += 1;
        self.current
            .get_or_insert_with(UndoCommand::default)
            .entry_points
            .get_or_insert_with(|| info.clone());
    }

    /// Ends the current command and takes the last one
    pub fn pop(&mut self) -> Option<UndoCommand> {
        self.end();
        self.commands.pop_back()
    }

    /// How often anything was recorded so far
    pub fn records(&self) -> u64 {
        self.records
    }

    /// Number of commands that can be undone
    pub fn depth(&self) -> usize {
        self.commands.len() + usize::from(self.is_recording())
    }

    /// Number of commands that can be redone, none while the current command changes something
    pub fn redo_depth(&self) -> usize {
        if self.is_recording() {
            0
        } else {
            self.redo.len()
        }
    }

    /// Whether the current command changed something
    fn is_recording(&self) -> bool {
        self.current.as_ref().is_some_and(|c| !c.is_empty())
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.redo.clear();
        self.current = None;
    }
}

/// Adds a command, dropping the oldest ones beyond `MAX_UNDO_COMMANDS`
fn push_limited(commands: &mut VecDeque<UndoCommand>, command: UndoCommand) {
    commands.push_back(command);
    while commands.len() > MAX_UNDO_COMMANDS {
        commands.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(height: u8) -> MapTile {
        MapTile {
            height,
            ..Default::default()
        }
    }

    fn grid(index: usize) -> GridNo {
        GridNo::new(index).unwrap()
    }

    #[test]
    fn a_command_keeps_the_first_state_of_each_tile() {
        let mut stack = UndoStack::default();
        stack.begin();
        stack.record_tile(grid(1), &tile(1));
        stack.record_tile(grid(1), &tile(2));
        stack.record_tile(grid(2), &tile(3));
        stack.end();
        // Empty commands are dropped
        stack.begin();
        stack.end();
        assert_eq!(stack.depth(), 1);

        let command = stack.pop().unwrap();
        assert_eq!(command.tiles.len(), 2);
        assert_eq!(command.tiles[&grid(1)].height, 1);
        assert_eq!(command.tiles[&grid(2)].height, 3);
        assert!(stack.pop().is_none());
    }

    #[test]
    fn only_the_last_commands_are_kept() {
        let mut stack = UndoStack::default();
        for height in 0..15 {
            stack.begin();
            stack.record_tile(grid(0), &tile(height));
        }
        // The current command counts once it changed something
        assert_eq!(stack.depth(), MAX_UNDO_COMMANDS + 1);
        stack.end();
        assert_eq!(stack.depth(), MAX_UNDO_COMMANDS);
        let heights: Vec<u8> = std::iter::from_fn(|| stack.pop())
            .map(|command| command.tiles[&grid(0)].height)
            .collect();
        assert_eq!(heights, (5..15).rev().collect::<Vec<_>>());
    }

    #[test]
    fn undone_commands_are_redone_until_something_is_edited() {
        let mut stack = UndoStack::default();
        stack.begin();
        stack.record_tile(grid(0), &tile(1));
        stack.end();
        let command = stack.pop().unwrap();
        stack.push_redo(command);
        assert_eq!((stack.depth(), stack.redo_depth()), (0, 1));

        // Redoing keeps what else can be redone
        stack.push_redo(UndoCommand::default());
        let command = stack.pop_redo().unwrap();
        stack.push_undo(command);
        assert_eq!((stack.depth(), stack.redo_depth()), (1, 1));

        // An edit in progress hides the redo commands, ending it drops them
        stack.begin();
        stack.record_tile(grid(1), &tile(2));
        assert_eq!(stack.redo_depth(), 0);
        assert!(stack.pop_redo().is_none());
        assert_eq!((stack.depth(), stack.redo_depth()), (2, 0));

        // Commands that changed nothing keep them
        stack.push_redo(UndoCommand::default());
        stack.begin();
        stack.end();
        assert_eq!(stack.redo_depth(), 1);
        stack.clear();
        assert_eq!(stack.redo_depth(), 0);
    }

    #[test]
    fn only_the_last_redo_commands_are_kept() {
        let mut stack = UndoStack::default();
        for height in 0..15 {
            stack.push_redo(UndoCommand {
                tiles: BTreeMap::from([(grid(0), tile(height))]),
                ..Default::default()
            });
        }
        assert_eq!(stack.redo_depth(), MAX_UNDO_COMMANDS);
        assert_eq!(stack.pop_redo().unwrap().tiles[&grid(0)].height, 14);
    }

    #[test]
    fn the_first_background_is_kept() {
        let mut stack = UndoStack::default();
        stack.record_background("TEXTURE3");
        stack.record_background("TEXTURE4");
        assert_eq!(stack.pop().unwrap().background.as_deref(), Some("TEXTURE3"));
    }
}
