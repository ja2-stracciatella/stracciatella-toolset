//! Movement costs of the ground level, like the game keeps them in `gubWorldMovementCosts`
//!
//! Port of `CompileWorldMovementCosts`, `CompileTileMovementCosts` and `CompileWorldTerrainIDs`
//! (`WorldDef.cc`), `GridNoOnVisibleWorldTile` (`Isometric_Utils.cc`) with the bounds of
//! `InitRenderParams` (`RenderWorld.cc`), and `DoorTravelCost` (`PathAI.cc`). Only level 0 is
//! compiled, the roof level never influences it.
//!
//! The game writes to neighbouring tiles without bounds checks. Writes outside of the map are
//! skipped here, and reads outside of the map count as "no structure" or "not blocked".

use std::collections::BTreeSet;

use bevy::prelude::{IVec2, Resource};

use crate::{
    components::map::JA2_MAP_SIZE,
    map::{
        Direction::{self, East, North, NorthEast, NorthWest, South, SouthEast, SouthWest, West},
        Door, GridArray, GridNo, TacticalMap,
    },
    render::world::MapWorld,
    tileset::{Orientation, StructureFlags, Terrain, TilesetId},
};

const WORLD_COLS: i32 = JA2_MAP_SIZE as i32;
const WORLD_ROWS: i32 = JA2_MAP_SIZE as i32;
const CELL_X_SIZE: i32 = 10;
const CELL_Y_SIZE: i32 = 10;

/// World cell at the center of the map (`gCenterWorldX`, `gCenterWorldY`)
const CENTER_WORLD: (i32, i32) = (800, 800);
/// Center of the map in screen coordinates (`gsCX`, `gsCY`)
const SCREEN_CENTER: (i32, i32) = (0, 1625);
/// Height of the roof level tab of the interface (`ROOF_LEVEL_HEIGHT`)
const ROOF_LEVEL_HEIGHT: i32 = 50;

/// Armour of sandbags (`MATERIAL_SANDBAG`)
const MATERIAL_SANDBAG: u8 = 18;

pub const TRAVELCOST_NONE: u8 = 0;
pub const TRAVELCOST_FLAT: u8 = 10;
pub const TRAVELCOST_BUMPY: u8 = 12;
pub const TRAVELCOST_GRASS: u8 = 12;
pub const TRAVELCOST_THICK: u8 = 16;
pub const TRAVELCOST_DEBRIS: u8 = 20;
pub const TRAVELCOST_SHORE: u8 = 30;
pub const TRAVELCOST_KNEEDEEP: u8 = 36;
pub const TRAVELCOST_DEEPWATER: u8 = 50;
pub const TRAVELCOST_FENCE: u8 = 40;

pub const TRAVELCOST_DOORS_CLOSED_W_SW: u8 = 212;
pub const TRAVELCOST_DOORS_CLOSED_HERE_S: u8 = 213;
pub const TRAVELCOST_DOORS_CLOSED_N_NW: u8 = 214;
pub const TRAVELCOST_DOORS_CLOSED_HERE_W: u8 = 215;
pub const TRAVELCOST_DOORS_CLOSED_W_NW: u8 = 216;
pub const TRAVELCOST_DOORS_CLOSED_HERE_N: u8 = 217;
pub const TRAVELCOST_DOORS_CLOSED_N_NE: u8 = 218;
pub const TRAVELCOST_DOORS_CLOSED_HERE_E: u8 = 219;
pub const TRAVELCOST_DOOR_CLOSED_HERE: u8 = 220;
pub const TRAVELCOST_DOOR_CLOSED_N: u8 = 221;
pub const TRAVELCOST_DOOR_CLOSED_W: u8 = 222;
pub const TRAVELCOST_DOOR_OPEN_HERE: u8 = 223;
pub const TRAVELCOST_DOOR_OPEN_N: u8 = 224;
pub const TRAVELCOST_DOOR_OPEN_NE: u8 = 225;
pub const TRAVELCOST_DOOR_OPEN_E: u8 = 226;
pub const TRAVELCOST_DOOR_OPEN_SE: u8 = 227;
pub const TRAVELCOST_DOOR_OPEN_S: u8 = 228;
pub const TRAVELCOST_DOOR_OPEN_SW: u8 = 229;
pub const TRAVELCOST_DOOR_OPEN_W: u8 = 230;
pub const TRAVELCOST_DOOR_OPEN_NW: u8 = 231;
pub const TRAVELCOST_DOOR_OPEN_N_N: u8 = 232;
pub const TRAVELCOST_DOOR_OPEN_NW_N: u8 = 233;
pub const TRAVELCOST_DOOR_OPEN_NE_N: u8 = 234;
pub const TRAVELCOST_DOOR_OPEN_W_W: u8 = 235;
pub const TRAVELCOST_DOOR_OPEN_SW_W: u8 = 236;
pub const TRAVELCOST_DOOR_OPEN_NW_W: u8 = 237;
pub const TRAVELCOST_NOT_STANDING: u8 = 248;
pub const TRAVELCOST_OFF_MAP: u8 = 249;
pub const TRAVELCOST_CAVEWALL: u8 = 250;
pub const TRAVELCOST_HIDDENOBSTACLE: u8 = 251;
pub const TRAVELCOST_DOOR: u8 = 252;
pub const TRAVELCOST_OBSTACLE: u8 = 253;
pub const TRAVELCOST_WALL: u8 = 254;
pub const TRAVELCOST_EXITGRID: u8 = 255;

pub const TRAVELCOST_TRAINTRACKS: u8 = 30;
pub const TRAVELCOST_DIRTROAD: u8 = 9;
pub const TRAVELCOST_PAVEDROAD: u8 = 9;
pub const TRAVELCOST_FLATFLOOR: u8 = 10;

pub const TRAVELCOST_BLOCKED: u8 = TRAVELCOST_OFF_MAP;

/// Cost of entering a tile of each terrain, indexed by `Terrain` (`gTileTypeMovementCost`)
pub const TILE_TYPE_MOVEMENT_COST: [u8; 11] = [
    TRAVELCOST_FLAT,        // NO_TERRAIN
    TRAVELCOST_FLAT,        // FLAT_GROUND
    TRAVELCOST_FLATFLOOR,   // FLAT_FLOOR
    TRAVELCOST_PAVEDROAD,   // PAVED_ROAD
    TRAVELCOST_DIRTROAD,    // DIRT_ROAD
    TRAVELCOST_FLAT,        // LOW_GRASS
    TRAVELCOST_FLAT,        // HIGH_GRASS
    TRAVELCOST_TRAINTRACKS, // TRAIN_TRACKS
    TRAVELCOST_SHORE,       // LOW_WATER
    TRAVELCOST_SHORE,       // MED_WATER
    TRAVELCOST_SHORE,       // DEEP_WATER
];

/// Whether a cost depends on a door (`IS_TRAVELCOST_DOOR`)
pub fn is_travelcost_door(cost: u8) -> bool {
    (TRAVELCOST_DOORS_CLOSED_W_SW..=TRAVELCOST_DOOR_OPEN_NW_W).contains(&cost)
}

/// Whether a cost depends on an open door (`IS_TRAVELCOST_OPEN_DOOR`)
pub fn is_travelcost_open_door(cost: u8) -> bool {
    (TRAVELCOST_DOOR_OPEN_HERE..=TRAVELCOST_DOOR_OPEN_NW_W).contains(&cost)
}

/// Cost of entering a tile with the given terrain (`gTileTypeMovementCost[terrain]`)
pub fn terrain_movement_cost(terrain: Terrain) -> u8 {
    TILE_TYPE_MOVEMENT_COST[terrain as usize]
}

/// Whether a tile is inside of the scrollable area of the map (`GridNoOnVisibleWorldTile`)
///
/// The area is the one `InitRenderParams` sets up for the scroll restriction of the map.
pub fn grid_no_on_visible_world_tile(grid_no: GridNo, restricted_scroll: bool) -> bool {
    let (left, top, right, bottom) = world_screen_bounds(restricted_scroll);
    let (x, y) = absolute_screen_xy_from_map_pos(grid_no, left, top);
    x > 0 && x < right - left - 20 && y > 20 && y < bottom - top - 20
}

/// Screen bounds of the world as left, top, right, bottom (`InitRenderParams`)
pub(crate) fn world_screen_bounds(restricted_scroll: bool) -> (i32, i32, i32, i32) {
    let (top_left, bottom_right) = if restricted_scroll {
        // Basement level
        (
            (
                CELL_X_SIZE * WORLD_ROWS * 3 / 10,
                CELL_X_SIZE * WORLD_ROWS / 2,
            ),
            (
                CELL_X_SIZE * WORLD_ROWS * 7 / 10,
                CELL_X_SIZE * WORLD_ROWS / 2,
            ),
        )
    } else {
        (
            (CELL_X_SIZE, CELL_X_SIZE * WORLD_ROWS / 2),
            (CELL_Y_SIZE * WORLD_COLS, CELL_X_SIZE * WORLD_ROWS / 2),
        )
    };
    let (left, top) = from_cell_to_screen_coordinates(top_left);
    let (right, bottom) = from_cell_to_screen_coordinates(bottom_right);
    (left, top + ROOF_LEVEL_HEIGHT, right, bottom)
}

/// The part of the map that the game can show as top left and bottom right in game pixels
///
/// `ApplyScrolling` keeps the screen inside of `world_screen_bounds` grown by the scroll paddings
/// (`SCROLL_LEFT_PADDING` etc.).
pub fn scrollable_area(restricted_scroll: bool) -> (IVec2, IVec2) {
    const LEFT_PADDING: i32 = -30;
    const RIGHT_PADDING: i32 = 10;
    const TOP_PADDING: i32 = -50;
    const BOTTOM_PADDING: i32 = -20;
    let (left, top, right, bottom) = world_screen_bounds(restricted_scroll);
    (
        IVec2::new(left + LEFT_PADDING, top + TOP_PADDING),
        IVec2::new(right + RIGHT_PADDING, bottom + BOTTOM_PADDING),
    )
}

/// Cartesian to isometric coordinates (`FromCellToScreenCoordinates`)
fn from_cell_to_screen_coordinates((cell_x, cell_y): (i32, i32)) -> (i32, i32) {
    const HALF_TILE_WIDTH: i32 = 2;
    const HALF_TILE_HEIGHT: i32 = 1;
    (
        (cell_x - cell_y) * HALF_TILE_WIDTH,
        (cell_x + cell_y) * HALF_TILE_HEIGHT,
    )
}

/// Position of a tile relative to the top left of the world (`GetAbsoluteScreenXYFromMapPos`)
fn absolute_screen_xy_from_map_pos(grid_no: GridNo, left: i32, top: i32) -> (i32, i32) {
    let position = grid_no.position();
    let cell_x = i32::from(position.x) * CELL_X_SIZE;
    let cell_y = i32::from(position.y) * CELL_Y_SIZE;
    let dist_x = cell_x - CENTER_WORLD.0;
    let dist_y = cell_y - CENTER_WORLD.1;
    let screen_x = 2 * dist_x - 2 * dist_y;
    let screen_y = dist_x + dist_y;
    (
        screen_x + SCREEN_CENTER.0 - left,
        screen_y + SCREEN_CENTER.1 - top,
    )
}

/// Moves an entry point into the area where the game can show it (`ValidateEntryPointGridNo`)
///
/// Entry points can't be on the top or bottom rows of the screen, the screen y is clamped and the
/// tile at the clamped position is taken.
pub fn validate_entry_point_grid_no(grid_no: GridNo, restricted_scroll: bool) -> GridNo {
    const TOP_LIMIT: i32 = 80;
    let (left, top, _, bottom) = world_screen_bounds(restricted_scroll);
    let bottom_limit = bottom - top - 40;
    let (x, y) = absolute_screen_xy_from_map_pos(grid_no, left, top);
    let clamped = if y < TOP_LIMIT {
        TOP_LIMIT
    } else if y > bottom_limit {
        bottom_limit
    } else {
        return grid_no;
    };
    map_pos_from_absolute_screen_xy(x, clamped, left, top).unwrap_or(grid_no)
}

/// The tile at a position relative to the top left of the world, the inverse of
/// `absolute_screen_xy_from_map_pos` (`GetMapPosFromAbsoluteScreenXY`)
fn map_pos_from_absolute_screen_xy(x: i32, y: i32, left: i32, top: i32) -> Option<GridNo> {
    let dist_x = x - SCREEN_CENTER.0 + left;
    let dist_y = y - SCREEN_CENTER.1 + top;
    let world_x = (dist_x + 2 * dist_y) / 4 + CENTER_WORLD.0;
    let world_y = (2 * dist_y - dist_x) / 4 + CENTER_WORLD.1;
    GridNo::from_i32((world_y / CELL_X_SIZE) * WORLD_COLS + world_x / CELL_Y_SIZE)
}

/// Whether a tile is next to the edge of the map, checked diagonally because of the tilt of the
/// isometric view (`GridNoOnEdgeOfMap`)
///
/// The game reads past the end of the cost array for tiles next to its ends, these never count.
pub fn grid_no_on_edge_of_map(costs: &MovementCosts, grid_no: GridNo) -> bool {
    [NorthEast, SouthEast, SouthWest, NorthWest]
        .into_iter()
        .any(|direction| {
            grid_no
                .offset(direction.grid_no_increment())
                .is_some_and(|next| costs.cost(next, direction) == TRAVELCOST_OFF_MAP)
        })
}

/// Terrain of every tile (`CompileWorldTerrainIDs`)
///
/// The game clears the terrain when it loads a map, so tiles off the visible map have no terrain,
/// unless a multi-tile land or object of a visible tile covers them.
pub fn compile_world_terrain_ids(world: &MapWorld, tileset_id: TilesetId) -> GridArray<Terrain> {
    let mut terrain = GridArray::filled(Terrain::NoTerrain);
    for (grid_no, tile) in world.tiles.iter() {
        if !grid_no_on_visible_world_tile(grid_no, world.restricted_scroll) {
            continue;
        }

        let object = tile.objects.first();
        if let Some(object) = object {
            // Special case of the game for the pool, its art has no terrain
            if tileset_id.is_pool_without_terrain(object.tile.id) {
                terrain[grid_no] = Terrain::LowWater;
                continue;
            }
        }

        let object_tile = object
            .map(|node| &node.tile)
            .filter(|tile| tile.terrain != Terrain::NoTerrain);
        // Try terrain instead, the game crashes if there is no land
        let Some(terrain_tile) = object_tile.or_else(|| Some(&tile.land.first()?.tile)) else {
            continue;
        };
        if terrain_tile.covered_tiles.len() > 1 {
            for &(x, y) in &terrain_tile.covered_tiles {
                let covered = grid_no.offset(i32::from(x) + i32::from(y) * WORLD_COLS);
                if let Some(covered) = covered {
                    terrain[covered] = terrain_tile.terrain;
                }
            }
        } else {
            terrain[grid_no] = terrain_tile.terrain;
        }
    }
    terrain
}

/// Whether a structure is a sandbag that can be jumped, computes and caches its height like the
/// game does
fn is_low_sandbag(world: &mut MapWorld, grid_no: GridNo, index: usize) -> bool {
    world.tiles[grid_no].structures[index]
        .tile
        .structure
        .as_ref()
        .is_some_and(|structure| structure.armour == MATERIAL_SANDBAG)
        && world.cache_structure_height(grid_no, index) < 2
}

/// `FindStructure(grid_no, STRUCTURE_OBSTACLE) != NULL`, false off the map
fn has_obstacle(world: &MapWorld, grid_no: Option<GridNo>) -> bool {
    grid_no.is_some_and(|grid_no| {
        world.tiles[grid_no]
            .find_structure(StructureFlags::OBSTACLE)
            .is_some()
    })
}

/// Movement costs of level 0 for every tile and direction of the step into the tile
/// (`gubWorldMovementCosts[grid_no][direction][0]`), and the terrain of the tiles
/// (`MAP_ELEMENT::ubTerrainID`)
#[derive(Resource, Clone, Debug, Default)]
pub struct MovementCosts {
    costs: GridArray<[u8; 8]>,
    terrain: GridArray<Terrain>,
    /// While part of the map is compiled again, the tiles whose costs may be written
    writable: Option<GridArray<bool>>,
}

/// The tiles within `radius` tiles of the given ones, with the grid number arithmetic of the
/// game, which wraps into the neighbouring rows at the left and right edges
fn dilate(tiles: &BTreeSet<GridNo>, radius: i32) -> BTreeSet<GridNo> {
    tiles
        .iter()
        .flat_map(|&grid_no| {
            (-radius..=radius).flat_map(move |dy| {
                (-radius..=radius).filter_map(move |dx| grid_no.offset(dy * WORLD_COLS + dx))
            })
        })
        .collect()
}

impl MovementCosts {
    /// Port of `CompileWorldMovementCosts` for level 0, including `CompileWorldTerrainIDs`
    ///
    /// Computes the cached heights of low sandbags (`StructureHeight`) in the world, like the
    /// game does.
    pub fn compile(world: &mut MapWorld, map: &TacticalMap) -> Self {
        let terrain = compile_world_terrain_ids(world, map.tileset);
        let exit_grids: Vec<GridNo> = map.exit_grids.iter().map(|grid| grid.grid_no).collect();
        Self::compile_with_terrain(world, terrain, &exit_grids, is_low_sandbag)
    }

    /// `CompileWorldMovementCosts` with known terrain, `is_low_sandbag` is the sandbag condition
    /// for a structure (grid number and index in the structure list)
    pub(crate) fn compile_with_terrain(
        world: &mut MapWorld,
        terrain: GridArray<Terrain>,
        exit_grids: &[GridNo],
        mut is_low_sandbag: impl FnMut(&mut MapWorld, GridNo, usize) -> bool,
    ) -> Self {
        let mut exit_grid_at = GridArray::filled(false);
        for &grid_no in exit_grids {
            exit_grid_at[grid_no] = true;
        }
        let mut costs = Self {
            costs: GridArray::filled([TRAVELCOST_NONE; 8]),
            terrain,
            writable: None,
        };
        for (grid_no, &exit_grid) in exit_grid_at.iter() {
            costs.compile_tile(world, grid_no, exit_grid, &mut is_low_sandbag);
        }
        costs
    }

    /// Compiles the costs again after the structures, the land, the objects or the heights of
    /// the `core` tiles changed, with the same result as `compile`
    ///
    /// Returns the tiles whose costs or terrain changed.
    pub fn recompile_area(
        &mut self,
        world: &mut MapWorld,
        map: &TacticalMap,
        core: &BTreeSet<GridNo>,
    ) -> BTreeSet<GridNo> {
        let exit_grids: Vec<GridNo> = map.exit_grids.iter().map(|grid| grid.grid_no).collect();
        self.recompile_area_with(world, map.tileset, core, &exit_grids, is_low_sandbag)
    }

    fn recompile_area_with(
        &mut self,
        world: &mut MapWorld,
        tileset: TilesetId,
        core: &BTreeSet<GridNo>,
        exit_grids: &[GridNo],
        mut is_low_sandbag: impl FnMut(&mut MapWorld, GridNo, usize) -> bool,
    ) -> BTreeSet<GridNo> {
        // The terrain of a tile can come from a multi-tile land or object of another tile, and
        // it is cheap to compile
        let terrain = compile_world_terrain_ids(world, tileset);
        let mut core = core.clone();
        let mut changed = BTreeSet::new();
        for grid_no in GridNo::all() {
            if terrain[grid_no] != self.terrain[grid_no] {
                core.insert(grid_no);
                changed.insert(grid_no);
            }
        }
        self.terrain = terrain;

        // A tile reads its own structures and terrain and the structures and heights of its
        // neighbours, and writes to itself and its neighbours. So the costs within 2 tiles of
        // the core can change, and they are written by the tiles within 3 tiles. Compiling
        // these in ascending order like `compile`, but only writing the affected costs, repeats
        // every write to the affected costs in the same order.
        let affected = dilate(&core, 2);
        let writers = dilate(&core, 3);
        let mut writable = GridArray::filled(false);
        let mut old = Vec::with_capacity(affected.len());
        for &grid_no in &affected {
            writable[grid_no] = true;
            old.push(self.costs[grid_no]);
            self.costs[grid_no] = [TRAVELCOST_NONE; 8];
        }
        self.writable = Some(writable);
        let mut exit_grid_at = GridArray::filled(false);
        for &grid_no in exit_grids {
            exit_grid_at[grid_no] = true;
        }
        for &grid_no in &writers {
            self.compile_tile(world, grid_no, exit_grid_at[grid_no], &mut is_low_sandbag);
        }
        self.writable = None;

        for (&grid_no, old) in affected.iter().zip(old) {
            if self.costs[grid_no] != old {
                changed.insert(grid_no);
            }
        }
        changed
    }

    /// Cost of stepping into the tile in the given direction
    pub fn cost(&self, grid_no: GridNo, direction: Direction) -> u8 {
        self.costs[grid_no][direction as usize]
    }

    /// Terrain of a tile (`ubTerrainID`)
    pub fn terrain(&self, grid_no: GridNo) -> Terrain {
        self.terrain[grid_no]
    }

    fn get(&self, grid_no: GridNo, direction: Direction) -> u8 {
        self.costs[grid_no][direction as usize]
    }

    /// Whether a cost may be written, see `recompile_area`
    fn is_writable(&self, grid_no: GridNo) -> bool {
        self.writable
            .as_ref()
            .is_none_or(|writable| writable[grid_no])
    }

    /// Raises a cost, never lowers it (`SET_MOVEMENTCOST`)
    fn set(&mut self, grid_no: GridNo, direction: Direction, cost: u8) {
        if !self.is_writable(grid_no) {
            return;
        }
        let current = &mut self.costs[grid_no][direction as usize];
        if *current < cost {
            *current = cost;
        }
    }

    /// `set` for the tile `delta` away from `grid_no`, skipped off the map
    fn set_at(&mut self, grid_no: GridNo, delta: i32, direction: Direction, cost: u8) {
        if let Some(other) = grid_no.offset(delta) {
            self.set(other, direction, cost);
        }
    }

    /// Overwrites a cost (`FORCE_SET_MOVEMENTCOST`)
    fn force_set(&mut self, grid_no: GridNo, direction: Direction, cost: u8) {
        if !self.is_writable(grid_no) {
            return;
        }
        self.costs[grid_no][direction as usize] = cost;
    }

    /// Makes a step free if nothing blocks it, for the tiles exited over a fence. Skipped off
    /// the map.
    fn clear_unless_blocked(&mut self, grid_no: Option<GridNo>, direction: Direction) {
        let Some(grid_no) = grid_no else {
            return;
        };
        if self.get(grid_no, direction) < TRAVELCOST_BLOCKED {
            // make sure no obstacle costs exists before changing path cost to 0
            self.force_set(grid_no, direction, TRAVELCOST_NONE);
        }
    }

    /// Port of `CompileTileMovementCosts` for level 0
    fn compile_tile(
        &mut self,
        world: &mut MapWorld,
        grid_no: GridNo,
        exit_grid: bool,
        is_low_sandbag: &mut impl FnMut(&mut MapWorld, GridNo, usize) -> bool,
    ) {
        let g = grid_no;
        if grid_no_on_visible_world_tile(g, world.restricted_scroll) {
            // check for land of a different height in adjacent locations
            let height = world.tiles[grid_no].height;
            for direction in Direction::ALL {
                let other = g.offset(direction.grid_no_increment());
                if other.is_some_and(|other| world.tiles[other].height != height) {
                    self.set(g, direction, TRAVELCOST_OBSTACLE);
                }
            }

            if exit_grid {
                for direction in Direction::ALL {
                    self.set(g, direction, TRAVELCOST_EXITGRID);
                }
            }
        } else {
            for direction in Direction::ALL {
                self.set(g, direction, TRAVELCOST_OFF_MAP);
            }
            if world.tiles[grid_no].structures.is_empty() {
                return;
            }
        }

        let terrain_cost = terrain_movement_cost(self.terrain[grid_no]);
        if world.tiles[grid_no].structures.is_empty() {
            // No structures, consider just the land
            for direction in Direction::ALL {
                self.set(g, direction, terrain_cost);
            }
            return;
        }

        if !world.tiles[grid_no].land.is_empty() {
            for direction in Direction::ALL {
                self.set(g, direction, terrain_cost);
            }
        }

        for index in 0..world.tiles[grid_no].structures.len() {
            let structure = &world.tiles[grid_no].structures[index];
            if structure.cube_offset != 0 {
                // Structures on the roof only affect level 1
                continue;
            }
            let flags = structure.flags;
            let orientation = structure.wall_orientation;
            if flags.contains(StructureFlags::PASSABLE) {
                if flags.contains(StructureFlags::WIREFENCE) && flags.contains(StructureFlags::OPEN)
                {
                    self.open_wire_fence(g, orientation);
                }
                // all other passable structures do not block movement in any way
            } else if flags.intersects(StructureFlags::BLOCKSMOVES) {
                if flags.contains(StructureFlags::FENCE) && !flags.contains(StructureFlags::SPECIAL)
                {
                    self.jumpable_fence(g, orientation);
                } else if is_low_sandbag(world, grid_no, index) {
                    for direction in Direction::ALL {
                        self.set(g, direction, TRAVELCOST_OBSTACLE);
                    }
                    if !has_obstacle(world, g.offset(-WORLD_COLS))
                        && !has_obstacle(world, g.offset(WORLD_COLS))
                    {
                        self.force_set(g, North, TRAVELCOST_FENCE);
                        self.force_set(g, South, TRAVELCOST_FENCE);
                    }
                    if !has_obstacle(world, g.offset(-1)) && !has_obstacle(world, g.offset(1)) {
                        self.force_set(g, East, TRAVELCOST_FENCE);
                        self.force_set(g, West, TRAVELCOST_FENCE);
                    }
                } else if flags.contains(StructureFlags::CAVEWALL) {
                    for direction in Direction::ALL {
                        self.set(g, direction, TRAVELCOST_CAVEWALL);
                    }
                } else {
                    for direction in Direction::ALL {
                        self.set(g, direction, TRAVELCOST_OBSTACLE);
                    }
                }
            } else if flags.intersects(StructureFlags::ANYDOOR) {
                // NB closed doors are treated just like walls
                let base_tile = flags.contains(StructureFlags::BASE_TILE);
                if flags.contains(StructureFlags::DDOOR_LEFT) {
                    self.double_door_left(g, orientation, base_tile);
                } else if flags.contains(StructureFlags::DDOOR_RIGHT) {
                    self.double_door_right(g, orientation, base_tile);
                } else if flags.contains(StructureFlags::GARAGEDOOR) {
                    self.garage_door(g, orientation, base_tile);
                } else {
                    let sliding = flags.contains(StructureFlags::SLIDINGDOOR);
                    self.standard_door(g, orientation, base_tile, sliding);
                }
            } else if flags.intersects(StructureFlags::WALLSTUFF) {
                self.wall(g, orientation);
            }
        }
    }

    /// Prevents movement along an open wire fence, allows it in all other directions
    fn open_wire_fence(&mut self, g: GridNo, orientation: Orientation) {
        if orientation.intersects(Orientation::LEFT) {
            self.set(g, North, TRAVELCOST_NOT_STANDING);
            self.set(g, NorthEast, TRAVELCOST_NOT_STANDING);
            self.set(g, East, TRAVELCOST_OBSTACLE);
            self.set(g, SouthEast, TRAVELCOST_NOT_STANDING);
            self.set(g, South, TRAVELCOST_NOT_STANDING);
            self.set(g, SouthWest, TRAVELCOST_NOT_STANDING);
            self.set(g, West, TRAVELCOST_OBSTACLE);
            self.set(g, NorthWest, TRAVELCOST_NOT_STANDING);
        } else if orientation.intersects(Orientation::RIGHT) {
            self.set(g, North, TRAVELCOST_OBSTACLE);
            self.set(g, NorthEast, TRAVELCOST_NOT_STANDING);
            self.set(g, East, TRAVELCOST_NOT_STANDING);
            self.set(g, SouthEast, TRAVELCOST_NOT_STANDING);
            self.set(g, South, TRAVELCOST_OBSTACLE);
            self.set(g, SouthWest, TRAVELCOST_NOT_STANDING);
            self.set(g, West, TRAVELCOST_NOT_STANDING);
            self.set(g, NorthWest, TRAVELCOST_NOT_STANDING);
        }
    }

    fn jumpable_fence(&mut self, g: GridNo, orientation: Orientation) {
        if orientation.intersects(Orientation::LEFT) {
            // can be jumped north and south
            self.set(g, North, TRAVELCOST_FENCE);
            self.set(g, NorthEast, TRAVELCOST_OBSTACLE);
            self.set(g, East, TRAVELCOST_OBSTACLE);
            self.set(g, SouthEast, TRAVELCOST_OBSTACLE);
            self.set(g, South, TRAVELCOST_FENCE);
            self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
            self.set(g, West, TRAVELCOST_OBSTACLE);
            self.set(g, NorthWest, TRAVELCOST_OBSTACLE);
            // set values for the tiles EXITED from this location
            self.clear_unless_blocked(g.offset(-WORLD_COLS), North);
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, 1, East, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.clear_unless_blocked(g.offset(WORLD_COLS), South);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, -1, West, TRAVELCOST_OBSTACLE);
            self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_OBSTACLE);
        } else if orientation.intersects(Orientation::RIGHT) {
            // can be jumped east and west
            self.set(g, North, TRAVELCOST_OBSTACLE);
            self.set(g, NorthEast, TRAVELCOST_OBSTACLE);
            self.set(g, East, TRAVELCOST_FENCE);
            self.set(g, SouthEast, TRAVELCOST_OBSTACLE);
            self.set(g, South, TRAVELCOST_OBSTACLE);
            self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
            self.set(g, West, TRAVELCOST_FENCE);
            self.set(g, NorthWest, TRAVELCOST_OBSTACLE);
            // set values for the tiles EXITED from this location
            self.set_at(g, -WORLD_COLS, North, TRAVELCOST_OBSTACLE);
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.clear_unless_blocked(g.offset(1), East);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_OBSTACLE);
            self.clear_unless_blocked(g.offset(-1), West);
            self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_OBSTACLE);
        } else {
            // corners aren't jumpable
            for direction in Direction::ALL {
                self.set(g, direction, TRAVELCOST_OBSTACLE);
            }
        }
    }

    /// Double door, left side as you look on the screen
    fn double_door_left(&mut self, g: GridNo, orientation: Orientation, base_tile: bool) {
        if orientation == Orientation::OUTSIDE_RIGHT {
            if base_tile {
                // doorpost
                self.set(g, NorthWest, TRAVELCOST_OBSTACLE);
                self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, SouthWest, TRAVELCOST_DOORS_CLOSED_HERE_N);
                self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, SouthEast, TRAVELCOST_DOORS_CLOSED_W_NW);
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_DOORS_CLOSED_W_SW);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_S);
                // corner
                self.set_at(g, 1 + WORLD_COLS, SouthEast, TRAVELCOST_OBSTACLE);
            } else {
                // door
                self.set(g, North, TRAVELCOST_DOOR_OPEN_W);
                self.set(g, NorthWest, TRAVELCOST_DOOR_OPEN_W);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_OPEN_NW);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOOR_OPEN_NW);
                self.set_at(g, 1, NorthEast, TRAVELCOST_DOOR_OPEN_W_W);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOOR_OPEN_NW_W);
            }
        } else if orientation == Orientation::OUTSIDE_LEFT {
            if base_tile {
                // doorframe
                self.set(g, NorthEast, TRAVELCOST_OBSTACLE);
                self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_E);

                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOORS_CLOSED_N_NE);

                // DO CORNERS
                self.set_at(g, -1, NorthWest, TRAVELCOST_OBSTACLE);
                self.set_at(g, 1, NorthEast, TRAVELCOST_DOORS_CLOSED_HERE_W);
                self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOORS_CLOSED_N_NW);
            } else {
                // door
                self.set(g, NorthEast, TRAVELCOST_DOOR_OPEN_N);
                self.set(g, East, TRAVELCOST_DOOR_OPEN_N);
                self.set_at(g, -1, West, TRAVELCOST_DOOR_OPEN_NE);
                self.set_at(g, -1, NorthWest, TRAVELCOST_DOOR_OPEN_NE);
                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOOR_OPEN_N_N);
                self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_DOOR_OPEN_NE_N);
            }
        } else if orientation == Orientation::INSIDE_LEFT {
            // doorframe
            self.set(g, NorthEast, TRAVELCOST_OBSTACLE);
            self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_E);

            self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOORS_CLOSED_N_NE);
            // DO CORNERS
            self.set_at(g, -1, NorthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, 1, NorthEast, TRAVELCOST_DOORS_CLOSED_HERE_W);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOORS_CLOSED_N_NW);
            // door
            self.set(g, East, TRAVELCOST_DOOR_OPEN_HERE);
            self.set(g, SouthEast, TRAVELCOST_DOOR_OPEN_HERE);
            self.set_at(g, -1, West, TRAVELCOST_DOOR_OPEN_E);
            self.set_at(g, -1, SouthWest, TRAVELCOST_DOOR_OPEN_E);
            self.set_at(g, -WORLD_COLS, NorthEast, TRAVELCOST_DOOR_OPEN_S);
            self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_DOOR_OPEN_SE);
        } else if orientation == Orientation::INSIDE_RIGHT {
            self.set(g, NorthWest, TRAVELCOST_OBSTACLE);
            self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, SouthWest, TRAVELCOST_DOORS_CLOSED_HERE_N);

            self.set_at(g, 1, SouthEast, TRAVELCOST_DOORS_CLOSED_W_NW);
            self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
            self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);

            // DO CORNERS
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_DOORS_CLOSED_W_SW);
            self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_S);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);

            // door
            self.set(g, North, TRAVELCOST_DOOR_OPEN_HERE);
            self.set(g, NorthEast, TRAVELCOST_DOOR_OPEN_HERE);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_OPEN_N);
            self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOOR_OPEN_N);
            self.set_at(g, -1, NorthWest, TRAVELCOST_DOOR_OPEN_E);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_DOOR_OPEN_NE);
        }
        // door with no orientation specified!?
    }

    /// Double door, right side as you look on the screen
    fn double_door_right(&mut self, g: GridNo, orientation: Orientation, base_tile: bool) {
        if orientation == Orientation::OUTSIDE_LEFT {
            if base_tile {
                // doorpost
                self.set(g, NorthWest, TRAVELCOST_OBSTACLE);
                self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthEast, TRAVELCOST_DOORS_CLOSED_HERE_W);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOORS_CLOSED_N_NW);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, -1, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_E);
                self.set_at(g, -1 + WORLD_COLS, SouthWest, TRAVELCOST_DOORS_CLOSED_N_NE);
                // corner
                self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);
            } else {
                // door
                self.set(g, NorthWest, TRAVELCOST_DOOR_OPEN_N);
                self.set(g, West, TRAVELCOST_DOOR_OPEN_N);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_OPEN_NW);
                self.set_at(g, 1, NorthEast, TRAVELCOST_DOOR_OPEN_NW);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOOR_OPEN_N_N);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOOR_OPEN_NW_N);
            }
        } else if orientation == Orientation::OUTSIDE_RIGHT {
            if base_tile {
                // doorframe
                self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
                self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_S);
                self.set_at(g, 1, SouthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, NorthEast, TRAVELCOST_DOORS_CLOSED_W_SW);
                // DO CORNERS
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOORS_CLOSED_W_NW);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOORS_CLOSED_HERE_N);
            } else {
                // door
                self.set(g, South, TRAVELCOST_DOOR_OPEN_W);
                self.set(g, SouthWest, TRAVELCOST_DOOR_OPEN_W);
                self.set_at(g, -WORLD_COLS, North, TRAVELCOST_DOOR_OPEN_SW);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_DOOR_OPEN_SW);
                self.set_at(g, 1, SouthEast, TRAVELCOST_DOOR_OPEN_W_W);
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_DOOR_OPEN_SW_W);
            }
        } else if orientation == Orientation::INSIDE_RIGHT {
            self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
            self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_S);

            self.set_at(g, 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
            self.set_at(g, 1, NorthEast, TRAVELCOST_DOORS_CLOSED_W_SW);

            // DO CORNERS
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_DOORS_CLOSED_W_NW);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOORS_CLOSED_HERE_N);

            // door
            self.set(g, South, TRAVELCOST_DOOR_OPEN_HERE);
            self.set(g, SouthEast, TRAVELCOST_DOOR_OPEN_HERE);
            self.set_at(g, -WORLD_COLS, North, TRAVELCOST_DOOR_OPEN_S);
            self.set_at(g, -WORLD_COLS, NorthEast, TRAVELCOST_DOOR_OPEN_S);
            self.set_at(g, -1, SouthWest, TRAVELCOST_DOOR_OPEN_E);
            self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_DOOR_OPEN_SE);
        } else if orientation == Orientation::INSIDE_LEFT {
            self.set(g, NorthEast, TRAVELCOST_DOORS_CLOSED_HERE_W);
            self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, NorthWest, TRAVELCOST_OBSTACLE);

            self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOORS_CLOSED_N_NW);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);

            // DO CORNERS
            self.set_at(g, -1, NorthWest, TRAVELCOST_DOORS_CLOSED_HERE_E);
            self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_DOORS_CLOSED_N_NE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
            // door
            self.set(g, West, TRAVELCOST_DOOR_OPEN_HERE);
            self.set(g, SouthWest, TRAVELCOST_DOOR_OPEN_HERE);
            self.set_at(g, 1, East, TRAVELCOST_DOOR_OPEN_W);
            self.set_at(g, 1, SouthEast, TRAVELCOST_DOOR_OPEN_W);
            self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_DOOR_OPEN_S);
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_DOOR_OPEN_SW);
        }
        // door with no orientation specified!?
    }

    fn garage_door(&mut self, g: GridNo, orientation: Orientation, base_tile: bool) {
        if orientation.intersects(Orientation::LEFT) {
            // doorframe post in one corner of each of the tiles
            if base_tile {
                self.set(g, NorthWest, TRAVELCOST_WALL);
                self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthEast, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, 1, NorthEast, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_WALL);
            } else {
                self.set(g, NorthWest, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthEast, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_WALL);
                self.set_at(g, -1, NorthWest, TRAVELCOST_WALL);
            }
        } else if orientation.intersects(Orientation::RIGHT) {
            // doorframe post in one corner of each of the tiles
            if base_tile {
                self.set(g, NorthWest, TRAVELCOST_WALL);
                self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, SouthWest, TRAVELCOST_DOOR_CLOSED_HERE);

                self.set_at(g, 1, NorthEast, TRAVELCOST_WALL);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, SouthEast, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_WALL);
            } else {
                self.set(g, NorthWest, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, SouthWest, TRAVELCOST_WALL);

                self.set_at(g, 1, NorthEast, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, SouthEast, TRAVELCOST_WALL);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_WALL);
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_WALL);
            }
        }
    }

    fn standard_door(
        &mut self,
        g: GridNo,
        orientation: Orientation,
        base_tile: bool,
        sliding: bool,
    ) {
        if orientation == Orientation::OUTSIDE_LEFT {
            if base_tile {
                // doorframe
                self.set(g, NorthEast, TRAVELCOST_WALL);
                self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthWest, TRAVELCOST_WALL);

                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_WALL);

                // DO CORNERS
                self.set_at(g, -1, NorthWest, TRAVELCOST_WALL);
                self.set_at(g, 1, NorthEast, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_WALL);
            } else if !sliding {
                // door
                self.set(g, NorthEast, TRAVELCOST_WALL);
                self.set(g, East, TRAVELCOST_DOOR_OPEN_N);
                self.set_at(g, -1, West, TRAVELCOST_DOOR_OPEN_NE);
                self.set_at(g, -1, NorthWest, TRAVELCOST_WALL);
                self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_DOOR_OPEN_N_N);
                self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_DOOR_OPEN_NE_N);
            }
        } else if orientation == Orientation::INSIDE_LEFT {
            self.set(g, NorthEast, TRAVELCOST_WALL);
            self.set(g, North, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, NorthWest, TRAVELCOST_WALL);

            self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_DOOR_CLOSED_N);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);

            // DO CORNERS
            self.set_at(g, -1, NorthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);

            if !sliding {
                self.set(g, East, TRAVELCOST_DOOR_OPEN_HERE);
                self.set(g, SouthEast, TRAVELCOST_DOOR_OPEN_HERE);
                self.set_at(g, -1, West, TRAVELCOST_DOOR_OPEN_E);
                self.set_at(g, -1, SouthWest, TRAVELCOST_DOOR_OPEN_E);
                self.set_at(g, -WORLD_COLS, NorthEast, TRAVELCOST_DOOR_OPEN_S);
                self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_DOOR_OPEN_SE);
            }
        } else if orientation == Orientation::OUTSIDE_RIGHT {
            if base_tile {
                // doorframe
                self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
                self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
                self.set(g, NorthWest, TRAVELCOST_OBSTACLE);

                self.set_at(g, 1, SouthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
                self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);

                // DO CORNERS
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
                self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);
            } else if !sliding {
                // door
                self.set(g, South, TRAVELCOST_DOOR_OPEN_W);
                self.set(g, SouthWest, TRAVELCOST_DOOR_OPEN_W);
                self.set_at(g, -WORLD_COLS, North, TRAVELCOST_DOOR_OPEN_SW);
                self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_DOOR_OPEN_SW);
                self.set_at(g, 1, SouthEast, TRAVELCOST_DOOR_OPEN_W_W);
                self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_DOOR_OPEN_SW_W);
            }
        } else if orientation == Orientation::INSIDE_RIGHT {
            self.set(g, SouthWest, TRAVELCOST_OBSTACLE);
            self.set(g, West, TRAVELCOST_DOOR_CLOSED_HERE);
            self.set(g, NorthWest, TRAVELCOST_OBSTACLE);

            self.set_at(g, 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, 1, East, TRAVELCOST_DOOR_CLOSED_W);
            self.set_at(g, 1, NorthEast, TRAVELCOST_OBSTACLE);

            // DO CORNERS
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_OBSTACLE);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_OBSTACLE);

            if !sliding {
                // door
                self.set(g, South, TRAVELCOST_DOOR_OPEN_HERE);
                self.set(g, SouthEast, TRAVELCOST_DOOR_OPEN_HERE);
                self.set_at(g, -WORLD_COLS, North, TRAVELCOST_DOOR_OPEN_S);
                self.set_at(g, -WORLD_COLS, NorthEast, TRAVELCOST_DOOR_OPEN_S);
                self.set_at(g, -1, SouthWest, TRAVELCOST_DOOR_OPEN_E);
                self.set_at(g, -WORLD_COLS - 1, NorthWest, TRAVELCOST_DOOR_OPEN_SE);
            }
        }
        // door with no orientation specified!?
    }

    /// Walls and windows, a wall without orientation (such as the non-base tile of a corner
    /// piece) blocks nothing
    fn wall(&mut self, g: GridNo, orientation: Orientation) {
        if orientation.intersects(Orientation::LEFT) {
            self.set(g, NorthEast, TRAVELCOST_WALL);
            self.set(g, North, TRAVELCOST_WALL);
            self.set(g, NorthWest, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS, SouthEast, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS, South, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_WALL);

            // DO CORNERS
            self.set_at(g, -1, NorthWest, TRAVELCOST_WALL);
            self.set_at(g, 1, NorthEast, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS - 1, SouthWest, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_WALL);
        } else if orientation.intersects(Orientation::RIGHT) {
            self.set(g, SouthWest, TRAVELCOST_WALL);
            self.set(g, West, TRAVELCOST_WALL);
            self.set(g, NorthWest, TRAVELCOST_WALL);
            self.set_at(g, 1, SouthEast, TRAVELCOST_WALL);
            self.set_at(g, 1, East, TRAVELCOST_WALL);
            self.set_at(g, 1, NorthEast, TRAVELCOST_WALL);

            // DO CORNERS
            self.set_at(g, -WORLD_COLS + 1, NorthEast, TRAVELCOST_WALL);
            self.set_at(g, -WORLD_COLS, NorthWest, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS + 1, SouthEast, TRAVELCOST_WALL);
            self.set_at(g, WORLD_COLS, SouthWest, TRAVELCOST_WALL);
        }
    }
}

/// The door tiles that a door cost depends on (`IsDoorObstacleIfClosed`)
struct DoorTiles {
    /// Whether the step is an obstacle if the door is closed
    obstacle_if_closed: bool,
    /// The door, `None` if it would be off the map
    door: Option<GridNo>,
    /// The second door of diagonal steps
    second_door: Option<Option<GridNo>>,
}

/// The door tiles of a door cost at `grid_no`, `None` for costs that don't depend on doors
fn is_door_obstacle_if_closed(cost: u8, grid_no: GridNo) -> Option<DoorTiles> {
    let at = |direction: Direction| grid_no.offset(direction.grid_no_increment());
    let at2 = |first: Direction, second: Direction| {
        grid_no.offset(first.grid_no_increment() + second.grid_no_increment())
    };
    let here = Some(grid_no);
    let (obstacle_if_closed, door, second_door) = match cost {
        TRAVELCOST_DOORS_CLOSED_W_SW => (true, at(West), Some(at(SouthWest))),
        TRAVELCOST_DOORS_CLOSED_HERE_S => (true, here, Some(at(South))),
        TRAVELCOST_DOORS_CLOSED_N_NW => (true, at(North), Some(at(NorthWest))),
        TRAVELCOST_DOORS_CLOSED_HERE_W => (true, here, Some(at(West))),
        TRAVELCOST_DOORS_CLOSED_W_NW => (true, at(West), Some(at(NorthWest))),
        TRAVELCOST_DOORS_CLOSED_HERE_N => (true, here, Some(at(North))),
        TRAVELCOST_DOORS_CLOSED_N_NE => (true, at(North), Some(at(NorthEast))),
        TRAVELCOST_DOORS_CLOSED_HERE_E => (true, here, Some(at(East))),
        TRAVELCOST_DOOR_CLOSED_HERE => (true, here, None),
        TRAVELCOST_DOOR_CLOSED_N => (true, at(North), None),
        TRAVELCOST_DOOR_CLOSED_W => (true, at(West), None),
        TRAVELCOST_DOOR_OPEN_HERE => (false, here, None),
        TRAVELCOST_DOOR_OPEN_N => (false, at(North), None),
        TRAVELCOST_DOOR_OPEN_NE => (false, at(NorthEast), None),
        TRAVELCOST_DOOR_OPEN_E => (false, at(East), None),
        TRAVELCOST_DOOR_OPEN_SE => (false, at(SouthEast), None),
        TRAVELCOST_DOOR_OPEN_S => (false, at(South), None),
        TRAVELCOST_DOOR_OPEN_SW => (false, at(SouthWest), None),
        TRAVELCOST_DOOR_OPEN_W => (false, at(West), None),
        TRAVELCOST_DOOR_OPEN_NW => (false, at(NorthWest), None),
        TRAVELCOST_DOOR_OPEN_N_N => (false, at2(North, North), None),
        TRAVELCOST_DOOR_OPEN_NW_N => (false, at2(NorthWest, North), None),
        TRAVELCOST_DOOR_OPEN_NE_N => (false, at2(NorthEast, North), None),
        TRAVELCOST_DOOR_OPEN_W_W => (false, at2(West, West), None),
        TRAVELCOST_DOOR_OPEN_SW_W => (false, at2(SouthWest, West), None),
        TRAVELCOST_DOOR_OPEN_NW_W => (false, at2(NorthWest, West), None),
        _ => return None,
    };
    Some(DoorTiles {
        obstacle_if_closed,
        door,
        second_door,
    })
}

/// Perceived state of the door on a tile (`GetDoorState` with `returnPerceivedValue`)
///
/// The game only uses the perceived state of a `DOOR_STATUS` record, which exists once a door
/// has been opened or closed in the game or the door states of a visited sector were loaded.
/// Right after a map is loaded (and always in the editor) there are none, so the game reads the
/// flags of the door structure, like this does. A tile without a door, or off the map, counts as
/// a destroyed door.
fn door_state(world: &MapWorld, grid_no: Option<GridNo>) -> StructureFlags {
    grid_no
        .and_then(|grid_no| world.tiles[grid_no].find_structure(StructureFlags::ANYDOOR))
        .map_or(StructureFlags::OPEN, |structure| structure.flags)
}

/// Cost of a step that depends on a door, for no soldier and the perceived door state
/// (`DoorTravelCost(NULL, grid_no, cost, TRUE, &door_grid_no)`)
///
/// Returns the cost and the grid number of the door (the base tile of the door the cost refers
/// to). Costs that don't depend on doors are returned unchanged without a door, where the game
/// leaves the grid number untouched. The door is also `None` if it would be off the map.
pub fn door_travel_cost(
    world: &MapWorld,
    costs: &MovementCosts,
    grid_no: GridNo,
    cost: u8,
) -> (u8, Option<GridNo>) {
    let Some(DoorTiles {
        obstacle_if_closed,
        door,
        second_door,
    }) = is_door_obstacle_if_closed(cost, grid_no)
    else {
        return (cost, None);
    };
    let replacement = if obstacle_if_closed {
        TRAVELCOST_DOOR
    } else {
        cost
    };

    let state = door_state(world, door);
    let open = match second_door {
        // this travel direction depends on the state of only one door structure
        None => {
            if !state.intersects(StructureFlags::ANYDOOR) {
                // door destroyed?
                return (cost, door);
            }
            state.contains(StructureFlags::OPEN)
        }
        // ... or two door structures
        Some(second_door) => {
            let state2 = door_state(world, second_door);
            if !state.intersects(StructureFlags::ANYDOOR)
                && !state2.intersects(StructureFlags::ANYDOOR)
            {
                // both doors destroyed?
                return (cost, door);
            } else if !state.contains(StructureFlags::OPEN)
                || !state2.contains(StructureFlags::OPEN)
            {
                // this direction is always diagonal so block it if one of the doors is closed
                return (replacement, door);
            }
            true
        }
    };

    let terrain_cost = terrain_movement_cost(costs.terrain(grid_no));
    let cost = match (open, obstacle_if_closed) {
        (true, true) => terrain_cost,
        (true, false) => replacement,
        // the perceived value of a closed door is an obstacle
        (false, true) => replacement,
        (false, false) => terrain_cost,
    };
    (cost, door)
}

/// Cost of a step that depends on a door, as `FindBestPath` sees it for a soldier of the enemy
/// team without keys, `None` if the step can't be made
///
/// Unlike [`door_travel_cost`], this reads the real state of the doors, and a closed door can be
/// opened unless the door table says that it is locked. Costs that don't depend on doors are
/// returned unchanged.
pub fn door_path_cost(
    world: &MapWorld,
    costs: &MovementCosts,
    doors: &[Door],
    grid_no: GridNo,
    cost: u8,
    diagonal: bool,
) -> Option<u8> {
    let Some(DoorTiles {
        obstacle_if_closed,
        door,
        second_door,
    }) = is_door_obstacle_if_closed(cost, grid_no)
    else {
        return Some(cost);
    };
    let terrain_cost = terrain_movement_cost(costs.terrain(grid_no));
    let state = door_state(world, door);
    let open = match second_door {
        // this travel direction depends on the state of only one door structure
        None => {
            if !state.intersects(StructureFlags::ANYDOOR) {
                // door destroyed?
                return Some(terrain_cost);
            }
            let open = state.contains(StructureFlags::OPEN);
            if state.contains(StructureFlags::GARAGEDOOR) && diagonal && !open {
                // cancel diagonal pathfinding through a closed sliding door
                return None;
            }
            open
        }
        // ... or two door structures and it's always diagonal
        Some(second_door) => {
            let state2 = door_state(world, second_door);
            if !state.intersects(StructureFlags::ANYDOOR)
                && !state2.intersects(StructureFlags::ANYDOOR)
            {
                // both doors destroyed?
                return Some(terrain_cost);
            }
            if !state.contains(StructureFlags::OPEN) || !state2.contains(StructureFlags::OPEN) {
                // cancel it if at least one of the doors is closed
                return None;
            }
            true
        }
    };
    let locked = || {
        door.and_then(|door| doors.iter().find(|d| d.grid_no == door))
            .is_some_and(|door| door.locked)
    };
    Some(match (open, obstacle_if_closed) {
        (true, true) => terrain_cost,
        (true, false) => TRAVELCOST_OBSTACLE,
        // the enemy opens closed doors, unless they are locked
        (false, true) if locked() => TRAVELCOST_OBSTACLE,
        (false, _) => terrain_cost,
    })
}

/// Whether a soldier can stand on a tile at the end of a path (`NewOKDestination` without
/// people and multi-tile soldiers): it is on the visible map and no structure blocks it
pub fn ok_destination(world: &MapWorld, grid_no: GridNo) -> bool {
    grid_no_on_visible_world_tile(grid_no, world.restricted_scroll)
        && !world.tiles[grid_no].structures.iter().any(|structure| {
            structure.flags.intersects(StructureFlags::BLOCKSMOVES)
                && !structure.flags.contains(StructureFlags::PASSABLE)
                && structure.cube_offset == 0
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        render::world::Structure,
        tileset::tests::{test_node, test_tileset, FIRSTTEXTURE},
    };

    /// A visible tile in the middle of the map
    const CENTER: usize = 80 * 160 + 80;

    /// The tile `delta` away from the center
    fn center(delta: i32) -> GridNo {
        grid(CENTER).offset(delta).unwrap()
    }

    fn grid(index: usize) -> GridNo {
        GridNo::new(index).unwrap()
    }

    fn structure(grid_no: GridNo, flags: StructureFlags, orientation: Orientation) -> Structure {
        Structure {
            id: 300,
            flags,
            wall_orientation: orientation,
            cube_offset: 0,
            grid_no,
            base_grid_no: grid_no,
            tile: test_node(&test_tileset(), FIRSTTEXTURE, 1).tile,
            structure_tile_index: 0,
            structure_height: 0,
        }
    }

    fn compile(world: &mut MapWorld) -> MovementCosts {
        let terrain = GridArray::filled(Terrain::FlatGround);
        MovementCosts::compile_with_terrain(world, terrain, &[], |_, _, _| false)
    }

    #[test]
    fn visible_area_depends_on_scroll_restriction() {
        assert!(!grid_no_on_visible_world_tile(grid(0), false));
        assert!(grid_no_on_visible_world_tile(grid(CENTER), false));
        assert!(grid_no_on_visible_world_tile(grid(CENTER), true));
        let (x, y) = (50, 50);
        assert!(grid_no_on_visible_world_tile(grid(y * 160 + x), false));
        assert!(!grid_no_on_visible_world_tile(grid(y * 160 + x), true));
        // x + y must be at least 86
        assert!(grid_no_on_visible_world_tile(grid(43 * 160 + 43), false));
        assert!(!grid_no_on_visible_world_tile(grid(42 * 160 + 43), false));
    }

    #[test]
    fn scrollable_area_depends_on_scroll_restriction() {
        assert_eq!(
            scrollable_area(false),
            (IVec2::new(-1610, 810), IVec2::new(1610, 2380))
        );
        assert_eq!(
            scrollable_area(true),
            (IVec2::new(-670, 1280), IVec2::new(650, 1900))
        );
    }

    #[test]
    fn off_map_tiles_and_terrain() {
        let mut world = MapWorld::new(false);
        let costs = compile(&mut world);
        assert_eq!(costs.cost(grid(0), North), TRAVELCOST_OFF_MAP);
        assert_eq!(costs.cost(grid(CENTER), North), TRAVELCOST_FLAT);
    }

    #[test]
    fn left_wall_blocks_north_and_south() {
        let mut world = MapWorld::new(false);
        world.tiles[grid(CENTER)].structures.push(structure(
            grid(CENTER),
            StructureFlags::WALL | StructureFlags::BASE_TILE,
            Orientation::INSIDE_LEFT,
        ));
        let costs = compile(&mut world);
        // Tiles with structures only get the terrain cost if they have land
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_NONE);
        world.tiles[grid(CENTER)]
            .land
            .push(test_node(&test_tileset(), FIRSTTEXTURE, 2));
        let costs = compile(&mut world);
        assert_eq!(costs.cost(grid(CENTER), North), TRAVELCOST_WALL);
        assert_eq!(costs.cost(grid(CENTER), NorthEast), TRAVELCOST_WALL);
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_FLAT);
        assert_eq!(costs.cost(center(160), South), TRAVELCOST_WALL);
        assert_eq!(costs.cost(center(-1), NorthWest), TRAVELCOST_WALL);
        assert_eq!(costs.cost(center(160), North), TRAVELCOST_FLAT);
    }

    #[test]
    fn set_only_raises_and_force_set_overwrites() {
        let mut costs = MovementCosts {
            costs: GridArray::filled([TRAVELCOST_NONE; 8]),
            terrain: GridArray::filled(Terrain::NoTerrain),
            writable: None,
        };
        costs.set(grid(5), North, TRAVELCOST_WALL);
        costs.set(grid(5), North, TRAVELCOST_FLAT);
        assert_eq!(costs.cost(grid(5), North), TRAVELCOST_WALL);
        costs.force_set(grid(5), North, TRAVELCOST_FENCE);
        assert_eq!(costs.cost(grid(5), North), TRAVELCOST_FENCE);
        // Writes off the map are ignored
        costs.set_at(grid(5), -160, North, TRAVELCOST_WALL);
        costs.clear_unless_blocked(None, North);
        assert_eq!(costs.cost(grid(5), North), TRAVELCOST_FENCE);
    }

    #[test]
    fn fence_clears_exits_depending_on_tile_order() {
        let mut world = MapWorld::new(false);
        world.tiles[grid(CENTER)].structures.push(structure(
            grid(CENTER),
            StructureFlags::FENCE | StructureFlags::BASE_TILE,
            Orientation::INSIDE_LEFT,
        ));
        let costs = compile(&mut world);
        assert_eq!(costs.cost(grid(CENTER), North), TRAVELCOST_FENCE);
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_OBSTACLE);
        // The tile above was compiled before and is cleared
        assert_eq!(costs.cost(center(-160), North), TRAVELCOST_NONE);
        // The tile below is compiled later and raises its cleared cost to the terrain again
        assert_eq!(costs.cost(center(160), South), TRAVELCOST_FLAT);
        assert_eq!(costs.cost(center(1), East), TRAVELCOST_OBSTACLE);
    }

    #[test]
    fn low_sandbags_can_be_jumped() {
        let mut world = MapWorld::new(false);
        world.tiles[grid(CENTER)].structures.push(structure(
            grid(CENTER),
            StructureFlags::GENERIC | StructureFlags::BASE_TILE,
            Orientation::empty(),
        ));
        world.tiles[center(1)].structures.push(structure(
            center(1),
            StructureFlags::GENERIC | StructureFlags::BASE_TILE,
            Orientation::empty(),
        ));
        let terrain = GridArray::filled(Terrain::FlatGround);
        let costs = MovementCosts::compile_with_terrain(&mut world, terrain, &[], |_, g, _| {
            g == grid(CENTER)
        });
        assert_eq!(costs.cost(grid(CENTER), North), TRAVELCOST_FENCE);
        assert_eq!(costs.cost(grid(CENTER), South), TRAVELCOST_FENCE);
        // Blocked by the obstacle next to it
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_OBSTACLE);
        assert_eq!(costs.cost(grid(CENTER), NorthEast), TRAVELCOST_OBSTACLE);
        assert_eq!(costs.cost(center(1), North), TRAVELCOST_OBSTACLE);
    }

    #[test]
    fn exit_grids_and_height_differences() {
        let mut world = MapWorld::new(false);
        world.tiles[center(1)].height = 50;
        let terrain = GridArray::filled(Terrain::FlatGround);
        let exit_grid = [center(-1)];
        let costs =
            MovementCosts::compile_with_terrain(&mut world, terrain, &exit_grid, |_, _, _| false);
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_OBSTACLE);
        assert_eq!(costs.cost(grid(CENTER), West), TRAVELCOST_FLAT);
        assert_eq!(costs.cost(center(-1), South), TRAVELCOST_EXITGRID);
    }

    #[test]
    fn door_costs_depend_on_the_door_state() {
        let mut world = MapWorld::new(false);
        world.tiles[grid(CENTER)].structures.push(structure(
            grid(CENTER),
            StructureFlags::DOOR | StructureFlags::BASE_TILE,
            Orientation::INSIDE_LEFT,
        ));
        let costs = compile(&mut world);
        assert_eq!(costs.cost(grid(CENTER), North), TRAVELCOST_DOOR_CLOSED_HERE);
        assert_eq!(costs.cost(grid(CENTER), East), TRAVELCOST_DOOR_OPEN_HERE);
        assert_eq!(costs.cost(center(160), South), TRAVELCOST_DOOR_CLOSED_N);

        let closed = |cost| door_travel_cost(&world, &costs, grid(CENTER), cost);
        assert_eq!(
            closed(TRAVELCOST_DOOR_CLOSED_HERE),
            (TRAVELCOST_DOOR, Some(grid(CENTER)))
        );
        assert_eq!(
            closed(TRAVELCOST_DOOR_OPEN_HERE),
            (TRAVELCOST_FLAT, Some(grid(CENTER)))
        );
        assert_eq!(
            door_travel_cost(&world, &costs, center(160), TRAVELCOST_DOOR_CLOSED_N),
            (TRAVELCOST_DOOR, Some(grid(CENTER)))
        );
        assert_eq!(closed(TRAVELCOST_WALL), (TRAVELCOST_WALL, None));

        world.tiles[grid(CENTER)].structures[0].flags |= StructureFlags::OPEN;
        let open = |cost| door_travel_cost(&world, &costs, grid(CENTER), cost);
        assert_eq!(
            open(TRAVELCOST_DOOR_CLOSED_HERE),
            (TRAVELCOST_FLAT, Some(grid(CENTER)))
        );
        assert_eq!(
            open(TRAVELCOST_DOOR_OPEN_HERE),
            (TRAVELCOST_DOOR_OPEN_HERE, Some(grid(CENTER)))
        );

        // No door at all counts as destroyed
        assert_eq!(
            door_travel_cost(&world, &costs, center(5), TRAVELCOST_DOOR_OPEN_HERE),
            (TRAVELCOST_DOOR_OPEN_HERE, Some(center(5)))
        );
    }
}
