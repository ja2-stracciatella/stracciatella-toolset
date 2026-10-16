//! Smoothing of the ground textures (`Smooth.cc`)
//!
//! Each texture has full tiles (sub indices 1 to 10) and edge tiles that blend it with the
//! texture below. After a texture is painted, the tiles around it get the edge tile that matches
//! which of their neighbours have the texture.

use std::sync::Arc;

use crate::{
    components::map::{GridPosition, JA2_MAP_TILES},
    map::GridNo,
    tileset::{Tile, TileCategory},
};

use super::document::EditContext;

/// Edge tiles of the land textures (`gbSmoothStruct`)
///
/// Each row is: the sum of the keys of the neighbours without the texture (3 above, 5 right,
/// 7 below, 11 left), the number of variations, and the sub indices of the variations. The last
/// two columns are unused.
const SMOOTH_STRUCT: [[i16; 6]; 15] = [
    [3, 2, 12, 27, 12, 0],
    [5, 2, 15, 30, 39, 0],
    [7, 2, 17, 32, 41, 0],
    [11, 2, 14, 29, 14, 0],
    [8, 2, 13, 28, 38, 0],
    [15, 1, 19, 0, 43, 0],
    [26, 1, 20, 0, 44, 0],
    [12, 2, 18, 33, 42, 0],
    [23, 1, 21, 0, 45, 0],
    [18, 2, 16, 31, 40, 0],
    [21, 1, 22, 0, 46, 0],
    [14, 2, 11, 26, 11, 0],
    [19, 1, 23, 0, 47, 0],
    [16, 2, 24, 34, 48, 0],
    [10, 2, 25, 35, 49, 0],
];

/// Shore tiles of the water (`gbSmoothWaterStruct`)
///
/// Each row is: the bits of the tiles around that have water, the number of variations, and the
/// sub indices of the variations. Some bit values appear more than once, the first row wins.
const SMOOTH_WATER_STRUCT: [[i16; 4]; 37] = [
    [1020, 1, 11, 0],
    [1000, 1, 12, 0],
    [510, 2, 13, 43],
    [190, 1, 14, 0],
    [894, 1, 15, 0],
    [622, 1, 16, 0],
    [1014, 2, 17, 41],
    [944, 1, 18, 24],
    [872, 1, 19, 0],
    [992, 1, 20, 0],
    [62, 1, 21, 0],
    [190, 2, 22, 14],
    [620, 1, 23, 0],
    [944, 1, 24, 18],
    [878, 2, 25, 32],
    [434, 1, 28, 0],
    [110, 1, 29, 0],
    [1010, 1, 30, 0],
    [876, 2, 31, 32],
    [878, 2, 32, 31],
    [1004, 2, 32, 31],
    [1006, 2, 33, 34],
    [1008, 2, 34, 33],
    [1016, 2, 33, 34],
    [126, 2, 35, 36],
    [254, 2, 35, 26],
    [638, 2, 36, 26],
    [438, 2, 38, 27],
    [446, 2, 37, 38],
    [950, 2, 37, 27],
    [864, 1, 39, 0],
    [1040, 1, 40, 0],
    [1014, 2, 41, 17],
    [432, 1, 42, 0],
    [510, 1, 43, 13],
    [54, 1, 44, 0],
    [108, 1, 45, 0],
];

const WORLD_COLS: i32 = 160;
const WORLD_MAX: i32 = JA2_MAP_TILES as i32;

/// Whether the tile at a grid number (with the game's arithmetic) has land of a type
fn has_type(context: &EditContext, grid_no: i32, type_name: &str) -> bool {
    GridNo::from_i32(grid_no)
        .is_some_and(|grid_no| context.find_type_in_land(grid_no, type_name).is_some())
}

/// Picks a variation of a smoothing table row, `None` if the tile already is one of them
fn pick_variation(context: &mut EditContext, row: &[i16], old_sub_index: u16) -> Option<u16> {
    let same_tile = i32::from(row[2]) == i32::from(old_sub_index)
        || (row[3] != 0 && i32::from(row[3]) == i32::from(old_sub_index));
    if same_tile {
        return None;
    }
    let variation = context.random(row[1] as u32) as usize;
    Some(row[2 + variation] as u16)
}

/// The tile that the land of a type should become, `None` if it stays (`SmoothTerrain`)
///
/// Full tiles are only smoothed if `force` is set.
pub fn smooth_terrain(
    context: &mut EditContext,
    grid_no: GridNo,
    type_name: &str,
    force: bool,
) -> Option<Arc<Tile>> {
    // Deep water is smoothed like land, like in the original
    if context.category_of(type_name) == Some(TileCategory::Water) {
        return smooth_water_terrain(context, grid_no, type_name, true);
    }
    let index = context.find_type_in_land(grid_no, type_name)?;
    let old_sub_index = context.land(grid_no)[index].sub_index;
    let full_tile = (1..=10).contains(&old_sub_index);
    if !force && full_tile {
        return None;
    }

    let g = grid_no.index() as i32;
    let mut key = 0;
    if g - WORLD_COLS >= 0 && !has_type(context, g - WORLD_COLS, type_name) {
        key += 3;
    }
    if (g + 1) % WORLD_COLS != 0 && !has_type(context, g + 1, type_name) {
        key += 5;
    }
    if g + WORLD_COLS < WORLD_MAX && !has_type(context, g + WORLD_COLS, type_name) {
        key += 7;
    }
    if g % WORLD_COLS != 0 && !has_type(context, g - 1, type_name) {
        key += 11;
    }

    let sub_index = match SMOOTH_STRUCT.iter().find(|row| row[0] == key) {
        Some(row) => pick_variation(context, row, old_sub_index)?,
        None if full_tile => return None,
        // A tile surrounded by its texture becomes a random full tile
        None => 1 + context.random(10) as u16,
    };
    context.tile(type_name, sub_index)
}

/// `SmoothTerrain` for water (`SmoothWaterTerrain`)
fn smooth_water_terrain(
    context: &mut EditContext,
    grid_no: GridNo,
    type_name: &str,
    force: bool,
) -> Option<Arc<Tile>> {
    let index = context.find_type_in_land(grid_no, type_name)?;
    let old_sub_index = context.land(grid_no)[index].sub_index;
    let full_tile = (1..=10).contains(&old_sub_index);
    if !force && full_tile {
        return None;
    }

    let g = grid_no.index() as i32;
    let has_top = g - WORLD_COLS >= 0;
    let has_right = (g + 1) % WORLD_COLS != 0;
    let has_bottom = g + WORLD_COLS < WORLD_MAX;
    let has_left = g % WORLD_COLS != 0;
    let neighbours = [
        (has_top, -WORLD_COLS, 4),
        (has_right, 1, 64),
        (has_bottom, WORLD_COLS, 256),
        (has_left, -1, 16),
        (has_right && has_top, -WORLD_COLS + 1, 8),
        (has_left && has_top, -WORLD_COLS - 1, 2),
        (has_right && has_bottom, WORLD_COLS + 1, 512),
        (has_left && has_bottom, WORLD_COLS - 1, 128),
        (true, 0, 32),
    ];
    let key = neighbours
        .iter()
        .filter(|&&(exists, offset, _)| exists && has_type(context, g + offset, type_name))
        .fold(0, |key, &(_, _, bit)| key | bit);

    let sub_index = match SMOOTH_WATER_STRUCT.iter().find(|row| row[0] == key) {
        Some(row) => pick_variation(context, row, old_sub_index)?,
        None if full_tile => return None,
        None => 1 + context.random(10) as u16,
    };
    context.tile(type_name, sub_index)
}

/// The tiles within `radius` of a tile, row by row, without wrapping into other rows
fn square(grid_no: GridNo, radius: u8) -> impl Iterator<Item = GridNo> {
    let position = grid_no.position();
    let (x, y) = (i32::from(position.x), i32::from(position.y));
    let radius = i32::from(radius);
    (-radius..=radius).flat_map(move |dy| {
        (-radius..=radius)
            .filter_map(move |dx| GridPosition::from_xy(x + dx, y + dy).map(GridNo::from))
    })
}

/// Smooths the land of a type around a tile (`SmoothTerrainRadius`)
pub fn smooth_terrain_radius(
    context: &mut EditContext,
    grid_no: GridNo,
    type_name: &str,
    radius: u8,
    force: bool,
) {
    // Floors don't need smoothing
    if context.category_of(type_name) == Some(TileCategory::Floor) {
        return;
    }
    for neighbour in square(grid_no, radius) {
        if context.find_type_in_land(neighbour, type_name).is_none() {
            continue;
        }
        if let Some(tile) = smooth_terrain(context, neighbour, type_name, force) {
            context.set_land_index(neighbour, tile);
        }
    }
}

/// Smooths the land textures (not the water) around a tile (`SmoothAllTerrainTypeRadius`)
pub fn smooth_all_terrain_type_radius(
    context: &mut EditContext,
    grid_no: GridNo,
    radius: u8,
    force: bool,
) {
    // The land textures in the order of their types. The next one is looked up after each, so a
    // background tile that smoothing adds is smoothed too if its type comes later.
    let mut previous: Option<String> = None;
    while let Some(type_name) = next_ground_type(context, grid_no, radius, previous.as_deref()) {
        for neighbour in square(grid_no, radius) {
            if context.find_type_in_land(neighbour, &type_name).is_none() {
                continue;
            }
            if let Some(tile) = smooth_terrain(context, neighbour, &type_name, force) {
                context.set_land_index(neighbour, tile);
            }
        }
        previous = Some(type_name);
    }
}

/// The first land texture type (in the order of the tile types) within `radius` of a tile that
/// comes after `previous`
fn next_ground_type(
    context: &EditContext,
    grid_no: GridNo,
    radius: u8,
    previous: Option<&str>,
) -> Option<String> {
    let start = |type_name: &str| context.tileset.type_start(type_name);
    let previous = previous.and_then(start);
    square(grid_no, radius)
        .flat_map(|neighbour| context.land(neighbour))
        .filter(|tile| tile.category == Some(TileCategory::Ground))
        .filter_map(|tile| Some((start(&tile.type_name)?, &tile.type_name)))
        .filter(|&(tile_start, _)| previous.is_none_or(|previous| tile_start > previous))
        .min()
        .map(|(_, type_name)| type_name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::document::tests::{grid, Fixture};

    /// Adds a tile of a type to the head of the land of a tile
    fn paint(fixture: &mut Fixture, grid_no: GridNo, type_name: &str, sub_index: u16) {
        let mut context = fixture.context();
        let tile = context.tile(type_name, sub_index).unwrap();
        context.add_land_to_head(grid_no, tile);
        context.finish();
    }

    fn smoothed(fixture: &mut Fixture, grid_no: GridNo, type_name: &str) -> Option<u16> {
        smooth_terrain(&mut fixture.context(), grid_no, type_name, true).map(|tile| tile.sub_index)
    }

    #[test]
    fn edge_tiles_follow_the_neighbours_without_the_texture() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        // Alone: no neighbour has the texture, 3 + 5 + 7 + 11 = 26
        paint(&mut fixture, center, "TEXTURE2", 19);
        assert_eq!(smoothed(&mut fixture, center, "TEXTURE2"), Some(20));
        // The tile is already the edge tile of its key
        paint(&mut fixture, center, "TEXTURE2", 20);
        assert_eq!(fixture.land(center).len(), 3);
        assert_eq!(smoothed(&mut fixture, center, "TEXTURE2"), None);

        // Only the tile above lacks the texture: 3, two variations
        let mut fixture = Fixture::new();
        for (x, y) in [(50, 50), (51, 50), (50, 51), (49, 50)] {
            paint(&mut fixture, grid(x, y), "TEXTURE2", 1);
        }
        let sub_index = smoothed(&mut fixture, center, "TEXTURE2").unwrap();
        assert!([12, 27].contains(&sub_index));

        // Surrounded by the texture: a random full tile, unless it already is one
        paint(&mut fixture, grid(50, 49), "TEXTURE2", 1);
        assert!(smooth_terrain(&mut fixture.context(), center, "TEXTURE2", false).is_none());
        let mut fixture = Fixture::new();
        for (x, y) in [(50, 49), (51, 50), (50, 51), (49, 50)] {
            paint(&mut fixture, grid(x, y), "TEXTURE2", 1);
        }
        paint(&mut fixture, center, "TEXTURE2", 19);
        let sub_index = smoothed(&mut fixture, center, "TEXTURE2").unwrap();
        assert!((1..=10).contains(&sub_index));
    }

    #[test]
    fn neighbours_off_the_map_count_as_having_the_texture() {
        let mut fixture = Fixture::new();
        // Top left corner: only right (5) and below (7) are checked
        paint(&mut fixture, grid(0, 0), "TEXTURE2", 19);
        let sub_index = smoothed(&mut fixture, grid(0, 0), "TEXTURE2").unwrap();
        assert!([18, 33].contains(&sub_index));
        // The right edge doesn't look at the next row
        paint(&mut fixture, grid(159, 10), "TEXTURE2", 19);
        paint(&mut fixture, grid(0, 11), "TEXTURE2", 1);
        // 3 + 7 + 11 = 21
        assert_eq!(smoothed(&mut fixture, grid(159, 10), "TEXTURE2"), Some(22));
    }

    #[test]
    fn water_takes_the_first_matching_row() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        // Water everywhere except the row above: 16 + 32 + 64 + 128 + 256 + 512 = 1008
        for y in 50..=51 {
            for x in 49..=51 {
                paint(&mut fixture, grid(x, y), "WATER1", 1);
            }
        }
        let sub_index = smoothed(&mut fixture, center, "WATER1").unwrap();
        assert!([34, 33].contains(&sub_index));

        // 944 appears twice, the first row has one variation
        let mut fixture = Fixture::new();
        // 16 + 32 + 128 + 256 + 512 = 944
        for (x, y) in [(49, 50), (50, 50), (49, 51), (50, 51), (51, 51)] {
            paint(&mut fixture, grid(x, y), "WATER1", 1);
        }
        assert_eq!(smoothed(&mut fixture, center, "WATER1"), Some(18));
        // Water is always smoothed, even full tiles and without force
        assert_eq!(
            smooth_terrain(&mut fixture.context(), center, "WATER1", false)
                .map(|tile| tile.sub_index),
            Some(18)
        );
    }

    #[test]
    fn radius_smoothing_changes_the_land_of_the_type_around() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        paint(&mut fixture, center, "TEXTURE2", 19);
        paint(&mut fixture, grid(51, 50), "TEXTURE2", 19);
        let mut context = fixture.context();
        smooth_terrain_radius(&mut context, center, "TEXTURE2", 1, true);
        context.finish();
        // Right neighbour has it: 3 + 7 + 11 = 21, left neighbour has it: 3 + 5 + 7 = 15
        assert_eq!(fixture.land(center).last(), Some(&("TEXTURE2", 22)));
        assert_eq!(fixture.land(grid(51, 50)).last(), Some(&("TEXTURE2", 19)));
        assert_eq!(fixture.document.undo.depth(), 1);
    }
}
