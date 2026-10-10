//! A tile of the tileset, with everything the renderer needs to know about it

use std::{fmt, sync::Arc};

use super::{BuildingTile, Orientation, TileStructure, ZStrip};
use crate::file_formats::ja2_tile_types::{self, tile_index, ANOTHERDEBRIS, TEMP_19};

/// Tile id, the database index that the map files use
pub type TileIndex = u16;

/// Number of tile ids, larger node indices are not tiles
pub const NUMBER_OF_TILES: usize = ja2_tile_types::NUMBER_OF_TILES;

/// A tileset of `ja2set.dat` (`TileSetID`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TilesetId(u8);

impl TilesetId {
    pub const fn new(id: u8) -> Self {
        Self(id)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The raw id, the inverse of [`TilesetId::new`]
    pub const fn number(self) -> u8 {
        self.0
    }
}

impl TilesetId {
    /// Whether the tile is a part of the pool of this tileset whose art has no terrain
    ///
    /// Special case of the game in `CompileWorldTerrainIDs`, these tiles are low water.
    pub fn is_pool_without_terrain(self, index: TileIndex) -> bool {
        const POOL_DEBRIS: [TileIndex; 3] = [
            tile_index(ANOTHERDEBRIS, 4),
            tile_index(ANOTHERDEBRIS, 6),
            tile_index(ANOTHERDEBRIS, 7),
        ];
        self == TEMP_19 && POOL_DEBRIS.contains(&index)
    }
}

impl fmt::Display for TilesetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Id of a surface of the tileset, the index into its surfaces
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SurfaceId(pub(super) u16);

impl SurfaceId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// An image file that tiles show sub images of
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TilesetSurface {
    /// Own id, for the sheet and palette lookup
    pub id: SurfaceId,
    /// Tileset the file was loaded from, differs from the map tileset for default surfaces
    pub tileset_id: TilesetId,
    pub filename: String,
}

/// Where the sprite of a tile comes from
#[derive(Clone, Debug)]
pub struct TileImage {
    pub surface: Arc<TilesetSurface>,
    /// Sub image of the surface, underflow fillers show sub image 0
    pub sub_image: u16,
    /// Whether the tile only pads a tile type whose image has fewer sub images than tiles
    pub underflow_filler: bool,
}

/// One tile, immutable once the tileset is built
#[derive(Clone, Debug)]
pub struct Tile {
    /// Own tile id, so a tile reached through an `Arc` (e.g. a shadow buddy) can be named, e.g.
    /// for its sprite and animation
    pub id: TileIndex,
    /// Name of the tile type, e.g. `TEXTURE1` for `FIRSTTEXTURE`
    pub type_name: String,
    /// Position within the tile type, 1-based like in the map files and the editor
    pub sub_index: u16,
    pub kind: TileKind,
    /// What the tile is to the editor, set for every tile of the type, also the ones that are not
    /// in a palette (e.g. the animation frames of trees)
    pub category: Option<TileCategory>,
    /// The editing mode that places the tile, `None` for items and effects, which are always
    /// drawn
    pub view_layer: Option<ViewLayer>,
    /// Removed by the structure eraser although it is an object (`DEBRIS2MISC` and
    /// `ANOTHERDEBRIS`, `EraseMapTile` for `DRAW_MODE_OSTRUCTS`)
    pub erased_with_structures: bool,
    /// The piece of a building the tile is, for the tiles of the building types
    pub building: Option<BuildingTile>,
    /// Removed where a new building is placed (`BulldozeNature`): outside and inside structures,
    /// their shadows, the shadows of cliffs and the debris
    pub bulldozed: bool,
    pub image: TileImage,
    /// Movement terrain, after the shore fix-ups of the water tiles
    pub terrain: Terrain,
    /// Shadow tile drawn with this structure outdoors
    pub shadow_buddy: Option<Arc<Tile>>,
    /// Structure data, already including the double door fix
    pub structure: Option<TileStructure>,
    /// Tiles (relative to this one) that the image covers, for the terrain of multi-tile objects
    pub covered_tiles: Vec<(i8, i8)>,
    /// Depth information of tiles that span several z levels
    pub z_strip: Option<ZStrip>,
    /// Animation that is shown instead of the tile
    pub animation: Option<TileAnimation>,
    /// Drawn in the dynamic passes only
    pub dynamic: bool,
    /// Not raised by the terrain height (cliffs)
    pub ignores_world_height: bool,
    /// Sorted by the z value of the tile row in the object layer, not below all structures
    pub uses_object_z_height: bool,
    /// Lit like a wall by light from across a wall edge, if it has no structure data
    ///
    /// The game checks the tile type range `FIRSTWALLDECAL..=EIGTHWALLDECAL`, which spans far
    /// more than the wall decals. Of those tile types, only the structures are kept (e.g. the
    /// cave structures of the basements), wall decorations are lit like this by their kind.
    pub lit_as_wall: bool,
}

/// The view layer of a tile, named after the editing mode that places it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViewLayer {
    /// Land textures, water, roads and cliffs, and the floors of buildings, which are land
    Terrain,
    /// Debris, trees, rocks, misc junk and furniture
    Objects,
    /// Walls, doors, wall decorations, roofs and what is on them
    Buildings,
}

/// What a tile is to the editor, by its tile type: the texture the ground brush paints, or the
/// palette of the terrain mode it is picked from (`InitJA2SelectionWindow`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileCategory {
    /// Land textures, `FIRSTTEXTURE` to `SEVENTHTEXTURE`
    Ground,
    /// Regular water including its shores
    Water,
    DeepWater,
    /// Floors of buildings, never painted over or smoothed
    Floor,
    Debris,
    /// Trees & bushes
    Trees,
    /// Rocks & barrels
    Rocks,
    /// Other junk
    Misc,
    /// `ISTRUCT1` to `ISTRUCT4` and `FIRSTCISTRUCT`
    Furniture,
    /// `FIFTHISTRUCT` to `EIGHTISTRUCT`
    GenericFurniture,
    /// The road images, the index of an image is its road macro
    Roads,
    /// The cliff pieces, `FIRSTCLIFF`
    Cliffs,
}

impl TileCategory {
    pub const ALL: [TileCategory; 12] = [
        TileCategory::Ground,
        TileCategory::Water,
        TileCategory::DeepWater,
        TileCategory::Floor,
        TileCategory::Debris,
        TileCategory::Trees,
        TileCategory::Rocks,
        TileCategory::Misc,
        TileCategory::Furniture,
        TileCategory::GenericFurniture,
        TileCategory::Roads,
        TileCategory::Cliffs,
    ];

    /// Whether the ground brush paints tiles of the category
    pub fn is_texture(self) -> bool {
        matches!(
            self,
            TileCategory::Ground | TileCategory::Water | TileCategory::DeepWater
        )
    }

    /// Whether the structure eraser removes tiles of the category
    pub fn is_structure(self) -> bool {
        matches!(
            self,
            TileCategory::Trees
                | TileCategory::Rocks
                | TileCategory::Misc
                | TileCategory::Furniture
                | TileCategory::GenericFurniture
        )
    }
}

/// Aux data animation of a tile
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TileAnimation {
    /// Tiles of the frames, including the tile itself. Empty for shadows of animated structures
    /// that have no frames of their own, those are never drawn.
    pub frames: Vec<TileIndex>,
    /// Frame shown when the map is loaded
    pub initial_frame: u8,
}

/// What a tile is, by its tile type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// Land textures and water
    Terrain(TerrainTile),
    /// Floors of buildings, always full tiles
    Floor,
    Cliff {
        /// The overhanging part of a cliff (`CLIFFHANG_TILE`)
        hang: bool,
    },
    /// Outside and inside structures, fences, vehicles and structure debris
    Structure,
    /// Shadows of structures, cliffs and doors
    Shadow,
    Wall(WallTile),
    Door(DoorTile),
    /// Wall decals, windows and switches
    WallDecoration {
        orientation: Orientation,
    },
    Roof(RoofKind),
    OnRoof,
    /// Roads, debris, decorations, blood, explosion debris, the mock floor and exits
    Object,
    /// Graphics of items lying on the ground
    Item,
    /// UI elements, pointers, footprints, bullets and explosions
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainTile {
    /// Whether the tile covers its whole tile, tiles below it are not drawn
    pub full: bool,
}

/// Movement terrain of a tile (`TerrainTypeDefines` in `TileDef.h`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Terrain {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WallTile {
    pub orientation: Orientation,
    /// A-frame pieces are drawn higher, for roofs
    pub aframe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoorTile {
    pub orientation: Orientation,
    /// Tiles of the opening or closing sequence that starts at this tile, empty for the others
    pub animation: Vec<TileIndex>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoofKind {
    /// `FIRSTROOF` to `LASTROOF`, items and soldiers can be on them
    Flat,
    /// Slant roofs and their ceiling pieces (`SLANTROOFCEILING`)
    Slant,
    High,
    /// Roofs that are shown when the building is entered
    Revealed,
}

impl Tile {
    /// Whether the tile hides the land tiles below it (`FULL_TILE` of land tiles)
    pub fn is_full_land(&self) -> bool {
        match self.kind {
            TileKind::Terrain(terrain) => terrain.full,
            TileKind::Floor => true,
            _ => false,
        }
    }

    /// Whether the tile is a flat roof (`FIRSTROOF` to `LASTROOF`)
    pub fn is_flat_roof(&self) -> bool {
        self.kind == TileKind::Roof(RoofKind::Flat)
    }

    /// Whether the tile is a floor (`FIRSTFLOOR` to `LASTFLOOR`)
    pub fn is_floor(&self) -> bool {
        self.kind == TileKind::Floor
    }

    /// Wall orientation of walls, doors and wall decorations (`GetWallOrientation`), empty for
    /// other tiles
    pub fn wall_orientation(&self) -> Orientation {
        match &self.kind {
            TileKind::Wall(wall) => wall.orientation,
            TileKind::Door(door) => door.orientation,
            TileKind::WallDecoration { orientation } => *orientation,
            _ => Orientation::empty(),
        }
    }

    /// The shadow that the editor places with a door, the tile of the same sub index of the door
    /// shadow type
    pub fn door_shadow(&self) -> Option<&Arc<Tile>> {
        self.building
            .filter(|building| building.piece.is_door())
            .and(self.shadow_buddy.as_ref())
    }
}
