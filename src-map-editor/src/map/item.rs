//! Items that are placed on the map (`WORLDITEM`) and item stacks (`OBJECTTYPE`)

use std::sync::Arc;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use super::{GridNo, Level};
use crate::{file_formats::ja2_map::Ja2Object, item::ItemId, tileset::Tile};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct WorldItemFlags: u16 {
        const DONTRENDER                     = 0x0001;
        const FIND_SWEETSPOT_FROM_GRIDNO     = 0x0002;
        const ARMED_BOMB                     = 0x0040;
        const SCIFI_ONLY                     = 0x0080;
        const REALISTIC_ONLY                 = 0x0100;
        const REACHABLE                      = 0x0200;
        const GRIDNO_NOT_SET_USE_ENTRY_POINT = 0x0400;
    }
}

/// A stack of items, as items on the map and in inventories are stored (`OBJECTTYPE`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: ItemId,
    /// Number of items in the stack
    pub number: u8,
    /// Item class specific data (gun, ammo, money, bomb, key, owner). The meaning depends on the
    /// item class, which is not part of the map file.
    pub data: [u8; 12],
    /// Attached items and their status
    pub attachments: Vec<(ItemId, i8)>,
    pub flags: i8,
    pub mission: u8,
    pub trap: i8,
    pub imprint_id: u8,
    pub weight: u8,
    pub used: u8,
}

/// How high an item is drawn (`bRenderZHeightAboveLevel`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemHeight {
    /// Always on the ground, never moved onto a structure (-1)
    ForceGround,
    /// Pixels above the level, unless the item is moved onto a structure
    Above(u8),
}

impl ItemHeight {
    /// The raw value of the map file
    pub fn number(self) -> i8 {
        match self {
            Self::ForceGround => -1,
            Self::Above(height) => height.min(i8::MAX as u8) as i8,
        }
    }
}

/// Whether an item can be seen (`Visibility`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    HiddenItem,
    Buried,
    HiddenInObject,
    Invisible,
    /// `VISIBILITY_0`
    Unseen,
    Visible,
}

impl Visibility {
    /// The visibility of a raw value of the game, `None` if it has no name
    pub fn from_number(number: i8) -> Option<Self> {
        Some(match number {
            -4 => Self::HiddenItem,
            -3 => Self::Buried,
            -2 => Self::HiddenInObject,
            -1 => Self::Invisible,
            0 => Self::Unseen,
            1 => Self::Visible,
            _ => return None,
        })
    }

    /// The raw value, the inverse of [`Visibility::from_number`]
    pub fn number(self) -> i8 {
        match self {
            Self::HiddenItem => -4,
            Self::Buried => -3,
            Self::HiddenInObject => -2,
            Self::Invisible => -1,
            Self::Unseen => 0,
            Self::Visible => 1,
        }
    }
}

/// Identifies a world item while the map is edited, it stays the same when the item is changed
/// or restored by undo
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorldItemId(pub u32);

/// An item placed on the map (`WORLDITEM`)
#[derive(Clone, Debug)]
pub struct MapItem {
    pub id: WorldItemId,
    /// Tile of the item, items of the map file without a tile are at the center of the map
    pub grid_no: GridNo,
    /// The raw grid number of items of the map file that are not on the map (`NOWHERE` and the
    /// like), which they are saved with unless they are moved
    pub file_grid_no: Option<i16>,
    pub level: Level,
    pub stack: ItemStack,
    /// The item stack of the map file if saving `stack` would change it, e.g. because its
    /// attachments are not in the first slots. It is saved as long as `stack` is unchanged.
    pub file_object: Option<Ja2Object>,
    /// The graphic of the item on the ground, `None` if it has none
    pub graphic: Option<Arc<Tile>>,
    pub render_height: ItemHeight,
    pub visibility: Visibility,
    pub flags: WorldItemFlags,
    /// Chance of the item not existing in the game, in percent
    pub non_exist_chance: u8,
}

impl PartialEq for MapItem {
    /// Graphics are equal if they are the same tile
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.grid_no == other.grid_no
            && self.file_grid_no == other.file_grid_no
            && self.level == other.level
            && self.stack == other.stack
            && self.file_object == other.file_object
            && self.graphic.as_ref().map(|tile| tile.id)
                == other.graphic.as_ref().map(|tile| tile.id)
            && self.render_height == other.render_height
            && self.visibility == other.visibility
            && self.flags == other.flags
            && self.non_exist_chance == other.non_exist_chance
    }
}
