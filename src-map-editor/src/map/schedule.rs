//! Schedules of civilians (`SCHEDULENODE`, `Scheduling.h`)

use bitflags::bitflags;

use super::GridNo;

/// A schedule, referred to by detailed soldier placements (`ubScheduleID`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScheduleId(u8);

impl ScheduleId {
    /// The schedule with the given id, `None` for 0, which is no schedule
    pub fn new(id: u8) -> Option<Self> {
        (id != 0).then_some(Self(id))
    }

    pub fn number(self) -> u8 {
        self.0
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ScheduleFlags: u16 {
        /// The time of an action varies
        const VARIANCE1 = 0x0001;
        const VARIANCE2 = 0x0002;
        const VARIANCE3 = 0x0004;
        const VARIANCE4 = 0x0008;
        const ACTIVE1 = 0x0010;
        const ACTIVE2 = 0x0020;
        const ACTIVE3 = 0x0040;
        const ACTIVE4 = 0x0080;
        const TEMPORARY = 0x0100;
        const SLEEP_CONVERTED = 0x0200;
        const NPC_SLEEPING = 0x0400;
    }
}

/// What a soldier does at a time of the schedule (`SCHEDULE_ACTION_*`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleAction {
    /// Locks the door next to `door`, then goes to `destination`
    LockDoor {
        door: GridNo,
        destination: GridNo,
    },
    UnlockDoor {
        door: GridNo,
        destination: GridNo,
    },
    OpenDoor {
        door: GridNo,
        destination: GridNo,
    },
    CloseDoor {
        door: GridNo,
        destination: GridNo,
    },
    /// Goes to a tile
    GridNo(GridNo),
    LeaveSector,
    /// Enters the sector at a tile
    EnterSector(GridNo),
    StayInSector,
    /// Sleeps at a tile
    Sleep(GridNo),
    /// Wakes up and goes back to the starting tile
    Wake,
}

/// An action at a time of day
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleEntry {
    /// Minutes since midnight, `None` if the action has no time
    pub time: Option<u16>,
    pub action: ScheduleAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub id: ScheduleId,
    /// Soldier the schedule was saved with, the game assigns the soldiers again
    pub soldier_id: u8,
    pub flags: ScheduleFlags,
    /// The actions by slot, the flags refer to the slots. `None` if the slot is not used.
    pub actions: [Option<ScheduleEntry>; 4],
}
