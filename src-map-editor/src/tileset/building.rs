//! What the tiles of buildings are to the editor
//!
//! The original editor knows the pieces of a building by the sub indices of their tile types
//! (the tables of `NewSmooth.cc`, `SmartMethod.cc` and `SelectWin.cc`). Here every tile of a
//! building type carries its piece, which the conversion of the tileset sets.

/// The building piece a tile is, and its style: the number of the tile type within its family,
/// e.g. 0 to 3 for `WALL1` to `WALL4`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BuildingTile {
    pub style: u8,
    pub piece: BuildingPiece,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingPiece {
    /// `WALL1` to `WALL4`
    Wall(WallPiece),
    /// The first or last frame of a door of `DOOR1` to `DOOR4`, its orientation is the one of
    /// the tile
    Door { open: bool },
    /// The frames of a door between closed and open
    DoorFrame,
    /// `DOORSH1` to `DOORSH4`
    DoorShadow,
    /// `FLOOR1` to `FLOOR4`
    Floor,
    /// `ROOF1` to `ROOF4`, `None` for the images that are not a piece of the roof smoothing
    Roof(Option<RoofPiece>),
    /// `SROOF1` and `SROOF2`, `None` for the images that are not a piece of a slant roof
    SlantRoof(Option<SlantPiece>),
    /// `ONROOF1` and `ONROOF2`
    OnRoof,
    /// `DECORATIONS1` to `DECORATIONS4`, the old broken walls
    Decoration,
    /// `WALLDECAL1` to `WALLDECAL8`
    WallDecal,
    /// `FIRSTSWITCHES`
    Switch,
    /// `ISTRUCT1` to `ISTRUCT4` and `FIRSTCISTRCUT`
    Furniture,
    /// `FIFTHISTRUCT` to `EIGHTISTRUCT`
    GenericFurniture,
}

impl BuildingPiece {
    pub fn is_wall(self) -> bool {
        matches!(self, BuildingPiece::Wall(_))
    }

    /// Doors in any state
    pub fn is_door(self) -> bool {
        matches!(self, BuildingPiece::Door { .. } | BuildingPiece::DoorFrame)
    }

    /// Flat roofs (`FIRSTROOF` to `LASTROOF`)
    pub fn is_flat_roof(self) -> bool {
        matches!(self, BuildingPiece::Roof(_))
    }

    /// Flat and slant roofs (`FIRSTROOF` to `LASTSLANTROOF`)
    pub fn is_roof(self) -> bool {
        matches!(self, BuildingPiece::Roof(_) | BuildingPiece::SlantRoof(_))
    }

    /// What the decoration erasers remove: decals, decorations, furniture and switches
    pub fn is_decoration(self) -> bool {
        matches!(
            self,
            BuildingPiece::WallDecal
                | BuildingPiece::Decoration
                | BuildingPiece::Furniture
                | BuildingPiece::GenericFurniture
                | BuildingPiece::Switch
        )
    }
}

/// The pieces of the walls (`gbWallTileLUT` and the offsets of `SelectWin.cc`)
///
/// The names are those of the wall classes of the original, e.g. `InteriorLeft` is the
/// horizontal wall seen from inside the building.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WallPiece {
    ExteriorRight,
    ExteriorLeft,
    InteriorRight,
    InteriorLeft,
    ExteriorBottomEnd,
    InteriorCorner,
    InteriorBottomEnd,
    InteriorExtended,
    ExteriorExtendedBottomEnd,
    InteriorExtendedBottomEnd,
    ExteriorExtended,
    /// The shadow of the exterior horizontal walls, sub index 30
    ExteriorLeftShadow,
    /// The shadow of a bottom corner, sub index 31
    InteriorBottomEndShadow,
    /// The walls of a slant roof, which go to the roof layer
    AFrame {
        vertical: bool,
        left: bool,
    },
    /// A wall with a window, 3 styles per orientation
    Window {
        style: u8,
    },
    /// 2 variants per orientation
    Broken {
        variant: u8,
    },
    Cracked,
    Smudged,
}

impl WallPiece {
    /// The wall shadows, which go to the shadow layer
    pub fn is_shadow(self) -> bool {
        matches!(
            self,
            WallPiece::ExteriorLeftShadow | WallPiece::InteriorBottomEndShadow
        )
    }
}

/// The pieces of the flat roofs, by the walls on their sides
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoofPiece {
    Left,
    Top,
    TopLeft,
    Bottom,
    Right,
    BottomRight,
    TopRight,
    BottomLeft,
    /// 3 variants
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlantSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// The pieces of the slant roofs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlantPiece {
    pub thick: bool,
    pub side: SlantSide,
}

/// The palettes of the buildings mode (`InitJA2SelectionWindow`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuildingPalette {
    /// The first wall, floor, roof and slant roof of each type, for new rooms
    Room,
    Walls,
    /// The closed and open frames of the doors
    Doors,
    Windows,
    /// Flat roofs, slant roofs, A-frames and on roofs
    Roofs,
    /// Decorations, broken, cracked and smudged walls
    BrokenWalls,
    /// Wall decals and switches
    Decals,
    /// The 8 floor variants of each floor type
    Floors,
    /// A wall of each wall type
    SmartWalls,
    /// The closed and open door of each door type
    SmartDoors,
    /// The 3 window styles
    SmartWindows,
    /// The 2 broken walls, the cracked and the smudged wall
    SmartBrokenWalls,
    /// A center piece of each flat roof style
    SmartRoofs,
}

impl BuildingPalette {
    pub const ALL: [BuildingPalette; 13] = [
        BuildingPalette::Room,
        BuildingPalette::Walls,
        BuildingPalette::Doors,
        BuildingPalette::Windows,
        BuildingPalette::Roofs,
        BuildingPalette::BrokenWalls,
        BuildingPalette::Decals,
        BuildingPalette::Floors,
        BuildingPalette::SmartWalls,
        BuildingPalette::SmartDoors,
        BuildingPalette::SmartWindows,
        BuildingPalette::SmartBrokenWalls,
        BuildingPalette::SmartRoofs,
    ];
}
