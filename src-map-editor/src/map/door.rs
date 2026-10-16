//! The door table of the map (`DOOR`)

use serde::{Deserialize, Serialize};

use super::GridNo;

/// A lock of the lock table (`ubLockID`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LockId(u8);

impl LockId {
    /// The lock with the given number, `None` for 0, which is no lock
    pub fn new(number: u8) -> Option<Self> {
        (number != 0).then_some(Self(number))
    }

    pub fn number(self) -> u8 {
        self.0
    }
}

/// Trap types of doors (`DoorTrapTypes`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DoorTrapKind {
    Explosion,
    Electric,
    Siren,
    SilentAlarm,
    BrothelSiren,
    SuperElectric,
}

impl DoorTrapKind {
    /// The trap of a raw trap id, `Ok(None)` for `NO_TRAP` and `Err` for unknown ids
    pub fn from_number(number: u8) -> Result<Option<Self>, u8> {
        Ok(Some(match number {
            0 => return Ok(None),
            1 => Self::Explosion,
            2 => Self::Electric,
            3 => Self::Siren,
            4 => Self::SilentAlarm,
            5 => Self::BrothelSiren,
            6 => Self::SuperElectric,
            _ => return Err(number),
        }))
    }

    /// The raw trap id, the inverse of [`DoorTrapKind::from_number`]
    pub fn number(self) -> u8 {
        match self {
            Self::Explosion => 1,
            Self::Electric => 2,
            Self::Siren => 3,
            Self::SilentAlarm => 4,
            Self::BrothelSiren => 5,
            Self::SuperElectric => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTrap {
    pub kind: DoorTrapKind,
    /// Difficulty of finding the trap, 0 to 10
    pub level: u8,
}

/// Lock and trap of a door
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Door {
    pub grid_no: GridNo,
    pub locked: bool,
    pub trap: Option<DoorTrap>,
    pub lock: Option<LockId>,
    /// What the player believes about the lock (`DOOR_PERCEIVED_*`)
    pub perceived_locked: i8,
    /// What the player believes about the trap (`DOOR_PERCEIVED_*`)
    pub perceived_trapped: i8,
    pub lock_damage: i8,
}
