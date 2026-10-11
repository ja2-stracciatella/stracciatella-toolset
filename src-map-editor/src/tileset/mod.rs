//! The tiles of the loaded map, the runtime form of the tile database
//!
//! [`Tileset::from_database`] turns the file format database into self-contained [`Tile`]s, so
//! the renderer doesn't need to know about the tile flags, tile type ranges and buddies of the
//! game.

use std::{collections::HashMap, sync::Arc};

use bevy::{
    app::{App, Plugin},
    asset::AssetApp,
    prelude::Resource,
};

use crate::file_formats::ja2_tile_types::{tile_type_start_index, TileType, NUMBER_OF_TILE_TYPES};

mod building;
mod convert;
mod sheet;
mod structure;
mod tile;

pub use building::*;
pub use sheet::*;
pub use structure::*;
pub use tile::*;

/// Number of road macros, the images of `FIRSTROAD` (`NUM_ROAD_MACROS`)
pub const NUM_ROAD_MACROS: u16 = 32;

/// Registers the sheet asset and its loader
pub struct TilesetPlugin;

impl Plugin for TilesetPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<StciSheet>()
            .init_asset_loader::<StciLoader>();
    }
}

/// All tiles that a map can reference
#[derive(Resource, Clone, Debug)]
pub struct Tileset {
    /// Only the tiles that exist, i.e. whose surface could be loaded
    tiles: Vec<Arc<Tile>>,
    /// `NUMBER_OF_TILES` entries, tile id to index into `tiles`
    tile_slots: Vec<Option<u32>>,
    /// Only the surfaces that could be loaded
    surfaces: Vec<Arc<TilesetSurface>>,
    /// `NUMBER_OF_TILE_TYPES` entries, the surface of each tile type of the tile database
    surface_slots: Vec<Option<SurfaceId>>,
    /// Current animation frame of each tile, parallel to `tiles`
    current_frames: Vec<u8>,
    /// The palette of each category, indexed by `TileCategory as usize`
    palettes: [Vec<Arc<Tile>>; TileCategory::ALL.len()],
    /// First tile id of every tile type, by the name of the type
    type_starts: HashMap<String, TileIndex>,
    /// The tiles of each building piece, in the order of their ids
    building_tiles: HashMap<BuildingTile, Vec<Arc<Tile>>>,
    /// The palettes of the buildings mode, indexed by `BuildingPalette as usize`
    building_palettes: [Vec<Arc<Tile>>; BuildingPalette::ALL.len()],
}

impl Tileset {
    /// Tile id of a tile in a map file (`GetTileIndexFromTypeSubIndex`)
    ///
    /// Like in the game, sub indices that are out of range for their type spill into the next
    /// type. Returns `None` for sub index 0, for indices after the last tile and for tiles of
    /// types that could not be loaded.
    pub fn index(&self, tile_type: TileType, sub_index: u16) -> Option<TileIndex> {
        if tile_type >= NUMBER_OF_TILE_TYPES || sub_index == 0 {
            return None;
        }
        let index = usize::from(tile_type_start_index(tile_type)) + usize::from(sub_index) - 1;
        let index = TileIndex::try_from(index).ok()?;
        self.tile(index)?;
        Some(index)
    }

    /// Tile id of a tile of the type with the given name, like [`Tileset::index`]
    pub fn tile_of(&self, type_name: &str, sub_index: u16) -> Option<TileIndex> {
        let start = self.type_start(type_name)?;
        if sub_index == 0 {
            return None;
        }
        let index = start.checked_add(sub_index - 1)?;
        self.tile(index)?;
        Some(index)
    }

    /// First tile id of the type with the given name, which orders the types like the tile
    /// database. `None` for names that are not tile types.
    pub fn type_start(&self, type_name: &str) -> Option<TileIndex> {
        self.type_starts.get(type_name).copied()
    }

    /// A tile, `None` if its type could not be loaded
    pub fn tile(&self, index: TileIndex) -> Option<&Arc<Tile>> {
        let slot = self.slot(index)?;
        Some(&self.tiles[slot])
    }

    /// Structure data of a tile, if it has any
    pub fn structure(&self, index: TileIndex) -> Option<&TileStructure> {
        self.tile(index)?.structure.as_ref()
    }

    /// The surface that a tile type of the tile database was built from, `None` if it could not
    /// be loaded
    pub fn surface_of_type(&self, tile_type: TileType) -> Option<&Arc<TilesetSurface>> {
        let id = (*self.surface_slots.get(tile_type)?)?;
        self.surface(id)
    }

    pub fn surface(&self, id: SurfaceId) -> Option<&Arc<TilesetSurface>> {
        self.surfaces.get(id.index())
    }

    /// All surfaces, indexed by [`SurfaceId`]
    pub fn surfaces(&self) -> &[Arc<TilesetSurface>] {
        &self.surfaces
    }

    /// The tiles of a category that the editor offers, in the order of the original selection
    /// window. For the textures the first tile of each type, for the floors their variants.
    pub fn palette(&self, category: TileCategory) -> &[Arc<Tile>] {
        &self.palettes[category as usize]
    }

    /// A palette of the buildings mode, in the order of the original selection window
    pub fn building_palette(&self, palette: BuildingPalette) -> &[Arc<Tile>] {
        &self.building_palettes[palette as usize]
    }

    /// The tiles of a building piece of a style, e.g. the variants of a wall piece to pick one
    /// from (`PickAWallPiece`)
    pub fn building_tiles(&self, style: u8, piece: BuildingPiece) -> &[Arc<Tile>] {
        self.building_tiles
            .get(&BuildingTile { style, piece })
            .map_or(&[], Vec::as_slice)
    }

    /// The animation frame that an animated tile currently shows
    pub fn current_frame(&self, index: TileIndex) -> Option<TileIndex> {
        let slot = self.slot(index)?;
        let frames = &self.tiles[slot].animation.as_ref()?.frames;
        if frames.is_empty() {
            return None;
        }
        Some(frames[usize::from(self.current_frames[slot]) % frames.len()])
    }

    /// Number of the frame that an animated tile currently shows, 0 for other tiles
    pub fn current_frame_number(&self, index: TileIndex) -> u8 {
        self.slot(index).map_or(0, |slot| self.current_frames[slot])
    }

    /// Advances the frame of every animated tile (`ANIMATETILES` in `RenderWorld`)
    pub fn advance_animations(&mut self) {
        for (tile, frame) in self.tiles.iter().zip(&mut self.current_frames) {
            let Some(animation) = &tile.animation else {
                continue;
            };
            if animation.frames.is_empty() {
                continue;
            }
            *frame = frame.wrapping_add(1);
            if usize::from(*frame) >= animation.frames.len() {
                *frame = 0;
            }
        }
    }

    fn slot(&self, index: TileIndex) -> Option<usize> {
        let slot = (*self.tile_slots.get(usize::from(index))?)?;
        Some(slot as usize)
    }
}

/// Also the fixtures of the benchmark
#[cfg(any(test, feature = "bench"))]
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
pub(crate) mod tests {
    use super::*;
    /// Tile types for building test maps
    pub(crate) use crate::file_formats::ja2_tile_types::{
        FIRSTFULLSTRUCT, FIRSTOSTRUCT, FIRSTROOF, FIRSTSHADOW, FIRSTTEXTURE, GUNS,
    };
    use crate::{
        file_formats::{
            ja2_structure::{
                tests::{aux_bytes, jsd_bytes, structure_bytes_with_tile_flags},
                Ja2StructureFile, StructureFlags as FileFlags, StructureTileFlags,
            },
            ja2_tile_database::{
                tests::{full_surfaces, test_database},
                Ja2TileDatabase, TileSurfaceInfo,
            },
            ja2_tile_types::{
                FENCESTRUCT, FIRSTDOOR, FIRSTWALL, FOURTHDOOR, FOURTHWALL, NUM_TILES_PER_TYPE,
                SECONDOSTRUCT,
            },
        },
        render::world::LevelNode,
    };

    /// A tileset with every tile type loaded and every tile a 40×20 image
    pub(crate) fn test_tileset() -> Tileset {
        Tileset::from_database(test_database())
    }

    /// `test_tileset` with structure data for some tiles:
    ///
    /// - `FIRSTOSTRUCT` 1: a single tile obstacle, 2: an obstacle on the tile and the one east of
    ///   it, 3: a 2×2 obstacle, 4: a passable structure
    /// - `SECONDOSTRUCT` 1: a single tile sandbag of 1 cube height
    /// - `FIRSTFULLSTRUCT` 1 to 3: single tile trees
    /// - `FENCESTRUCT` 1: a fence on the left side of the tile, which can be jumped
    pub(crate) fn structure_tileset() -> Tileset {
        Tileset::from_database(Ja2TileDatabase::build(structure_surfaces()).unwrap())
    }

    /// The surfaces of `structure_tileset`
    fn structure_surfaces() -> Vec<Option<TileSurfaceInfo>> {
        let blocking = |number, flags, tiles: &[(i8, i8, u8)]| {
            structure_bytes_with_tile_flags(number, flags, 0, tiles, StructureTileFlags::empty())
        };
        let file_of = |tile_type: usize, structures: &[Vec<u8>]| {
            let count = NUM_TILES_PER_TYPE[tile_type];
            Some(Ja2StructureFile::parse(&jsd_bytes(count, None, structures)).unwrap())
        };
        let file = |structures: &[Vec<u8>]| file_of(FIRSTOSTRUCT, structures);
        let mut surfaces = full_surfaces();
        let full = 0b1111;
        surfaces[FIRSTOSTRUCT].as_mut().unwrap().structure = file(&[
            blocking(0, FileFlags::GENERIC, &[(0, 0, full)]),
            blocking(1, FileFlags::GENERIC, &[(0, 0, full), (1, 0, full)]),
            blocking(
                2,
                FileFlags::GENERIC,
                &[(0, 0, full), (1, 0, full), (0, 1, full), (1, 1, full)],
            ),
            structure_bytes_with_tile_flags(
                3,
                FileFlags::GENERIC,
                0,
                &[(0, 0, 0b0001)],
                StructureTileFlags::PASSABLE,
            ),
        ]);
        let mut sandbag = blocking(0, FileFlags::GENERIC, &[(0, 0, 0b0001)]);
        // The armour is the first byte, 18 is `MATERIAL_SANDBAG`
        sandbag[0] = 18;
        surfaces[SECONDOSTRUCT].as_mut().unwrap().structure = file(&[sandbag]);
        surfaces[FIRSTFULLSTRUCT].as_mut().unwrap().structure = file(&[
            blocking(0, FileFlags::TREE, &[(0, 0, full)]),
            blocking(1, FileFlags::TREE, &[(0, 0, full)]),
            blocking(2, FileFlags::TREE, &[(0, 0, full)]),
        ]);
        surfaces[FENCESTRUCT].as_mut().unwrap().structure = file_of(
            FENCESTRUCT,
            &[structure_bytes_with_tile_flags(
                0,
                FileFlags::FENCE,
                1,
                &[(0, 0, 0b0011)],
                StructureTileFlags::empty(),
            )],
        );
        surfaces
    }

    /// `structure_tileset` with the orientations and structures of the building types, like the
    /// tilesets of the game:
    ///
    /// - `WALL1` to `WALL4`: each piece has the orientation of its wall class (e.g. the exterior
    ///   left pieces are outside left), windows, broken and weathered walls have the orientation
    ///   of their position in the image. All but the shadows and A-frames have a single tile
    ///   wall structure.
    /// - `DOOR1` to `DOOR4`: 5 frames for each of outside right, outside left, inside right and
    ///   inside left, each with a single tile door structure that is open in the last frame.
    pub(crate) fn building_tileset() -> Tileset {
        Tileset::from_database(Ja2TileDatabase::build(building_surfaces()).unwrap())
    }

    /// The surfaces of `building_tileset`
    pub(crate) fn building_surfaces() -> Vec<Option<TileSurfaceInfo>> {
        // The orientations of the structure files, which the game converts to the flags
        const IL: u8 = 1;
        const IR: u8 = 2;
        const OL: u8 = 3;
        const OR: u8 = 4;
        let wall_orientation = |sub_index: u16| -> u8 {
            match sub_index {
                1..=3 | 18..=20 | 13 | 17 | 57 => OR,
                4..=6 | 21..=23 => OL,
                7..=9 | 24..=26 | 14..=16 | 56 => IR,
                10..=12 | 27..=29 => IL,
                36..=38 | 52 | 53 | 58 | 60 => IL,
                39..=41 | 54 | 55 | 59 | 61 => IR,
                42..=44 | 48 | 49 | 62 | 64 => OL,
                45..=47 | 50 | 51 | 63 | 65 => OR,
                _ => 0,
            }
        };
        let full = 0b1111;
        let mut surfaces = structure_surfaces();
        for tile_type in FIRSTWALL..=FOURTHWALL {
            let count = NUM_TILES_PER_TYPE[tile_type];
            let aux: Vec<Vec<u8>> = (1..=count)
                .map(|sub_index| plain_aux_bytes(wall_orientation(sub_index)))
                .collect();
            let structures: Vec<Vec<u8>> = (1..=count)
                .filter(|&sub_index| wall_orientation(sub_index) != 0)
                .map(|sub_index| {
                    let orientation = wall_orientation(sub_index);
                    let flags = if (36..=47).contains(&sub_index) {
                        FileFlags::WALLNWINDOW
                    } else {
                        FileFlags::WALL
                    };
                    let tiles = [(0, 0, full)];
                    structure_bytes_with_tile_flags(
                        sub_index - 1,
                        flags,
                        orientation,
                        &tiles,
                        StructureTileFlags::empty(),
                    )
                })
                .collect();
            surfaces[tile_type].as_mut().unwrap().structure = Some(
                Ja2StructureFile::parse(&jsd_bytes(count, Some((&aux, &[])), &structures)).unwrap(),
            );
        }
        for tile_type in FIRSTDOOR..=FOURTHDOOR {
            let count = NUM_TILES_PER_TYPE[tile_type];
            let orientation = |sub_index: u16| [OR, OL, IR, IL][usize::from((sub_index - 1) / 5)];
            let aux: Vec<Vec<u8>> = (1..=count)
                .map(|sub_index| plain_aux_bytes(orientation(sub_index)))
                .collect();
            let structures: Vec<Vec<u8>> = (1..=count)
                .map(|sub_index| {
                    let mut flags = FileFlags::DOOR | FileFlags::OPENABLE;
                    if sub_index % 5 == 0 {
                        flags |= FileFlags::OPEN;
                    }
                    let tiles = [(0, 0, full)];
                    structure_bytes_with_tile_flags(
                        sub_index - 1,
                        flags,
                        orientation(sub_index),
                        &tiles,
                        StructureTileFlags::empty(),
                    )
                })
                .collect();
            surfaces[tile_type].as_mut().unwrap().structure = Some(
                Ja2StructureFile::parse(&jsd_bytes(count, Some((&aux, &[])), &structures)).unwrap(),
            );
        }
        surfaces
    }

    /// Aux data of a sub image that only has a wall orientation, unlike `aux_bytes` it is not
    /// animated
    fn plain_aux_bytes(wall_orientation: u8) -> Vec<u8> {
        let mut bytes = aux_bytes(wall_orientation, 0, 0);
        bytes[4..10].fill(0);
        bytes
    }

    /// Id of a tile of the test tileset
    pub(crate) fn test_tile_id(
        tileset: &Tileset,
        tile_type: TileType,
        sub_index: u16,
    ) -> TileIndex {
        tileset.index(tile_type, sub_index).unwrap()
    }

    /// A level node of a tile of the test tileset
    pub(crate) fn test_node(tileset: &Tileset, tile_type: TileType, sub_index: u16) -> LevelNode {
        let id = test_tile_id(tileset, tile_type, sub_index);
        LevelNode::new(tileset.tile(id).unwrap().clone())
    }
}
