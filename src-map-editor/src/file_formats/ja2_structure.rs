//! Structure data files (`.jsd`, "J2SD")
//!
//! A JSD file lies next to a tile STI and describes the structures of its sub images: which tiles
//! they cover, their line of sight profile, armour, etc. It can also replace the app data of the
//! STI (aux data and relative tile locations). Port of `STRUCTURE_FILE_REF`,
//! `CreateFileStructureArrays` and `AddZStripInfoToVObject` in `Structure.cc` of the game.

use std::io::{self, ErrorKind::InvalidData, Read};

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use byteorder::{ReadBytesExt, LE};

use super::{
    ja2_stci::SubImageInfo, ja2_tile_types::Orientation, stci::StciAppData, StracciatellaReadExt,
};

/// Width of the map in tiles (`WORLD_COLS`)
const WORLD_COLS: i16 = 160;
/// Width of a tile in pixels (`WORLD_TILE_X`)
const WORLD_TILE_X: i32 = 40;

const STRUCTURE_FILE_ID: &[u8; 4] = b"J2SD";
const STRUCTURE_FILE_HEADER_SIZE: usize = 16;
const STRUCTURE_FILE_CONTAINS_AUXIMAGEDATA: u8 = 0x01;
const STRUCTURE_FILE_CONTAINS_STRUCTUREDATA: u8 = 0x02;

/// Size of `DB_STRUCTURE` in bytes
pub const DB_STRUCTURE_SIZE: usize = 16;
/// Size of `DB_STRUCTURE_TILE` in bytes
pub const DB_STRUCTURE_TILE_SIZE: usize = 32;

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
    /// Flags of a structure tile
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StructureTileFlags: u8 {
        const ON_ROOF = 0x01;
        const PASSABLE = 0x02;
    }
}

/// One tile of a structure (`DB_STRUCTURE_TILE`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbStructureTile {
    /// Position relative to the base tile as a grid number offset, `x + y * 160`
    pub pos_rel_to_base: i16,
    pub x_pos_rel_to_base: i8,
    pub y_pos_rel_to_base: i8,
    /// Line of sight profile, each byte has one bit per height level
    pub shape: [[u8; PROFILE_Y_SIZE]; PROFILE_X_SIZE],
    pub flags: StructureTileFlags,
    pub vehicle_hit_location: u8,
}

impl DbStructureTile {
    fn from_input(input: &mut impl Read) -> io::Result<Self> {
        let pos_rel_to_base = input.read_i16::<LE>()?;
        let x_pos_rel_to_base = input.read_i8()?;
        let y_pos_rel_to_base = input.read_i8()?;
        let mut shape = [[0; PROFILE_Y_SIZE]; PROFILE_X_SIZE];
        for row in &mut shape {
            input.read_exact(row)?;
        }
        let flags = StructureTileFlags::from_bits_retain(input.read_u8()?);
        let vehicle_hit_location = input.read_u8()?;
        input.read_unused(1)?;
        Ok(Self {
            pos_rel_to_base,
            x_pos_rel_to_base,
            y_pos_rel_to_base,
            shape,
            flags,
            vehicle_hit_location,
        })
    }

    /// Number of filled cubes in the profile (`FilledTilePositions`)
    fn filled_positions(&self) -> u32 {
        self.shape
            .iter()
            .flatten()
            .map(|&value| (value & 0x0F).count_ones())
            .sum()
    }
}

/// A structure, the structure data of one sub image (`DB_STRUCTURE`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DbStructure {
    pub armour: u8,
    /// Computed from the profile when loading, like in the game
    pub hit_points: u8,
    pub density: u8,
    pub flags: StructureFlags,
    pub structure_number: u16,
    pub wall_orientation: Orientation,
    /// > 0: debris number (value - 1), < 0: partner graphic
    pub destruction_partner: i8,
    /// Opened/closed version etc., 0 if unused
    pub partner_delta: i8,
    pub z_tile_offset_x: i8,
    pub z_tile_offset_y: i8,
    pub tiles: Vec<DbStructureTile>,
}

/// Header of a `DB_STRUCTURE`, the tiles follow it
struct DbStructureHeader {
    armour: u8,
    density: u8,
    number_of_tiles: u8,
    flags: StructureFlags,
    structure_number: u16,
    wall_orientation: u8,
    destruction_partner: i8,
    partner_delta: i8,
    z_tile_offset_x: i8,
    z_tile_offset_y: i8,
}

impl DbStructureHeader {
    fn from_input(input: &mut impl Read) -> io::Result<Self> {
        let armour = input.read_u8()?;
        let _hit_points = input.read_u8()?;
        let density = input.read_u8()?;
        let number_of_tiles = input.read_u8()?;
        let flags = StructureFlags::from_bits_retain(input.read_u32::<LE>()?);
        let structure_number = input.read_u16::<LE>()?;
        let wall_orientation = input.read_u8()?;
        let destruction_partner = input.read_i8()?;
        let partner_delta = input.read_i8()?;
        let z_tile_offset_x = input.read_i8()?;
        let z_tile_offset_y = input.read_i8()?;
        input.read_unused(1)?;
        Ok(Self {
            armour,
            density,
            number_of_tiles,
            flags,
            structure_number,
            wall_orientation,
            destruction_partner,
            partner_delta,
            z_tile_offset_x,
            z_tile_offset_y,
        })
    }
}

/// Z-buffer info of a sub image that spans multiple tiles (`ZStripInfo`)
///
/// The image is split into vertical strips of half a tile width, and the z value changes from
/// strip to strip.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ZStripInfo {
    /// Difference in z between the leftmost and the base strip
    pub initial_z_change: i8,
    /// Width of the leftmost strip in pixels
    pub first_z_strip_width: u8,
    /// Change of the z value in each strip after the first
    pub z_changes: Vec<i8>,
}

/// A structure data file
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct Ja2StructureFile {
    /// Number of sub images the file describes
    pub number_of_structures: u16,
    /// Aux data for each sub image, empty if the file has none
    pub aux_data: Vec<StciAppData>,
    /// Relative tile locations, referenced by the aux data
    pub tile_locs: Vec<(i8, i8)>,
    /// Structure of each sub image, empty if the file has no structure data
    pub structures: Vec<Option<DbStructure>>,
}

#[derive(Default, TypePath)]
pub struct Ja2StructureFileLoader;

impl AssetLoader for Ja2StructureFileLoader {
    type Asset = Ja2StructureFile;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Ja2StructureFile, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ja2StructureFile::parse(&bytes)
    }

    fn extensions(&self) -> &[&str] {
        &["jsd", "JSD"]
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(InvalidData, message.into())
}

impl Ja2StructureFile {
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut input = io::Cursor::new(bytes);

        let mut id = [0; 4];
        input.read_exact(&mut id)?;
        let number_of_structures = input.read_u16::<LE>()?;
        let number_of_structures_stored = input.read_u16::<LE>()?;
        let data_size = input.read_u16::<LE>()?;
        let flags = input.read_u8()?;
        input.read_unused(3)?;
        let number_of_tile_locs = input.read_u16::<LE>()?;
        debug_assert_eq!(input.position(), STRUCTURE_FILE_HEADER_SIZE as u64);
        if &id != STRUCTURE_FILE_ID {
            return Err(invalid_data("not a structure file, expected J2SD"));
        }
        if number_of_structures == 0 {
            return Err(invalid_data("structure file has no structures"));
        }

        let mut file = Ja2StructureFile {
            number_of_structures,
            ..Default::default()
        };
        if flags & STRUCTURE_FILE_CONTAINS_AUXIMAGEDATA != 0 {
            file.aux_data = (0..number_of_structures)
                .map(|_| StciAppData::from_input(&mut input))
                .collect::<io::Result<_>>()?;
            file.tile_locs = (0..number_of_tile_locs)
                .map(|_| Ok((input.read_i8()?, input.read_i8()?)))
                .collect::<io::Result<_>>()?;
        }
        if flags & STRUCTURE_FILE_CONTAINS_STRUCTUREDATA != 0 && data_size > 0 {
            let mut data = vec![0; usize::from(data_size)];
            input.read_exact(&mut data)?;
            file.read_structures(&data, number_of_structures_stored)?;
        }
        Ok(file)
    }

    /// Port of `CreateFileStructureArrays`
    fn read_structures(&mut self, data: &[u8], number_stored: u16) -> io::Result<()> {
        let too_short = || invalid_data("structure data is too short");
        let mut input = io::Cursor::new(data);
        self.structures = vec![None; usize::from(self.number_of_structures)];
        for _ in 0..number_stored {
            let header = DbStructureHeader::from_input(&mut input).map_err(|_| too_short())?;
            let mut tiles = (0..header.number_of_tiles)
                .map(|_| DbStructureTile::from_input(&mut input).map_err(|_| too_short()))
                .collect::<io::Result<Vec<_>>>()?;

            let mut hit_points = 0;
            for tile in &mut tiles {
                tile.pos_rel_to_base = i16::from(tile.x_pos_rel_to_base)
                    + i16::from(tile.y_pos_rel_to_base) * WORLD_COLS;
                hit_points += tile.filled_positions();
            }
            normalize_structure_tiles(&mut tiles);

            let index = usize::from(header.structure_number);
            if index >= self.structures.len() {
                return Err(invalid_data(format!(
                    "structure number {} is out of range, the file has {} structures",
                    index,
                    self.structures.len()
                )));
            }

            // Convert contiguous orientation values to flags
            let mut wall_orientation = header.wall_orientation;
            if !self.aux_data.is_empty() && wall_orientation > 2 {
                wall_orientation = match wall_orientation {
                    3 => Orientation::OUTSIDE_LEFT.bits(),
                    4 => Orientation::OUTSIDE_RIGHT.bits(),
                    other => other,
                };
                self.aux_data[index].wall_orientation = wall_orientation;
            }

            self.structures[index] = Some(DbStructure {
                armour: header.armour,
                // Scale hit points down to something reasonable
                hit_points: (hit_points * 100 / 255) as u8,
                density: header.density,
                flags: header.flags,
                structure_number: header.structure_number,
                wall_orientation: Orientation::from_bits_retain(wall_orientation),
                destruction_partner: header.destruction_partner,
                partner_delta: header.partner_delta,
                z_tile_offset_x: header.z_tile_offset_x,
                z_tile_offset_y: header.z_tile_offset_y,
                tiles,
            });
        }
        Ok(())
    }

    /// Whether the file has structure data, i.e. whether `structures` is filled
    pub fn has_structure_data(&self) -> bool {
        !self.structures.is_empty()
    }

    /// Z-strip info for each sub image of a tile surface, port of `AddZStripInfoToVObject` for
    /// tile surfaces (not animations)
    ///
    /// Returns one entry per sub image, all `None` if no structure spans multiple tiles.
    pub fn z_strip_info(&self, sub_images: &[SubImageInfo]) -> Vec<Option<ZStripInfo>> {
        let mut result = vec![None; sub_images.len()];
        if !self.has_structure_data() {
            return result;
        }

        let spans_multiple_tiles = self.structures.iter().flatten().any(|structure| {
            structure.tiles.len() > 1
                && structure.tiles[1..]
                    .iter()
                    .any(|tile| tile.pos_rel_to_base != 0)
        });
        // The game checks the corpse flag of the last structure only
        let last_is_corpse = self
            .structures
            .last()
            .and_then(Option::as_ref)
            .is_some_and(|structure| structure.flags.contains(StructureFlags::CORPSE));
        if !spans_multiple_tiles && !last_is_corpse {
            return result;
        }

        for (index, sub_image) in sub_images.iter().enumerate() {
            let Some(Some(structure)) = self.structures.get(index) else {
                continue;
            };
            if structure.tiles.len() <= 1 && !structure.flags.contains(StructureFlags::CORPSE) {
                continue;
            }
            if structure.flags.intersects(StructureFlags::ANYDOOR)
                && !structure.flags.contains(StructureFlags::GARAGEDOOR)
            {
                continue;
            }
            result[index] = Some(compute_z_strip_info(structure, sub_image));
        }
        result
    }
}

/// Makes sure that the tile closest to the base has position 0 (`NormalizeStructureTiles`)
fn normalize_structure_tiles(tiles: &mut [DbStructureTile]) {
    let Some(min_dist) = tiles
        .iter()
        .map(|tile| tile.pos_rel_to_base)
        .reduce(|min, pos| if min.abs() > pos.abs() { pos } else { min })
    else {
        return;
    };
    if min_dist == 0 {
        return;
    }
    let x_dist = (min_dist % WORLD_COLS) as i8;
    let y_dist = (min_dist / WORLD_COLS) as i8;
    for tile in tiles {
        tile.pos_rel_to_base -= min_dist;
        tile.x_pos_rel_to_base = tile.x_pos_rel_to_base.wrapping_sub(x_dist);
        tile.y_pos_rel_to_base = tile.y_pos_rel_to_base.wrapping_sub(y_dist);
    }
}

fn compute_z_strip_info(structure: &DbStructure, sub_image: &SubImageInfo) -> ZStripInfo {
    const HALF_TILE: i32 = WORLD_TILE_X / 2;

    let mut offset_x = i32::from(sub_image.offset.0);
    let width = i32::from(sub_image.dimensions.0);
    if structure
        .flags
        .intersects(StructureFlags::MOBILE | StructureFlags::CORPSE)
    {
        // Adjust for the difference between the animation and structure base tile, and for the
        // tile offset. The y offset is irrelevant for the strips.
        offset_x += HALF_TILE;
        offset_x = offset_x - i32::from(structure.z_tile_offset_x) * HALF_TILE
            + i32::from(structure.z_tile_offset_y) * HALF_TILE;
    }

    // How much of the image is on each side of the bottom corner of the base tile
    let (left_half_width, right_half_width) = if offset_x <= 0 {
        let right = width + offset_x - HALF_TILE;
        if right >= 0 {
            // Negative image offset, image straddles bottom corner
            (-offset_x + HALF_TILE, right)
        } else {
            // Negative image offset, image all on the left side
            (width - (right % HALF_TILE), 0)
        }
    } else if offset_x < HALF_TILE {
        let left = HALF_TILE - offset_x;
        let right = width - left;
        if right <= 0 {
            // Positive offset < 20, image all on the left side
            (HALF_TILE, 0)
        } else {
            // Positive offset < 20, image straddles bottom corner
            (left, right)
        }
    } else {
        // Positive offset, image all on the right side
        (0, width)
    };

    let mut num_increasing: u8 = 0;
    let mut num_stable: u8 = 0;
    let mut num_decreasing: u8 = 0;
    if left_half_width > 0 {
        num_increasing = (left_half_width / HALF_TILE) as u8;
    }
    if right_half_width > 0 {
        num_stable = 1;
        if right_half_width > HALF_TILE {
            num_decreasing = (right_half_width / HALF_TILE) as u8;
        }
    }

    let mut first_z_strip_width;
    if left_half_width > 0 {
        first_z_strip_width = (left_half_width % HALF_TILE) as u8;
        if first_z_strip_width == 0 {
            num_increasing = num_increasing.saturating_sub(1);
            first_z_strip_width = HALF_TILE as u8;
        }
    } else {
        // Right side only, the offset is at least half a tile
        first_z_strip_width = if offset_x > WORLD_TILE_X {
            (HALF_TILE - (offset_x - WORLD_TILE_X) % HALF_TILE) as u8
        } else {
            (WORLD_TILE_X - offset_x) as u8
        };
        if first_z_strip_width == 0 {
            // The game underflows here for images narrower than a tile half
            num_decreasing = num_decreasing.saturating_sub(1);
            first_z_strip_width = HALF_TILE as u8;
        }
    }

    let mut z_changes = Vec::with_capacity(
        usize::from(num_increasing) + usize::from(num_stable) + usize::from(num_decreasing),
    );
    z_changes.extend(std::iter::repeat_n(1, num_increasing.into()));
    z_changes.extend(std::iter::repeat_n(0, num_stable.into()));
    z_changes.extend(std::iter::repeat_n(-1, num_decreasing.into()));

    let initial_z_change = if num_increasing > 0 {
        -(num_increasing as i8)
    } else if num_stable > 0 {
        0
    } else {
        -(num_decreasing as i8)
    };

    ZStripInfo {
        initial_z_change,
        first_z_strip_width,
        z_changes,
    }
}

/// Also the fixtures of the benchmark
#[cfg(any(test, feature = "bench"))]
#[cfg_attr(not(test), allow(dead_code, unused_imports))]
pub(crate) mod tests {
    use super::*;
    use crate::file_formats::stci::{indexed::STCI_APP_DATA_SIZE, StciAppDataFlags};

    /// Builds the bytes of a `DB_STRUCTURE` and its tiles, tiles are `(x, y, shape byte)`
    pub fn structure_bytes(
        number: u16,
        flags: StructureFlags,
        wall_orientation: u8,
        tiles: &[(i8, i8, u8)],
    ) -> Vec<u8> {
        structure_bytes_with_tile_flags(
            number,
            flags,
            wall_orientation,
            tiles,
            StructureTileFlags::PASSABLE,
        )
    }

    /// A structure whose tiles all have the given flags
    pub fn structure_bytes_with_tile_flags(
        number: u16,
        flags: StructureFlags,
        wall_orientation: u8,
        tiles: &[(i8, i8, u8)],
        tile_flags: StructureTileFlags,
    ) -> Vec<u8> {
        let mut bytes = vec![7, 0, 3, tiles.len() as u8];
        bytes.extend(flags.bits().to_le_bytes());
        bytes.extend(number.to_le_bytes());
        bytes.extend([wall_orientation, 0xFE_u8, 1, 0, 0, 0]);
        for &(x, y, shape) in tiles {
            bytes.extend(0x7FFF_i16.to_le_bytes());
            bytes.extend([x as u8, y as u8]);
            bytes.extend([shape; PROFILE_X_SIZE * PROFILE_Y_SIZE]);
            bytes.extend([tile_flags.bits(), 2, 0]);
        }
        bytes
    }

    pub fn aux_bytes(wall_orientation: u8, number_of_tiles: u8, tile_loc_index: u16) -> Vec<u8> {
        let mut bytes = vec![wall_orientation, number_of_tiles];
        bytes.extend(tile_loc_index.to_le_bytes());
        bytes.extend([0, 0, 0, 1, 3, StciAppDataFlags::ANIMATED_TILE.bits()]);
        bytes.extend([0; 6]);
        assert_eq!(bytes.len(), STCI_APP_DATA_SIZE);
        bytes
    }

    /// Builds a JSD file
    pub fn jsd_bytes(
        number_of_structures: u16,
        aux: Option<(&[Vec<u8>], &[(i8, i8)])>,
        structures: &[Vec<u8>],
    ) -> Vec<u8> {
        let data: Vec<u8> = structures.concat();
        let mut flags = 0;
        if aux.is_some() {
            flags |= STRUCTURE_FILE_CONTAINS_AUXIMAGEDATA;
        }
        if !structures.is_empty() {
            flags |= STRUCTURE_FILE_CONTAINS_STRUCTUREDATA;
        }
        let tile_locs = aux.map_or(&[][..], |(_, locs)| locs);
        let mut bytes = b"J2SD".to_vec();
        bytes.extend(number_of_structures.to_le_bytes());
        bytes.extend((structures.len() as u16).to_le_bytes());
        bytes.extend((data.len() as u16).to_le_bytes());
        bytes.extend([flags, 0, 0, 0]);
        bytes.extend((tile_locs.len() as u16).to_le_bytes());
        if let Some((aux, locs)) = aux {
            bytes.extend(aux.concat());
            for &(x, y) in locs {
                bytes.extend([x as u8, y as u8]);
            }
        }
        bytes.extend(data);
        bytes
    }

    fn sub_image(offset_x: i16, width: u16) -> SubImageInfo {
        SubImageInfo {
            offset: (offset_x, 0),
            dimensions: (width, 50),
            app_data: None,
        }
    }

    #[test]
    fn record_sizes() {
        assert_eq!(
            structure_bytes(0, StructureFlags::empty(), 0, &[]).len(),
            DB_STRUCTURE_SIZE
        );
        assert_eq!(
            structure_bytes(0, StructureFlags::empty(), 0, &[(0, 0, 0)]).len(),
            DB_STRUCTURE_SIZE + DB_STRUCTURE_TILE_SIZE
        );
    }

    #[test]
    fn rejects_invalid_header() {
        let mut bytes = jsd_bytes(1, None, &[]);
        bytes[0] = b'X';
        assert!(Ja2StructureFile::parse(&bytes).is_err());
        assert!(Ja2StructureFile::parse(&jsd_bytes(0, None, &[])).is_err());
        assert!(Ja2StructureFile::parse(b"J2SD").is_err());
    }

    #[test]
    fn parses_aux_data_and_tile_locs() {
        let aux = [aux_bytes(1, 2, 0), aux_bytes(2, 0, 0)];
        let bytes = jsd_bytes(2, Some((&aux, &[(0, 0), (-1, 1)])), &[]);
        let file = Ja2StructureFile::parse(&bytes).unwrap();
        assert_eq!(file.number_of_structures, 2);
        assert_eq!(file.aux_data.len(), 2);
        assert_eq!(file.aux_data[0].wall_orientation, 1);
        assert_eq!(file.aux_data[0].number_of_tiles, 2);
        assert_eq!(file.aux_data[1].number_of_frames, 3);
        assert_eq!(file.tile_locs, vec![(0, 0), (-1, 1)]);
        assert!(!file.has_structure_data());
    }

    #[test]
    fn parses_structures() {
        let structures = [structure_bytes(
            1,
            StructureFlags::WALL,
            2,
            &[(0, 0, 0x0F), (1, 0, 0x01)],
        )];
        let bytes = jsd_bytes(3, None, &structures);
        let file = Ja2StructureFile::parse(&bytes).unwrap();
        assert_eq!(file.structures.len(), 3);
        assert!(file.structures[0].is_none());
        assert!(file.structures[2].is_none());
        let structure = file.structures[1].as_ref().unwrap();
        assert_eq!(structure.armour, 7);
        assert_eq!(structure.density, 3);
        assert_eq!(structure.flags, StructureFlags::WALL);
        assert_eq!(structure.structure_number, 1);
        assert_eq!(structure.wall_orientation, Orientation::INSIDE_RIGHT);
        assert_eq!(structure.destruction_partner, -2);
        assert_eq!(structure.partner_delta, 1);
        // 25 * 4 + 25 * 1 filled cubes
        assert_eq!(structure.hit_points, (125 * 100 / 255) as u8);
        assert_eq!(structure.tiles.len(), 2);
        assert_eq!(structure.tiles[0].pos_rel_to_base, 0);
        assert_eq!(structure.tiles[1].pos_rel_to_base, 1);
        assert_eq!(structure.tiles[1].shape[4][4], 0x01);
        assert_eq!(structure.tiles[1].flags, StructureTileFlags::PASSABLE);
        assert_eq!(structure.tiles[1].vehicle_hit_location, 2);
    }

    #[test]
    fn rejects_short_structure_data() {
        let mut structure = structure_bytes(0, StructureFlags::empty(), 0, &[(0, 0, 0)]);
        structure.truncate(DB_STRUCTURE_SIZE + 10);
        assert!(Ja2StructureFile::parse(&jsd_bytes(1, None, &[structure])).is_err());
        let structure = structure_bytes(5, StructureFlags::empty(), 0, &[]);
        assert!(Ja2StructureFile::parse(&jsd_bytes(1, None, &[structure])).is_err());
    }

    #[test]
    fn converts_orientation_with_aux_data() {
        let aux = [aux_bytes(3, 0, 0), aux_bytes(4, 0, 0)];
        let structures = [
            structure_bytes(0, StructureFlags::WALL, 3, &[(0, 0, 0)]),
            structure_bytes(1, StructureFlags::WALL, 4, &[(0, 0, 0)]),
        ];
        let file = Ja2StructureFile::parse(&jsd_bytes(2, Some((&aux, &[])), &structures)).unwrap();
        let orientation = |i: usize| file.structures[i].as_ref().unwrap().wall_orientation;
        assert_eq!(orientation(0), Orientation::OUTSIDE_LEFT);
        assert_eq!(orientation(1), Orientation::OUTSIDE_RIGHT);
        assert_eq!(file.aux_data[0].wall_orientation, 0x04);
        assert_eq!(file.aux_data[1].wall_orientation, 0x08);

        // Without aux data the values are not converted
        let file = Ja2StructureFile::parse(&jsd_bytes(2, None, &structures)).unwrap();
        assert_eq!(
            file.structures[0].as_ref().unwrap().wall_orientation.bits(),
            3
        );
    }

    #[test]
    fn normalizes_tiles_without_base() {
        let structures = [structure_bytes(
            0,
            StructureFlags::VEHICLE,
            0,
            &[(1, 1, 0), (2, 1, 0), (1, 2, 0)],
        )];
        let file = Ja2StructureFile::parse(&jsd_bytes(1, None, &structures)).unwrap();
        let tiles = &file.structures[0].as_ref().unwrap().tiles;
        let positions: Vec<_> = tiles
            .iter()
            .map(|t| (t.pos_rel_to_base, t.x_pos_rel_to_base, t.y_pos_rel_to_base))
            .collect();
        assert_eq!(positions, vec![(0, 0, 0), (1, 1, 0), (160, 0, 1)]);
    }

    #[test]
    fn z_strip_info() {
        let structures = [
            // Multi tile structure
            structure_bytes(0, StructureFlags::GENERIC, 0, &[(0, 0, 0), (0, 1, 0)]),
            // Single tile structure
            structure_bytes(1, StructureFlags::GENERIC, 0, &[(0, 0, 0)]),
            // Doors don't get z-strips
            structure_bytes(2, StructureFlags::DOOR, 0, &[(0, 0, 0), (1, 0, 0)]),
        ];
        let file = Ja2StructureFile::parse(&jsd_bytes(4, None, &structures)).unwrap();
        let sub_images = [
            sub_image(-30, 70),
            sub_image(-30, 70),
            sub_image(-30, 70),
            sub_image(-30, 70),
        ];
        let info = file.z_strip_info(&sub_images);
        assert_eq!(info.len(), 4);
        // 50 px left of the bottom corner (2 increasing, first strip 10 px wide), 20 px right
        assert_eq!(
            info[0],
            Some(ZStripInfo {
                initial_z_change: -2,
                first_z_strip_width: 10,
                z_changes: vec![1, 1, 0],
            })
        );
        assert_eq!(info[1], None);
        assert_eq!(info[2], None);
        assert_eq!(info[3], None);

        // Image only right of the bottom corner
        let info = file.z_strip_info(&[sub_image(30, 50)]);
        assert_eq!(
            info[0],
            Some(ZStripInfo {
                initial_z_change: 0,
                first_z_strip_width: 10,
                z_changes: vec![0, -1, -1],
            })
        );
    }

    #[test]
    fn no_z_strip_info_for_single_tile_files() {
        let structures = [structure_bytes(
            0,
            StructureFlags::GENERIC,
            0,
            &[(0, 0, 0), (0, 0, 0)],
        )];
        let file = Ja2StructureFile::parse(&jsd_bytes(1, None, &structures)).unwrap();
        assert_eq!(file.z_strip_info(&[sub_image(-30, 70)]), vec![None]);
    }
}
