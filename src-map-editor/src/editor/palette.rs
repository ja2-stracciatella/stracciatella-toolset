//! The palettes of the loaded map, which the host shows to pick tiles from

use bevy::prelude::*;
use serde::Serialize;

use crate::{
    host::report_editor_event,
    map::TacticalMap,
    states::loading::MapToLoad,
    tileset::{BuildingPalette, Tile, TileCategory, TileIndex, Tileset},
};

/// A tile of a palette
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteEntry {
    pub tile_id: TileIndex,
    pub type_name: String,
    pub sub_index: u16,
    /// The image file, relative to the data root
    pub file: String,
    pub sub_image: u16,
}

impl PaletteEntry {
    fn new(tile: &Tile) -> Self {
        let image = &tile.image;
        Self {
            tile_id: tile.id,
            type_name: tile.type_name.clone(),
            sub_index: tile.sub_index,
            file: format!(
                "tilesets/{}/{}",
                image.surface.tileset_id, image.surface.filename
            ),
            sub_image: image.sub_image,
        }
    }
}

/// The palettes of the buildings mode, see [`BuildingPalette`]
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingPalettes {
    pub room: Vec<PaletteEntry>,
    pub walls: Vec<PaletteEntry>,
    pub doors: Vec<PaletteEntry>,
    pub windows: Vec<PaletteEntry>,
    pub roofs: Vec<PaletteEntry>,
    pub broken_walls: Vec<PaletteEntry>,
    pub decals: Vec<PaletteEntry>,
    pub floors: Vec<PaletteEntry>,
    pub smart_walls: Vec<PaletteEntry>,
    pub smart_doors: Vec<PaletteEntry>,
    pub smart_windows: Vec<PaletteEntry>,
    pub smart_broken_walls: Vec<PaletteEntry>,
    pub smart_roofs: Vec<PaletteEntry>,
}

impl BuildingPalettes {
    pub fn new(tileset: &Tileset) -> Self {
        let entries = |palette| {
            tileset
                .building_palette(palette)
                .iter()
                .map(|t| PaletteEntry::new(t))
                .collect()
        };
        Self {
            room: entries(BuildingPalette::Room),
            walls: entries(BuildingPalette::Walls),
            doors: entries(BuildingPalette::Doors),
            windows: entries(BuildingPalette::Windows),
            roofs: entries(BuildingPalette::Roofs),
            broken_walls: entries(BuildingPalette::BrokenWalls),
            decals: entries(BuildingPalette::Decals),
            floors: entries(BuildingPalette::Floors),
            smart_walls: entries(BuildingPalette::SmartWalls),
            smart_doors: entries(BuildingPalette::SmartDoors),
            smart_windows: entries(BuildingPalette::SmartWindows),
            smart_broken_walls: entries(BuildingPalette::SmartBrokenWalls),
            smart_roofs: entries(BuildingPalette::SmartRoofs),
        }
    }
}

/// The palettes of the terrain and buildings modes
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename = "palette")]
pub struct PaletteEvent {
    pub map: String,
    pub tileset: u8,
    /// The first tile of each ground texture and of the water that exists, in the order of the
    /// tile types
    pub ground: Vec<PaletteEntry>,
    pub roads: Vec<PaletteEntry>,
    pub cliffs: Vec<PaletteEntry>,
    pub debris: Vec<PaletteEntry>,
    pub trees: Vec<PaletteEntry>,
    pub rocks: Vec<PaletteEntry>,
    pub misc: Vec<PaletteEntry>,
    pub furniture: Vec<PaletteEntry>,
    #[serde(rename = "genericFurniture")]
    pub generic_furniture: Vec<PaletteEntry>,
    pub buildings: BuildingPalettes,
}

impl PaletteEvent {
    pub fn new(map: String, tileset_id: u8, tileset: &Tileset) -> Self {
        let entries = |categories: &[TileCategory]| {
            categories
                .iter()
                .flat_map(|&category| tileset.palette(category))
                .map(|t| PaletteEntry::new(t))
                .collect()
        };
        Self {
            map,
            tileset: tileset_id,
            ground: entries(&[
                TileCategory::Ground,
                TileCategory::Water,
                TileCategory::DeepWater,
            ]),
            roads: entries(&[TileCategory::Roads]),
            cliffs: entries(&[TileCategory::Cliffs]),
            debris: entries(&[TileCategory::Debris]),
            trees: entries(&[TileCategory::Trees]),
            rocks: entries(&[TileCategory::Rocks]),
            misc: entries(&[TileCategory::Misc]),
            furniture: entries(&[TileCategory::Furniture]),
            generic_furniture: entries(&[TileCategory::GenericFurniture]),
            buildings: BuildingPalettes::new(tileset),
        }
    }
}

pub fn report_palette(map_to_load: Res<MapToLoad>, map: Res<TacticalMap>, tileset: Res<Tileset>) {
    let event = PaletteEvent::new(map_to_load.0.clone(), map.tileset.index() as u8, &tileset);
    report_editor_event(&event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tileset::tests::test_tileset;

    #[test]
    fn palette_events_are_json_for_the_host() {
        let tileset = test_tileset();
        let event = PaletteEvent::new("maps/a9.dat".to_owned(), 0, &tileset);
        // The land textures, the water and the deep water
        assert_eq!(event.ground.len(), 9);
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "palette");
        assert_eq!(json["map"], "maps/a9.dat");
        let tree = &json["trees"][1];
        assert_eq!(tree["typeName"], "OFSTRUCT1");
        assert_eq!(tree["subIndex"], 4);
        assert_eq!(tree["subImage"], 3);
        assert_eq!(tree["file"], "tilesets/0/type20.sti");
        assert!(tree["tileId"].is_u64());
        assert_eq!(json["roads"].as_array().unwrap().len(), 32);
        assert_eq!(json["cliffs"].as_array().unwrap().len(), 17);
        assert_eq!(json["cliffs"][0]["typeName"], "FIRSTCLIFF");
        let buildings = &json["buildings"];
        assert_eq!(buildings["room"][0]["typeName"], "WALL1");
        assert_eq!(buildings["smartRoofs"][0]["subIndex"], 10);
        assert_eq!(buildings["smartBrokenWalls"].as_array().unwrap().len(), 4);
        assert_eq!(json["genericFurniture"][0]["typeName"], "FIFTHISTRUCT");
    }
}
