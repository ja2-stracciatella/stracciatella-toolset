//! New maps, like the original editor creates them (`CreateNewMap`, `NewWorld`)

use crate::{
    components::map::JA2_MAP_TILES,
    editor::lights::SURFACE_AMBIENT_LEVEL,
    file_formats::{
        ja2_map::{
            Ja2AmbientLight, Ja2Map, Ja2MapElement, Ja2MapFlags, Ja2MapInfo, Ja2MapVersion,
            Ja2TileLayer,
        },
        ja2_tile_types::{TileType, FIRSTROOF, FIRSTTEXTURE, FIRSTWALL},
    },
    random::Random,
    tileset::TilesetId,
};

use super::{convert::NOWHERE, EditorSmoothing};

/// Minor version of new maps, the one of the vanilla maps (6.00/26 is the russian one)
const NEW_MAP_MINOR_VERSION: u8 = 25;

fn piece(tile_type: TileType, sub_index: u32) -> Ja2TileLayer {
    Ja2TileLayer {
        kind: tile_type as u8,
        sub_index: sub_index as u16,
    }
}

/// A map file of a new map: every tile has a random texture, basements are roofed and caves are
/// solid rock
///
/// Nothing else is placed, the entry points are not set. Basements and caves get the ambient
/// light that switching to them gets in the editor.
pub fn new_ja2_map(tileset: TilesetId, smoothing: EditorSmoothing, random: &mut Random) -> Ja2Map {
    let map_elements = (0..JA2_MAP_TILES)
        .map(|_| {
            let mut element = Ja2MapElement {
                // `NewWorld`: one of the first 10 textures
                land: vec![piece(FIRSTTEXTURE, 1 + random.next(10))],
                ..Default::default()
            };
            match smoothing {
                EditorSmoothing::Normal => {}
                EditorSmoothing::Basement => {
                    element.roofs = vec![piece(FIRSTROOF, 9 + random.next(3))];
                }
                // `AddCave`, the editor marks the rock as caves once the map is loaded
                EditorSmoothing::Caves => {
                    element.structs = vec![piece(FIRSTWALL, 60 + random.next(6))];
                }
            }
            element
        })
        .collect();
    let ambient_light = (smoothing != EditorSmoothing::Normal).then(|| Ja2AmbientLight {
        basement: smoothing == EditorSmoothing::Basement,
        caves: smoothing == EditorSmoothing::Caves,
        level: SURFACE_AMBIENT_LEVEL,
    });
    Ja2Map {
        version: Ja2MapVersion {
            major: 6.,
            minor: NEW_MAP_MINOR_VERSION,
        },
        flags: if ambient_light.is_some() {
            Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED
        } else {
            Ja2MapFlags::empty()
        },
        tileset_id: tileset.index() as i32,
        map_elements,
        ambient_light,
        info: Ja2MapInfo {
            north_grid_no: NOWHERE,
            east_grid_no: NOWHERE,
            south_grid_no: NOWHERE,
            west_grid_no: NOWHERE,
            center_grid_no: NOWHERE,
            isolated_grid_no: NOWHERE,
            map_version: NEW_MAP_MINOR_VERSION,
            editor_smoothing_type: smoothing.number(),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        editor::caves::mark_caves,
        file_formats::ja2_tile_types::TILE_TYPE_NAMES,
        map::{convert_tests::Fixture, AmbientLight, TacticalMap},
        tileset::Tile,
    };

    fn new_map(smoothing: EditorSmoothing) -> Ja2Map {
        new_ja2_map(TilesetId::new(3), smoothing, &mut Random::new(1))
    }

    /// The tile types and sub indexes of the tiles of a layer
    fn pieces(tiles: &[Arc<Tile>]) -> Vec<(&str, u16)> {
        tiles
            .iter()
            .map(|tile| (tile.type_name.as_str(), tile.sub_index))
            .collect()
    }

    #[test]
    fn new_maps_survive_a_round_trip() {
        for smoothing in [
            EditorSmoothing::Normal,
            EditorSmoothing::Basement,
            EditorSmoothing::Caves,
        ] {
            let map = new_map(smoothing);
            let parsed = Ja2Map::parse(&map.to_bytes().unwrap()).unwrap();
            assert_eq!(parsed, map, "{:?}", smoothing);
            assert_eq!(parsed.tileset_id, 3);
            assert_eq!((parsed.version.major, parsed.version.minor), (6., 25));
        }
    }

    #[test]
    fn outdoor_maps_have_textures() {
        let fixture = Fixture::new();
        let map = fixture.convert(&new_map(EditorSmoothing::Normal));
        assert_eq!(map.tileset, TilesetId::new(3));
        assert_eq!(map.info.editor_smoothing, EditorSmoothing::Normal);
        assert_eq!(map.ambient_light, None);
        assert_eq!(map.info.north_entry, None);
        assert_eq!(map.info.center, None);
        let mut sub_indexes = vec![];
        for tile in map.tiles.values() {
            let land = pieces(&tile.land);
            assert_eq!(land.len(), 1);
            assert_eq!(land[0].0, TILE_TYPE_NAMES[FIRSTTEXTURE]);
            sub_indexes.push(land[0].1);
            assert!(tile.structs.is_empty() && tile.roofs.is_empty());
        }
        assert!(sub_indexes.iter().all(|i| (1..=10).contains(i)));
        // The textures are mixed
        assert!(sub_indexes.contains(&1) && sub_indexes.contains(&10));
    }

    #[test]
    fn basements_are_roofed() {
        let fixture = Fixture::new();
        let map = fixture.convert(&new_map(EditorSmoothing::Basement));
        assert_eq!(map.info.editor_smoothing, EditorSmoothing::Basement);
        assert_eq!(
            map.ambient_light,
            Some(AmbientLight {
                basement: true,
                caves: false,
                level: SURFACE_AMBIENT_LEVEL,
            })
        );
        for tile in map.tiles.values() {
            let roofs = pieces(&tile.roofs);
            assert_eq!(roofs.len(), 1);
            assert_eq!(roofs[0].0, TILE_TYPE_NAMES[FIRSTROOF]);
            assert!((9..=11).contains(&roofs[0].1));
            assert_eq!(pieces(&tile.land)[0].0, TILE_TYPE_NAMES[FIRSTTEXTURE]);
            assert!(tile.structs.is_empty());
        }
    }

    #[test]
    fn caves_are_solid_rock() {
        let fixture = Fixture::new();
        let mut map: TacticalMap = fixture.convert(&new_map(EditorSmoothing::Caves));
        assert_eq!(map.info.editor_smoothing, EditorSmoothing::Caves);
        assert_eq!(
            map.ambient_light,
            Some(AmbientLight {
                basement: false,
                caves: true,
                level: SURFACE_AMBIENT_LEVEL,
            })
        );
        for tile in map.tiles.values() {
            let structs = pieces(&tile.structs);
            assert_eq!(structs.len(), 1);
            assert_eq!(structs[0].0, TILE_TYPE_NAMES[FIRSTWALL]);
            assert!((60..=65).contains(&structs[0].1));
            assert!(tile.roofs.is_empty());
        }
        // The editor knows the rock is caves
        mark_caves(&mut map);
        assert!(map.tiles.values().all(|tile| tile.cave));
    }
}
