//! Shade levels of the level nodes: the ambient light and the light sprites of the map
//!
//! Port of `LightSetBaseLevel`, `LightSpriteRenderAll`, `LightDraw`, `IlluminationFilter` and
//! `LightAddTile` (`Lighting.cc`), and of the light switching of `LoadMapLights` and
//! `ExtractLightSprite`. Lights combine by their maximum, so the order of the light sprites does
//! not matter.

use std::{collections::BTreeSet, ops::RangeInclusive};

use bevy::prelude::Resource;
use serde::Deserialize;

use crate::{
    components::map::GridPosition,
    light::{LightTemplate, LIGHT_BACKLIGHT, LIGHT_NEW_RAY},
    map::{Direction, GridArray, GridNo, LightTime, MapLight, TacticalMap},
    render::{
        movement_costs::{
            door_travel_cost, is_travelcost_door, is_travelcost_open_door, MovementCosts,
            TRAVELCOST_CAVEWALL, TRAVELCOST_DOOR, TRAVELCOST_OBSTACLE, TRAVELCOST_WALL,
        },
        world::{MapWorld, NodeLight, WorldTile},
    },
    tileset::{Orientation, StructureFlags, TileKind, PROFILE_Z_SIZE},
};

/// Ambient light level at day and at night, higher is darker (`NORMAL_LIGHTLEVEL_DAY`,
/// `NORMAL_LIGHTLEVEL_NIGHT`)
const NORMAL_LIGHTLEVEL_DAY: u8 = 3;
const NORMAL_LIGHTLEVEL_NIGHT: u8 = 12;
/// From this ambient level on, the static shadows are not drawn (`LIGHT_DUSK_CUTOFF`)
const LIGHT_DUSK_CUTOFF: u8 = 8;
/// Brightest and darkest shade level that lights produce (`SHADE_MAX`, `SHADE_MIN`)
const SHADE_MAX: i32 = 1;
const SHADE_MIN: i32 = 15;

/// A structure blocks light if this many columns of its profile are at least
/// [`STANDING_CUBES`] high (`MIN_BOX_COLUMNS`)
const MIN_BOX_COLUMNS: usize = 8;
const STANDING_CUBES: u8 = 3;

/// Time of day that the map is shown at
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightingMode {
    /// Daylight, only the lights without a time flag are on
    #[default]
    Day,
    /// Evening (21:00 to midnight), the night and prime time lights are on as well
    Night,
}

/// Lighting of the map that the renderer needs besides the node shades
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lighting {
    pub ambient: u8,
    /// Whether the static shadows are drawn (`RENDER_FLAG_SHADOWS`)
    pub shadows: bool,
    /// Added to all palettes (`g_light_color`)
    pub light_color: [u8; 3],
}

/// Ambient light level of a map
pub fn ambient_light_level(map: &TacticalMap, mode: LightingMode) -> u8 {
    match &map.ambient_light {
        // Only basements and caves have a saved level
        Some(ambient) => ambient.level,
        None => match mode {
            LightingMode::Day => NORMAL_LIGHTLEVEL_DAY,
            LightingMode::Night => NORMAL_LIGHTLEVEL_NIGHT,
        },
    }
}

/// Whether a light is on at the given time of day
fn light_on(time: LightTime, mode: LightingMode) -> bool {
    match mode {
        LightingMode::Day => time == LightTime::Always,
        LightingMode::Night => true,
    }
}

/// The lighting of a map at the given time of day, besides the node shades
pub fn map_lighting(map: &TacticalMap, mode: LightingMode) -> Lighting {
    let ambient = ambient_light_level(map, mode);
    Lighting {
        ambient,
        shadows: ambient < LIGHT_DUSK_CUTOFF,
        light_color: map.light_color,
    }
}

/// The light of every node before the light sprites are drawn (`LightSetBaseLevel`)
fn base_node_light(lighting: &Lighting) -> NodeLight {
    let shade = i32::from(lighting.ambient).clamp(SHADE_MAX, SHADE_MIN) as u8;
    NodeLight {
        shade,
        natural_shade: shade,
        max_lights: 0,
    }
}

/// Resets the nodes of a tile to the ambient light (`LightSetBaseLevel` for one tile)
pub fn base_light(tile: &mut WorldTile, lighting: &Lighting) {
    let base_light = base_node_light(lighting);
    let lists = [
        &mut tile.land,
        &mut tile.objects,
        &mut tile.structs,
        &mut tile.roofs,
        &mut tile.on_roofs,
        &mut tile.topmost,
    ];
    for node in lists.into_iter().flatten() {
        node.light = base_light;
    }
    for node in &mut tile.mercs {
        node.light = base_light;
    }
}

/// A light that the scene is lit with as if it was placed, without being a light of the map:
/// the light that a click of the lights mode would place
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PreviewLight(pub Option<MapLight>);

/// The lights that the scene is lit with: the lights of the map and the preview, which replaces
/// the lights of its tile like placing it does
pub fn scene_lights<'a>(
    map: &'a TacticalMap,
    preview: Option<&'a MapLight>,
) -> impl Iterator<Item = &'a MapLight> {
    let hidden = preview.map(|light| light.position);
    map.lights
        .iter()
        .filter(move |light| Some(light.position) != hidden)
        .chain(preview)
}

/// The scene lights (see [`scene_lights`]) that are on at the given time of day
pub fn active_lights<'a>(
    map: &'a TacticalMap,
    preview: Option<&'a MapLight>,
    mode: LightingMode,
) -> impl Iterator<Item = &'a MapLight> {
    // Caves have no lights on
    let caves = map.ambient_light.as_ref().is_some_and(|a| a.caves);
    scene_lights(map, preview).filter(move |light| !caves && light_on(light.time, mode))
}

/// Draws a light sprite onto the nodes it reaches (`LightDraw`)
///
/// Lights combine by their maximum, so drawing a light again changes nothing.
pub fn draw_light(world: &mut MapWorld, costs: &MovementCosts, light: &MapLight) {
    draw_light_filtered(world, costs, light, |_| true);
}

/// Sets the shade levels of all nodes for the map lighting at the given time of day, with the
/// preview light if there is one
pub fn light_all(
    world: &mut MapWorld,
    map: &TacticalMap,
    preview: Option<&MapLight>,
    costs: &MovementCosts,
    mode: LightingMode,
) -> Lighting {
    let lighting = map_lighting(map, mode);
    for tile in world.tiles.values_mut() {
        base_light(tile, &lighting);
    }
    for light in active_lights(map, preview, mode) {
        draw_light(world, costs, light);
    }
    lighting
}

/// How far a light can reach beyond the tiles of its template: the base tile of an open door is
/// lit instead of the door, and it can be 2 tiles away (`DoorTravelCost`)
const LIGHT_EXTENT_MARGIN: i32 = 2;

/// The tiles (as x and y ranges) that a light can light or look at, including the tiles its rays
/// pass through
fn light_extent(light: &MapLight) -> Option<(RangeInclusive<i32>, RangeInclusive<i32>)> {
    let lights = &light.template.lights;
    let (x, y) = (i32::from(light.position.x), i32::from(light.position.y));
    let min_x = lights.iter().map(|node| i32::from(node.dx)).min()?;
    let max_x = lights.iter().map(|node| i32::from(node.dx)).max()?;
    let min_y = lights.iter().map(|node| i32::from(node.dy)).min()?;
    let max_y = lights.iter().map(|node| i32::from(node.dy)).max()?;
    let margin = LIGHT_EXTENT_MARGIN;
    Some((
        (x + min_x - margin)..=(x + max_x + margin),
        (y + min_y - margin)..=(y + max_y + margin),
    ))
}

/// The tiles that a light can light or look at, see [`relight_area`]: once a light is added,
/// removed or changed, relighting these tiles brings the lighting up to date
pub fn light_tiles(light: &MapLight) -> Vec<GridNo> {
    let Some((xs, ys)) = light_extent(light) else {
        return vec![];
    };
    ys.flat_map(|y| xs.clone().filter_map(move |x| grid_no(x, y)))
        .collect()
}

fn in_extent(extent: &(RangeInclusive<i32>, RangeInclusive<i32>), grid_no: GridNo) -> bool {
    let position = grid_no.position();
    extent.0.contains(&i32::from(position.x)) && extent.1.contains(&i32::from(position.y))
}

/// Lights the nodes again after the tiles in `seed` changed: their nodes, structures, heights or
/// movement costs. The result is the same as `light_all`.
///
/// The rays of a light only depend on the tiles within its extent, so the lights whose extent
/// contains a seed tile can light different tiles now. Their extents and the seed tiles are
/// reset to the ambient light, and every light that reaches into them is drawn onto them again.
/// Returns the tiles whose lights may have changed.
pub fn relight_area(
    world: &mut MapWorld,
    map: &TacticalMap,
    preview: Option<&MapLight>,
    costs: &MovementCosts,
    mode: LightingMode,
    seed: &BTreeSet<GridNo>,
) -> BTreeSet<GridNo> {
    let lighting = map_lighting(map, mode);
    let lights: Vec<_> = active_lights(map, preview, mode)
        .filter_map(|light| Some((light, light_extent(light)?)))
        .collect();
    let mut area = seed.clone();
    for (_, extent) in &lights {
        if seed.iter().any(|&grid_no| in_extent(extent, grid_no)) {
            for y in extent.1.clone() {
                for x in extent.0.clone() {
                    if let Some(grid_no) = grid_no(x, y) {
                        area.insert(grid_no);
                    }
                }
            }
        }
    }

    let mut in_area = GridArray::filled(false);
    for &grid_no in &area {
        in_area[grid_no] = true;
        base_light(&mut world.tiles[grid_no], &lighting);
    }
    for (light, extent) in &lights {
        if area.iter().any(|&grid_no| in_extent(extent, grid_no)) {
            draw_light_filtered(world, costs, light, |grid_no| in_area[grid_no]);
        }
    }
    area
}

/// `draw_light` that only lights the tiles for which `lit` is true
fn draw_light_filtered(
    world: &mut MapWorld,
    costs: &MovementCosts,
    light: &MapLight,
    lit: impl Fn(GridNo) -> bool,
) {
    LightDrawing { world, costs, lit }.draw(&light.template, light.position);
}

/// Grid number of a tile position, `None` outside of the map (`MAPROWCOLTOPOS`)
fn grid_no(x: i32, y: i32) -> Option<GridNo> {
    GridPosition::from_xy(x, y).map(GridNo::from)
}

/// Direction from one tile to another (`atan8`)
fn atan8(x: i32, y: i32, x2: i32, y2: i32) -> Direction {
    use std::f64::consts::PI;
    let dx = f64::from(x2 - x);
    // Screen y points south, atan2 expects it to point north
    let dy = -f64::from(y2 - y);
    if dx == 0. && dy == 0. {
        return Direction::East;
    }
    let mut theta = dy.atan2(dx);
    if theta < 0. {
        theta += 2. * PI;
    }
    let index = ((8. * theta + PI) / (2. * PI)) as usize % 8;
    [
        Direction::East,
        Direction::NorthEast,
        Direction::North,
        Direction::NorthWest,
        Direction::West,
        Direction::SouthWest,
        Direction::South,
        Direction::SouthEast,
    ][index]
}

/// Which structures a ray from one tile to the next lights and whether it goes on
/// (`IlluminationFilter`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct IlluminationFilter {
    blocked: bool,
    illuminate_nothing: bool,
    /// Only walls and doors are lit
    illuminate_oriented_blocks_only: bool,
    allowed_orients: Orientation,
    source_to_destination: Option<Direction>,
    /// Base tile of an open door that is lit instead of the destination
    base_grid_no: Option<GridNo>,
}

struct LightDrawing<'a, L> {
    world: &'a mut MapWorld,
    costs: &'a MovementCosts,
    /// Whether the light is added to a tile, the rays go on either way
    lit: L,
}

impl<L: Fn(GridNo) -> bool> LightDrawing<'_, L> {
    fn filter(&self, src_x: i32, src_y: i32, x: i32, y: i32) -> IlluminationFilter {
        let mut filter = IlluminationFilter::default();
        let (Some(dst), Some(src)) = (grid_no(x, y), grid_no(src_x, src_y)) else {
            filter.blocked = true;
            filter.illuminate_nothing = true;
            return filter;
        };
        let tiles = &self.world.tiles;
        if tiles[dst].height > tiles[src].height {
            return filter;
        }

        let direction = atan8(src_x, src_y, x, y);
        filter.source_to_destination = Some(direction);
        let mut cost = self.costs.cost(dst, direction);

        let window_tile = if matches!(direction, Direction::North | Direction::West) {
            dst
        } else {
            src
        };
        // A window does not block
        if !tiles[window_tile].structs.is_empty()
            && cost == TRAVELCOST_WALL
            && tiles[window_tile]
                .find_structure(StructureFlags::WALLNWINDOW)
                .is_some()
        {
            if window_tile == dst {
                match direction {
                    Direction::North => filter.allowed_orients = Orientation::LEFT,
                    Direction::West => filter.allowed_orients = Orientation::RIGHT,
                    _ => {}
                }
            }
            return filter;
        }

        if dst == src {
            return filter;
        }

        // Walls only light the walls on the side of the light
        if cost == TRAVELCOST_WALL {
            filter.blocked = true;
            match direction {
                Direction::North => {
                    filter.illuminate_oriented_blocks_only = true;
                    filter.allowed_orients = Orientation::LEFT;
                }
                Direction::West => {
                    filter.illuminate_oriented_blocks_only = true;
                    filter.allowed_orients = Orientation::RIGHT;
                }
                _ => filter.illuminate_nothing = true,
            }
            return filter;
        }

        if is_travelcost_door(cost) {
            let (door_cost, base_grid_no) = door_travel_cost(self.world, self.costs, dst, cost);
            cost = door_cost;
            if is_travelcost_open_door(cost) && base_grid_no != Some(src) {
                // The door slab is lit from its base tile
                filter.base_grid_no = base_grid_no;
                filter.blocked = true;
                filter.illuminate_oriented_blocks_only = true;
                match direction {
                    Direction::North => filter.allowed_orients = Orientation::RIGHT,
                    Direction::West => filter.allowed_orients = Orientation::LEFT,
                    _ => filter.illuminate_nothing = true,
                }
                return filter;
            }
            // Closed doors light only the walls and doors on the side of the light
            if cost == TRAVELCOST_DOOR || is_travelcost_door(cost) {
                filter.blocked = true;
                filter.illuminate_oriented_blocks_only = true;
                match direction {
                    Direction::North => filter.allowed_orients = Orientation::LEFT,
                    Direction::West => filter.allowed_orients = Orientation::RIGHT,
                    _ => filter.illuminate_nothing = true,
                }
                return filter;
            }
        }

        if cost == TRAVELCOST_OBSTACLE {
            for structure in &tiles[dst].structures {
                // The height is only known if the game computed it before
                if structure.flags.intersects(
                    StructureFlags::PASSABLE
                        | StructureFlags::ANYDOOR
                        | StructureFlags::WALLSTUFF
                        | StructureFlags::TREE,
                ) || structure.structure_height < STANDING_CUBES
                    || structure.cube_offset == PROFILE_Z_SIZE as i16
                {
                    continue;
                }
                let Some(tile) = structure.structure_tile() else {
                    continue;
                };
                // Columns that are at least 3 cubes high
                let box_columns = tile
                    .shape
                    .iter()
                    .flatten()
                    .filter(|&&column| column == 0b0111 || column == 0b1111)
                    .count();
                if box_columns >= MIN_BOX_COLUMNS {
                    filter.blocked = true;
                    return filter;
                }
            }
        }

        if cost == TRAVELCOST_CAVEWALL {
            filter.blocked = true;
        }
        filter
    }

    /// Port of `LightDraw` for a light at `position`
    fn draw(&mut self, template: &LightTemplate, position: GridPosition) {
        const DRAWN_FULLY: u8 = 0x01;
        const DRAWN_NONWALL: u8 = 0x02;

        if template.lights.is_empty() {
            return;
        }
        let mut node_flags = vec![0u8; template.lights.len()];
        let (center_x, center_y) = (i32::from(position.x), i32::from(position.y));
        let (mut src_x, mut src_y) = (center_x, center_y);

        let mut i = 0;
        while i < template.rays.len() {
            let entry = template.rays[i];
            if entry & LIGHT_NEW_RAY != 0 {
                src_x = center_x;
                src_y = center_y;
                i += 1;
                continue;
            }
            let node_index = usize::from(entry & !LIGHT_BACKLIGHT);
            let Some(&node) = template.lights.get(node_index) else {
                i += 1;
                continue;
            };
            // The game computes the destination as INT16
            let dst_x = i32::from((center_x + i32::from(node.dx)) as i16);
            let dst_y = i32::from((center_y + i32::from(node.dy)) as i16);

            let filter = self.filter(src_x, src_y, dst_x, dst_y);
            if filter.blocked {
                // Skip the rest of the ray (`LightFindNextRay`), the loop skips its start
                // marker as well
                while i < template.rays.len() && template.rays[i] & LIGHT_NEW_RAY == 0 {
                    i += 1;
                }
            }

            let south_west = node.dx < 0 && node.dy > 0;
            let north_east = node.dx > 0 && node.dy < 0;
            let north_west = node.dx < 0 && node.dy < 0;
            let flags = &mut node_flags[node_index];
            if *flags & DRAWN_FULLY == 0
                && *flags & DRAWN_NONWALL != 0
                && (south_west
                    || north_east
                    || (north_west && !filter.illuminate_oriented_blocks_only))
            {
                *flags |= DRAWN_FULLY;
            }

            if !filter.illuminate_nothing && *flags & DRAWN_FULLY == 0 && node.light != 0 {
                let backlight = entry & LIGHT_BACKLIGHT != 0;
                if let Some(target) = filter.base_grid_no.or_else(|| grid_no(dst_x, dst_y)) {
                    if (self.lit)(target) {
                        self.add_tile(target, node.light, backlight, &filter);
                    }
                }

                let flags = &mut node_flags[node_index];
                if center_x == dst_x || center_y == dst_y || (node.dx > 0 && node.dy > 0) {
                    *flags |= DRAWN_FULLY;
                } else if filter.illuminate_oriented_blocks_only {
                } else if south_west || north_east {
                    if matches!(
                        filter.source_to_destination,
                        Some(Direction::West | Direction::North)
                    ) {
                        *flags |= DRAWN_FULLY;
                    } else {
                        *flags |= DRAWN_NONWALL;
                    }
                } else if north_west {
                    *flags |= DRAWN_NONWALL;
                }
            }

            src_x = center_x;
            src_y = center_y;
            if !filter.blocked {
                src_x += i32::from(node.dx);
                src_y += i32::from(node.dy);
            }
            i += 1;
        }
    }

    /// Port of `LightAddTile` for map lights (not roof only, not fake)
    fn add_tile(
        &mut self,
        grid_no: GridNo,
        light: u8,
        backlight: bool,
        filter: &IlluminationFilter,
    ) {
        let tile = &mut self.world.tiles[grid_no];
        let structures = &tile.structures;
        for node in &mut tile.structs {
            // `pStructureData` is the base tile of the structure
            let structure = node.structure_id.and_then(|id| {
                structures
                    .iter()
                    .find(|s| s.id == id && s.structure_tile_index == 0)
            });
            let (oriented_block, wall_orientation) = match structure {
                Some(structure) => (
                    structure
                        .flags
                        .intersects(StructureFlags::WALLSTUFF | StructureFlags::ANYDOOR),
                    structure.wall_orientation,
                ),
                None if filter.illuminate_oriented_blocks_only
                    && (matches!(node.tile.kind, TileKind::WallDecoration { .. })
                        || node.tile.lit_as_wall) =>
                {
                    (true, filter.allowed_orients)
                }
                None => (false, Orientation::empty()),
            };
            if (filter.illuminate_oriented_blocks_only
                && (!oriented_block || !wall_orientation.intersects(filter.allowed_orients)))
                || (oriented_block && filter.allowed_orients.is_empty())
                || node.tile.kind == (TileKind::Cliff { hang: true })
            {
                continue;
            }
            add_light(&mut node.light, light);
        }

        if !filter.illuminate_oriented_blocks_only {
            for node in tile.land.iter_mut().chain(&mut tile.objects) {
                add_light(&mut node.light, light);
            }
            let merc_light = if backlight {
                (i32::from(light) * 7 / 10) as u8
            } else {
                light
            };
            for node in &mut tile.mercs {
                add_light(&mut node.light, merc_light);
            }
        }
    }
}

/// Adds a light to a node (`LightAddTileNode`)
fn add_light(node: &mut NodeLight, light: u8) {
    node.max_lights = node.max_lights.max(light);
    node.shade = (i32::from(node.natural_shade) - i32::from(node.max_lights))
        .clamp(SHADE_MAX, SHADE_MIN) as u8;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{components::map::JA2_MAP_SIZE, map::LightId, tileset::TilesetId};

    fn light(id: u32, x: u8, y: u8, time: LightTime) -> MapLight {
        MapLight {
            id: LightId(id),
            position: GridPosition::new(x, y),
            time,
            template_name: "L-R01.LHT".to_owned(),
            template: Arc::new(LightTemplate::default()),
            file_sprite: None,
        }
    }

    #[test]
    fn the_preview_replaces_the_lights_of_its_tile() {
        let mut map = TacticalMap::empty(TilesetId::new(0));
        map.lights = vec![
            light(0, 10, 10, LightTime::Always),
            light(1, 20, 20, LightTime::Always),
            light(2, 10, 10, LightTime::Night),
        ];
        let ids = |preview: Option<&MapLight>| -> Vec<u32> {
            scene_lights(&map, preview)
                .map(|light| light.id.0)
                .collect()
        };
        assert_eq!(ids(None), [0, 1, 2]);
        assert_eq!(ids(Some(&light(3, 10, 10, LightTime::Always))), [1, 3]);
        assert_eq!(
            ids(Some(&light(3, 30, 30, LightTime::Always))),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn the_preview_is_only_on_when_a_placed_light_would_be() {
        let map = TacticalMap::empty(TilesetId::new(0));
        let prime_time = light(0, 10, 10, LightTime::PrimeTime);
        let active = |mode| active_lights(&map, Some(&prime_time), mode).count();
        assert_eq!(active(LightingMode::Day), 0);
        assert_eq!(active(LightingMode::Night), 1);
    }

    #[test]
    fn directions() {
        assert_eq!(atan8(5, 5, 5, 4), Direction::North);
        assert_eq!(atan8(5, 5, 6, 4), Direction::NorthEast);
        assert_eq!(atan8(5, 5, 6, 5), Direction::East);
        assert_eq!(atan8(5, 5, 6, 6), Direction::SouthEast);
        assert_eq!(atan8(5, 5, 5, 6), Direction::South);
        assert_eq!(atan8(5, 5, 4, 6), Direction::SouthWest);
        assert_eq!(atan8(5, 5, 4, 5), Direction::West);
        assert_eq!(atan8(5, 5, 4, 4), Direction::NorthWest);
        assert_eq!(atan8(5, 5, 5, 5), Direction::East);
        // Each direction covers 45 degrees
        assert_eq!(atan8(0, 0, 3, -1), Direction::East);
        assert_eq!(atan8(0, 0, 2, -1), Direction::NorthEast);
        assert_eq!(atan8(0, 0, -1, 3), Direction::South);
    }

    #[test]
    fn grid_numbers_outside_of_the_map() {
        assert_eq!(grid_no(0, 0), GridNo::new(0));
        assert_eq!(grid_no(3, 2), GridNo::new(323));
        assert_eq!(grid_no(i32::from(JA2_MAP_SIZE), 0), None);
        assert_eq!(grid_no(-1, 5), None);
    }

    #[test]
    fn lights_combine_by_maximum() {
        let mut node = NodeLight {
            shade: 12,
            natural_shade: 12,
            max_lights: 0,
        };
        add_light(&mut node, 3);
        assert_eq!(node.shade, 9);
        add_light(&mut node, 2);
        assert_eq!(node.shade, 9);
        add_light(&mut node, 20);
        assert_eq!(node.shade, 1);
    }
}
