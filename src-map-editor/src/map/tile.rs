//! A tile of the map with its tile layers

use std::{num::NonZeroU8, sync::Arc};

use crate::tileset::Tile;

/// A room of the map (`ubRoomNum`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoomNo(NonZeroU8);

impl RoomNo {
    /// The room with the given number, `None` for 0, which is no room
    pub fn new(number: u8) -> Option<Self> {
        NonZeroU8::new(number).map(Self)
    }

    pub fn number(self) -> u8 {
        self.0.get()
    }
}

/// A tile of the map (`MAP_ELEMENT` as saved in the map file)
///
/// The layers are in the order of the map file, which is the order the game adds them in: land
/// with `AddLandToHead`, the others with `AddObjectToTail`, `AddStructToTail` and so on. So the
/// head of the game's land list (the topmost land tile) is the last element of `land`, while the
/// heads of the other lists are their first elements. The editor keeps this order:
/// `AddLandToHead` is `land.push`, `AddLandToTail` is `land.insert(0, ..)`, `AddStructToHead` is
/// `structs.insert(0, ..)` and `AddObjectToHead` is `objects.insert(0, ..)`.
#[derive(Clone, Debug, Default)]
pub struct MapTile {
    /// Terrain height in pixels
    pub height: u8,
    /// From the tail to the head, the last element is the topmost land tile
    pub land: Vec<Arc<Tile>>,
    /// From the head to the tail
    pub objects: Vec<Arc<Tile>>,
    /// From the head to the tail
    pub structs: Vec<Arc<Tile>>,
    pub shadows: Vec<Arc<Tile>>,
    pub roofs: Vec<Arc<Tile>>,
    pub on_roofs: Vec<Arc<Tile>>,
    pub room: Option<RoomNo>,
    /// An enemy mine is on the tile (`MAPELEMENT_ENEMY_MINE_PRESENT`)
    pub enemy_mine: bool,
    /// A mine of the player is on the tile (`MAPELEMENT_PLAYER_MINE_PRESENT`)
    pub player_mine: bool,
    /// The wall of the tile is a piece of a cave that the editor placed (`LEVELNODE_CAVE`), the
    /// map file doesn't have it
    pub cave: bool,
}
