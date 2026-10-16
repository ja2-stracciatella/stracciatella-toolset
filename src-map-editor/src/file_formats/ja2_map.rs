use std::io;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use bitflags::bitflags;
use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use super::{StracciatellaReadExt, StracciatellaWriteExt};

pub const JA2_MAP_ROWS: usize = 160;
pub const JA2_MAP_COLS: usize = 160;
const JA2_TILE_COUNT: usize = JA2_MAP_ROWS * JA2_MAP_COLS;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Ja2MapFlags: u32 {
        const MAP_FULLSOLDIER_SAVED       = 0x00000001;
        const MAP_WORLDLIGHTS_SAVED       = 0x00000004;
        const MAP_WORLDITEMS_SAVED        = 0x00000008;
        const MAP_EXITGRIDS_SAVED	      = 0x00000010;
        const MAP_DOORTABLE_SAVED	      = 0x00000020;
        const MAP_EDGEPOINTS_SAVED	      = 0x00000040;
        const MAP_AMBIENTLIGHTLEVEL_SAVED = 0x00000080;
        const MAP_NPCSCHEDULES_SAVED      = 0x00000100;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Ja2MapElementFlags: u8 {
        const MAPELEMENT_REDUNDANT             = 0x0001;
        const MAPELEMENT_REEVALUATE_REDUNDANCY = 0x0002;
        const MAPELEMENT_ENEMY_MINE_PRESENT    = 0x0004;
        const MAPELEMENT_PLAYER_MINE_PRESENT   = 0x0008;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2MapVersion {
    pub major: f32,
    pub minor: u8,
}

/// One entry of a tile layer (land, objects, structs, shadows, roofs, on roofs)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2TileLayer {
    pub kind: u8,
    pub sub_index: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2MapElement {
    pub flags: Ja2MapElementFlags,
    pub height: u8,
    pub land: Vec<Ja2TileLayer>,
    pub objects: Vec<Ja2TileLayer>,
    pub structs: Vec<Ja2TileLayer>,
    pub shadows: Vec<Ja2TileLayer>,
    pub roofs: Vec<Ja2TileLayer>,
    pub on_roofs: Vec<Ja2TileLayer>,
    pub room: u8,
}

/// OBJECTTYPE, an item stack as stored in world items and soldier inventories
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2Object {
    pub item: u16,
    pub number: u8,
    /// Item class specific data (gun, ammo, money, bomb, key, owner), the meaning depends on the
    /// item class which is not part of the map file
    pub data: [u8; 12],
    pub attach_items: [u16; 4],
    pub attach_status: [i8; 4],
    pub flags: i8,
    pub mission: u8,
    pub trap: i8,
    pub imprint_id: u8,
    pub weight: u8,
    pub used: u8,
}

/// WORLDITEM, an item placed in the world
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2WorldItem {
    pub exists: bool,
    pub grid_no: i16,
    pub level: u8,
    pub object: Ja2Object,
    /// `WorldItemFlags`
    pub flags: u16,
    pub render_z_height_above_level: i8,
    pub visible: i8,
    /// Chance of the item not existing (reversed)
    pub non_exist_chance: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2AmbientLight {
    pub basement: bool,
    pub caves: bool,
    pub level: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2LightSprite {
    pub x: i16,
    pub y: i16,
    pub flags: u32,
    pub template_name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2Lights {
    /// Light colors as r, g, b, unused
    pub colors: Vec<[u8; 4]>,
    pub sprites: Vec<Ja2LightSprite>,
}

/// MAPCREATE_STRUCT
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2MapInfo {
    pub north_grid_no: i16,
    pub east_grid_no: i16,
    pub south_grid_no: i16,
    pub west_grid_no: i16,
    pub num_individuals: u8,
    pub map_version: u8,
    pub restricted_scroll_id: u8,
    pub editor_smoothing_type: u8,
    pub center_grid_no: i16,
    pub isolated_grid_no: i16,
}

/// BASIC_SOLDIERCREATE_STRUCT
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2BasicSoldierPlacement {
    pub detailed_placement: bool,
    pub starting_grid_no: u16,
    pub team: i8,
    pub relative_attribute_level: i8,
    pub relative_equipment_level: i8,
    pub direction: i8,
    pub orders: i8,
    pub attitude: i8,
    pub body_type: i8,
    pub patrol_grid: [i16; 10],
    pub patrol_cnt: i8,
    pub on_roof: bool,
    pub soldier_class: u8,
    pub civilian_group: u8,
    pub priority_existance: bool,
    pub has_keys: bool,
}

/// SOLDIERCREATE_STRUCT
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2DetailedSoldierPlacement {
    pub is_static: bool,
    pub profile: u8,
    pub copy_profile_items_over: bool,
    pub sector_x: i16,
    pub sector_y: i16,
    pub direction: i8,
    pub insertion_grid_no: i16,
    pub team: i8,
    pub body_type: i8,
    pub attitude: i8,
    pub orders: i8,
    pub life_max: i8,
    pub life: i8,
    pub agility: i8,
    pub dexterity: i8,
    pub exp_level: i8,
    pub marksmanship: i8,
    pub medical: i8,
    pub mechanical: i8,
    pub explosive: i8,
    pub leadership: i8,
    pub strength: i8,
    pub wisdom: i8,
    pub morale: i8,
    pub ai_morale: i8,
    pub inventory: Vec<Ja2Object>,
    pub head_pal: String,
    pub pants_pal: String,
    pub vest_pal: String,
    pub skin_pal: String,
    pub patrol_grid: [i16; 10],
    pub patrol_cnt: i8,
    pub visible: bool,
    pub name: String,
    pub soldier_class: u8,
    pub on_roof: bool,
    pub sector_z: i8,
    pub civilian_group: u8,
    pub schedule_id: u8,
    pub use_given_vehicle: bool,
    pub use_given_vehicle_id: i8,
    pub has_keys: bool,
}

/// A soldier placement as stored in the map, see `map::placement` for the runtime form
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2SoldierPlacement {
    pub basic: Ja2BasicSoldierPlacement,
    pub detailed: Option<Ja2DetailedSoldierPlacement>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2ExitGrid {
    pub grid_no: u16,
    pub dest_grid_no: u16,
    /// Destination sector as x, y, z
    pub dest_sector: (u8, u8, u8),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2Door {
    pub grid_no: i16,
    pub locked: bool,
    pub trap_level: u8,
    pub trap_id: u8,
    pub lock_id: u8,
    pub perceived_locked: i8,
    pub perceived_trapped: i8,
    pub lock_damage: i8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2EdgePointList {
    pub middle_index: u16,
    pub grid_nos: Vec<i16>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2EdgePointSet {
    pub north: Ja2EdgePointList,
    pub east: Ja2EdgePointList,
    pub south: Ja2EdgePointList,
    pub west: Ja2EdgePointList,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2EdgePoints {
    pub primary: Ja2EdgePointSet,
    /// Only present in maps with map version >= 17
    pub secondary: Option<Ja2EdgePointSet>,
}

/// SCHEDULENODE
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ja2Schedule {
    pub time: [u16; 4],
    pub data1: [u16; 4],
    pub data2: [u16; 4],
    pub action: [u8; 4],
    pub schedule_id: u8,
    pub soldier_id: u8,
    pub flags: u16,
}

#[derive(Asset, TypePath, Clone, Debug, Default, PartialEq)]
pub struct Ja2Map {
    pub version: Ja2MapVersion,
    pub flags: Ja2MapFlags,
    pub tileset_id: i32,
    pub soldier_size: u32,
    pub map_elements: Vec<Ja2MapElement>,
    /// Unknown data present in russian 6.00/26 maps
    pub russian_unknown: Option<[u8; 148]>,
    pub items: Vec<Ja2WorldItem>,
    pub ambient_light: Option<Ja2AmbientLight>,
    pub lights: Option<Ja2Lights>,
    pub info: Ja2MapInfo,
    pub placements: Vec<Ja2SoldierPlacement>,
    pub exit_grids: Vec<Ja2ExitGrid>,
    pub doors: Vec<Ja2Door>,
    pub edge_points: Option<Ja2EdgePoints>,
    pub schedules: Vec<Ja2Schedule>,
}

#[derive(Default, TypePath)]
pub struct Ja2MapLoader;

impl AssetLoader for Ja2MapLoader {
    type Asset = Ja2Map;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Ja2Map, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ja2Map::parse(&bytes)
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn read_bool(r: &mut dyn io::Read) -> io::Result<bool> {
    Ok(r.read_u8()? != 0)
}

fn read_bytes<const N: usize>(r: &mut dyn io::Read) -> io::Result<[u8; N]> {
    let mut buffer = [0; N];
    r.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn read_i8s<const N: usize>(r: &mut dyn io::Read) -> io::Result<[i8; N]> {
    let mut buffer = [0; N];
    r.read_i8_into(&mut buffer)?;
    Ok(buffer)
}

fn read_u16s<const N: usize>(r: &mut dyn io::Read) -> io::Result<[u16; N]> {
    let mut buffer = [0; N];
    r.read_u16_into::<LE>(&mut buffer)?;
    Ok(buffer)
}

fn read_i16s<const N: usize>(r: &mut dyn io::Read) -> io::Result<[i16; N]> {
    let mut buffer = [0; N];
    r.read_i16_into::<LE>(&mut buffer)?;
    Ok(buffer)
}

/// Reads a fixed size string that is nul terminated unless it fills the whole buffer.
/// Invalid utf8 is replaced.
fn read_lossy_string(r: &mut dyn io::Read, num_bytes: usize) -> io::Result<String> {
    let mut buffer = vec![0u8; num_bytes];
    r.read_exact(&mut buffer)?;
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(num_bytes);
    Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
}

/// Reads a fixed size UTF-16LE string that is nul terminated unless it fills the whole buffer
fn read_utf16_string(r: &mut dyn io::Read, num_units: usize) -> io::Result<String> {
    let mut buffer = vec![0u16; num_units];
    r.read_u16_into::<LE>(&mut buffer)?;
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(num_units);
    Ok(String::from_utf16_lossy(&buffer[..end]))
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// A count of `what` that must fit into the field it is saved in
fn count<T: TryFrom<usize>>(len: usize, what: &str) -> io::Result<T> {
    T::try_from(len).map_err(|_| invalid_input(format!("too many {} ({})", what, len)))
}

/// A part of the map that the map flags say is saved
fn saved<'a, T>(value: &'a Option<T>, what: &str) -> io::Result<&'a T> {
    value.as_ref().ok_or_else(|| {
        invalid_input(format!(
            "the map flags say {} are saved, but there are none",
            what
        ))
    })
}

fn write_i8s(w: &mut dyn io::Write, values: &[i8]) -> io::Result<()> {
    values.iter().try_for_each(|&value| w.write_i8(value))
}

fn write_u16s(w: &mut dyn io::Write, values: &[u16]) -> io::Result<()> {
    values
        .iter()
        .try_for_each(|&value| w.write_u16::<LE>(value))
}

fn write_i16s(w: &mut dyn io::Write, values: &[i16]) -> io::Result<()> {
    values
        .iter()
        .try_for_each(|&value| w.write_i16::<LE>(value))
}

/// Writes a fixed size string like [`read_lossy_string`] reads it, it may fill the whole buffer
fn write_lossy_string(w: &mut dyn io::Write, num_bytes: usize, string: &str) -> io::Result<()> {
    let bytes = string.as_bytes();
    if bytes.len() > num_bytes {
        return Err(invalid_input(format!(
            "\"{}\" is longer than {} bytes",
            string, num_bytes
        )));
    }
    w.write_all(bytes)?;
    w.write_unused(num_bytes - bytes.len())
}

/// Writes a fixed size UTF-16LE string like [`read_utf16_string`] reads it
fn write_utf16_string(w: &mut dyn io::Write, num_units: usize, string: &str) -> io::Result<()> {
    let units: Vec<u16> = string.encode_utf16().collect();
    if units.len() > num_units {
        return Err(invalid_input(format!(
            "\"{}\" is longer than {} characters",
            string, num_units
        )));
    }
    write_u16s(w, &units)?;
    w.write_unused(2 * (num_units - units.len()))
}

impl Ja2Object {
    /// Number of inventory slots in a detailed soldier placement
    pub const NUM_INV_SLOTS: usize = 19;

    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let item = r.read_u16::<LE>()?;
        let number = r.read_u8()?;
        r.read_unused(1)?;
        let data = read_bytes(r)?;
        let attach_items = read_u16s(r)?;
        let attach_status = read_i8s(r)?;
        let flags = r.read_i8()?;
        let mission = r.read_u8()?;
        let trap = r.read_i8()?;
        let imprint_id = r.read_u8()?;
        let weight = r.read_u8()?;
        let used = r.read_u8()?;
        r.read_unused(2)?;
        Ok(Self {
            item,
            number,
            data,
            attach_items,
            attach_status,
            flags,
            mission,
            trap,
            imprint_id,
            weight,
            used,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u16::<LE>(self.item)?;
        w.write_u8(self.number)?;
        w.write_unused(1)?;
        w.write_all(&self.data)?;
        write_u16s(w, &self.attach_items)?;
        write_i8s(w, &self.attach_status)?;
        w.write_i8(self.flags)?;
        w.write_u8(self.mission)?;
        w.write_i8(self.trap)?;
        w.write_u8(self.imprint_id)?;
        w.write_u8(self.weight)?;
        w.write_u8(self.used)?;
        w.write_unused(2)
    }
}

impl Ja2WorldItem {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let exists = read_bool(r)?;
        r.read_unused(1)?;
        let grid_no = r.read_i16::<LE>()?;
        let level = r.read_u8()?;
        r.read_unused(3)?;
        let object = Ja2Object::read(r)?;
        let flags = r.read_u16::<LE>()?;
        let render_z_height_above_level = r.read_i8()?;
        let visible = r.read_i8()?;
        let non_exist_chance = r.read_u8()?;
        r.read_unused(3)?;
        Ok(Self {
            exists,
            grid_no,
            level,
            object,
            flags,
            render_z_height_above_level,
            visible,
            non_exist_chance,
        })
    }

    fn read_all(r: &mut dyn io::Read) -> io::Result<Vec<Self>> {
        let count = r.read_u32::<LE>()?;
        (0..count).map(|_| Self::read(r)).collect()
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(self.exists.into())?;
        w.write_unused(1)?;
        w.write_i16::<LE>(self.grid_no)?;
        w.write_u8(self.level)?;
        w.write_unused(3)?;
        self.object.write(w)?;
        w.write_u16::<LE>(self.flags)?;
        w.write_i8(self.render_z_height_above_level)?;
        w.write_i8(self.visible)?;
        w.write_u8(self.non_exist_chance)?;
        w.write_unused(3)
    }

    fn write_all(items: &[Self], w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u32::<LE>(count(items.len(), "world items")?)?;
        items.iter().try_for_each(|item| item.write(w))
    }
}

impl Ja2AmbientLight {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        Ok(Self {
            basement: read_bool(r)?,
            caves: read_bool(r)?,
            level: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(self.basement.into())?;
        w.write_u8(self.caves.into())?;
        w.write_u8(self.level)
    }
}

impl Ja2LightSprite {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let x = r.read_i16::<LE>()?;
        let y = r.read_i16::<LE>()?;
        r.read_unused(12)?;
        let flags = r.read_u32::<LE>()?;
        r.read_unused(4)?;
        let name_len = r.read_u8()?;
        if name_len == 0 {
            return Err(invalid_data("light sprite template name is empty"));
        }
        let template_name = read_lossy_string(r, name_len.into())?;
        Ok(Self {
            x,
            y,
            flags,
            template_name,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_i16::<LE>(self.x)?;
        w.write_i16::<LE>(self.y)?;
        w.write_unused(12)?;
        w.write_u32::<LE>(self.flags)?;
        w.write_unused(4)?;
        // Like the game, the length includes the nul terminator
        let name_len = self.template_name.len() + 1;
        let name_len: u8 = count(name_len, "bytes in a light template name")?;
        w.write_u8(name_len)?;
        write_lossy_string(w, name_len.into(), &self.template_name)
    }
}

impl Ja2Lights {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let num_colors = r.read_u8()?;
        let colors = (0..num_colors)
            .map(|_| read_bytes(r))
            .collect::<io::Result<_>>()?;
        let num_lights = r.read_u16::<LE>()?;
        let sprites = (0..num_lights)
            .map(|_| Ja2LightSprite::read(r))
            .collect::<io::Result<_>>()?;
        Ok(Self { colors, sprites })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(count(self.colors.len(), "light colors")?)?;
        self.colors
            .iter()
            .try_for_each(|color| w.write_all(color))?;
        w.write_u16::<LE>(count(self.sprites.len(), "lights")?)?;
        self.sprites.iter().try_for_each(|sprite| sprite.write(w))
    }
}

impl Ja2MapInfo {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let info = Self {
            north_grid_no: r.read_i16::<LE>()?,
            east_grid_no: r.read_i16::<LE>()?,
            south_grid_no: r.read_i16::<LE>()?,
            west_grid_no: r.read_i16::<LE>()?,
            num_individuals: r.read_u8()?,
            map_version: r.read_u8()?,
            restricted_scroll_id: r.read_u8()?,
            editor_smoothing_type: r.read_u8()?,
            center_grid_no: r.read_i16::<LE>()?,
            isolated_grid_no: r.read_i16::<LE>()?,
        };
        // 83 padding bytes + 1 alignment byte, the struct is 100 bytes in total
        r.read_unused(84)?;
        Ok(info)
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_i16::<LE>(self.north_grid_no)?;
        w.write_i16::<LE>(self.east_grid_no)?;
        w.write_i16::<LE>(self.south_grid_no)?;
        w.write_i16::<LE>(self.west_grid_no)?;
        w.write_u8(self.num_individuals)?;
        w.write_u8(self.map_version)?;
        w.write_u8(self.restricted_scroll_id)?;
        w.write_u8(self.editor_smoothing_type)?;
        w.write_i16::<LE>(self.center_grid_no)?;
        w.write_i16::<LE>(self.isolated_grid_no)?;
        w.write_unused(84)
    }
}

impl Ja2BasicSoldierPlacement {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let detailed_placement = read_bool(r)?;
        r.read_unused(1)?;
        let starting_grid_no = r.read_u16::<LE>()?;
        let team = r.read_i8()?;
        let relative_attribute_level = r.read_i8()?;
        let relative_equipment_level = r.read_i8()?;
        let direction = r.read_i8()?;
        let orders = r.read_i8()?;
        let attitude = r.read_i8()?;
        let body_type = r.read_i8()?;
        r.read_unused(1)?;
        let patrol_grid = read_i16s(r)?;
        let patrol_cnt = r.read_i8()?;
        let on_roof = read_bool(r)?;
        let soldier_class = r.read_u8()?;
        let civilian_group = r.read_u8()?;
        let priority_existance = read_bool(r)?;
        let has_keys = read_bool(r)?;
        r.read_unused(14)?;
        Ok(Self {
            detailed_placement,
            starting_grid_no,
            team,
            relative_attribute_level,
            relative_equipment_level,
            direction,
            orders,
            attitude,
            body_type,
            patrol_grid,
            patrol_cnt,
            on_roof,
            soldier_class,
            civilian_group,
            priority_existance,
            has_keys,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(self.detailed_placement.into())?;
        w.write_unused(1)?;
        w.write_u16::<LE>(self.starting_grid_no)?;
        w.write_i8(self.team)?;
        w.write_i8(self.relative_attribute_level)?;
        w.write_i8(self.relative_equipment_level)?;
        w.write_i8(self.direction)?;
        w.write_i8(self.orders)?;
        w.write_i8(self.attitude)?;
        w.write_i8(self.body_type)?;
        w.write_unused(1)?;
        write_i16s(w, &self.patrol_grid)?;
        w.write_i8(self.patrol_cnt)?;
        w.write_u8(self.on_roof.into())?;
        w.write_u8(self.soldier_class)?;
        w.write_u8(self.civilian_group)?;
        w.write_u8(self.priority_existance.into())?;
        w.write_u8(self.has_keys.into())?;
        w.write_unused(14)
    }
}

impl Ja2DetailedSoldierPlacement {
    const PALETTE_LENGTH: usize = 30;
    const NAME_LENGTH: usize = 10;

    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let is_static = read_bool(r)?;
        let profile = r.read_u8()?;
        r.read_unused(2)?;
        let copy_profile_items_over = read_bool(r)?;
        r.read_unused(1)?;
        let sector_x = r.read_i16::<LE>()?;
        let sector_y = r.read_i16::<LE>()?;
        let direction = r.read_i8()?;
        r.read_unused(1)?;
        let insertion_grid_no = r.read_i16::<LE>()?;
        let team = r.read_i8()?;
        let body_type = r.read_i8()?;
        let attitude = r.read_i8()?;
        let orders = r.read_i8()?;
        let life_max = r.read_i8()?;
        let life = r.read_i8()?;
        let agility = r.read_i8()?;
        let dexterity = r.read_i8()?;
        let exp_level = r.read_i8()?;
        let marksmanship = r.read_i8()?;
        let medical = r.read_i8()?;
        let mechanical = r.read_i8()?;
        let explosive = r.read_i8()?;
        let leadership = r.read_i8()?;
        let strength = r.read_i8()?;
        let wisdom = r.read_i8()?;
        let morale = r.read_i8()?;
        let ai_morale = r.read_i8()?;
        let inventory = (0..Ja2Object::NUM_INV_SLOTS)
            .map(|_| Ja2Object::read(r))
            .collect::<io::Result<_>>()?;
        let head_pal = read_lossy_string(r, Self::PALETTE_LENGTH)?;
        let pants_pal = read_lossy_string(r, Self::PALETTE_LENGTH)?;
        let vest_pal = read_lossy_string(r, Self::PALETTE_LENGTH)?;
        let skin_pal = read_lossy_string(r, Self::PALETTE_LENGTH)?;
        r.read_unused(30)?;
        let patrol_grid = read_i16s(r)?;
        let patrol_cnt = r.read_i8()?;
        let visible = read_bool(r)?;
        let name = read_utf16_string(r, Self::NAME_LENGTH)?;
        let soldier_class = r.read_u8()?;
        let on_roof = read_bool(r)?;
        let sector_z = r.read_i8()?;
        r.read_unused(6)?;
        let civilian_group = r.read_u8()?;
        r.read_unused(1)?;
        let schedule_id = r.read_u8()?;
        let use_given_vehicle = read_bool(r)?;
        let use_given_vehicle_id = r.read_i8()?;
        let has_keys = read_bool(r)?;
        r.read_unused(117)?;
        Ok(Self {
            is_static,
            profile,
            copy_profile_items_over,
            sector_x,
            sector_y,
            direction,
            insertion_grid_no,
            team,
            body_type,
            attitude,
            orders,
            life_max,
            life,
            agility,
            dexterity,
            exp_level,
            marksmanship,
            medical,
            mechanical,
            explosive,
            leadership,
            strength,
            wisdom,
            morale,
            ai_morale,
            inventory,
            head_pal,
            pants_pal,
            vest_pal,
            skin_pal,
            patrol_grid,
            patrol_cnt,
            visible,
            name,
            soldier_class,
            on_roof,
            sector_z,
            civilian_group,
            schedule_id,
            use_given_vehicle,
            use_given_vehicle_id,
            has_keys,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        if self.inventory.len() != Ja2Object::NUM_INV_SLOTS {
            return Err(invalid_input(format!(
                "a detailed placement has {} inventory slots instead of {}",
                self.inventory.len(),
                Ja2Object::NUM_INV_SLOTS
            )));
        }
        w.write_u8(self.is_static.into())?;
        w.write_u8(self.profile)?;
        w.write_unused(2)?;
        w.write_u8(self.copy_profile_items_over.into())?;
        w.write_unused(1)?;
        w.write_i16::<LE>(self.sector_x)?;
        w.write_i16::<LE>(self.sector_y)?;
        w.write_i8(self.direction)?;
        w.write_unused(1)?;
        w.write_i16::<LE>(self.insertion_grid_no)?;
        write_i8s(
            w,
            &[
                self.team,
                self.body_type,
                self.attitude,
                self.orders,
                self.life_max,
                self.life,
                self.agility,
                self.dexterity,
                self.exp_level,
                self.marksmanship,
                self.medical,
                self.mechanical,
                self.explosive,
                self.leadership,
                self.strength,
                self.wisdom,
                self.morale,
                self.ai_morale,
            ],
        )?;
        self.inventory
            .iter()
            .try_for_each(|object| object.write(w))?;
        write_lossy_string(w, Self::PALETTE_LENGTH, &self.head_pal)?;
        write_lossy_string(w, Self::PALETTE_LENGTH, &self.pants_pal)?;
        write_lossy_string(w, Self::PALETTE_LENGTH, &self.vest_pal)?;
        write_lossy_string(w, Self::PALETTE_LENGTH, &self.skin_pal)?;
        w.write_unused(30)?;
        write_i16s(w, &self.patrol_grid)?;
        w.write_i8(self.patrol_cnt)?;
        w.write_u8(self.visible.into())?;
        write_utf16_string(w, Self::NAME_LENGTH, &self.name)?;
        w.write_u8(self.soldier_class)?;
        w.write_u8(self.on_roof.into())?;
        w.write_i8(self.sector_z)?;
        w.write_unused(6)?;
        w.write_u8(self.civilian_group)?;
        w.write_unused(1)?;
        w.write_u8(self.schedule_id)?;
        w.write_u8(self.use_given_vehicle.into())?;
        w.write_i8(self.use_given_vehicle_id)?;
        w.write_u8(self.has_keys.into())?;
        w.write_unused(117)
    }
}

impl Ja2SoldierPlacement {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let basic = Ja2BasicSoldierPlacement::read(r)?;
        let detailed = if basic.detailed_placement {
            Some(Ja2DetailedSoldierPlacement::read(r)?)
        } else {
            None
        };
        Ok(Self { basic, detailed })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        if self.basic.detailed_placement != self.detailed.is_some() {
            return Err(invalid_input(
                "the detailed placement flag of a placement does not match its data",
            ));
        }
        self.basic.write(w)?;
        match &self.detailed {
            Some(detailed) => detailed.write(w),
            None => Ok(()),
        }
    }
}

impl Ja2ExitGrid {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        Ok(Self {
            grid_no: r.read_u16::<LE>()?,
            dest_grid_no: r.read_u16::<LE>()?,
            dest_sector: (r.read_u8()?, r.read_u8()?, r.read_u8()?),
        })
    }

    fn read_all(r: &mut dyn io::Read) -> io::Result<Vec<Self>> {
        let count = r.read_u16::<LE>()?;
        (0..count).map(|_| Self::read(r)).collect()
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u16::<LE>(self.grid_no)?;
        w.write_u16::<LE>(self.dest_grid_no)?;
        let (x, y, z) = self.dest_sector;
        w.write_all(&[x, y, z])
    }

    fn write_all(exit_grids: &[Self], w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u16::<LE>(count(exit_grids.len(), "exit grids")?)?;
        exit_grids
            .iter()
            .try_for_each(|exit_grid| exit_grid.write(w))
    }
}

impl Ja2Door {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let door = Self {
            grid_no: r.read_i16::<LE>()?,
            locked: read_bool(r)?,
            trap_level: r.read_u8()?,
            trap_id: r.read_u8()?,
            lock_id: r.read_u8()?,
            perceived_locked: r.read_i8()?,
            perceived_trapped: r.read_i8()?,
            lock_damage: r.read_i8()?,
        };
        r.read_unused(5)?;
        Ok(door)
    }

    fn read_all(r: &mut dyn io::Read) -> io::Result<Vec<Self>> {
        let count = r.read_u8()?;
        (0..count).map(|_| Self::read(r)).collect()
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_i16::<LE>(self.grid_no)?;
        w.write_u8(self.locked.into())?;
        w.write_u8(self.trap_level)?;
        w.write_u8(self.trap_id)?;
        w.write_u8(self.lock_id)?;
        w.write_i8(self.perceived_locked)?;
        w.write_i8(self.perceived_trapped)?;
        w.write_i8(self.lock_damage)?;
        w.write_unused(5)
    }

    fn write_all(doors: &[Self], w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(count(doors.len(), "doors")?)?;
        doors.iter().try_for_each(|door| door.write(w))
    }
}

impl Ja2EdgePointList {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        let count = r.read_u16::<LE>()?;
        let middle_index = r.read_u16::<LE>()?;
        let mut grid_nos = vec![0; count.into()];
        r.read_i16_into::<LE>(&mut grid_nos)?;
        Ok(Self {
            middle_index,
            grid_nos,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u16::<LE>(count(self.grid_nos.len(), "edge points")?)?;
        w.write_u16::<LE>(self.middle_index)?;
        write_i16s(w, &self.grid_nos)
    }
}

impl Ja2EdgePointSet {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        Ok(Self {
            north: Ja2EdgePointList::read(r)?,
            east: Ja2EdgePointList::read(r)?,
            south: Ja2EdgePointList::read(r)?,
            west: Ja2EdgePointList::read(r)?,
        })
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        self.north.write(w)?;
        self.east.write(w)?;
        self.south.write(w)?;
        self.west.write(w)
    }
}

impl Ja2EdgePoints {
    fn read(info: &Ja2MapInfo, r: &mut dyn io::Read) -> io::Result<Self> {
        let primary = Ja2EdgePointSet::read(r)?;
        let secondary = if info.map_version >= 17 {
            Some(Ja2EdgePointSet::read(r)?)
        } else {
            None
        };
        Ok(Self { primary, secondary })
    }

    fn write(&self, info: &Ja2MapInfo, w: &mut dyn io::Write) -> io::Result<()> {
        self.primary.write(w)?;
        match (&self.secondary, info.map_version >= 17) {
            (Some(secondary), true) => secondary.write(w),
            (None, false) => Ok(()),
            _ => Err(invalid_input(
                "secondary edge points must exist exactly in maps with map version >= 17",
            )),
        }
    }
}

impl Ja2Schedule {
    fn read(r: &mut dyn io::Read) -> io::Result<Self> {
        r.read_unused(4)?;
        Ok(Self {
            time: read_u16s(r)?,
            data1: read_u16s(r)?,
            data2: read_u16s(r)?,
            action: read_bytes(r)?,
            schedule_id: r.read_u8()?,
            soldier_id: r.read_u8()?,
            flags: r.read_u16::<LE>()?,
        })
    }

    fn read_all(r: &mut dyn io::Read) -> io::Result<Vec<Self>> {
        let count = r.read_u8()?;
        (0..count).map(|_| Self::read(r)).collect()
    }

    fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_unused(4)?;
        write_u16s(w, &self.time)?;
        write_u16s(w, &self.data1)?;
        write_u16s(w, &self.data2)?;
        w.write_all(&self.action)?;
        w.write_u8(self.schedule_id)?;
        w.write_u8(self.soldier_id)?;
        w.write_u16::<LE>(self.flags)
    }

    fn write_all(schedules: &[Self], w: &mut dyn io::Write) -> io::Result<()> {
        w.write_u8(count(schedules.len(), "schedules")?)?;
        schedules.iter().try_for_each(|schedule| schedule.write(w))
    }
}

/// Indices into the per tile layer counts
const LAND: usize = 0;
const OBJECTS: usize = 1;
const STRUCTS: usize = 2;
const SHADOWS: usize = 3;
const ROOFS: usize = 4;
const ON_ROOFS: usize = 5;

impl Ja2MapElement {
    /// The tile layer with the given index
    fn layer(&self, layer: usize) -> &[Ja2TileLayer] {
        match layer {
            LAND => &self.land,
            OBJECTS => &self.objects,
            STRUCTS => &self.structs,
            SHADOWS => &self.shadows,
            ROOFS => &self.roofs,
            _ => &self.on_roofs,
        }
    }
}

impl Ja2Map {
    fn read_heights(map_elements: &mut [Ja2MapElement], r: &mut dyn io::Read) -> io::Result<()> {
        for element in map_elements.iter_mut() {
            // Second byte is unused filler
            let [height, _] = read_bytes(r)?;
            element.height = height;
        }
        Ok(())
    }

    fn read_counts(
        map_elements: &mut [Ja2MapElement],
        r: &mut dyn io::Read,
    ) -> io::Result<Vec<[u8; 6]>> {
        let mut counts = Vec::with_capacity(map_elements.len());
        for element in map_elements.iter_mut() {
            let count: [u8; 4] = read_bytes(r)?;
            element.flags = Ja2MapElementFlags::from_bits(count[0] >> 4)
                .ok_or_else(|| invalid_data("unknown map element bitflags"))?;
            counts.push([
                count[0] & 0x0F,
                count[1] & 0x0F,
                count[1] >> 4,
                count[2] & 0x0F,
                count[2] >> 4,
                count[3] & 0x0F,
            ]);
        }
        Ok(counts)
    }

    /// Reads one tile layer for all map elements. `wide_sub_index` is used for
    /// objects in newer maps that store the sub index as u16.
    fn read_layer(
        map_elements: &mut [Ja2MapElement],
        counts: &[[u8; 6]],
        layer: usize,
        wide_sub_index: bool,
        r: &mut dyn io::Read,
    ) -> io::Result<()> {
        for (element, count) in map_elements.iter_mut().zip(counts) {
            let tiles = (0..count[layer])
                .map(|_| {
                    let kind = r.read_u8()?;
                    let sub_index = if wide_sub_index {
                        r.read_u16::<LE>()?
                    } else {
                        r.read_u8()?.into()
                    };
                    Ok(Ja2TileLayer { kind, sub_index })
                })
                .collect::<io::Result<Vec<_>>>()?;
            let target = match layer {
                LAND => &mut element.land,
                OBJECTS => &mut element.objects,
                STRUCTS => &mut element.structs,
                SHADOWS => &mut element.shadows,
                ROOFS => &mut element.roofs,
                _ => &mut element.on_roofs,
            };
            *target = tiles;
        }
        Ok(())
    }

    fn read_rooms(map_elements: &mut [Ja2MapElement], r: &mut dyn io::Read) -> io::Result<()> {
        let mut rooms = vec![0; map_elements.len()];
        r.read_exact(&mut rooms)?;
        for (element, room) in map_elements.iter_mut().zip(rooms) {
            element.room = room;
        }
        Ok(())
    }

    fn write_heights(map_elements: &[Ja2MapElement], w: &mut dyn io::Write) -> io::Result<()> {
        for element in map_elements {
            // Second byte is unused filler
            w.write_all(&[element.height, 0])?;
        }
        Ok(())
    }

    fn write_counts(map_elements: &[Ja2MapElement], w: &mut dyn io::Write) -> io::Result<()> {
        for element in map_elements {
            let mut counts = [0u8; 6];
            for (layer, count) in counts.iter_mut().enumerate() {
                let len = element.layer(layer).len();
                *count = u8::try_from(len)
                    .ok()
                    .filter(|&count| count <= 0x0F)
                    .ok_or_else(|| {
                        invalid_input(format!("a tile has {} tiles in one layer, max is 15", len))
                    })?;
            }
            w.write_all(&[
                counts[LAND] | element.flags.bits() << 4,
                counts[OBJECTS] | counts[STRUCTS] << 4,
                counts[SHADOWS] | counts[ROOFS] << 4,
                counts[ON_ROOFS],
            ])?;
        }
        Ok(())
    }

    /// Writes one tile layer for all map elements, like [`Ja2Map::read_layer`] reads it
    fn write_layer(
        map_elements: &[Ja2MapElement],
        layer: usize,
        wide_sub_index: bool,
        w: &mut dyn io::Write,
    ) -> io::Result<()> {
        for element in map_elements {
            for tile in element.layer(layer) {
                w.write_u8(tile.kind)?;
                if wide_sub_index {
                    w.write_u16::<LE>(tile.sub_index)?;
                } else {
                    let sub_index = u8::try_from(tile.sub_index).map_err(|_| {
                        invalid_input(format!(
                            "sub index {} does not fit into the map version",
                            tile.sub_index
                        ))
                    })?;
                    w.write_u8(sub_index)?;
                }
            }
        }
        Ok(())
    }
}

impl Ja2Map {
    pub fn parse(mut bytes: &[u8]) -> io::Result<Ja2Map> {
        let r: &mut dyn io::Read = &mut bytes;

        let version = {
            let major = r.read_f32::<LE>()?;
            let minor = if major >= 4.0 { r.read_u8()? } else { 0 };
            Ja2MapVersion { major, minor }
        };
        if version.major > 6. || version.minor > 26 {
            return Err(invalid_data(format!(
                "map version too high, was {}.{}, max is {}.{}",
                version.major, version.minor, 6., 26,
            )));
        }

        let flags = Ja2MapFlags::from_bits(r.read_u32::<LE>()?)
            .ok_or_else(|| invalid_data("unknown map bitflags"))?;
        let tileset_id = r.read_i32::<LE>()?;
        let soldier_size = r.read_u32::<LE>()?;

        let mut map_elements = vec![Ja2MapElement::default(); JA2_TILE_COUNT];
        Self::read_heights(&mut map_elements, r)?;
        let counts = Self::read_counts(&mut map_elements, r)?;
        for layer in [LAND, OBJECTS, STRUCTS, SHADOWS, ROOFS, ON_ROOFS] {
            let wide_sub_index = layer == OBJECTS && version.minor >= 15;
            Self::read_layer(&mut map_elements, &counts, layer, wide_sub_index, r)?;
        }

        let russian_unknown = if version.major == 6. && version.minor == 26 {
            Some(read_bytes(r)?)
        } else {
            None
        };

        Self::read_rooms(&mut map_elements, r)?;

        let items = if flags.contains(Ja2MapFlags::MAP_WORLDITEMS_SAVED) {
            Ja2WorldItem::read_all(r)?
        } else {
            vec![]
        };
        let ambient_light = if flags.contains(Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED) {
            Some(Ja2AmbientLight::read(r)?)
        } else {
            None
        };
        let lights = if flags.contains(Ja2MapFlags::MAP_WORLDLIGHTS_SAVED) {
            Some(Ja2Lights::read(r)?)
        } else {
            None
        };

        let info = Ja2MapInfo::read(r)?;
        if version.major >= 4. && info.map_version != version.minor {
            return Err(invalid_data(format!(
                "map info version {} does not match minor version {}",
                info.map_version, version.minor
            )));
        }

        let placements = if flags.contains(Ja2MapFlags::MAP_FULLSOLDIER_SAVED) {
            (0..info.num_individuals)
                .map(|_| Ja2SoldierPlacement::read(r))
                .collect::<io::Result<_>>()?
        } else {
            vec![]
        };
        let exit_grids = if flags.contains(Ja2MapFlags::MAP_EXITGRIDS_SAVED) {
            Ja2ExitGrid::read_all(r)?
        } else {
            vec![]
        };
        let doors = if flags.contains(Ja2MapFlags::MAP_DOORTABLE_SAVED) {
            Ja2Door::read_all(r)?
        } else {
            vec![]
        };
        let edge_points = if flags.contains(Ja2MapFlags::MAP_EDGEPOINTS_SAVED) {
            Some(Ja2EdgePoints::read(&info, r)?)
        } else {
            None
        };
        let schedules = if flags.contains(Ja2MapFlags::MAP_NPCSCHEDULES_SAVED) {
            Ja2Schedule::read_all(r)?
        } else {
            vec![]
        };
        // Any remaining data is ignored, like the game does

        Ok(Ja2Map {
            version,
            flags,
            tileset_id,
            soldier_size,
            map_elements,
            russian_unknown,
            items,
            ambient_light,
            lights,
            info,
            placements,
            exit_grids,
            doors,
            edge_points,
            schedules,
        })
    }

    /// Writes the map in its version, the inverse of [`Ja2Map::parse`]
    pub fn write(&self, w: &mut dyn io::Write) -> io::Result<()> {
        let version = &self.version;
        w.write_f32::<LE>(version.major)?;
        if version.major >= 4.0 {
            w.write_u8(version.minor)?;
        }
        w.write_u32::<LE>(self.flags.bits())?;
        w.write_i32::<LE>(self.tileset_id)?;
        w.write_u32::<LE>(self.soldier_size)?;

        if self.map_elements.len() != JA2_TILE_COUNT {
            return Err(invalid_input(format!(
                "the map has {} tiles instead of {}",
                self.map_elements.len(),
                JA2_TILE_COUNT
            )));
        }
        Self::write_heights(&self.map_elements, w)?;
        Self::write_counts(&self.map_elements, w)?;
        for layer in [LAND, OBJECTS, STRUCTS, SHADOWS, ROOFS, ON_ROOFS] {
            let wide_sub_index = layer == OBJECTS && version.minor >= 15;
            Self::write_layer(&self.map_elements, layer, wide_sub_index, w)?;
        }

        let is_russian = version.major == 6. && version.minor == 26;
        match (&self.russian_unknown, is_russian) {
            (Some(russian_unknown), true) => w.write_all(russian_unknown)?,
            (None, true) => w.write_unused(148)?,
            _ => {}
        }

        let rooms: Vec<u8> = self
            .map_elements
            .iter()
            .map(|element| element.room)
            .collect();
        w.write_all(&rooms)?;

        let flags = self.flags;
        if flags.contains(Ja2MapFlags::MAP_WORLDITEMS_SAVED) {
            Ja2WorldItem::write_all(&self.items, w)?;
        }
        if flags.contains(Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED) {
            saved(&self.ambient_light, "ambient lights")?.write(w)?;
        }
        if flags.contains(Ja2MapFlags::MAP_WORLDLIGHTS_SAVED) {
            saved(&self.lights, "lights")?.write(w)?;
        }

        self.info.write(w)?;

        if flags.contains(Ja2MapFlags::MAP_FULLSOLDIER_SAVED) {
            if usize::from(self.info.num_individuals) != self.placements.len() {
                return Err(invalid_input(format!(
                    "the map info says there are {} placements, but there are {}",
                    self.info.num_individuals,
                    self.placements.len()
                )));
            }
            self.placements
                .iter()
                .try_for_each(|placement| placement.write(w))?;
        }
        if flags.contains(Ja2MapFlags::MAP_EXITGRIDS_SAVED) {
            Ja2ExitGrid::write_all(&self.exit_grids, w)?;
        }
        if flags.contains(Ja2MapFlags::MAP_DOORTABLE_SAVED) {
            Ja2Door::write_all(&self.doors, w)?;
        }
        if flags.contains(Ja2MapFlags::MAP_EDGEPOINTS_SAVED) {
            saved(&self.edge_points, "edge points")?.write(&self.info, w)?;
        }
        if flags.contains(Ja2MapFlags::MAP_NPCSCHEDULES_SAVED) {
            Ja2Schedule::write_all(&self.schedules, w)?;
        }
        Ok(())
    }

    /// The map as written by [`Ja2Map::write`]
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bytes = vec![];
        self.write(&mut bytes)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses every `.dat` file in the directory given by `JA2_MAPS_DIR`
    #[test]
    fn parse_maps_from_dir() {
        let Ok(dir) = std::env::var("JA2_MAPS_DIR") else {
            eprintln!("JA2_MAPS_DIR not set, skipping");
            return;
        };
        let mut parsed = 0;
        let mut failures = vec![];
        for entry in std::fs::read_dir(&dir).expect("JA2_MAPS_DIR should be readable") {
            let path = entry.expect("dir entry should be readable").path();
            let is_dat = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"));
            if !is_dat {
                continue;
            }
            let bytes = std::fs::read(&path).expect("map should be readable");
            match Ja2Map::parse(&bytes) {
                Ok(map) => {
                    if map.version.major >= 4. {
                        assert_eq!(map.info.map_version, map.version.minor, "{:?}", path);
                    }
                    parsed += 1;
                }
                Err(e) => failures.push(format!("{}: {}", path.display(), e)),
            }
        }
        assert!(
            failures.is_empty(),
            "failed to parse maps:\n{}",
            failures.join("\n")
        );
        assert!(parsed > 0, "no maps found in {}", dir);
    }

    /// A map that uses every part of the map file
    fn synthetic_map(major: f32, minor: u8) -> Ja2Map {
        let mut map_elements = vec![Ja2MapElement::default(); JA2_TILE_COUNT];
        map_elements[3] = Ja2MapElement {
            flags: Ja2MapElementFlags::MAPELEMENT_ENEMY_MINE_PRESENT
                | Ja2MapElementFlags::MAPELEMENT_REDUNDANT,
            height: 20,
            land: vec![
                Ja2TileLayer {
                    kind: 1,
                    sub_index: 3,
                };
                15
            ],
            objects: vec![Ja2TileLayer {
                kind: 40,
                sub_index: if minor >= 15 { 300 } else { 200 },
            }],
            structs: vec![Ja2TileLayer {
                kind: 50,
                sub_index: 7,
            }],
            shadows: vec![Ja2TileLayer {
                kind: 60,
                sub_index: 1,
            }],
            roofs: vec![Ja2TileLayer {
                kind: 70,
                sub_index: 2,
            }],
            on_roofs: vec![Ja2TileLayer {
                kind: 80,
                sub_index: 4,
            }],
            room: 9,
        };
        let edge_list = |grid_nos: Vec<i16>| Ja2EdgePointList {
            middle_index: 1,
            grid_nos,
        };
        let edge_set = Ja2EdgePointSet {
            north: edge_list(vec![1, 2, 3]),
            east: edge_list(vec![]),
            south: edge_list(vec![-1]),
            west: edge_list(vec![5, 6]),
        };
        let object = Ja2Object {
            item: 12,
            number: 2,
            data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            attach_items: [1, 0, 3, 0],
            attach_status: [100, 0, -5, 0],
            flags: -3,
            mission: 4,
            trap: 1,
            imprint_id: 7,
            weight: 8,
            used: 1,
        };
        let placement = Ja2SoldierPlacement {
            basic: Ja2BasicSoldierPlacement {
                detailed_placement: true,
                starting_grid_no: 1234,
                team: 1,
                direction: 3,
                body_type: -1,
                patrol_grid: [1, 2, 3, 4, 5, 6, 7, 8, 9, -1],
                patrol_cnt: 3,
                on_roof: true,
                soldier_class: 2,
                has_keys: true,
                ..default()
            },
            detailed: Some(Ja2DetailedSoldierPlacement {
                profile: 200,
                sector_x: 9,
                sector_y: 1,
                insertion_grid_no: -1,
                life: 50,
                ai_morale: -1,
                inventory: vec![object.clone(); Ja2Object::NUM_INV_SLOTS],
                head_pal: "BROWNHEAD".to_owned(),
                skin_pal: "X".repeat(Ja2DetailedSoldierPlacement::PALETTE_LENGTH),
                name: "Kingpin".to_owned(),
                sector_z: 1,
                use_given_vehicle_id: -1,
                ..default()
            }),
        };
        Ja2Map {
            version: Ja2MapVersion { major, minor },
            flags: Ja2MapFlags::all(),
            tileset_id: 3,
            soldier_size: 1040,
            map_elements,
            russian_unknown: (major == 6. && minor == 26).then(|| {
                let mut unknown = [0; 148];
                unknown[0] = 1;
                unknown[147] = 2;
                unknown
            }),
            items: vec![Ja2WorldItem {
                exists: true,
                grid_no: 555,
                level: 1,
                object,
                flags: 0x10,
                render_z_height_above_level: -1,
                visible: 1,
                non_exist_chance: 20,
            }],
            ambient_light: Some(Ja2AmbientLight {
                basement: true,
                caves: false,
                level: 12,
            }),
            lights: Some(Ja2Lights {
                colors: vec![[10, 20, 30, 0]],
                sprites: vec![Ja2LightSprite {
                    x: 10,
                    y: 20,
                    flags: 0x80,
                    template_name: "L-R01.LHT".to_owned(),
                }],
            }),
            info: Ja2MapInfo {
                north_grid_no: -1,
                east_grid_no: 100,
                num_individuals: 1,
                map_version: minor,
                editor_smoothing_type: 2,
                center_grid_no: 12880,
                ..default()
            },
            placements: vec![placement],
            exit_grids: vec![Ja2ExitGrid {
                grid_no: 7,
                dest_grid_no: 8,
                dest_sector: (9, 1, 2),
            }],
            doors: vec![Ja2Door {
                grid_no: 99,
                locked: true,
                trap_level: 3,
                trap_id: 2,
                lock_id: 4,
                perceived_locked: -1,
                perceived_trapped: 1,
                lock_damage: 5,
            }],
            edge_points: Some(Ja2EdgePoints {
                primary: edge_set.clone(),
                secondary: (minor >= 17).then_some(edge_set),
            }),
            schedules: vec![Ja2Schedule {
                time: [1, 2, 3, 4],
                data1: [5, 6, 7, 8],
                data2: [9, 10, 11, 12],
                action: [1, 2, 3, 4],
                schedule_id: 1,
                soldier_id: 2,
                flags: 3,
            }],
        }
    }

    #[test]
    fn written_maps_parse_the_same() {
        for (major, minor) in [(6., 26), (6., 25), (6., 15), (5., 14)] {
            let map = synthetic_map(major, minor);
            let bytes = map.to_bytes().unwrap();
            let parsed = Ja2Map::parse(&bytes).unwrap();
            assert_eq!(parsed, map, "version {}.{}", major, minor);
            assert_eq!(parsed.to_bytes().unwrap(), bytes);
        }
    }

    #[test]
    fn rejects_what_does_not_fit() {
        let mut map = synthetic_map(6., 26);
        map.map_elements[0].structs = vec![Ja2TileLayer::default(); 16];
        assert!(map.to_bytes().is_err());

        let mut map = synthetic_map(6., 26);
        map.doors = vec![Ja2Door::default(); 256];
        assert!(map.to_bytes().is_err());

        let mut map = synthetic_map(5., 14);
        map.map_elements[0].objects = vec![Ja2TileLayer {
            kind: 1,
            sub_index: 256,
        }];
        assert!(map.to_bytes().is_err());

        let mut map = synthetic_map(6., 26);
        map.info.num_individuals = 2;
        assert!(map.to_bytes().is_err());
    }

    /// Writes every `.dat` file in the directory given by `JA2_MAPS_DIR` and parses it again
    #[test]
    fn write_maps_from_dir() {
        let Ok(dir) = std::env::var("JA2_MAPS_DIR") else {
            eprintln!("JA2_MAPS_DIR not set, skipping");
            return;
        };
        let mut written = 0;
        for entry in std::fs::read_dir(&dir).expect("JA2_MAPS_DIR should be readable") {
            let path = entry.expect("dir entry should be readable").path();
            let is_dat = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"));
            if !is_dat {
                continue;
            }
            let bytes = std::fs::read(&path).expect("map should be readable");
            let map = Ja2Map::parse(&bytes).expect("map should parse");
            let written_bytes = map
                .to_bytes()
                .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
            let reparsed = Ja2Map::parse(&written_bytes)
                .unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
            assert!(reparsed == map, "{} changed", path.display());
            written += 1;
        }
        assert!(written > 0, "no maps found in {}", dir);
    }
}
