//! Structure data of the tiles, the runtime form of the structure files

/// Size of the line of sight profile of a structure tile
pub const PROFILE_X_SIZE: usize = 5;
pub const PROFILE_Y_SIZE: usize = 5;
pub const PROFILE_Z_SIZE: usize = 4;

bitflags::bitflags! {
    /// Flags of a structure (`StructureFlags` in `Structure_Internals.h`)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StructureFlags: u32 {
        const BASE_TILE = 0x0000_0001;
        const OPEN = 0x0000_0002;
        const OPENABLE = 0x0000_0004;
        const GARAGEDOOR = 0x0000_0008;
        const MOBILE = 0x0000_0010;
        const PASSABLE = 0x0000_0020;
        const EXPLOSIVE = 0x0000_0040;
        const TRANSPARENT = 0x0000_0080;
        const GENERIC = 0x0000_0100;
        const TREE = 0x0000_0200;
        const FENCE = 0x0000_0400;
        const WIREFENCE = 0x0000_0800;
        const HASITEMONTOP = 0x0000_1000;
        const SPECIAL = 0x0000_2000;
        const LIGHTSOURCE = 0x0000_4000;
        const VEHICLE = 0x0000_8000;
        const WALL = 0x0001_0000;
        const WALLNWINDOW = 0x0002_0000;
        const SLIDINGDOOR = 0x0004_0000;
        const DOOR = 0x0008_0000;
        const MULTI = 0x0010_0000;
        const CAVEWALL = 0x0020_0000;
        const DDOOR_LEFT = 0x0040_0000;
        const DDOOR_RIGHT = 0x0080_0000;
        const NORMAL_ROOF = 0x0100_0000;
        const SLANTED_ROOF = 0x0200_0000;
        const TALL_ROOF = 0x0400_0000;
        const SWITCH = 0x0800_0000;
        const ON_LEFT_WALL = 0x1000_0000;
        const ON_RIGHT_WALL = 0x2000_0000;
        const CORPSE = 0x4000_0000;
        const PERSON = 0x8000_0000;

        const ANYFENCE = 0x0000_0C00;
        const ANYDOOR = 0x00CC_0008;
        const OBSTACLE = 0x0000_8F00;
        const WALLSTUFF = 0x00CF_0008;
        const BLOCKSMOVES = 0x0020_8F00;
        const TYPE_DEFINED = 0x8FEF_8F08;
        const ROOF = 0x0700_0000;
    }
}

bitflags::bitflags! {
    /// Wall orientation of a tile or structure (`Orientation` in `TileDef.h`)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Orientation: u8 {
        const INSIDE_LEFT = 0x01;
        const INSIDE_RIGHT = 0x02;
        const OUTSIDE_LEFT = 0x04;
        const OUTSIDE_RIGHT = 0x08;

        const LEFT = 0x05;
        const RIGHT = 0x0A;
        const INSIDE = 0x03;
        const OUTSIDE = 0x0C;
    }
}

impl Orientation {
    /// Walls that the editor calls horizontal (`ORIENT_LEFT`): on the edge between a tile and the
    /// tile of the next row
    pub fn is_horizontal(self) -> bool {
        self.intersects(Orientation::LEFT)
    }

    /// Walls that the editor calls vertical (`ORIENT_RIGHT`): on the edge between a tile and the
    /// tile of the next column
    pub fn is_vertical(self) -> bool {
        self.intersects(Orientation::RIGHT)
    }
}

/// One tile of a structure (`DB_STRUCTURE_TILE`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureTile {
    /// Position relative to the base tile as a grid number offset, `x + y * 160`
    pub pos_rel_to_base: i16,
    /// The tile is on the roof level of the structure's level
    pub on_roof: bool,
    pub passable: bool,
    /// Line of sight profile, each byte has one bit per height level
    pub shape: [[u8; PROFILE_Y_SIZE]; PROFILE_X_SIZE],
}

/// The structure of a tile (`DB_STRUCTURE`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileStructure {
    /// Material of the structure
    pub armour: u8,
    pub flags: StructureFlags,
    pub wall_orientation: Orientation,
    pub tiles: Vec<StructureTile>,
}

/// Z-buffer info of a sprite that spans multiple tiles (`ZStripInfo`)
///
/// The image is split into vertical strips of half a tile width, and the z value changes from
/// strip to strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ZStrip {
    /// Difference in z between the leftmost and the base strip
    pub initial_z_change: i8,
    /// Width of the leftmost strip in pixels
    pub first_z_strip_width: u8,
    /// Change of the z value in each strip after the first
    pub z_changes: Vec<i8>,
}
