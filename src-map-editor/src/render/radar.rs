//! Radar maps, the small images of the maps that the game shows in the strategic screen
//!
//! The game's map utility (`MapUtility.cc`) draws the whole map with shrunk tiles at a fifth of
//! the size into a 640×320 overhead image (`RenderOverheadMap` in `Overhead_Map.cc`), samples
//! that down to 88×44 pixels, reduces the colours to a palette and writes the result as an STI to
//! `radarmaps/`. Here the map is drawn at full size instead (`systems/radar.rs`) with the
//! overhead draw order and lighting, then shrunk to the overhead image by averaging. The rest is
//! a port of the map utility.

use std::io::{self, Cursor};

use bevy::prelude::*;

use crate::{
    components::map::GridPosition,
    file_formats::stci::{Stci, StciPalette, StciRgb888, StciSubImage},
    map::{GridNo, TacticalMap},
    render::{
        draw_list::{
            ground_draw_position, sprite_bounds, Blitter, Draw, DrawSprite, PaletteKey,
            SpriteGeometry, SpriteKey, WALL_HEIGHT,
        },
        lighting::{base_light, Lighting},
        movement_costs::world_screen_bounds,
        quantize::quantize,
        world::{LevelNode, MapWorld, NodeFlags},
    },
};

/// Size of a radar map (`MINIMAP_X_SIZE`, `MINIMAP_Y_SIZE`)
pub const RADAR_WIDTH: usize = 88;
pub const RADAR_HEIGHT: usize = 44;

/// Size of the overhead image that the radar map is sampled from. The game uses the screen
/// width, the radar maps of the game were made at 640.
const OVERHEAD_WIDTH: usize = 640;
const OVERHEAD_HEIGHT: usize = 320;
/// The overhead image is a fifth of the size of the map
const OVERHEAD_SCALE: usize = 5;

/// Game pixel at the top left corner of the overhead image: the tile box of tile (0, 80), where
/// `RenderOverheadMap` starts
pub const OVERHEAD_ORIGIN: IVec2 = IVec2::new(-1600, 800);

/// Ambient light level of the maps above ground in the map utility (`LoadWorld`)
const RADAR_AMBIENT_LEVEL: u8 = 4;

/// Half of the window that is averaged for each radar pixel (`WINDOW_SIZE`)
const WINDOW_SIZE: i32 = 2;

/// Game pixel area of the overhead image
pub fn radar_rect() -> IRect {
    let size = IVec2::new(
        (OVERHEAD_WIDTH * OVERHEAD_SCALE) as i32,
        (OVERHEAD_HEIGHT * OVERHEAD_SCALE) as i32,
    );
    IRect::from_corners(OVERHEAD_ORIGIN, OVERHEAD_ORIGIN + size)
}

/// Whether `RenderOverheadMap` draws a tile: it walks 160 rows of 80 tiles from tile (0, 80)
pub fn is_overhead_tile(position: GridPosition) -> bool {
    let (x, y) = (i32::from(position.x), i32::from(position.y));
    (80..240).contains(&(x + y)) && (-80..80).contains(&(x - y))
}

/// The lighting of the map utility: no lights are added, the maps above ground have a fixed
/// ambient light and the static shadows are always drawn
pub fn radar_lighting(map: &TacticalMap) -> Lighting {
    Lighting {
        ambient: map
            .ambient_light
            .as_ref()
            .map_or(RADAR_AMBIENT_LEVEL, |ambient| ambient.level),
        shadows: true,
        light_color: map.light_color,
    }
}

/// Lights all nodes like the map utility does (`LightSetBaseLevel` without light sprites)
pub fn light_for_radar(world: &mut MapWorld, map: &TacticalMap) -> Lighting {
    let lighting = radar_lighting(map);
    for tile in world.tiles.values_mut() {
        base_light(tile, &lighting);
    }
    lighting
}

/// The layers of `RenderOverheadMap`, each pass goes over all rows of the map
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverheadLayer {
    Land,
    Objects,
    Shadows,
    Structs,
    Roofs,
}

/// The passes over the map, the layers of a pass are drawn tile by tile
const OVERHEAD_PASSES: [&[OverheadLayer]; 3] = [
    &[OverheadLayer::Land],
    &[
        OverheadLayer::Objects,
        OverheadLayer::Shadows,
        OverheadLayer::Structs,
    ],
    &[OverheadLayer::Roofs],
];

/// Position of a draw in the overhead draw list: by pass, row, tile and layer within the tile
fn overhead_key(pass: usize, position: GridPosition, layer: usize, node: usize) -> u64 {
    let row = u64::from(position.x) + u64::from(position.y);
    ((pass as u64) << 48)
        | (row << 36)
        | (u64::from(position.x) << 24)
        | ((layer as u64) << 20)
        | node as u64
}

/// The draws of `RenderOverheadMap` at full size, in its order
///
/// There is no z buffer: land, then objects, shadows and structs tile by tile, then all roofs.
/// Items, soldiers, the tiles on the roofs and the topmost tiles are not drawn, animated tiles
/// are drawn with their first frame. Like the game's blitter, sprites that reach past the top or
/// left edge of the overhead image are left out.
pub fn overhead_draws(
    world: &MapWorld,
    sprites: impl Fn(SpriteKey) -> Option<SpriteGeometry>,
) -> Vec<Draw> {
    let origin = radar_rect().min;
    let mut draws = Vec::new();
    for grid_no in GridNo::all() {
        let position = grid_no.position();
        if !is_overhead_tile(position) {
            continue;
        }
        let tile = world.tile(position);
        let height = i32::from(tile.height);
        let tile_box = position.tile_box();
        for (pass, layers) in OVERHEAD_PASSES.iter().enumerate() {
            for (layer_index, &layer) in layers.iter().enumerate() {
                let nodes: Box<dyn Iterator<Item = &LevelNode>> = match layer {
                    // The land list is drawn from its start towards the head
                    OverheadLayer::Land => match tile.land_start {
                        Some(start) => Box::new(tile.land[..=start].iter().rev()),
                        None => Box::new(std::iter::empty()),
                    },
                    OverheadLayer::Objects => Box::new(tile.objects.iter()),
                    OverheadLayer::Shadows => Box::new(tile.shadows.iter()),
                    OverheadLayer::Structs => Box::new(tile.structs.iter()),
                    OverheadLayer::Roofs => Box::new(tile.roofs.iter()),
                };
                let nodes = nodes.filter(|node| !node.flags.contains(NodeFlags::ITEM));
                for (index, node) in nodes.enumerate() {
                    let draw_position = match layer {
                        OverheadLayer::Objects | OverheadLayer::Structs => {
                            ground_draw_position(position, tile, &node.tile)
                        }
                        OverheadLayer::Land | OverheadLayer::Shadows => {
                            tile_box - IVec2::new(0, height)
                        }
                        OverheadLayer::Roofs => tile_box - IVec2::new(0, height + WALL_HEIGHT),
                    };
                    let key = SpriteKey::Tile(node.tile.id);
                    let Some(geometry) = sprites(key) else {
                        continue;
                    };
                    let bounds = sprite_bounds(draw_position, geometry);
                    if bounds.min.x < origin.x || bounds.min.y < origin.y {
                        continue;
                    }
                    let blitter = if layer == OverheadLayer::Shadows {
                        Blitter::Shadow
                    } else {
                        Blitter::Transparent
                    };
                    draws.push((
                        overhead_key(pass, position, layer_index, index),
                        Draw {
                            position: draw_position,
                            sprite: DrawSprite::Fixed(key),
                            palette: PaletteKey::Tile(node.tile.image.surface.id),
                            shade: node.light.shade,
                            blitter,
                            z: 0,
                            z_strip: None,
                            outline: [0; 3],
                            bounds,
                            grid_no,
                        },
                    ));
                }
            }
        }
    }
    draws.sort_unstable_by_key(|(key, _)| *key);
    draws.into_iter().map(|(_, draw)| draw).collect()
}

/// Shrinks the map drawn over [`radar_rect`] to the overhead image
///
/// `rgba` has `stride` bytes per row. Each overhead pixel is the average of its 5×5 pixels,
/// truncated to the 16 bit colours of the game's frame buffer.
pub fn overhead_image(rgba: &[u8], width: usize, height: usize, stride: usize) -> Vec<[u8; 3]> {
    let mut image = vec![[0; 3]; OVERHEAD_WIDTH * OVERHEAD_HEIGHT];
    let scale = OVERHEAD_SCALE;
    for y in 0..OVERHEAD_HEIGHT.min(height / scale) {
        for x in 0..OVERHEAD_WIDTH.min(width / scale) {
            let mut sums = [0u32; 3];
            for row in 0..scale {
                let row = &rgba[(y * scale + row) * stride..];
                for pixel in row[x * scale * 4..][..scale * 4].chunks_exact(4) {
                    for (sum, &channel) in sums.iter_mut().zip(pixel) {
                        *sum += u32::from(channel);
                    }
                }
            }
            let count = (scale * scale) as u32;
            let [r, g, b] = sums.map(|sum| (sum / count) as u8);
            // `GetRGBColor` of a 565 colour
            image[y * OVERHEAD_WIDTH + x] = [r & 0xF8, g & 0xFC, b & 0xF8];
        }
    }
    image
}

/// The part of the overhead image that a map with restricted scrolling shows, as left, top,
/// right and bottom (`CalculateRestrictedMapCoords`)
pub fn restricted_crop() -> (i32, i32, i32, i32) {
    // The game's map screen around the scroll area of a normal map
    const NORMAL_MAP_SCREEN_WIDTH: i32 = 3160;
    const NORMAL_MAP_SCREEN_HEIGHT: i32 = 1540;
    const NORMAL_MAP_SCREEN_X: i32 = 1580;
    const NORMAL_MAP_SCREEN_BY: i32 = 2400;
    const NORMAL_MAP_SCREEN_TY: i32 = 860;
    let scale = OVERHEAD_SCALE as i32;
    let (left, top, right, bottom) = world_screen_bounds(true);
    (
        (-NORMAL_MAP_SCREEN_X - left).abs() / scale,
        (NORMAL_MAP_SCREEN_TY - top).abs() / scale,
        (NORMAL_MAP_SCREEN_WIDTH - (NORMAL_MAP_SCREEN_X - right).abs()) / scale,
        (NORMAL_MAP_SCREEN_HEIGHT - (NORMAL_MAP_SCREEN_BY - bottom).abs()) / scale,
    )
}

/// Blackens what a map with restricted scrolling doesn't show
fn blacken_restricted(image: &mut [[u8; 3]]) {
    let (left, top, right, bottom) = restricted_crop();
    for (index, pixel) in image.iter_mut().enumerate() {
        let x = (index % OVERHEAD_WIDTH) as i32;
        let y = (index / OVERHEAD_WIDTH) as i32;
        if x < left || x >= right || y < top || y >= bottom {
            *pixel = [0; 3];
        }
    }
}

/// Samples the overhead image down to the radar map, row by row
///
/// Each radar pixel is the average of the 4×4 overhead pixels around its top left corner. Maps
/// with restricted scrolling only sample the part they show.
pub fn sample_radar(image: &[[u8; 3]], restricted: bool) -> Vec<[u8; 3]> {
    let (start, step) = if restricted {
        let (left, top, right, bottom) = restricted_crop();
        (
            Vec2::new(left as f32, top as f32),
            Vec2::new(
                (right - left) as f32 / RADAR_WIDTH as f32,
                (bottom - top) as f32 / RADAR_HEIGHT as f32,
            ),
        )
    } else {
        (
            Vec2::ZERO,
            Vec2::new(
                OVERHEAD_WIDTH as f32 / RADAR_WIDTH as f32,
                OVERHEAD_HEIGHT as f32 / RADAR_HEIGHT as f32,
            ),
        )
    };
    let mut radar = vec![[0; 3]; RADAR_WIDTH * RADAR_HEIGHT];
    // A window outside of the image keeps the previous colour
    let mut average = [0; 3];
    // The positions add up in floats like in the game, column by column
    let mut dx = start.x;
    for x in 0..RADAR_WIDTH {
        let mut dy = start.y;
        for y in 0..RADAR_HEIGHT {
            let (center_x, center_y) = (dx as i32, dy as i32);
            let mut sums = [0u32; 3];
            let mut count = 0;
            for window_x in center_x - WINDOW_SIZE..center_x + WINDOW_SIZE {
                for window_y in center_y - WINDOW_SIZE..center_y + WINDOW_SIZE {
                    if (0..OVERHEAD_WIDTH as i32).contains(&window_x)
                        && (0..OVERHEAD_HEIGHT as i32).contains(&window_y)
                    {
                        let pixel = image[window_y as usize * OVERHEAD_WIDTH + window_x as usize];
                        for (sum, channel) in sums.iter_mut().zip(pixel) {
                            *sum += u32::from(channel);
                        }
                        count += 1;
                    }
                }
            }
            if count > 0 {
                average = sums.map(|sum| (sum / count) as u8);
            }
            radar[y * RADAR_WIDTH + x] = average;
            dy += step.y;
        }
        dx += step.x;
    }
    radar
}

/// Writes a radar map as an 8 bit STI
pub fn encode_radar_sti(pixels: &[[u8; 3]]) -> io::Result<Vec<u8>> {
    let (colors, indices) = quantize(pixels);
    let mut palette = Box::new(StciPalette::default());
    for (entry, [r, g, b]) in palette.colors.iter_mut().zip(colors.iter()) {
        *entry = StciRgb888(*r, *g, *b);
    }
    let stci = Stci::Indexed {
        palette,
        sub_images: vec![StciSubImage {
            offset: (0, 0),
            dimensions: (RADAR_WIDTH as u16, RADAR_HEIGHT as u16),
            app_data: None,
            data: indices,
        }],
    };
    let mut output = Cursor::new(Vec::new());
    stci.to_output(&mut output)?;
    Ok(output.into_inner())
}

/// The radar map STI of a map drawn over [`radar_rect`], see [`overhead_image`] for `rgba`
pub fn radar_map(
    rgba: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    restricted: bool,
) -> io::Result<Vec<u8>> {
    let mut image = overhead_image(rgba, width, height, stride);
    if restricted {
        blacken_restricted(&mut image);
    }
    encode_radar_sti(&sample_radar(&image, restricted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::map::tile_box,
        file_formats::ja2_tile_types::FIRSTWALL,
        tileset::tests::{
            test_node, test_tileset, FIRSTFULLSTRUCT, FIRSTOSTRUCT, FIRSTROOF, FIRSTSHADOW,
            FIRSTTEXTURE, GUNS,
        },
    };

    fn tile_sprites(_: SpriteKey) -> Option<SpriteGeometry> {
        Some(SpriteGeometry {
            offset: IVec2::ZERO,
            size: UVec2::new(40, 20),
        })
    }

    fn grid_no(x: u8, y: u8) -> GridNo {
        GridNo::from(GridPosition::new(x, y))
    }

    #[test]
    fn overhead_geometry() {
        assert_eq!(tile_box(0, 80), OVERHEAD_ORIGIN);
        // One overhead step is 8 pixels to the right, the next row starts 4 to the right and 2
        // down
        assert_eq!(tile_box(1, 79) - tile_box(0, 80), IVec2::new(8, 0) * 5);
        assert_eq!(tile_box(1, 80) - tile_box(0, 80), IVec2::new(4, 2) * 5);
        assert_eq!(radar_rect().size(), IVec2::new(3200, 1600));
    }

    #[test]
    fn overhead_tiles_are_those_of_the_game() {
        // `RenderOverheadMap` with the size of the map utility
        let mut tiles = Vec::new();
        let (mut anchor_x, mut anchor_y) = (0i32, 80i32);
        let mut odd = false;
        for _ in 0..OVERHEAD_HEIGHT / 2 {
            let (mut x, mut y) = (anchor_x, anchor_y);
            let mut screen_x = if odd { 4 } else { 0 };
            while screen_x < OVERHEAD_WIDTH {
                if (0..160).contains(&x) && (0..160).contains(&y) {
                    tiles.push(grid_no(x as u8, y as u8));
                }
                screen_x += 8;
                x += 1;
                y -= 1;
            }
            if odd {
                anchor_y += 1;
            } else {
                anchor_x += 1;
            }
            odd = !odd;
        }
        tiles.sort();
        let ours: Vec<_> = GridNo::all()
            .filter(|g| is_overhead_tile(g.position()))
            .collect();
        assert_eq!(ours.len(), 12800);
        assert_eq!(tiles, ours);
    }

    #[test]
    fn restricted_crop_of_basements() {
        assert_eq!(restricted_crop(), (188, 94, 444, 212));
    }

    #[test]
    fn overhead_draw_order() {
        let tileset = test_tileset();
        let node = |tile_type, sub_index| test_node(&tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        let (a, b) = (grid_no(40, 60), grid_no(41, 60));
        for g in [a, b] {
            let tile = &mut world.tiles[g];
            tile.land = vec![node(FIRSTTEXTURE, 1)];
            tile.land_start = Some(0);
            tile.structs.push(node(FIRSTOSTRUCT, 1));
            tile.shadows.push(node(FIRSTSHADOW, 1));
            tile.objects.push(node(FIRSTFULLSTRUCT, 1));
            tile.roofs.push(node(FIRSTROOF, 1));
            tile.on_roofs.push(node(FIRSTWALL, 1));
            tile.topmost.push(node(FIRSTWALL, 1));
        }
        let mut item = node(GUNS, 1);
        item.flags |= NodeFlags::ITEM;
        world.tiles[a].structs.push(item);
        world.tiles[b].height = 30;
        // Outside of the overhead image
        world.tiles[grid_no(0, 0)].structs.push(node(FIRSTOSTRUCT, 1));

        let draws = overhead_draws(&world, tile_sprites);
        let order: Vec<_> = draws
            .iter()
            .map(|draw| (draw.grid_no, draw.blitter))
            .collect();
        let t = Blitter::Transparent;
        assert_eq!(
            order,
            vec![
                (a, t),
                (b, t),
                // Tile by tile, a is in an earlier row
                (a, t),
                (a, Blitter::Shadow),
                (a, t),
                (b, t),
                (b, Blitter::Shadow),
                (b, t),
                (a, t),
                (b, t),
            ]
        );
        assert!(draws.iter().all(|draw| draw.z == 0 && draw.z_strip.is_none()));
        // Raised by the terrain height, roofs a wall height higher
        let land = tile_box(41, 60) - IVec2::new(0, 30);
        assert_eq!(draws[1].position, land);
        assert_eq!(draws[9].position, land - IVec2::new(0, WALL_HEIGHT));
    }

    #[test]
    fn sprites_past_the_top_or_left_are_left_out() {
        let tileset = test_tileset();
        let mut world = MapWorld::new(false);
        // The top corner of the overhead image
        let top = grid_no(40, 40);
        world.tiles[top].land = vec![test_node(&tileset, FIRSTTEXTURE, 1)];
        world.tiles[top].land_start = Some(0);
        let left = grid_no(0, 80);
        world.tiles[left].land = vec![test_node(&tileset, FIRSTTEXTURE, 1)];
        world.tiles[left].land_start = Some(0);
        assert_eq!(overhead_draws(&world, tile_sprites).len(), 2);
        let raised = |dx: i32, dy: i32| {
            move |_: SpriteKey| {
                Some(SpriteGeometry {
                    offset: IVec2::new(dx, dy),
                    size: UVec2::new(40, 20),
                })
            }
        };
        assert!(overhead_draws(&world, raised(0, -1))
            .iter()
            .all(|draw| draw.grid_no != top));
        assert!(overhead_draws(&world, raised(-1, 0))
            .iter()
            .all(|draw| draw.grid_no != left));
    }

    /// An RGBA image of the overhead area in one colour, with padded rows
    fn uniform_rgba(color: [u8; 3]) -> (Vec<u8>, usize) {
        let (width, height) = (OVERHEAD_WIDTH * 5, OVERHEAD_HEIGHT * 5);
        let stride = width * 4 + 256;
        let mut rgba = vec![0; stride * height];
        for row in rgba.chunks_mut(stride) {
            for pixel in row[..width * 4].chunks_exact_mut(4) {
                pixel.copy_from_slice(&[color[0], color[1], color[2], 255]);
            }
        }
        (rgba, stride)
    }

    #[test]
    fn overhead_pixels_average_and_truncate() {
        let (mut rgba, stride) = uniform_rgba([200, 100, 50]);
        // Half of the first overhead pixel is white
        for y in 0..5 {
            for x in 0..2 {
                rgba[y * stride + x * 4..][..3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let image = overhead_image(&rgba, 3200, 1600, stride);
        assert_eq!(image.len(), OVERHEAD_WIDTH * OVERHEAD_HEIGHT);
        // (3 * 200 + 2 * 255) / 5 = 222, (3 * 100 + 2 * 255) / 5 = 162, (3 * 50 + 2 * 255) / 5 = 132
        assert_eq!(image[0], [222 & 0xF8, 162 & 0xFC, 132 & 0xF8]);
        assert_eq!(image[1], [200, 100, 48]);
    }

    #[test]
    fn sampling_a_uniform_image() {
        let image = vec![[200, 100, 48]; OVERHEAD_WIDTH * OVERHEAD_HEIGHT];
        for restricted in [false, true] {
            let radar = sample_radar(&image, restricted);
            assert_eq!(radar.len(), RADAR_WIDTH * RADAR_HEIGHT);
            assert!(radar.iter().all(|&pixel| pixel == [200, 100, 48]));
        }
    }

    #[test]
    fn sampling_windows() {
        // A white column at x = 0, the first window covers x -2 to 1 of which 0 and 1 are inside
        let mut image = vec![[0; 3]; OVERHEAD_WIDTH * OVERHEAD_HEIGHT];
        for y in 0..OVERHEAD_HEIGHT {
            image[y * OVERHEAD_WIDTH] = [240, 240, 240];
        }
        let radar = sample_radar(&image, false);
        // 2 of 4 pixels at the corner, 4 of 8 below it
        assert_eq!(radar[0], [120, 120, 120]);
        assert_eq!(radar[RADAR_WIDTH], [120, 120, 120]);
        assert_eq!(radar[1], [0, 0, 0]);

        // Restricted maps start sampling at the crop
        let (left, top, _, _) = restricted_crop();
        let mut image = vec![[0; 3]; OVERHEAD_WIDTH * OVERHEAD_HEIGHT];
        image[top as usize * OVERHEAD_WIDTH + left as usize] = [160, 160, 160];
        let radar = sample_radar(&image, true);
        assert_eq!(radar[0], [10, 10, 10]);
        // The steps are smaller than the windows
        let lit: Vec<_> = (0..radar.len())
            .filter(|&index| radar[index] != [0, 0, 0])
            .collect();
        assert_eq!(lit, vec![0, 1, RADAR_WIDTH, RADAR_WIDTH + 1]);
    }

    #[test]
    fn restricted_maps_are_blackened() {
        let (rgba, stride) = uniform_rgba([200, 100, 48]);
        let sti = radar_map(&rgba, 3200, 1600, stride, true).unwrap();
        let Stci::Indexed {
            palette,
            sub_images,
        } = Stci::from_input(&mut sti.as_slice()).unwrap()
        else {
            panic!("not indexed");
        };
        let pixels = &sub_images[0].data;
        let color = |x: usize, y: usize| {
            let StciRgb888(r, g, b) = palette.colors[usize::from(pixels[y * RADAR_WIDTH + x])];
            [r, g, b]
        };
        // The windows at the edges of the crop reach into the black
        assert_eq!(color(0, 0), [200 / 4, 100 / 4, 48 / 4]);
        assert_eq!(color(40, 20), [200, 100, 48]);
    }

    /// The colours of the pixels of a radar map STI
    fn radar_colors(sti: &[u8]) -> Vec<[u8; 3]> {
        let Stci::Indexed {
            palette,
            sub_images,
        } = Stci::from_input(&mut io::Cursor::new(sti)).unwrap()
        else {
            panic!("not indexed");
        };
        sub_images[0]
            .data
            .iter()
            .map(|&index| {
                let StciRgb888(r, g, b) = palette.colors[usize::from(index)];
                [r, g, b]
            })
            .collect()
    }

    /// Renders the radar maps of the maps in the game data in `JA2_DATA_DIR` (or the maps whose
    /// name starts with `JA2_RADAR_MAPS`) on the CPU and compares them with the radar maps of the
    /// game, see `generated_edge_points_match_the_maps_of_the_game` for the directory
    ///
    /// With `JA2_RADAR_OUT`, the overhead images and both radar maps are written there as PPM
    /// files. Run with `--release`, replaying all maps takes a while.
    #[test]
    fn radar_maps_are_close_to_those_of_the_game() {
        use std::collections::HashMap;

        use crate::{
            file_formats::{ja2_map::Ja2Map, ja2_tileset::Ja2Tilesets},
            map::convert_tests::Fixture,
            render::{
                atlas::{GpuScene, SpriteSheets},
                bins::Bins,
                edge_points::tests::{data_tileset, find_file},
                palette::pack_rgba,
            },
            tileset::{StciSheet, TilesetId},
        };

        let Ok(dir) = std::env::var("JA2_DATA_DIR") else {
            eprintln!("JA2_DATA_DIR not set, skipping");
            return;
        };
        let prefix = std::env::var("JA2_RADAR_MAPS").unwrap_or_default();
        let out = std::env::var("JA2_RADAR_OUT").ok();
        let dir = std::path::Path::new(&dir);
        let ja2set = find_file(dir, "binarydata/ja2set.dat")
            .or_else(|| find_file(dir, "tilesets/ja2set.dat"))
            .expect("ja2set.dat should exist");
        let tilesets = Ja2Tilesets::parse(&std::fs::read(ja2set).unwrap()).unwrap();
        let mut fixtures: HashMap<TilesetId, (Fixture, Vec<Option<StciSheet>>)> = HashMap::new();
        let mut paths: Vec<_> = std::fs::read_dir(find_file(dir, "maps").unwrap())
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path.file_name().unwrap().to_string_lossy().to_lowercase();
                name.ends_with(".dat") && name.starts_with(&prefix.to_lowercase())
            })
            .collect();
        paths.sort();

        let write_ppm = |name: &str, width: usize, pixels: &[[u8; 3]]| {
            let Some(out) = &out else {
                return;
            };
            let height = pixels.len() / width;
            let mut bytes = format!("P6\n{} {}\n255\n", width, height).into_bytes();
            bytes.extend(pixels.iter().flatten());
            std::fs::write(std::path::Path::new(out).join(name), bytes).unwrap();
        };

        let mut differences = vec![];
        for path in paths {
            let name = path.file_stem().unwrap().to_string_lossy().to_lowercase();
            let Some(vanilla) = find_file(dir, &format!("radarmaps/{}.sti", name)) else {
                continue;
            };
            let Ok(file) = Ja2Map::parse(&std::fs::read(&path).unwrap()) else {
                continue;
            };
            let tileset_id = TilesetId::new(file.tileset_id as u8);
            let (fixture, sheets) = fixtures.entry(tileset_id).or_insert_with(|| {
                let tileset = data_tileset(dir, &tilesets, tileset_id);
                let sheets = (0..tileset.surfaces().len())
                    .map(|_| None)
                    .collect::<Vec<Option<StciSheet>>>();
                let mut sheets = sheets;
                for tile_type in 0..crate::file_formats::ja2_tile_types::NUMBER_OF_TILE_TYPES {
                    let (Some(surface), Some(resolved)) = (
                        tileset.surface_of_type(tile_type),
                        tilesets.resolve_tile_surface(tileset_id, tile_type),
                    ) else {
                        continue;
                    };
                    let file = find_file(
                        dir,
                        &format!("tilesets/{}/{}", resolved.tileset_id, resolved.filename),
                    );
                    sheets[surface.id.index()] = file
                        .and_then(|file| StciSheet::parse(&std::fs::read(file).ok()?).ok());
                }
                (Fixture::with_tileset(tileset), sheets)
            });
            let map = fixture.convert(&file);
            let mut world = MapWorld::build(&map);
            let lighting = light_for_radar(&mut world, &map);
            let sprite_sheets = SpriteSheets {
                tileset: &fixture.tileset,
                surface_sheets: sheets.iter().map(Option::as_ref).collect(),
                soldier_sheets: vec![],
                soldier_palettes: vec![],
            };
            let draws = overhead_draws(&world, |key| sprite_sheets.geometry(key));
            let rect = radar_rect();
            let bins = Bins::build(rect, draws.iter().map(|draw| draw.bounds));
            let scene = GpuScene::build(&draws, &bins, &sprite_sheets, lighting.light_color);
            let size = rect.size().as_uvec2();
            let (width, height) = (size.x as usize, size.y as usize);
            let clear = pack_rgba([0, 0, 0]);
            let mut rgba = vec![0u8; width * height * 4];
            for y in 0..height {
                for x in 0..width {
                    let color = scene.replay_pixel(rect.min + IVec2::new(x as i32, y as i32), clear);
                    rgba[(y * width + x) * 4..][..4].copy_from_slice(&color.to_le_bytes());
                }
            }

            let ours = radar_colors(
                &radar_map(&rgba, width, height, width * 4, world.restricted_scroll).unwrap(),
            );
            let theirs = radar_colors(&std::fs::read(vanilla).unwrap());
            let difference = ours
                .iter()
                .zip(&theirs)
                .flat_map(|(a, b)| a.iter().zip(b).map(|(&a, &b)| (i32::from(a) - i32::from(b)).abs()))
                .sum::<i32>() as f64
                / (ours.len() * 3) as f64;
            eprintln!("{}: mean difference {:.1}", name, difference);
            write_ppm(
                &format!("{}_overhead.ppm", name),
                OVERHEAD_WIDTH,
                &overhead_image(&rgba, width, height, width * 4),
            );
            write_ppm(&format!("{}_ours.ppm", name), RADAR_WIDTH, &ours);
            write_ppm(&format!("{}_game.ppm", name), RADAR_WIDTH, &theirs);
            differences.push((name, difference));
        }
        differences.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mean =
            differences.iter().map(|(_, d)| d).sum::<f64>() / differences.len().max(1) as f64;
        eprintln!(
            "{} maps, mean difference {:.1}, the largest: {:?}",
            differences.len(),
            mean,
            &differences[..differences.len().min(10)]
        );
        assert!(mean < 12., "the radar maps differ too much");
    }

    #[test]
    fn radar_sti() {
        let pixels: Vec<[u8; 3]> = (0..RADAR_WIDTH * RADAR_HEIGHT)
            .map(|i| [(i % 256) as u8, (i / 16) as u8, 0])
            .collect();
        let sti = encode_radar_sti(&pixels).unwrap();
        let Stci::Indexed {
            palette,
            sub_images,
        } = Stci::from_input(&mut sti.as_slice()).unwrap()
        else {
            panic!("not indexed");
        };
        assert_eq!(palette.colors[0], StciRgb888(0, 0, 0));
        assert_eq!(sub_images.len(), 1);
        let image = &sub_images[0];
        assert_eq!(image.dimensions, (88, 44));
        assert_eq!(image.offset, (0, 0));
        assert_eq!(image.data.len(), 88 * 44);
        // No pixel is transparent
        assert!(image.data.iter().all(|&index| index != 0));
    }
}
