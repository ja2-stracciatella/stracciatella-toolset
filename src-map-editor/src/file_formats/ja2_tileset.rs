use std::io;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use byteorder::{ReadBytesExt, LE};

use super::{
    ja2_tile_types::{
        TerrainType, TileType, AIRSTRIP, CAVES_1, DEAD_AIRSTRIP, DEEPWATERTEXTURE,
        DEFAULT_JA25_TILESET, FIFTHTEXTURE, FIRSTFLOOR, FIRSTROAD, FIRSTTEXTURE, FOURTHFLOOR,
        FOURTHTEXTURE, GENERIC_1, JA25_NUM_TILESETS, NUMBER_OF_TILE_TYPES, REGWATERTEXTURE,
        ROADPIECES, SECONDFLOOR, SECONDTEXTURE, SEVENTHTEXTURE, SIXTHTEXTURE, SPECIALTILES,
        TEMP_14, TEMP_18, TEMP_19, TEMP_20, TEMP_26, TEMP_27, TEMP_28, TEMP_29, THIRDFLOOR,
        THIRDTEXTURE, TROPICAL_1, VANILLA_NUM_TILESETS,
    },
    StracciatellaReadExt,
};
use crate::tileset::TilesetId;

#[derive(Clone, Debug)]
pub struct Ja2Tile {
    pub filename: String,
}

/// Which of the `Set*TerrainValues` functions of the game applies to a tileset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TilesetTerrainValues {
    #[default]
    One,
    /// Floors are flat ground
    Two,
    /// Roads are paved
    Three,
    /// The second floor is flat ground
    Four,
}

impl TilesetTerrainValues {
    /// Terrain values of a tileset, as assigned in `InitEngineTilesets`
    fn for_tileset(tileset_id: TilesetId) -> Self {
        match tileset_id {
            CAVES_1 => Self::Two,
            AIRSTRIP | DEAD_AIRSTRIP | TEMP_14 | TEMP_18 | TEMP_19 | TEMP_26 | TEMP_27
            | TEMP_28 | TEMP_29 => Self::Three,
            TROPICAL_1 | TEMP_20 => Self::Four,
            _ => Self::One,
        }
    }

    /// Terrain of a tile type, `NoTerrain` for types that the game does not set
    pub fn terrain_id(self, tile_type: TileType) -> TerrainType {
        match tile_type {
            FIRSTTEXTURE | SECONDTEXTURE | THIRDTEXTURE | FOURTHTEXTURE | SEVENTHTEXTURE => {
                TerrainType::FlatGround
            }
            FIFTHTEXTURE | SIXTHTEXTURE => TerrainType::LowGrass,
            REGWATERTEXTURE => TerrainType::LowWater,
            DEEPWATERTEXTURE => TerrainType::DeepWater,
            FIRSTROAD | ROADPIECES => match self {
                Self::Three => TerrainType::PavedRoad,
                _ => TerrainType::DirtRoad,
            },
            FIRSTFLOOR | THIRDFLOOR | FOURTHFLOOR => match self {
                Self::Two => TerrainType::FlatGround,
                _ => TerrainType::FlatFloor,
            },
            SECONDFLOOR => match self {
                Self::Two | Self::Four => TerrainType::FlatGround,
                _ => TerrainType::FlatFloor,
            },
            _ => TerrainType::NoTerrain,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ja2Tileset {
    pub name: String,
    pub ambient_id: u8,
    /// File name for each tile type, `None` if the tileset uses the file of the default tileset
    pub tiles: Vec<Option<Ja2Tile>>,
    pub terrain_values: TilesetTerrainValues,
}

impl Ja2Tileset {
    /// Terrain of a tile type in this tileset
    pub fn terrain_id(&self, tile_type: TileType) -> TerrainType {
        self.terrain_values.terrain_id(tile_type)
    }
}

/// The tileset and file a tile type is loaded from
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTileSurface {
    pub tileset_id: TilesetId,
    pub filename: String,
}

#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct Ja2Tilesets {
    pub(crate) tilesets: Vec<Ja2Tileset>,
}

#[derive(Default, TypePath)]
pub struct Ja2TilesetsLoader;

impl AssetLoader for Ja2TilesetsLoader {
    type Asset = Ja2Tilesets;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Ja2Tilesets, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ja2Tilesets::parse(&bytes)
    }
}

pub const TILESET_NAME_LENGTH: usize = 32;
pub const TILE_SURFACE_FILENAME_LENGTH: usize = 32;

impl Ja2Tilesets {
    /// Parses `JA2SET.DAT` and validates it like `InitEngineTilesets` in the game
    pub fn parse(bytes: &[u8]) -> io::Result<Ja2Tilesets> {
        let mut handle = io::Cursor::new(bytes);
        let number_of_tilesets = usize::from(handle.read_u8()?);
        if number_of_tilesets != VANILLA_NUM_TILESETS && number_of_tilesets != JA25_NUM_TILESETS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "expected {} or {} tilesets, got {}",
                    VANILLA_NUM_TILESETS, JA25_NUM_TILESETS, number_of_tilesets
                ),
            ));
        }
        let number_of_tiles = handle.read_u32::<LE>()?;
        if number_of_tiles as usize != NUMBER_OF_TILE_TYPES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "expected {} tile types per tileset, got {}",
                    NUMBER_OF_TILE_TYPES, number_of_tiles
                ),
            ));
        }

        let mut tilesets = Vec::with_capacity(number_of_tilesets);
        for tileset_id in 0..number_of_tilesets {
            let name = handle.read_fixed_string(TILESET_NAME_LENGTH)?;
            let ambient_id = handle.read_u8()?;
            let mut tiles = Vec::with_capacity(NUMBER_OF_TILE_TYPES);
            for _ in 0..NUMBER_OF_TILE_TYPES {
                let filename = handle.read_fixed_string(TILE_SURFACE_FILENAME_LENGTH)?;

                tiles.push(if filename.is_empty() {
                    None
                } else {
                    Some(Ja2Tile { filename })
                });
            }
            tilesets.push(Ja2Tileset {
                name,
                ambient_id,
                tiles,
                terrain_values: TilesetTerrainValues::for_tileset(TilesetId::new(tileset_id as u8)),
            })
        }

        Ok(Ja2Tilesets { tilesets })
    }

    pub fn get(&self, tileset_id: usize) -> Option<&Ja2Tileset> {
        self.tilesets.get(tileset_id)
    }

    /// The tilesets by id, in ascending order
    pub fn ids(&self) -> impl Iterator<Item = TilesetId> {
        (0..self.tilesets.len()).map(|id| TilesetId::new(id as u8))
    }

    /// Tileset that provides the tile types a tileset has no file for (`GetDefaultTileset`)
    pub fn default_tileset_id(&self) -> TilesetId {
        if self.tilesets.len() == JA25_NUM_TILESETS {
            DEFAULT_JA25_TILESET
        } else {
            GENERIC_1
        }
    }

    /// The file a tile type is loaded from for a tileset, falls back to the default tileset
    /// (`GetAdjustedTilesetResource`)
    ///
    /// Returns `None` if the tileset does not exist, or if neither the tileset nor the default
    /// tileset has a file for the tile type.
    pub fn resolve_tile_surface(
        &self,
        tileset_id: TilesetId,
        tile_type: TileType,
    ) -> Option<ResolvedTileSurface> {
        let resolve = |tileset_id: TilesetId| {
            let tile = self
                .get(tileset_id.index())?
                .tiles
                .get(tile_type)?
                .as_ref()?;
            Some(ResolvedTileSurface {
                tileset_id,
                filename: tile.filename.clone(),
            })
        };
        self.get(tileset_id.index())?;
        resolve(tileset_id).or_else(|| {
            let default_id =
                if self.tilesets.len() == JA25_NUM_TILESETS && tile_type == SPECIALTILES {
                    DEFAULT_JA25_TILESET
                } else {
                    self.default_tileset_id()
                };
            resolve(default_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_formats::ja2_tile_types::{FIRSTWALL, WIREFRAMES};

    fn fixed_string(value: &str, length: usize) -> Vec<u8> {
        let mut bytes = value.as_bytes().to_vec();
        bytes.resize(length, 0);
        bytes
    }

    /// Builds a `JA2SET.DAT`, `files` returns the file name of a tileset and tile type
    fn ja2set(
        number_of_tilesets: u8,
        number_of_tiles: u32,
        files: impl Fn(usize, usize) -> &'static str,
    ) -> Vec<u8> {
        let mut bytes = vec![number_of_tilesets];
        bytes.extend(number_of_tiles.to_le_bytes());
        for tileset_id in 0..usize::from(number_of_tilesets) {
            bytes.extend(fixed_string(
                &format!("Tileset {}", tileset_id),
                TILESET_NAME_LENGTH,
            ));
            bytes.push(tileset_id as u8);
            for tile_type in 0..number_of_tiles as usize {
                bytes.extend(fixed_string(
                    files(tileset_id, tile_type),
                    TILE_SURFACE_FILENAME_LENGTH,
                ));
            }
        }
        bytes
    }

    fn only_default(default_id: usize) -> impl Fn(usize, usize) -> &'static str {
        move |tileset_id, tile_type| match (tileset_id, tile_type) {
            (id, _) if id == default_id => "default.sti",
            (5, FIRSTWALL) => "wall.sti",
            _ => "",
        }
    }

    #[test]
    fn parses_tilesets() {
        let bytes = ja2set(50, NUMBER_OF_TILE_TYPES as u32, only_default(0));
        let tilesets = Ja2Tilesets::parse(&bytes).unwrap();
        assert_eq!(tilesets.tilesets.len(), 50);
        let tileset = tilesets.get(5).unwrap();
        assert_eq!(tileset.name, "Tileset 5");
        assert_eq!(tileset.ambient_id, 5);
        assert_eq!(tileset.tiles.len(), NUMBER_OF_TILE_TYPES);
        assert_eq!(
            tileset.tiles[FIRSTWALL].as_ref().unwrap().filename,
            "wall.sti"
        );
        assert!(tileset.tiles[FIRSTTEXTURE].is_none());
    }

    #[test]
    fn validates_counts() {
        for count in [0, 49, 51, 69, 71] {
            let bytes = ja2set(count, NUMBER_OF_TILE_TYPES as u32, |_, _| "");
            assert!(Ja2Tilesets::parse(&bytes).is_err(), "{} tilesets", count);
        }
        let bytes = ja2set(50, NUMBER_OF_TILE_TYPES as u32 - 1, |_, _| "");
        assert!(Ja2Tilesets::parse(&bytes).is_err());
        let bytes = ja2set(70, NUMBER_OF_TILE_TYPES as u32, |_, _| "");
        assert!(Ja2Tilesets::parse(&bytes).is_ok());
    }

    #[test]
    fn default_tileset_id() {
        let vanilla = Ja2Tilesets::parse(&ja2set(50, NUMBER_OF_TILE_TYPES as u32, |_, _| ""));
        assert_eq!(vanilla.unwrap().default_tileset_id(), GENERIC_1);
        let ja25 = Ja2Tilesets::parse(&ja2set(70, NUMBER_OF_TILE_TYPES as u32, |_, _| ""));
        assert_eq!(ja25.unwrap().default_tileset_id(), DEFAULT_JA25_TILESET);
    }

    #[test]
    fn resolves_with_fallback() {
        let tilesets =
            Ja2Tilesets::parse(&ja2set(50, NUMBER_OF_TILE_TYPES as u32, only_default(0))).unwrap();
        assert_eq!(
            tilesets.resolve_tile_surface(TilesetId::new(5), FIRSTWALL),
            Some(ResolvedTileSurface {
                tileset_id: TilesetId::new(5),
                filename: "wall.sti".to_owned()
            })
        );
        assert_eq!(
            tilesets.resolve_tile_surface(TilesetId::new(5), FIRSTTEXTURE),
            Some(ResolvedTileSurface {
                tileset_id: TilesetId::new(0),
                filename: "default.sti".to_owned()
            })
        );
        assert_eq!(
            tilesets.resolve_tile_surface(TilesetId::new(50), FIRSTTEXTURE),
            None
        );

        let tilesets =
            Ja2Tilesets::parse(&ja2set(70, NUMBER_OF_TILE_TYPES as u32, only_default(50))).unwrap();
        assert_eq!(
            tilesets
                .resolve_tile_surface(TilesetId::new(5), WIREFRAMES)
                .unwrap()
                .tileset_id,
            TilesetId::new(50)
        );
        assert_eq!(
            tilesets
                .resolve_tile_surface(TilesetId::new(5), SPECIALTILES)
                .unwrap()
                .tileset_id,
            TilesetId::new(50)
        );
    }

    #[test]
    fn missing_in_default_tileset() {
        let tilesets =
            Ja2Tilesets::parse(&ja2set(50, NUMBER_OF_TILE_TYPES as u32, only_default(1))).unwrap();
        assert_eq!(
            tilesets.resolve_tile_surface(TilesetId::new(5), FIRSTTEXTURE),
            None
        );
    }

    #[test]
    fn terrain_values() {
        let tilesets =
            Ja2Tilesets::parse(&ja2set(50, NUMBER_OF_TILE_TYPES as u32, |_, _| "")).unwrap();
        let terrain = |tileset_id: TilesetId, tile_type| {
            tilesets
                .get(tileset_id.index())
                .unwrap()
                .terrain_id(tile_type)
        };
        assert_eq!(terrain(GENERIC_1, FIRSTROAD), TerrainType::DirtRoad);
        assert_eq!(terrain(AIRSTRIP, FIRSTROAD), TerrainType::PavedRoad);
        assert_eq!(terrain(GENERIC_1, FIRSTFLOOR), TerrainType::FlatFloor);
        assert_eq!(terrain(CAVES_1, FIRSTFLOOR), TerrainType::FlatGround);
        assert_eq!(terrain(TROPICAL_1, SECONDFLOOR), TerrainType::FlatGround);
        assert_eq!(terrain(TROPICAL_1, THIRDFLOOR), TerrainType::FlatFloor);
        assert_eq!(terrain(GENERIC_1, FIFTHTEXTURE), TerrainType::LowGrass);
        assert_eq!(terrain(GENERIC_1, FIRSTWALL), TerrainType::NoTerrain);
    }
}
