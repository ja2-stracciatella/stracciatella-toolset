//! The loaded map, the runtime form of the map file
//!
//! [`TacticalMap::from_ja2`] turns the file format map (`Ja2Map`) into a [`TacticalMap`] once the
//! tileset, the item files, the light templates and the soldier palettes are loaded. Raw numbers
//! become typed values: grid numbers are [`GridNo`]s, tiles are the [`Tile`](crate::tileset::Tile)s
//! of the tileset, light templates, items and soldier definitions are resolved, the random choices
//! of the game for soldiers are made, and the version specific fix-ups of the game are applied.
//! Magic values like `NOWHERE` or `NO_PROFILE` become `None`. Anything that can't be converted is
//! left out with a warning, so the renderer never needs to know about the map file.

use std::{collections::HashMap, sync::Arc};

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

use crate::{
    file_formats::ja2_map::{Ja2ExitGrid, Ja2LightSprite, Ja2SoldierPlacement, Ja2WorldItem},
    item::ItemTable,
    light::LightTemplate,
    soldier::{palette_files::PaletteReplacements, SoldierDefinitions},
    tileset::{Tileset, TilesetId},
};

mod convert;
pub mod door;
pub mod edge_points;
pub mod exit_grid;
pub mod grid;
pub mod item;
pub mod item_data;
pub mod light;
mod new_map;
pub mod placement;
mod save;
pub mod schedule;
pub mod tile;

#[cfg(test)]
pub(crate) use convert::tests as convert_tests;
pub use convert::{convert_ambient_light, item_graphic};
pub use door::*;
pub use edge_points::*;
pub use exit_grid::*;
pub use grid::*;
pub use item::*;
pub use light::*;
pub use new_map::new_ja2_map;
pub use placement::*;
pub use save::{MAX_NUM_SOLDIERS, MAX_SCHEDULES, OBJECT_UNDROPPABLE};
pub use schedule::*;
pub use tile::*;

/// How the editor smooths the terrain of the map (`ubEditorSmoothingType`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorSmoothing {
    #[default]
    Normal,
    Basement,
    Caves,
}

impl EditorSmoothing {
    pub fn from_number(number: u8) -> Option<Self> {
        Some(match number {
            0 => Self::Normal,
            1 => Self::Basement,
            2 => Self::Caves,
            _ => return None,
        })
    }

    /// The raw smoothing type, the inverse of [`EditorSmoothing::from_number`]
    pub fn number(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Basement => 1,
            Self::Caves => 2,
        }
    }
}

/// General information of the map (`MAPCREATE_STRUCT`)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapInfo {
    /// Where soldiers enter the map from the neighbouring sectors, `None` if they can't
    pub north_entry: Option<GridNo>,
    pub east_entry: Option<GridNo>,
    pub south_entry: Option<GridNo>,
    pub west_entry: Option<GridNo>,
    pub center: Option<GridNo>,
    /// Entry point of the part of the map that is isolated from the edges
    pub isolated: Option<GridNo>,
    /// Scrolling restriction of basements and caves (`ubRestrictedScrollID`)
    pub restricted_scroll: bool,
    pub editor_smoothing: EditorSmoothing,
}

/// One of the entry points of [`MapInfo`]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryPoint {
    #[default]
    North,
    East,
    South,
    West,
    Center,
    Isolated,
}

impl EntryPoint {
    pub const ALL: [EntryPoint; 6] = [
        EntryPoint::North,
        EntryPoint::East,
        EntryPoint::South,
        EntryPoint::West,
        EntryPoint::Center,
        EntryPoint::Isolated,
    ];
}

impl MapInfo {
    /// The tile of an entry point
    pub fn entry(&self, point: EntryPoint) -> Option<GridNo> {
        match point {
            EntryPoint::North => self.north_entry,
            EntryPoint::East => self.east_entry,
            EntryPoint::South => self.south_entry,
            EntryPoint::West => self.west_entry,
            EntryPoint::Center => self.center,
            EntryPoint::Isolated => self.isolated,
        }
    }

    pub fn entry_mut(&mut self, point: EntryPoint) -> &mut Option<GridNo> {
        match point {
            EntryPoint::North => &mut self.north_entry,
            EntryPoint::East => &mut self.east_entry,
            EntryPoint::South => &mut self.south_entry,
            EntryPoint::West => &mut self.west_entry,
            EntryPoint::Center => &mut self.center,
            EntryPoint::Isolated => &mut self.isolated,
        }
    }
}

/// The loaded map
#[derive(Resource, Clone, Debug)]
pub struct TacticalMap {
    pub tileset: TilesetId,
    pub tiles: GridArray<MapTile>,
    pub info: MapInfo,
    /// Saved ambient light, only basements and caves have one
    pub ambient_light: Option<AmbientLight>,
    /// Added to all palettes (`g_light_color`), black if the map has no light colour
    pub light_color: [u8; 3],
    pub lights: Vec<MapLight>,
    /// The light sprites of the map file that could not be converted, saved as they are
    pub skipped_lights: Vec<Ja2LightSprite>,
    pub items: Vec<MapItem>,
    /// The world items of the map file that could not be converted, saved as they are
    pub skipped_items: Vec<Ja2WorldItem>,
    pub soldiers: Vec<SoldierPlacement>,
    /// The placements of the map file that could not be converted, saved as they are
    pub skipped_placements: Vec<Ja2SoldierPlacement>,
    pub exit_grids: Vec<ExitGrid>,
    /// The exit grids of the map file that could not be converted, saved as they are
    pub skipped_exit_grids: Vec<Ja2ExitGrid>,
    pub doors: Vec<Door>,
    pub edge_points: EdgePoints,
    /// Whether the edge points were generated again since the map was loaded, only then they are
    /// saved, so the edge points of the map file are kept otherwise
    pub edge_points_stale: bool,
    pub schedules: Vec<Schedule>,
}

impl TacticalMap {
    /// A map without any tiles, lights, items or soldiers
    pub fn empty(tileset: TilesetId) -> Self {
        Self {
            tileset,
            tiles: GridArray::default(),
            info: MapInfo::default(),
            ambient_light: None,
            light_color: [0; 3],
            lights: vec![],
            skipped_lights: vec![],
            items: vec![],
            skipped_items: vec![],
            soldiers: vec![],
            skipped_placements: vec![],
            exit_grids: vec![],
            skipped_exit_grids: vec![],
            doors: vec![],
            edge_points: EdgePoints::default(),
            edge_points_stale: false,
            schedules: vec![],
        }
    }
}

/// Everything besides the map file that the conversion needs
pub struct MapContext<'a> {
    pub tileset: &'a Tileset,
    pub items: &'a ItemTable,
    pub soldiers: SoldierDefinitions<'a>,
    /// `None` if `ja2pal.dat` could not be loaded, soldiers then keep their animation palette
    pub palettes: Option<&'a PaletteReplacements>,
    /// Light templates by lower case file name, lights whose template is missing are left out
    pub light_templates: &'a HashMap<String, Arc<LightTemplate>>,
}
