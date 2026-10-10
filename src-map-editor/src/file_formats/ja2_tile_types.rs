//! Tile types and the fixed layout of the tile database
//!
//! Port of `TileDat.h/.cc`, the tile flags of `TileDef.h` and the tileset ids of
//! `World_Tileset_Enums.h` in the game. The tile database has exactly
//! `NUM_TILES_PER_TYPE[type]` entries per tile type, so the index of a tile only depends on its
//! type and sub index. This is the value of the `TileDefines` enum in the game, e.g. `FIRSTDOOR1`
//! is `tile_index(FIRSTDOOR, 1)`.

use crate::tileset::TilesetId;

/// Index of a tile type, the `kind` of a tile in a map file
pub type TileType = usize;

// Generated from `TileTypeDefines` in `TileDat.h`

pub const FIRSTTEXTURE: TileType = 0;
pub const SECONDTEXTURE: TileType = 1;
pub const THIRDTEXTURE: TileType = 2;
pub const FOURTHTEXTURE: TileType = 3;
pub const FIFTHTEXTURE: TileType = 4;
pub const SIXTHTEXTURE: TileType = 5;
pub const SEVENTHTEXTURE: TileType = 6;
pub const REGWATERTEXTURE: TileType = 7;
pub const DEEPWATERTEXTURE: TileType = 8;
pub const FIRSTCLIFFHANG: TileType = 9;
pub const FIRSTCLIFF: TileType = 10;
pub const FIRSTCLIFFSHADOW: TileType = 11;
pub const FIRSTOSTRUCT: TileType = 12;
pub const SECONDOSTRUCT: TileType = 13;
pub const THIRDOSTRUCT: TileType = 14;
pub const FOURTHOSTRUCT: TileType = 15;
pub const FIFTHOSTRUCT: TileType = 16;
pub const SIXTHOSTRUCT: TileType = 17;
pub const SEVENTHOSTRUCT: TileType = 18;
pub const EIGHTOSTRUCT: TileType = 19;
pub const FIRSTFULLSTRUCT: TileType = 20;
pub const SECONDFULLSTRUCT: TileType = 21;
pub const THIRDFULLSTRUCT: TileType = 22;
pub const FOURTHFULLSTRUCT: TileType = 23;
pub const FIRSTSHADOW: TileType = 24;
pub const SECONDSHADOW: TileType = 25;
pub const THIRDSHADOW: TileType = 26;
pub const FOURTHSHADOW: TileType = 27;
pub const FIFTHSHADOW: TileType = 28;
pub const SIXTHSHADOW: TileType = 29;
pub const SEVENTHSHADOW: TileType = 30;
pub const EIGHTSHADOW: TileType = 31;
pub const FIRSTFULLSHADOW: TileType = 32;
pub const SECONDFULLSHADOW: TileType = 33;
pub const THIRDFULLSHADOW: TileType = 34;
pub const FOURTHFULLSHADOW: TileType = 35;
pub const FIRSTWALL: TileType = 36;
pub const SECONDWALL: TileType = 37;
pub const THIRDWALL: TileType = 38;
pub const FOURTHWALL: TileType = 39;
pub const FIRSTDOOR: TileType = 40;
pub const SECONDDOOR: TileType = 41;
pub const THIRDDOOR: TileType = 42;
pub const FOURTHDOOR: TileType = 43;
pub const FIRSTDOORSHADOW: TileType = 44;
pub const SECONDDOORSHADOW: TileType = 45;
pub const THIRDDOORSHADOW: TileType = 46;
pub const FOURTHDOORSHADOW: TileType = 47;
pub const SLANTROOFCEILING: TileType = 48;
pub const ANOTHERDEBRIS: TileType = 49;
pub const ROADPIECES: TileType = 50;
pub const FOURTHWINDOW: TileType = 51;
pub const FIRSTDECORATIONS: TileType = 52;
pub const SECONDDECORATIONS: TileType = 53;
pub const THIRDDECORATIONS: TileType = 54;
pub const FOURTHDECORATIONS: TileType = 55;
pub const FIRSTWALLDECAL: TileType = 56;
pub const SECONDWALLDECAL: TileType = 57;
pub const THIRDWALLDECAL: TileType = 58;
pub const FOURTHWALLDECAL: TileType = 59;
pub const FIRSTFLOOR: TileType = 60;
pub const SECONDFLOOR: TileType = 61;
pub const THIRDFLOOR: TileType = 62;
pub const FOURTHFLOOR: TileType = 63;
pub const FIRSTROOF: TileType = 64;
pub const SECONDROOF: TileType = 65;
pub const THIRDROOF: TileType = 66;
pub const FOURTHROOF: TileType = 67;
pub const FIRSTSLANTROOF: TileType = 68;
pub const SECONDSLANTROOF: TileType = 69;
pub const FIRSTONROOF: TileType = 70;
pub const SECONDONROOF: TileType = 71;
pub const MOCKFLOOR: TileType = 72;
pub const FIRSTISTRUCT: TileType = 73;
pub const SECONDISTRUCT: TileType = 74;
pub const THRIDISTRUCT: TileType = 75;
pub const FOURTHISTRUCT: TileType = 76;
pub const FIRSTCISTRUCT: TileType = 77;
pub const FIRSTROAD: TileType = 78;
pub const DEBRISROCKS: TileType = 79;
pub const DEBRISWOOD: TileType = 80;
pub const DEBRISWEEDS: TileType = 81;
pub const DEBRISGRASS: TileType = 82;
pub const DEBRISSAND: TileType = 83;
pub const DEBRISMISC: TileType = 84;
pub const ANIOSTRUCT: TileType = 85;
pub const FENCESTRUCT: TileType = 86;
pub const FENCESHADOW: TileType = 87;
pub const FIRSTVEHICLE: TileType = 88;
pub const SECONDVEHICLE: TileType = 89;
pub const FIRSTVEHICLESHADOW: TileType = 90;
pub const SECONDVEHICLESHADOW: TileType = 91;
pub const DEBRIS2MISC: TileType = 92;
pub const FIRSTDEBRISSTRUCT: TileType = 93;
pub const SECONDDEBRISSTRUCT: TileType = 94;
pub const FIRSTDEBRISSTRUCTSHADOW: TileType = 95;
pub const SECONDDEBRISSTRUCTSHADOW: TileType = 96;
pub const NINTHOSTRUCT: TileType = 97;
pub const TENTHOSTRUCT: TileType = 98;
pub const NINTHOSTRUCTSHADOW: TileType = 99;
pub const TENTHOSTRUCTSHADOW: TileType = 100;
pub const FIRSTEXPLDEBRIS: TileType = 101;
pub const SECONDEXPLDEBRIS: TileType = 102;
pub const FIRSTLARGEEXPDEBRIS: TileType = 103;
pub const SECONDLARGEEXPDEBRIS: TileType = 104;
pub const FIRSTLARGEEXPDEBRISSHADOW: TileType = 105;
pub const SECONDLARGEEXPDEBRISSHADOW: TileType = 106;
pub const FIFTHISTRUCT: TileType = 107;
pub const SIXTHISTRUCT: TileType = 108;
pub const SEVENTHISTRUCT: TileType = 109;
pub const EIGHTISTRUCT: TileType = 110;
pub const FIRSTHIGHROOF: TileType = 111;
pub const SECONDHIGHROOF: TileType = 112;
pub const FIFTHWALLDECAL: TileType = 113;
pub const SIXTHWALLDECAL: TileType = 114;
pub const SEVENTHWALLDECAL: TileType = 115;
pub const EIGTHWALLDECAL: TileType = 116;
pub const HUMANBLOOD: TileType = 117;
pub const CREATUREBLOOD: TileType = 118;
pub const FIRSTSWITCHES: TileType = 119;
pub const REVEALEDSLANTROOFS: TileType = 120;
pub const FIRSTREVEALEDHIGHROOFS: TileType = 121;
pub const SECONDREVEALEDHIGHROOFS: TileType = 122;
pub const GUNS: TileType = 123;
pub const P1ITEMS: TileType = 124;
pub const P2ITEMS: TileType = 125;
pub const WINDOWSHATTER: TileType = 126;
pub const P3ITEMS: TileType = 127;
pub const BODYEXPLOSION: TileType = 128;
pub const EXITTEXTURE: TileType = 129;
pub const FOOTPRINTS: TileType = 130;
pub const FIRSTPOINTERS: TileType = 131;
pub const SECONDPOINTERS: TileType = 132;
pub const THIRDPOINTERS: TileType = 133;
pub const GOODRUN: TileType = 134;
pub const GOODWALK: TileType = 135;
pub const GOODSWAT: TileType = 136;
pub const GOODPRONE: TileType = 137;
pub const CONFIRMMOVE: TileType = 138;
pub const VEHICLEMOVE: TileType = 139;
pub const ACTIONTWO: TileType = 140;
pub const XMARKER: TileType = 141;
pub const GOODRING: TileType = 142;
pub const ROTATINGKEY: TileType = 143;
pub const SELRING: TileType = 144;
pub const SPECIALTILES: TileType = 145;
pub const BULLETTILE: TileType = 146;
pub const FIRSTMISS: TileType = 147;
pub const SECONDMISS: TileType = 148;
pub const THIRDMISS: TileType = 149;
pub const WIREFRAMES: TileType = 150;
pub const NUMBER_OF_TILE_TYPES: usize = 151;

/// Names of the tile types (`gTileSurfaceName`)
pub const TILE_TYPE_NAMES: [&str; NUMBER_OF_TILE_TYPES] = [
    "TEXTURE1",
    "TEXTURE2",
    "TEXTURE3",
    "TEXTURE4",
    "TEXTURE5",
    "TEXTURE6",
    "TEXTURE7",
    "WATER1",
    "DEEP WATER",
    "FIRSTCLIFFHANG",
    "FIRSTCLIFF",
    "FIRSTCLIFFSHADOW",
    "OSTRUCT1",
    "OSTRUCT2",
    "OSTRUCT3",
    "OSTRUCT4",
    "OSTRUCT5",
    "OSTRUCT6",
    "OSTRUCT7",
    "OSTRUCT8",
    "OFSTRUCT1",
    "OFSTRUCT2",
    "PLACEHOLDER1",
    "PLACEHOLDER2",
    "SHADOW1",
    "SHADOW2",
    "SHADOW3",
    "SHADOW4",
    "SHADOW5",
    "SHADOW6",
    "SHADOW7",
    "SHADOW8",
    "FSHADOW1",
    "FSHADOW2",
    "PLACEHOLDER3",
    "PLACEHOLDER4",
    "WALL1",
    "WALL2",
    "WALL3",
    "WALL4",
    "DOOR1",
    "DOOR2",
    "DOOR3",
    "DOOR4",
    "DOORSH1",
    "DOORSH2",
    "DOORSH3",
    "DOORSH4",
    "SLANTFLATPEICE",
    "ANOTHERDEBRIS",
    "ROADPIECES",
    "WINDOW4",
    "DECORATIONS1",
    "DECORATIONS2",
    "DECORATIONS3",
    "DECORATIONS4",
    "WALLDECAL1",
    "WALLDECAL2",
    "WALLDECAL3",
    "WALLDECAL4",
    "FLOOR1",
    "FLOOR2",
    "FLOOR3",
    "FLOOR4",
    "ROOF1",
    "ROOF2",
    "ROOF3",
    "ROOF4",
    "SROOF1",
    "SROOF2",
    "ONROOF1",
    "ONROOF2",
    "MOCKF1",
    "ISTRUCT1",
    "ISTRUCT2",
    "ISTRUCT3",
    "ISTRUCT4",
    "FIRSTCISTRCUT",
    "FIRSTROAD",
    "ROCKS",
    "WOOD",
    "WEEDS",
    "GRASS",
    "SAND",
    "MISC",
    "ANIOSTRUCT",
    "FENCESTRUCT",
    "FENCESHADOW",
    "FIRSTVEHICLE",
    "SECONDVEHICLE",
    "FIRSTVEHICLESHADOW",
    "SECONDVEHICLESHADOW",
    "MISC2",
    "FIRSTDEBRISSTRUCT",
    "SECONDDEBRISSTRUCT",
    "FIRSTDEBRISSTRUCTSHADOW",
    "SECONDDEBRISSTRUCTSHADOW",
    "NINTHOSTRUCT",
    "TENTHOSTRUCT",
    "NINTHOSTRUCTSHADOW",
    "TENTHOSTRUCTSHADOW",
    "FIRSTEXPLODEDEBRIS",
    "SECONDEXPLODEDEBRIS",
    "FIRSTLARGEEXPLODEDEBRIS",
    "SECONDLARGEEXPLODEDEBRIS",
    "FIRSTLARGEEXPLODEDEBRISSHADOW",
    "SECONDLARGEEXPLODEDEBRISSHADOW",
    "FIFTHISTRUCT",
    "SIXTHISTRUCT",
    "SEVENTHISTRUCT",
    "EIGHTISTRUCT",
    "FIRSTHIGHROOF",
    "SECONDHIGHROOF",
    "WALLDECAL5",
    "WALLDECAL6",
    "WALLDECAL7",
    "WALLDECAL8",
    "HUMANBLOOD",
    "CREATUREBLOOD",
    "FIRSTSWITCHES",
    "REVEALEDSLANTROOF",
    "1stREVEALEDHIGHROOF",
    "2ndREVEALEDHIGHROOF",
    "GUNS",
    "ITEMS",
    "ITEMS2",
    "GLASSSHATTER",
    "ITEMS3",
    "BODYBLOW",
    "EXITTEXTURE",
    "FOOTPRINTS",
    "POINTERS",
    "POINTERS2",
    "POINTERS3",
    "GOODRUN",
    "GOODWALK",
    "GOODSWAT",
    "GOODSCOOT",
    "CONFIRMMOVE",
    "VEHICLEMOVE",
    "ACTIONTWO",
    "BADMARKER",
    "GRING",
    "ROTATINGKEY",
    "SELRING",
    "SPECIAL",
    "BULLET",
    "MISS1",
    "MISS2",
    "MISS3",
    "WIREFRAME",
];

/// Number of database entries of each tile type (`gNumTilesPerType`)
pub const NUM_TILES_PER_TYPE: [u16; NUMBER_OF_TILE_TYPES] = [
    35, 35, 35, 35, 35, 37, 49, 50, 37, 17, 17, 17, 12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 2, 2,
    12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 2, 2, 65, 65, 65, 65, 20, 20, 20, 20, 20, 20, 20, 20,
    2, 10, 400, 2, 10, 10, 10, 10, 10, 10, 10, 10, 8, 8, 8, 8, 14, 14, 14, 14, 20, 20, 12, 12, 1,
    24, 24, 24, 24, 24, 35, 10, 10, 10, 10, 10, 10, 20, 22, 22, 12, 12, 12, 12, 10, 10, 10, 10, 10,
    12, 12, 12, 12, 40, 40, 10, 10, 10, 10, 24, 24, 24, 24, 15, 15, 10, 10, 10, 10, 16, 16, 21, 8,
    11, 11, 60, 149, 45, 20, 16, 15, 35, 80, 24, 8, 3, 11, 11, 11, 11, 11, 10, 11, 11, 10, 8, 10,
    6, 2, 5, 5, 14, 15,
];

/// `gFullBaseTileValues`, the fullness of the land tiles up to `DEEPWATERTEXTURE10`
pub const FULL_BASE_TILE_VALUES: [u8; 321] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1,
];

/// Types before this one belong to the tilesets, the rest are items, animations and UI elements
pub const LAST_TILESET_ELEMENT: TileType = GUNS - 1;
pub const LAST_UI_ELEMENT: TileType = WIREFRAMES - 1;

/// Index of the first database entry of a tile type (`gTileTypeStartIndex`)
pub const fn tile_type_start_index(tile_type: TileType) -> u16 {
    let mut start = 0;
    let mut i = 0;
    while i < tile_type {
        start += NUM_TILES_PER_TYPE[i];
        i += 1;
    }
    start
}

/// Database index of a tile, `sub_index` is 1-based like the names of the `TileDefines` enum
pub const fn tile_index(tile_type: TileType, sub_index: u16) -> u16 {
    tile_type_start_index(tile_type) + sub_index - 1
}

/// Number of entries in the tile database
pub const NUMBER_OF_TILES: usize = tile_type_start_index(NUMBER_OF_TILE_TYPES) as usize;

/// Logical height of the land tile types, used for land layer ordering (`gTileTypeLogicalHeight`)
pub const TILE_TYPE_LOGICAL_HEIGHT: [u8; NUMBER_OF_TILE_TYPES] = {
    let mut heights = [0; NUMBER_OF_TILE_TYPES];
    let mut i = FIRSTTEXTURE;
    while i <= SEVENTHTEXTURE {
        heights[i] = 2;
        i += 1;
    }
    heights[REGWATERTEXTURE] = 10;
    heights[DEEPWATERTEXTURE] = 10;
    heights
};

// A-frame wall pieces within a wall type, 0-based (`WALL_AFRAME_START`, `WALL_AFRAME_END`)
pub const WALL_AFRAME_START: u16 = 31;
pub const WALL_AFRAME_END: u16 = 34;

bitflags::bitflags! {
    /// Flags of a tile database entry (`TileElementFlags` in `TileDef.h`)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct TileElementFlags: u32 {
        const WALL_TILE = 1 << 0;
        const ANIMATED_TILE = 1 << 1;
        const DYNAMIC_TILE = 1 << 2;
        const IGNORE_WORLD_HEIGHT = 1 << 3;
        const FULL3D_TILE = 1 << 4;
        const MULTI_Z_TILE = 1 << 5;
        const OBJECTLAYER_USEZHEIGHT = 1 << 6;
        const ROOFSHADOW_TILE = 1 << 7;
        const ROOF_TILE = 1 << 8;
        const HAS_SHADOW_BUDDY = 1 << 9;
        const AFRAME_TILE = 1 << 10;
        const CLIFFHANG_TILE = 1 << 11;
        const UNDERFLOW_FILLER = 1 << 12;
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

/// Terrain of a tile (`TerrainTypeDefines` in `TileDef.h`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TerrainType {
    #[default]
    NoTerrain,
    FlatGround,
    FlatFloor,
    PavedRoad,
    DirtRoad,
    LowGrass,
    HighGrass,
    TrainTracks,
    LowWater,
    MedWater,
    DeepWater,
}

/// A struct tile type and its shadow tile type, the tiles of both are buddies of each other
pub struct ShadowBuddies {
    pub structure: TileType,
    pub shadow: TileType,
}

const fn buddies(structure: TileType, shadow: TileType) -> ShadowBuddies {
    ShadowBuddies { structure, shadow }
}

/// Bidirectional mapping between structs and shadows (`g_shadow_buddies`)
pub const SHADOW_BUDDIES: [ShadowBuddies; 26] = [
    buddies(FIRSTCLIFF, FIRSTCLIFFSHADOW),
    buddies(FIRSTOSTRUCT, FIRSTSHADOW),
    buddies(SECONDOSTRUCT, SECONDSHADOW),
    buddies(THIRDOSTRUCT, THIRDSHADOW),
    buddies(FOURTHOSTRUCT, FOURTHSHADOW),
    buddies(FIFTHOSTRUCT, FIFTHSHADOW),
    buddies(SIXTHOSTRUCT, SIXTHSHADOW),
    buddies(SEVENTHOSTRUCT, SEVENTHSHADOW),
    buddies(EIGHTOSTRUCT, EIGHTSHADOW),
    buddies(FIRSTFULLSTRUCT, FIRSTFULLSHADOW),
    buddies(SECONDFULLSTRUCT, SECONDFULLSHADOW),
    buddies(THIRDFULLSTRUCT, THIRDFULLSHADOW),
    buddies(FOURTHFULLSTRUCT, FOURTHFULLSHADOW),
    buddies(FIRSTDOOR, FIRSTDOORSHADOW),
    buddies(SECONDDOOR, SECONDDOORSHADOW),
    buddies(THIRDDOOR, THIRDDOORSHADOW),
    buddies(FOURTHDOOR, FOURTHDOORSHADOW),
    buddies(FENCESTRUCT, FENCESHADOW),
    buddies(FIRSTVEHICLE, FIRSTVEHICLESHADOW),
    buddies(SECONDVEHICLE, SECONDVEHICLESHADOW),
    buddies(FIRSTDEBRISSTRUCT, FIRSTDEBRISSTRUCTSHADOW),
    buddies(SECONDDEBRISSTRUCT, SECONDDEBRISSTRUCTSHADOW),
    buddies(NINTHOSTRUCT, NINTHOSTRUCTSHADOW),
    buddies(TENTHOSTRUCT, TENTHOSTRUCTSHADOW),
    buddies(FIRSTLARGEEXPDEBRIS, FIRSTLARGEEXPDEBRISSHADOW),
    buddies(SECONDLARGEEXPDEBRIS, SECONDLARGEEXPDEBRISSHADOW),
];

/// Tiles 1, 6, 11 and 16 of all four types, the first frame of each door opening animation
const fn door_frames(first: TileType, sub_index: u16) -> [u16; 16] {
    let mut list = [0; 16];
    let mut i = 0;
    while i < 16 {
        list[i] = tile_index(first + i % 4, sub_index + (i / 4) as u16 * 5);
        i += 1;
    }
    list
}

/// Open doors, animated forwards (`gOpenDoorList`)
pub const OPEN_DOOR_LIST: [u16; 16] = door_frames(FIRSTDOOR, 1);
/// Shadows of open doors, animated forwards (`gOpenDoorShadowList`)
pub const OPEN_DOOR_SHADOW_LIST: [u16; 16] = door_frames(FIRSTDOORSHADOW, 1);
/// Closed doors, animated backwards (`gClosedDoorList`)
pub const CLOSED_DOOR_LIST: [u16; 16] = door_frames(FIRSTDOOR, 5);
/// Shadows of closed doors, animated backwards (`gClosedDoorShadowList`)
pub const CLOSED_DOOR_SHADOW_LIST: [u16; 16] = door_frames(FIRSTDOORSHADOW, 5);

/// Number of tilesets of vanilla JA2
pub const VANILLA_NUM_TILESETS: usize = 50;
/// Number of tilesets of Unfinished Business
pub const JA25_NUM_TILESETS: usize = 70;

// Tileset ids (`TileSetID`), only the ones the code refers to
pub const GENERIC_1: TilesetId = TilesetId::new(0);
pub const CAVES_1: TilesetId = TilesetId::new(1);
pub const TROPICAL_1: TilesetId = TilesetId::new(4);
pub const AIRSTRIP: TilesetId = TilesetId::new(17);
pub const DEAD_AIRSTRIP: TilesetId = TilesetId::new(19);
pub const TEMP_14: TilesetId = TilesetId::new(33);
pub const TEMP_18: TilesetId = TilesetId::new(37);
pub const TEMP_19: TilesetId = TilesetId::new(38);
pub const TEMP_20: TilesetId = TilesetId::new(39);
pub const TEMP_26: TilesetId = TilesetId::new(45);
pub const TEMP_27: TilesetId = TilesetId::new(46);
pub const TEMP_28: TilesetId = TilesetId::new(47);
pub const TEMP_29: TilesetId = TilesetId::new(48);
pub const DEFAULT_JA25_TILESET: TilesetId = TilesetId::new(50);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_indices_match_tile_defines() {
        // Values of the `TileDefines` enum in the game
        assert_eq!(tile_index(FIRSTTEXTURE, 1), 0);
        assert_eq!(tile_index(FIRSTCLIFFHANG, 1), 348);
        assert_eq!(tile_index(FIRSTCLIFFSHADOW, 17), 398);
        assert_eq!(tile_index(FIRSTFULLSTRUCT, 1), 495);
        assert_eq!(tile_index(FIRSTWALL, 1), 647);
        assert_eq!(tile_index(FOURTHWALL, 65), 906);
        assert_eq!(tile_index(FIRSTDOOR, 1), 907);
        assert_eq!(tile_index(FOURTHDOORSHADOW, 20), 1066);
        assert_eq!(tile_index(FIRSTFLOOR, 1), 1561);
        assert_eq!(tile_index(MOCKFLOOR, 1), 1713);
        assert_eq!(tile_index(WINDOWSHATTER, 1), 2762);
        assert_eq!(tile_index(FOOTPRINTS, 1), 2848);
        assert_eq!(tile_index(SPECIALTILES, 1), 3078);
        assert_eq!(tile_index(FIRSTMISS, 1), 3086);
        assert_eq!(NUMBER_OF_TILES, 3125);
    }

    #[test]
    fn full_base_tile_values_cover_the_land_types() {
        assert_eq!(
            FULL_BASE_TILE_VALUES.len(),
            usize::from(tile_index(DEEPWATERTEXTURE, 10)) + 1
        );
    }

    #[test]
    fn door_lists() {
        assert_eq!(OPEN_DOOR_LIST[0], tile_index(FIRSTDOOR, 1));
        assert_eq!(OPEN_DOOR_LIST[3], tile_index(FOURTHDOOR, 1));
        assert_eq!(OPEN_DOOR_LIST[4], tile_index(FIRSTDOOR, 6));
        assert_eq!(CLOSED_DOOR_LIST[15], tile_index(FOURTHDOOR, 20));
        assert_eq!(
            CLOSED_DOOR_SHADOW_LIST[15],
            tile_index(FOURTHDOORSHADOW, 20)
        );
    }
}
