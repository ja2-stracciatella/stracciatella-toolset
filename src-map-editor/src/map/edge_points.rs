//! The tiles at the edges of the map where soldiers enter the sector
//!
//! The editor generates them again when the map changed in a way that can move them, see
//! `render/edge_points.rs`.

use super::GridNo;

/// The entry tiles of one side of the map
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EdgePointList {
    /// Index of the first tile of the inner row, the outer row comes first
    /// (`gus1stNorthEdgepointMiddleIndex` and so on)
    ///
    /// The game moves it around when it removes tiles, so it can be past the end of the list.
    pub middle_index: u16,
    pub grid_nos: Vec<GridNo>,
}

/// The entry tiles of all sides of the map
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EdgePointSet {
    pub north: EdgePointList,
    pub east: EdgePointList,
    pub south: EdgePointList,
    pub west: EdgePointList,
}

impl EdgePointSet {
    /// The lists in the order north, east, south, west
    pub fn sides(&self) -> [&EdgePointList; 4] {
        [&self.north, &self.east, &self.south, &self.west]
    }

    pub fn sides_mut(&mut self) -> [&mut EdgePointList; 4] {
        [
            &mut self.north,
            &mut self.east,
            &mut self.south,
            &mut self.west,
        ]
    }
}

/// The edge points of the map
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EdgePoints {
    pub primary: EdgePointSet,
    /// The entry tiles of the part of the map that is isolated from the primary edge points
    pub secondary: Option<EdgePointSet>,
}
