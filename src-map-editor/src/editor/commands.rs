//! The requests of the host: the mode, undo and the map background, and the state of the editor
//! that the host is told about

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    host::report_editor_event,
    map::{EditorSmoothing, GridNo, TacticalMap},
    states::loading::MapToLoad,
    take_editor_commands, take_requested_editor_mode,
};

use super::{
    building::roof_all_rooms,
    caves::mark_caves,
    document::EditorDocument,
    doors::DoorLock,
    ground::set_background_texture,
    input::EditorStroke,
    item_events::{run_item_command, update_item, ItemCommand, ItemCommandParams, ItemEdit},
    light_events::{run_light_command, update_light, LightCommand, LightCommandParams, LightEdit},
    mode::EditorMode,
    soldier_events::{
        run_soldier_command, update_soldier, SoldierCommand, SoldierCommandParams, SoldierEdit,
    },
    soldiers::edit_once,
    traversal::TraversalSelection,
    traversal_events::{update_traversal, TraversalEdit},
    undo::UndoCommand,
    MapEdit,
};

/// A request of the host that must not get lost, unlike the mode, which is replaced
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    /// Undoes the last stroke or background change
    Undo,
    /// Redoes the last undone command
    Redo,
    /// Replaces the ground of the whole map by a texture, by the name of its tile type
    /// (`SetBackgroundTexture`)
    SetBackground(String),
    /// Sets how the editor smooths the map (`SetEditorSmoothingMode`)
    SetSmoothing(EditorSmoothing),
    /// Sets whether the game only scrolls over the middle of the map (`ubRestrictedScrollID`)
    SetRestrictedScroll(bool),
    /// Gives the rooms a flat roof of the roofs selection of the buildings mode
    RoofAllRooms,
    /// Adds, replaces or removes the door of a tile in the door table
    SetDoor(DoorRequest),
    /// Edits the selected soldier
    UpdateSoldier(Box<SoldierEdit>),
    Soldier(SoldierCommand),
    /// Edits the selected world item
    UpdateItem(Box<ItemEdit>),
    Item(ItemCommand),
    /// Edits the selected light
    UpdateLight(LightEdit),
    Light(LightCommand),
    /// Edits the selected entry point or exit grid
    UpdateTraversal(TraversalEdit),
}

/// The door of a tile as the host sets it, `door` is `None` to remove it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorRequest {
    pub grid_no: u16,
    pub door: Option<DoorLock>,
}

/// The mode the mouse edits with
#[derive(Resource, Clone, Debug, Default)]
pub struct CurrentMode(pub EditorMode);

pub fn apply_requested_mode(mut mode: ResMut<CurrentMode>) {
    if let Some(requested) = take_requested_editor_mode() {
        if mode.0 != requested {
            mode.0 = requested;
        }
    }
}

/// Starts editing a newly loaded map
pub fn start_editing(
    time: Res<Time<Real>>,
    mut map: ResMut<TacticalMap>,
    mut document: ResMut<EditorDocument>,
    mut stroke: ResMut<EditorStroke>,
    mut reported: ResMut<ReportedEditorState>,
) {
    if map.info.editor_smoothing == EditorSmoothing::Caves {
        mark_caves(&mut map);
    }
    *document = EditorDocument::for_map(time.elapsed().as_nanos() as u64, &map);
    *stroke = EditorStroke::default();
    reported.0 = None;
    // The commands were meant for the map before
    take_editor_commands();
}

pub fn run_editor_commands(
    mut edit: MapEdit,
    mut stroke: ResMut<EditorStroke>,
    mode: Res<CurrentMode>,
    mut soldiers: SoldierCommandParams,
    mut items: ItemCommandParams,
    mut lights: LightCommandParams,
    mut traversal: ResMut<TraversalSelection>,
) {
    for command in take_editor_commands() {
        // A stroke in progress is finished first
        *stroke = EditorStroke::default();
        let document = edit.document();
        document.undo.end();
        match command {
            EditorCommand::Undo => {
                if let Some(command) = document.undo.pop() {
                    let mut inverse = UndoCommand::default();
                    edit.edit(|context| inverse = context.restore(command));
                    // Restoring is not recorded, but it changes the map
                    let document = edit.document();
                    document.revision += 1;
                    document.undo.push_redo(inverse);
                }
            }
            EditorCommand::Redo => {
                if let Some(command) = document.undo.pop_redo() {
                    let mut inverse = UndoCommand::default();
                    edit.edit(|context| inverse = context.restore(command));
                    let document = edit.document();
                    document.revision += 1;
                    document.undo.push_undo(inverse);
                }
            }
            EditorCommand::SetBackground(background) => {
                document.undo.begin();
                edit.edit(|context| set_background_texture(context, &background));
                edit.document().undo.end();
            }
            EditorCommand::SetSmoothing(smoothing) => {
                document.undo.begin();
                edit.edit(|context| context.set_smoothing(smoothing));
                edit.document().undo.end();
            }
            EditorCommand::SetRestrictedScroll(restricted_scroll) => {
                document.undo.begin();
                edit.edit(|context| context.set_restricted_scroll(restricted_scroll));
                edit.document().undo.end();
            }
            EditorCommand::SetDoor(request) => {
                let Some(grid_no) = GridNo::new(usize::from(request.grid_no)) else {
                    continue;
                };
                document.undo.begin();
                edit.edit(|context| context.set_door(grid_no, request.door));
                edit.document().undo.end();
            }
            EditorCommand::RoofAllRooms => {
                document.undo.begin();
                edit.edit(|context| {
                    let style = match &mode.0 {
                        EditorMode::Buildings(buildings) => buildings.roof_style(context),
                        _ => None,
                    };
                    roof_all_rooms(context, style.unwrap_or(0));
                });
                edit.document().undo.end();
            }
            EditorCommand::UpdateSoldier(soldier) => {
                if let Some(sc) = soldiers.sources.context() {
                    edit_once(&mut edit, |context| update_soldier(context, &sc, &soldier));
                }
            }
            EditorCommand::Soldier(command) => {
                run_soldier_command(command, &mut edit, &mut soldiers);
            }
            EditorCommand::UpdateItem(item) => {
                if let Some(table) = &items.items {
                    edit_once(&mut edit, |context| update_item(context, table, &item));
                }
            }
            EditorCommand::Item(command) => {
                let view = soldiers.view.as_deref_mut();
                run_item_command(command, &mut edit, &mut items, view);
            }
            EditorCommand::UpdateLight(light) => {
                if let Some(templates) = &lights.templates {
                    let mut result = Ok(());
                    edit_once(&mut edit, |context| {
                        result = update_light(context, templates, &light)
                    });
                    if let Err(notice) = result {
                        lights.selection.notice = Some(notice);
                    }
                }
            }
            EditorCommand::Light(command) => {
                run_light_command(command, &mut edit, &mut lights);
            }
            EditorCommand::UpdateTraversal(traversal) => {
                edit_once(&mut edit, |context| update_traversal(context, &traversal));
            }
        }
        soldiers.selection.validate(&edit.map);
        lights.selection.validate(&edit.map);
        traversal.validate(&edit.map);
    }
}

/// The state of the editor, reported to the host whenever it changes
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "state", rename_all = "camelCase")]
pub struct EditorStateEvent {
    pub map: String,
    pub can_undo: bool,
    pub undo_depth: usize,
    pub can_redo: bool,
    pub redo_depth: usize,
    /// Name of the tile type of the background texture
    pub background: String,
    /// One more than the highest room number of the map, the number of a new room
    pub next_room_number: u8,
    /// How the editor smooths the map, which decides the building tools
    pub smoothing: EditorSmoothing,
    /// Whether the game only scrolls over the middle of the map
    pub restricted_scroll: bool,
    /// Counts the changes of the map since it was loaded
    pub revision: u64,
}

/// One more than the highest room number of the map, at most 255 (`gubMaxRoomNumber`)
pub fn next_room_number(map: &TacticalMap) -> u8 {
    let max = map
        .tiles
        .values()
        .filter_map(|tile| tile.room)
        .map(|room| room.number())
        .max()
        .unwrap_or(0);
    max.saturating_add(1)
}

/// The state that was reported last, `None` for a new map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReportedEditorState(Option<EditorStateEvent>);

pub fn report_editor_state(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    document: Res<EditorDocument>,
    mut reported: ResMut<ReportedEditorState>,
    mut room_number: Local<u8>,
) {
    if map.is_changed() || reported.0.is_none() {
        *room_number = next_room_number(&map);
    }
    let depth = document.undo.depth();
    let redo_depth = document.undo.redo_depth();
    let state = EditorStateEvent {
        map: map_to_load.0.clone(),
        can_undo: depth > 0,
        undo_depth: depth,
        can_redo: redo_depth > 0,
        redo_depth,
        background: document.background.clone(),
        next_room_number: *room_number,
        smoothing: map.info.editor_smoothing,
        restricted_scroll: map.info.restricted_scroll,
        revision: document.revision,
    };
    if reported.0.as_ref() != Some(&state) {
        report_editor_event(&state);
        reported.0 = Some(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        map::RoomNo,
    };

    #[test]
    fn new_rooms_get_the_next_number() {
        let mut fixture = Fixture::new();
        assert_eq!(next_room_number(&fixture.map), 1);
        fixture.map.tiles[grid(3, 3)].room = RoomNo::new(4);
        fixture.map.tiles[grid(5, 3)].room = RoomNo::new(2);
        assert_eq!(next_room_number(&fixture.map), 5);
        fixture.map.tiles[grid(6, 3)].room = RoomNo::new(255);
        assert_eq!(next_room_number(&fixture.map), 255);

        let state = EditorStateEvent {
            map: "maps/a9.dat".to_owned(),
            can_undo: false,
            undo_depth: 0,
            can_redo: false,
            redo_depth: 0,
            background: "TEXTURE1".to_owned(),
            next_room_number: 5,
            smoothing: EditorSmoothing::Basement,
            restricted_scroll: true,
            revision: 3,
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["type"], "state");
        assert_eq!(json["nextRoomNumber"], 5);
        assert_eq!(json["smoothing"], "basement");
        assert_eq!(json["restrictedScroll"], true);
        assert_eq!(json["revision"], 3);
    }

    #[test]
    fn door_requests_are_read_from_json() {
        let request: DoorRequest = serde_json::from_str(r#"{"gridNo":805,"door":null}"#).unwrap();
        assert_eq!(request.grid_no, 805);
        assert_eq!(request.door, None);
        let request: DoorRequest =
            serde_json::from_str(r#"{"gridNo":1,"door":{"lockId":2,"trap":null,"locked":true}}"#)
                .unwrap();
        assert!(request.door.unwrap().locked);
    }
}
