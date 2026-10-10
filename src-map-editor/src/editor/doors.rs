//! Locks and traps of doors (`InitDoorEditing` and `ExtractAndUpdateDoorInfo` of
//! `EditorBuildings.cc`)
//!
//! Clicking a door with the door locks tool asks the host to edit its lock with a `door`
//! event. The host sets the lock with `set_door`, which can be undone. While the tool is
//! active, the doors with a lock or trap are marked (`AddLockedDoorCursors`).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    components::map::{game_to_world, JA2_TILE_HEIGHT, JA2_TILE_WIDTH},
    host::report_editor_event,
    map::{Door, DoorTrap, DoorTrapKind, GridNo, LockId, TacticalMap},
    render::world::MapWorld,
    states::loading::MapToLoad,
    tileset::StructureFlags,
};

use super::{
    building_mode::BuildingTool, commands::CurrentMode, document::EditContext,
    document::EditorDocument, mode::EditorMode,
};

/// Number of locks of the lock table (`NUM_LOCKS`)
pub const NUM_LOCKS: u8 = 64;
/// Highest difficulty of a trap
pub const MAX_TRAP_LEVEL: u8 = 20;

/// A trap of a door as the host edits it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorTrapValue {
    pub kind: DoorTrapKind,
    /// 1 to 20
    pub level: u8,
}

/// The lock and trap of a door as the host edits them
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorLock {
    /// The lock of the lock table, 0 for none
    pub lock_id: u8,
    pub trap: Option<DoorTrapValue>,
    pub locked: bool,
}

impl DoorLock {
    pub fn of(door: &Door) -> Self {
        Self {
            lock_id: door.lock.map_or(0, LockId::number),
            trap: door.trap.map(|trap| DoorTrapValue {
                kind: trap.kind,
                level: trap.level,
            }),
            locked: door.locked,
        }
    }

    /// The door of the door table, with the values clamped like the original: the lock to the
    /// lock table and the trap level to 1 to 20. What the player believes about the door is
    /// reset.
    pub fn door(self, grid_no: GridNo) -> Door {
        Door {
            grid_no,
            locked: self.locked,
            trap: self.trap.map(|trap| DoorTrap {
                kind: trap.kind,
                level: trap.level.clamp(1, MAX_TRAP_LEVEL),
            }),
            lock: LockId::new(self.lock_id.min(NUM_LOCKS - 1)),
            perceived_locked: 0,
            perceived_trapped: 0,
            lock_damage: 0,
        }
    }
}

/// The door of a tile of the door table
pub fn door_at(map: &TacticalMap, grid_no: GridNo) -> Option<&Door> {
    map.doors.iter().find(|door| door.grid_no == grid_no)
}

/// Whether a tile has a door or another structure that can be opened (`DoorAtGridNo` and
/// `OpenableAtGridNo`)
pub fn openable_at(world: &MapWorld, grid_no: GridNo) -> bool {
    world.tiles[grid_no]
        .find_structure(StructureFlags::ANYDOOR | StructureFlags::OPENABLE)
        .is_some()
}

/// Asks the host to edit the door of a tile, if the tile has one
pub fn request_door_editing(context: &mut EditContext, grid_no: GridNo) {
    context.commit();
    if openable_at(context.world, grid_no) {
        context.document.door_request = Some(grid_no);
    }
}

impl EditContext<'_> {
    /// Adds, replaces or removes the door of a tile in the door table, it is recorded for undo
    pub fn set_door(&mut self, grid_no: GridNo, lock: Option<DoorLock>) {
        let door = lock.map(|lock| lock.door(grid_no));
        if door_at(self.map, grid_no) == door.as_ref() {
            return;
        }
        self.document.undo.record_doors(&self.map.doors);
        // Enemies can't open locked doors
        self.document.edge_points_dirty = true;
        let doors = &mut self.map.doors;
        match (doors.iter().position(|d| d.grid_no == grid_no), door) {
            (Some(index), Some(door)) => doors[index] = door,
            (Some(index), None) => {
                doors.remove(index);
            }
            (None, Some(door)) => doors.push(door),
            (None, None) => {}
        }
    }
}

/// Asks the host to edit a door: `door` is the lock of its door, `null` if it has none
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "door", rename_all = "camelCase")]
pub struct DoorEvent {
    pub map: String,
    pub grid_no: u16,
    pub door: Option<DoorLock>,
}

/// Sends the door editing request of the last click to the host
pub fn report_door_request(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    mut document: ResMut<EditorDocument>,
) {
    let Some(grid_no) = document.door_request.take() else {
        return;
    };
    report_editor_event(&DoorEvent {
        map: map_to_load.0.clone(),
        grid_no: grid_no.index() as u16,
        door: door_at(&map, grid_no).map(DoorLock::of),
    });
}

/// Marks the doors of the door table while the door locks tool is active
pub fn draw_door_markers(mut gizmos: Gizmos, mode: Res<CurrentMode>, map: Res<TacticalMap>) {
    let EditorMode::Buildings(buildings) = &mode.0 else {
        return;
    };
    if buildings.tool != BuildingTool::DoorLocks {
        return;
    }
    let color = Srgba::rgb(1., 0.85, 0.);
    for door in &map.doors {
        // Raised by the terrain height, like the door is drawn
        let height = i32::from(map.tiles[door.grid_no].height);
        let center = game_to_world(
            door.grid_no.position().tile_box() + IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT) / 2
                - IVec2::new(0, height),
        );
        // A key: a ring and a bit
        let ring = center + Vec2::new(-5., 0.);
        gizmos.circle_2d(Isometry2d::from_translation(ring), 3., color);
        gizmos.line_2d(ring + Vec2::new(3., 0.), center + Vec2::new(7., 0.), color);
        gizmos.line_2d(
            center + Vec2::new(5., 0.),
            center + Vec2::new(5., -3.),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        building::tests::{add_room, building_fixture, rect},
        building_mode::paste_single,
        document::tests::grid,
    };

    fn lock() -> DoorLock {
        DoorLock {
            lock_id: 3,
            trap: Some(DoorTrapValue {
                kind: DoorTrapKind::SilentAlarm,
                level: 0,
            }),
            locked: true,
        }
    }

    #[test]
    fn door_locks_are_json_for_the_host() {
        let json = r#"{"lockId":70,"trap":{"kind":"brothelSiren","level":30},"locked":false}"#;
        let parsed: DoorLock = serde_json::from_str(json).unwrap();
        // Clamped like the original
        let door = parsed.door(grid(5, 5));
        assert_eq!(door.lock.map(LockId::number), Some(63));
        assert_eq!(door.trap.unwrap().level, 20);
        assert_eq!(parsed.trap.unwrap().kind, DoorTrapKind::BrothelSiren);
        // A trap needs a level
        assert_eq!(lock().door(grid(5, 5)).trap.unwrap().level, 1);
        // Lock 0 is no lock
        let open = DoorLock {
            lock_id: 0,
            trap: None,
            locked: false,
        };
        assert_eq!(open.door(grid(5, 5)).lock, None);
        assert_eq!(DoorLock::of(&open.door(grid(5, 5))), open);

        let event = DoorEvent {
            map: "maps/a9.dat".to_owned(),
            grid_no: 805,
            door: Some(lock()),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "door");
        assert_eq!(json["gridNo"], 805);
        assert_eq!(json["door"]["trap"]["kind"], "silentAlarm");
        assert_eq!(json["door"]["lockId"], 3);
    }

    #[test]
    fn doors_are_set_and_undone() {
        let mut fixture = building_fixture();
        let mut context = fixture.context();
        context.set_door(grid(5, 5), Some(lock()));
        context.set_door(grid(6, 5), Some(lock()));
        context.finish();
        assert_eq!(fixture.map.doors.len(), 2);
        fixture.document.undo.end();

        let mut context = fixture.context();
        context.set_door(grid(5, 5), None);
        let unlocked = DoorLock {
            locked: false,
            ..lock()
        };
        context.set_door(grid(6, 5), Some(unlocked));
        context.finish();
        assert_eq!(fixture.map.doors.len(), 1);
        assert!(!fixture.map.doors[0].locked);
        assert_eq!(fixture.document.undo.depth(), 2);

        // Undo restores the whole table of the command
        let command = fixture.document.undo.pop().unwrap();
        assert!(command.tiles.is_empty());
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert_eq!(fixture.map.doors.len(), 2);
        assert!(fixture.map.doors.iter().all(|door| door.locked));

        // Setting the same door changes nothing
        let mut context = fixture.context();
        context.set_door(grid(5, 5), Some(lock()));
        context.finish();
        assert_eq!(fixture.document.undo.depth(), 1);
    }

    #[test]
    fn only_doors_can_be_edited() {
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let mut context = fixture.context();
        request_door_editing(&mut context, grid(52, 51));
        assert_eq!(context.document.door_request, None);
        let door = context.tile("DOOR1", 1).unwrap();
        paste_single(&mut context, grid(52, 51), door);
        request_door_editing(&mut context, grid(52, 51));
        assert_eq!(context.document.door_request, Some(grid(52, 51)));
    }
}
