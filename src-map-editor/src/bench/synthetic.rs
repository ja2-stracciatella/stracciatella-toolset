//! A synthetic tileset with pixels and a map that uses all of its layers
//!
//! The tile types have structure data like `building_tileset`, the sheets have sizes like the
//! sprites of the game, so that the scene and the renderer have realistic amounts of work and memory.

use std::{collections::HashMap, sync::Arc};

use crate::{
    components::map::GridPosition,
    file_formats::{
        ja2_stci::SubImageInfo,
        ja2_tile_database::Ja2TileDatabase,
        ja2_tile_types::{
            TileType, FIRSTCLIFFHANG, FIRSTFLOOR, FIRSTFULLSTRUCT, FIRSTOSTRUCT, FIRSTROAD,
            FIRSTROOF, FIRSTSHADOW, FIRSTTEXTURE, FIRSTWALL, FOURTHDOOR, FOURTHFLOOR, GUNS,
            NUMBER_OF_TILE_TYPES, SECONDOSTRUCT,
        },
    },
    item::{HandItem, ItemFile, ItemId, ItemInfo, ItemTable},
    light::{LightNode, LightTemplate, LIGHT_NEW_RAY},
    map::{
        GridNo, ItemHeight, ItemStack, Level, LightId, LightTime, MapItem, MapLight, MapTile,
        Visibility, WorldItemFlags, WorldItemId,
    },
    soldier::{palette_files::PaletteReplacements, StandingSurfaces},
    tileset::{tests::building_surfaces, IndexedImages, StciSheet, SubImage, Tile, Tileset},
};

/// Size of the map in tiles in each direction
const MAP_SIZE: u8 = 160;
/// The map is made of square blocks of this many tiles: buildings, forests and yards
const BLOCK_SIZE: u8 = 8;
/// Frames of each direction of the soldier sheets
const SOLDIER_FRAMES: u8 = 4;

/// The lights of the map, `L-R03.LHT`
pub const LIGHT_TEMPLATE: &str = "L-R03.LHT";
pub const LIGHT_RADIUS: u8 = 3;
pub const LIGHTS: usize = 64;
pub const ITEMS: usize = 400;

/// Offset and size of the sub images of a tile type, like the sprites of the game
fn sprite_geometry(tile_type: TileType) -> ((i16, i16), (u16, u16)) {
    let ground = tile_type < FIRSTCLIFFHANG
        || (FIRSTFLOOR..=FOURTHFLOOR).contains(&tile_type)
        || (FIRSTROAD..FIRSTROAD + 9).contains(&tile_type);
    if ground {
        ((0, 0), (40, 20))
    } else if (FIRSTWALL..=FOURTHDOOR).contains(&tile_type) {
        ((0, -50), (22, 70))
    } else if (FIRSTROOF..FIRSTROOF + 4).contains(&tile_type) {
        ((0, -50), (40, 40))
    } else {
        ((0, -30), (40, 50))
    }
}

/// Palette indices of a sub image: the tile diamond for ground tiles, otherwise a box with
/// stripes. Index 0 is transparent.
fn sprite_pixels(ground: bool, sub_index: usize, (width, height): (u16, u16)) -> Vec<u8> {
    let (width, height) = (usize::from(width), usize::from(height));
    let mut pixels = vec![0; width * height];
    for y in 0..height {
        for x in 0..width {
            let inside = if ground {
                // |dx| / 20 + |dy| / 10 <= 1
                let dx = (2 * x as i32 + 1 - width as i32).abs();
                let dy = (2 * y as i32 + 1 - height as i32).abs();
                dx * height as i32 + dy * width as i32 <= (width * height) as i32
            } else {
                true
            };
            if inside {
                pixels[y * width + x] = 1 + ((x + y / 2 + sub_index * 7) % 254) as u8;
            }
        }
    }
    pixels
}

/// A gradient in a colour of its own for each tile type
fn palette(seed: usize) -> Box<[[u8; 3]; 256]> {
    let base = [
        (seed * 67 % 256) as u8,
        (seed * 131 % 256) as u8,
        (seed * 193 % 256) as u8,
    ];
    let mut palette = Box::new([[0; 3]; 256]);
    for (index, color) in palette.iter_mut().enumerate() {
        let shade = index as u16;
        *color = base.map(|c| ((u16::from(c) + shade) / 2) as u8);
    }
    palette
}

/// A sheet with a palette of its own for each `seed`
fn sheet(
    sub_images: Vec<SubImageInfo>,
    frames_per_direction: u8,
    ground: bool,
    seed: usize,
) -> StciSheet {
    StciSheet {
        sub_images: sub_images
            .iter()
            .map(|info| SubImage {
                offset: info.offset,
                dimensions: info.dimensions,
            })
            .collect(),
        frames_per_direction,
        indexed: Some(IndexedImages {
            palette: palette(seed),
            pixels: sub_images
                .iter()
                .enumerate()
                .map(|(index, info)| sprite_pixels(ground, index, info.dimensions))
                .collect(),
        }),
        database_info: sub_images,
    }
}

/// The tileset and the sheets of its surfaces, indexed by `SurfaceId`
pub fn tileset() -> (Tileset, Vec<StciSheet>) {
    let mut surfaces = building_surfaces();
    for (tile_type, surface) in surfaces.iter_mut().enumerate() {
        let Some(surface) = surface else {
            continue;
        };
        let (offset, dimensions) = sprite_geometry(tile_type);
        for info in &mut surface.sub_images {
            info.offset = offset;
            info.dimensions = dimensions;
        }
    }
    let database = Ja2TileDatabase::build(surfaces.clone())
        .expect("Expected the synthetic tile database to be valid.");
    let tileset = Tileset::from_database(database);
    let mut sheets: Vec<Option<StciSheet>> = (0..tileset.surfaces().len()).map(|_| None).collect();
    for (tile_type, info) in surfaces.iter().enumerate() {
        if let (Some(surface), Some(info)) = (tileset.surface_of_type(tile_type), info) {
            let ground = sprite_geometry(tile_type).1 == (40, 20);
            sheets[surface.id.index()] = Some(sheet(info.sub_images.clone(), 0, ground, tile_type));
        }
    }
    let sheets = sheets
        .into_iter()
        .map(|sheet| sheet.unwrap_or_else(|| self::sheet(vec![], 0, false, 0)))
        .collect();
    (tileset, sheets)
}

/// Standing animation sheets, indexed by `StandingSurfaceId`
pub fn soldier_sheets(surfaces: &StandingSurfaces) -> Vec<StciSheet> {
    surfaces
        .iter()
        .enumerate()
        .map(|(index, surface)| {
            let count = usize::from(surface.directions) * usize::from(SOLDIER_FRAMES);
            let sub_images = (0..count)
                .map(|_| SubImageInfo {
                    offset: (-12, -40),
                    dimensions: (24, 48),
                    app_data: None,
                })
                .collect();
            sheet(
                sub_images,
                SOLDIER_FRAMES,
                false,
                NUMBER_OF_TILE_TYPES + index,
            )
        })
        .collect()
}

/// Palette replacements with the names that new soldiers are given
pub fn soldier_palettes() -> PaletteReplacements {
    PaletteReplacements::coloured(&[
        "PINKSKIN",
        "TANSKIN",
        "DARKSKIN",
        "BLACKSKIN",
        "BROWNHEAD",
        "BLACKHEAD",
        "WHITEHEAD",
        "BLONDHEAD",
        "REDHEAD",
        "REDVEST",
        "YELLOWVEST",
        "BLACKSHIRT",
        "WHITEVEST",
        "GREENPANTS",
        "BLACKPANTS",
        "TANPANTS",
        "BLUEPANTS",
    ])
}

/// A single gun, which soldiers hold and items show
pub fn item_table() -> ItemTable {
    ItemTable::new([&ItemFile {
        items: vec![ItemInfo {
            index: ItemId::new(1).expect("Expected 1 to be a valid item id."),
            graphic: Some((GUNS, 1)),
            hand: Some(HandItem::OneHandedGun),
            kind: Default::default(),
        }],
    }])
}

/// A round light of `radius` tiles like the game creates them (`LightCreateOmni`): rays from the
/// light to points on a circle a quarter tile apart, with a node for every quarter tile step, so
/// that a tile repeats. The tiles are brighter closer to the light. Rays through the same steps
/// are left out. The real `L-R08.LHT` has 214 nodes, 180 rays and 5600 ray entries, this one 241,
/// 190 and 6272.
pub fn omni_light_template(radius: u8) -> Arc<LightTemplate> {
    // `DISTANCE_SCALE` of the game
    const STEPS_PER_TILE: f32 = 4.;
    let radius = f32::from(radius);
    let mut lights = vec![LightNode {
        dx: 0,
        dy: 0,
        light: radius as u8 + 1,
    }];
    let mut node_indices = HashMap::from([((0, 0), 0u16)]);
    let mut rays: Vec<Vec<u16>> = vec![];
    let ray_count = (std::f32::consts::TAU * radius * STEPS_PER_TILE).ceil() as usize;
    for index in 0..ray_count {
        let angle = std::f32::consts::TAU * index as f32 / ray_count as f32;
        let mut ray = vec![];
        for step in 1..=(radius * STEPS_PER_TILE) as usize {
            let distance = step as f32 / STEPS_PER_TILE;
            let tile = (
                (angle.cos() * distance).round() as i16,
                (angle.sin() * distance).round() as i16,
            );
            let node = *node_indices.entry(tile).or_insert_with(|| {
                let tile_distance = f32::from(tile.0).hypot(f32::from(tile.1));
                lights.push(LightNode {
                    dx: tile.0,
                    dy: tile.1,
                    light: (radius + 1. - tile_distance).max(1.) as u8,
                });
                lights.len() as u16 - 1
            });
            ray.push(node);
        }
        if !rays.contains(&ray) {
            rays.push(ray);
        }
    }
    let mut ray_list = vec![LIGHT_NEW_RAY, 0];
    for ray in rays {
        ray_list.push(LIGHT_NEW_RAY);
        ray_list.extend(ray);
    }
    Arc::new(LightTemplate {
        lights,
        rays: ray_list,
    })
}

/// The tiles that the map is made of
pub struct MapTiles {
    land: Vec<Arc<Tile>>,
    floors: Vec<Arc<Tile>>,
    walls_left: Vec<Arc<Tile>>,
    walls_top: Vec<Arc<Tile>>,
    roofs: Vec<Arc<Tile>>,
    trees: Vec<Arc<Tile>>,
    shadows: Vec<Arc<Tile>>,
    obstacles: Vec<Arc<Tile>>,
    objects: Vec<Arc<Tile>>,
    gun: Arc<Tile>,
}

impl MapTiles {
    pub fn new(tileset: &Tileset) -> Self {
        let tiles = |tile_type: TileType, sub_indices: &[u16]| -> Vec<Arc<Tile>> {
            sub_indices
                .iter()
                .map(|&sub_index| {
                    let index = tileset
                        .index(tile_type, sub_index)
                        .expect("Expected the synthetic tile to exist.");
                    tileset
                        .tile(index)
                        .expect("Expected the synthetic tile to exist.")
                        .clone()
                })
                .collect()
        };
        let land = (FIRSTTEXTURE..FIRSTTEXTURE + 4)
            .flat_map(|tile_type| tiles(tile_type, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
            .collect();
        Self {
            land,
            floors: tiles(FIRSTFLOOR, &[1, 2, 3, 4]),
            // The orientations of `building_tileset`
            walls_left: tiles(FIRSTWALL, &[4, 5, 6]),
            walls_top: tiles(FIRSTWALL, &[1, 2, 3]),
            roofs: tiles(FIRSTROOF, &[1, 2, 3, 4]),
            trees: tiles(FIRSTFULLSTRUCT, &[1, 2, 3]),
            shadows: tiles(FIRSTSHADOW, &[1, 2, 3]),
            obstacles: [tiles(FIRSTOSTRUCT, &[1]), tiles(SECONDOSTRUCT, &[1])].concat(),
            // Without structure data
            objects: tiles(FIRSTOSTRUCT, &[7, 9]),
            gun: tiles(GUNS, &[1]).remove(0),
        }
    }
}

/// What a block of the map is
#[derive(Clone, Copy, PartialEq, Eq)]
enum Block {
    Building,
    Forest,
    Yard,
}

fn block(position: GridPosition, phase: u32) -> Block {
    let (bx, by) = (position.x / BLOCK_SIZE, position.y / BLOCK_SIZE);
    match (u32::from(bx) + u32::from(by) + phase) % 3 {
        0 => Block::Building,
        1 => Block::Forest,
        _ => Block::Yard,
    }
}

/// Every tile of the map, row by row
pub fn positions() -> impl Iterator<Item = GridPosition> {
    (0..MAP_SIZE).flat_map(|y| (0..MAP_SIZE).map(move |x| GridPosition::new(x, y)))
}

/// Sets all layers of a tile for the `phase`, every phase changes every tile
pub fn fill_tile(tile: &mut MapTile, tiles: &MapTiles, position: GridPosition, phase: u32) {
    let (x, y) = (usize::from(position.x), usize::from(position.y));
    let phase_offset = phase as usize;
    let pick = |list: &[Arc<Tile>], seed: usize| list[seed % list.len()].clone();
    let (local_x, local_y) = (position.x % BLOCK_SIZE, position.y % BLOCK_SIZE);

    tile.land = vec![pick(&tiles.land, x * 7 + y * 3 + phase_offset)];
    tile.objects.clear();
    tile.structs.clear();
    tile.shadows.clear();
    tile.roofs.clear();
    tile.on_roofs.clear();
    match block(position, phase) {
        Block::Building => {
            tile.land.push(pick(&tiles.floors, x + y + phase_offset));
            if local_y == 0 {
                tile.structs.push(pick(&tiles.walls_top, x + phase_offset));
            } else if local_x == 0 {
                tile.structs.push(pick(&tiles.walls_left, y + phase_offset));
            }
            if local_x > 0 && local_y > 0 {
                tile.roofs
                    .push(pick(&tiles.roofs, x * 3 + y + phase_offset));
            }
        }
        Block::Forest => {
            if (x + y + phase_offset).is_multiple_of(3) {
                tile.structs.push(pick(&tiles.trees, x + phase_offset));
            } else {
                tile.shadows.push(pick(&tiles.shadows, y + phase_offset));
            }
        }
        Block::Yard => {
            if (x * 3 + y + phase_offset).is_multiple_of(5) {
                tile.structs
                    .push(pick(&tiles.obstacles, x + y + phase_offset));
            } else if (x + y * 2 + phase_offset).is_multiple_of(4) {
                tile.objects.push(pick(&tiles.objects, x + phase_offset));
            }
        }
    }
}

/// Free tiles of the yards for the `phase`, where soldiers can stand
pub fn free_tiles(phase: u32) -> impl Iterator<Item = GridNo> {
    positions()
        .filter(move |&position| {
            let (x, y) = (usize::from(position.x), usize::from(position.y));
            let phase_offset = phase as usize;
            block(position, phase) == Block::Yard
                && !(x * 3 + y + phase_offset).is_multiple_of(5)
                && !(x + y * 2 + phase_offset).is_multiple_of(4)
        })
        .map(GridNo::from)
}

/// Spreads `count` positions over the free tiles, a different set for every phase
pub fn spread(count: usize, phase: u32) -> Vec<GridNo> {
    let free: Vec<GridNo> = free_tiles(phase).collect();
    if free.is_empty() {
        return vec![];
    }
    let step = (free.len() / count.max(1)).max(1);
    (0..count)
        .map(|index| free[(index * step + phase as usize * 13) % free.len()])
        .collect()
}

pub fn lights(template: &Arc<LightTemplate>, phase: u32) -> Vec<MapLight> {
    spread(LIGHTS, phase)
        .into_iter()
        .enumerate()
        .map(|(index, grid_no)| MapLight {
            id: LightId(index as u32),
            position: grid_no.position(),
            time: LightTime::Always,
            template_name: LIGHT_TEMPLATE.to_owned(),
            template: template.clone(),
            file_sprite: None,
        })
        .collect()
}

pub fn items(tiles: &MapTiles, phase: u32) -> Vec<MapItem> {
    spread(ITEMS, phase + 1)
        .into_iter()
        .enumerate()
        .map(|(index, grid_no)| MapItem {
            id: WorldItemId(index as u32),
            grid_no,
            file_grid_no: None,
            level: Level::Ground,
            stack: ItemStack {
                item: ItemId::new(1).expect("Expected 1 to be a valid item id."),
                number: 1,
                data: [0; 12],
                attachments: vec![],
                flags: 0,
                mission: 0,
                trap: 0,
                imprint_id: 0,
                weight: 0,
                used: 0,
            },
            file_object: None,
            graphic: Some(tiles.gun.clone()),
            render_height: ItemHeight::ForceGround,
            visibility: Visibility::Visible,
            flags: WorldItemFlags::empty(),
            non_exist_chance: 0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omni_lights_reach_their_radius_in_every_direction() {
        let template = omni_light_template(8);
        let reach = |dx: i16, dy: i16| {
            template
                .lights
                .iter()
                .any(|node| node.dx == dx && node.dy == dy)
        };
        assert!(reach(8, 0) && reach(-8, 0) && reach(0, 8) && reach(0, -8));
        // The tiles are rounded, so they can be half a tile further away in both directions
        let max_distance = 8. + std::f32::consts::FRAC_1_SQRT_2;
        assert!(template.lights.iter().all(|node| {
            f32::from(node.dx).hypot(f32::from(node.dy)) <= max_distance && node.light > 0
        }));
        // Every ray starts with a marker and goes through nodes of the template
        assert_eq!(template.rays[0], LIGHT_NEW_RAY);
        assert!(template
            .rays
            .iter()
            .all(|&entry| entry == LIGHT_NEW_RAY || usize::from(entry) < template.lights.len()));
    }
}
