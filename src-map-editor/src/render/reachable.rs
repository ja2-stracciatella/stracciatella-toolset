//! Which tiles a soldier can walk to, like the reachable tests of the game
//!
//! Port of `FindBestPath(s, destination, 0, WALKING, COPYREACHABLE, PATH_THROUGH_PEOPLE)`
//! (`PathAI.cc`) for the dummy soldier that the edge point functions use: a soldier of team 1
//! with a merc body (so it jumps fences and swims), on the friendly side, without keys, on the
//! ground level. People are ignored.
//!
//! A reachable test is not a shortest path search. Every tile is put into the queue once, the
//! first time a step reaches it, and the queue is ordered by the cost so far, the leg distance to
//! the destination and the time of insertion, newest first (the order of the game's skip list).
//! A tile is reachable when it is taken from the queue, unless the step that reached it jumped a
//! fence. So the order of the queue decides about the tiles behind fences, and it is kept. So are
//! the limits of the game: a test fails when the queue holds `MAX_QUEUED` tiles or the trail tree
//! is full, and the tiles taken from the queue until then stay reachable.

use std::{cmp::Reverse, collections::BinaryHeap};

use crate::{
    map::{Direction, GridArray, GridNo, TacticalMap},
    render::{
        movement_costs::{
            door_path_cost, grid_no_on_visible_world_tile, is_travelcost_door, ok_destination,
            terrain_movement_cost, MovementCosts, TRAVELCOST_BLOCKED, TRAVELCOST_DEEPWATER,
            TRAVELCOST_FENCE, TRAVELCOST_FLAT, TRAVELCOST_KNEEDEEP, TRAVELCOST_NOT_STANDING,
        },
        world::MapWorld,
    },
};

/// How many tiles the queue can hold, the path nodes of the pool minus the heads of the queue
/// and the closed list (`MAX_PATHQ`)
const MAX_QUEUED: usize = 1022;
/// Size of the trail tree of the global reachable test (`ABSMAX_TRAIL_TREE`)
pub const GLOBAL_TRAIL_TREE: usize = 65536;
/// Size of the trail tree of the other tests (`MAX_TRAIL_TREE`)
pub const TRAIL_TREE: usize = 16384;
/// The estimated cost to go of reachable tests
const COST_TO_GO: i32 = 100;
/// Raw grid number of nowhere (`NOWHERE`), the leg distance of tests without destination is
/// measured to it
const NOWHERE: i32 = 25601;
/// Cost of water on water to water paths (`EASYWATERCOST`)
const EASY_WATER_COST: i32 = TRAVELCOST_FLAT as i32 / 2;

/// The directions in the order the game tries them: north, then counter-clockwise
const DIRECTIONS: [Direction; 8] = [
    Direction::North,
    Direction::NorthWest,
    Direction::West,
    Direction::SouthWest,
    Direction::South,
    Direction::SouthEast,
    Direction::East,
    Direction::NorthEast,
];

/// A reachable test
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathRequest {
    pub origin: GridNo,
    /// The test ends once the destination is next in the queue
    pub destination: Option<GridNo>,
    /// How many tiles away from the origin the test goes, in rows or columns, 0 for no limit
    /// (`gubNPCDistLimit`)
    pub distance_limit: u8,
    /// `GLOBAL_TRAIL_TREE` or `TRAIL_TREE`
    pub trail_tree: usize,
}

/// A tile in the queue
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct QueueKey {
    total_cost: Reverse<u16>,
    leg_distance: Reverse<u8>,
    /// Newer tiles go before older ones with the same cost and leg distance
    order: u32,
}

#[derive(Clone, Copy, Debug)]
struct Queued {
    key: QueueKey,
    grid_no: GridNo,
    cost_so_far: u16,
    /// The direction of the step that reached the tile, `None` for the origin
    entered: Option<Direction>,
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Queued {}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// The queue pops the greatest, i.e. the cheapest tile
impl Ord for Queued {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

/// `abs(dx) + abs(dy)` in rows and columns of raw grid numbers, truncated like the game
/// (`LEGDISTANCE`)
fn leg_distance(grid_no: GridNo, destination: i32) -> u8 {
    let (x, y) = (grid_no.index() as i32 % 160, grid_no.index() as i32 / 160);
    ((destination % 160 - x).abs() + (destination / 160 - y).abs()) as u8
}

/// The larger of the row and column distance (`SpacesAway`)
pub fn spaces_away(a: GridNo, b: GridNo) -> i32 {
    let (a, b) = (a.position(), b.position());
    let rows = (i32::from(a.y) - i32::from(b.y)).abs();
    let columns = (i32::from(a.x) - i32::from(b.x)).abs();
    rows.max(columns)
}

/// Distance in tiles, truncated (`PythSpacesAway`)
pub fn pyth_spaces_away(a: GridNo, b: GridNo) -> i32 {
    let (a, b) = (a.position(), b.position());
    let rows = f64::from(i32::from(a.y) - i32::from(b.y));
    let columns = f64::from(i32::from(a.x) - i32::from(b.x));
    rows.hypot(columns) as i32
}

/// Water that soldiers swim through (`ISWATER`)
fn is_water(cost: i32) -> bool {
    cost == i32::from(TRAVELCOST_KNEEDEEP) || cost == i32::from(TRAVELCOST_DEEPWATER)
}

/// Runs reachable tests on a map, the counterpart of the path finding state of the game
pub struct PathFinder<'a> {
    world: &'a MapWorld,
    costs: &'a MovementCosts,
    map: &'a TacticalMap,
    /// The test that put a tile into the queue (`trailCostUsed`)
    queued_by: GridArray<u32>,
    test: u32,
    queue: BinaryHeap<Queued>,
}

impl<'a> PathFinder<'a> {
    pub fn new(world: &'a MapWorld, costs: &'a MovementCosts, map: &'a TacticalMap) -> Self {
        Self {
            world,
            costs,
            map,
            queued_by: GridArray::filled(0),
            test: 0,
            queue: BinaryHeap::new(),
        }
    }

    pub fn costs(&self) -> &'a MovementCosts {
        self.costs
    }

    /// The tiles reachable from `origin`, without limits (`GlobalReachableTest`)
    pub fn global_reachable(&mut self, origin: GridNo) -> GridArray<bool> {
        let mut reachable = GridArray::filled(false);
        self.find_path(
            &PathRequest {
                origin,
                destination: None,
                distance_limit: 0,
                trail_tree: GLOBAL_TRAIL_TREE,
            },
            |grid_no| reachable[grid_no] = true,
        );
        reachable
    }

    /// Runs a reachable test (`FindBestPath`), `reach` is called with every tile that is
    /// reachable (some more than once), returns whether the destination was found
    pub fn find_path(&mut self, request: &PathRequest, mut reach: impl FnMut(GridNo)) -> bool {
        let PathRequest {
            origin,
            destination,
            distance_limit,
            trail_tree,
        } = *request;
        if !grid_no_on_visible_world_tile(origin, self.world.restricted_scroll) {
            return false;
        }
        if let Some(destination) = destination {
            // the very first thing to do is make sure the destination tile is reachable
            if !ok_destination(self.world, destination) || destination == origin {
                return false;
            }
        }
        let raw_destination = destination.map_or(NOWHERE, |d| d.index() as i32);
        // if origin and dest is water, then user wants to stay in water!
        let water_to_water = destination.is_some_and(|destination| {
            is_water(self.costs.cost(origin, Direction::North).into())
                && is_water(self.costs.cost(destination, Direction::North).into())
        });

        self.test += 1;
        self.queue.clear();
        let mut order = 0;
        let mut trail_tree_index = 1;
        // The origin is not marked as queued, so a step back to it queues it again, like in the
        // game
        self.queue.push(Queued {
            key: QueueKey {
                total_cost: Reverse(COST_TO_GO as u16),
                leg_distance: Reverse(leg_distance(origin, raw_destination)),
                order,
            },
            grid_no: origin,
            cost_so_far: 0,
            entered: None,
        });

        while let Some(current) = self.queue.pop() {
            // The cost of the step here, the origin reads the cost of the (zeroed) first entry
            // of the trail tree
            let previous_cost = match current.entered {
                Some(direction) => self.costs.cost(current.grid_no, direction),
                None => self.costs.cost(GridNo::new(0).unwrap(), Direction::North),
            };
            if previous_cost != TRAVELCOST_FENCE {
                reach(current.grid_no);
            }

            for direction in DIRECTIONS {
                let Some(next) = current.grid_no.offset(direction.grid_no_increment()) else {
                    continue;
                };
                // on a "reachable" test, never revisit locations!
                if self.queued_by[next] == self.test {
                    continue;
                }
                let tiles = &self.world.tiles;
                if tiles[next].height != tiles[current.grid_no].height {
                    continue;
                }
                if distance_limit != 0 && spaces_away(origin, next) > i32::from(distance_limit) {
                    continue;
                }
                // A friendly soldier of another team avoids the player's mines
                if self.map.tiles[next].player_mine {
                    continue;
                }
                let Some(cost) = self.step_cost(next, direction) else {
                    continue;
                };
                // make the destination look very attractive
                let mut cost = if Some(next) == destination { 0 } else { cost };
                if water_to_water && is_water(cost) {
                    cost = EASY_WATER_COST;
                }
                if direction.is_diagonal() {
                    cost = cost * 14 / 10;
                }
                let total_cost = i32::from(current.cost_so_far) + cost;

                if self.queue.len() >= MAX_QUEUED {
                    return false;
                }
                trail_tree_index += 1;
                if trail_tree_index >= trail_tree {
                    return false;
                }
                order += 1;
                self.queued_by[next] = self.test;
                self.queue.push(Queued {
                    key: QueueKey {
                        total_cost: Reverse((total_cost + COST_TO_GO) as u16),
                        leg_distance: Reverse(leg_distance(next, raw_destination)),
                        order,
                    },
                    grid_no: next,
                    cost_so_far: total_cost as u16,
                    entered: Some(direction),
                });
            }

            if let Some(head) = self.queue.peek() {
                if Some(head.grid_no) == destination {
                    return true;
                }
            }
        }
        false
    }

    /// The cost of stepping into a tile, `None` if the step can't be made
    fn step_cost(&self, grid_no: GridNo, direction: Direction) -> Option<i32> {
        let terrain_cost = || terrain_movement_cost(self.costs.terrain(grid_no));
        let cost = self.costs.cost(grid_no, direction);
        let cost = if cost == TRAVELCOST_NOT_STANDING {
            // for path plotting purposes, use the terrain value
            terrain_cost()
        } else if is_travelcost_door(cost) {
            door_path_cost(
                self.world,
                self.costs,
                &self.map.doors,
                grid_no,
                cost,
                direction.is_diagonal(),
            )?
        } else {
            // Exit grids block, merc bodies jump fences and swim
            cost
        };
        (cost < TRAVELCOST_BLOCKED).then_some(i32::from(cost))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        map::{Door, GridArray},
        render::{
            movement_costs::{grid_no_on_edge_of_map, TRAVELCOST_OBSTACLE},
            world::Structure,
        },
        tileset::{
            tests::{test_node, test_tileset, FIRSTTEXTURE},
            Orientation, StructureFlags, Terrain, TilesetId,
        },
    };

    pub(crate) fn grid(x: u8, y: u8) -> GridNo {
        GridNo::from(GridPosition::new(x, y))
    }

    /// A flat open map with exit grids on some tiles
    pub(crate) struct Open {
        pub world: MapWorld,
        pub map: TacticalMap,
    }

    impl Open {
        pub(crate) fn new() -> Self {
            Self {
                world: MapWorld::new(false),
                map: TacticalMap::empty(TilesetId::new(0)),
            }
        }

        /// Adds a structure on the ground level
        pub(crate) fn add(
            &mut self,
            grid_no: GridNo,
            flags: StructureFlags,
            orientation: Orientation,
        ) {
            let tile = &mut self.world.tiles[grid_no];
            if tile.land.is_empty() {
                tile.land.push(test_node(&test_tileset(), FIRSTTEXTURE, 1));
            }
            tile.structures.push(Structure {
                id: 300,
                flags,
                wall_orientation: orientation,
                cube_offset: 0,
                grid_no,
                base_grid_no: grid_no,
                tile: test_node(&test_tileset(), FIRSTTEXTURE, 1).tile,
                structure_tile_index: 0,
                structure_height: 0,
            });
        }

        /// A wall between a column and the next one, from row `y0` to `y1`
        pub(crate) fn wall_column(&mut self, x: u8, y0: u8, y1: u8) {
            for y in y0..=y1 {
                self.add(
                    grid(x, y),
                    StructureFlags::WALL | StructureFlags::BASE_TILE,
                    Orientation::INSIDE_RIGHT,
                );
            }
        }

        pub(crate) fn costs(&mut self) -> MovementCosts {
            let exit_grids: Vec<GridNo> = self
                .map
                .exit_grids
                .iter()
                .map(|grid| grid.grid_no)
                .collect();
            MovementCosts::compile_with_terrain(
                &mut self.world,
                GridArray::filled(Terrain::FlatGround),
                &exit_grids,
                |_, _, _| false,
            )
        }

        /// The tiles reachable from a tile within a limit, 0 for none
        pub(crate) fn reachable(&mut self, origin: GridNo, limit: u8) -> GridArray<bool> {
            let costs = self.costs();
            let mut finder = PathFinder::new(&self.world, &costs, &self.map);
            let mut reachable = GridArray::filled(false);
            finder.find_path(
                &PathRequest {
                    origin,
                    destination: None,
                    distance_limit: limit,
                    trail_tree: GLOBAL_TRAIL_TREE,
                },
                |grid_no| reachable[grid_no] = true,
            );
            reachable
        }

        pub(crate) fn path(&mut self, from: GridNo, to: GridNo, limit: u8) -> bool {
            let costs = self.costs();
            let mut finder = PathFinder::new(&self.world, &costs, &self.map);
            finder.find_path(
                &PathRequest {
                    origin: from,
                    destination: Some(to),
                    distance_limit: limit,
                    trail_tree: TRAIL_TREE,
                },
                |_| {},
            )
        }
    }

    #[test]
    fn distances_are_measured_like_the_game() {
        assert_eq!(spaces_away(grid(10, 10), grid(13, 5)), 5);
        assert_eq!(pyth_spaces_away(grid(10, 10), grid(13, 14)), 5);
        assert_eq!(pyth_spaces_away(grid(10, 10), grid(11, 11)), 1);
        // Leg distances to nowhere wrap around
        assert_eq!(leg_distance(grid(0, 0), NOWHERE), 161);
        assert_eq!(leg_distance(grid(159, 159), NOWHERE), (158 + 1) as u8);
        assert_eq!(leg_distance(grid(100, 0), NOWHERE), (99 + 160_i32) as u8);
    }

    #[test]
    fn open_maps_are_reachable_up_to_the_edge() {
        let mut open = Open::new();
        let reachable = open.reachable(grid(80, 80), 0);
        assert!(reachable[grid(80, 80)] && reachable[grid(80, 10)]);
        // Off the visible map
        assert!(!reachable[grid(0, 0)]);
        let costs = open.costs();
        let edge = GridNo::all()
            .filter(|&g| reachable[g] && grid_no_on_edge_of_map(&costs, g))
            .count();
        assert!(edge > 300, "{edge} tiles on the edge");
        assert!(!grid_no_on_edge_of_map(&costs, grid(80, 80)));
        // Tests from off the visible map find nothing
        assert!(!open.reachable(grid(0, 0), 0).values().any(|&r| r));
    }

    #[test]
    fn limits_keep_the_test_near_the_origin() {
        let mut open = Open::new();
        let reachable = open.reachable(grid(80, 80), 5);
        let count = reachable.values().filter(|&&r| r).count();
        assert_eq!(count, 11 * 11);
        assert!(reachable[grid(85, 75)] && !reachable[grid(86, 80)]);
        assert!(open.path(grid(80, 80), grid(84, 84), 5));
        assert!(!open.path(grid(80, 80), grid(86, 80), 5));
        // A path to the origin fails like in the game
        assert!(!open.path(grid(80, 80), grid(80, 80), 5));
    }

    #[test]
    fn walls_heights_and_exit_grids_block() {
        let mut open = Open::new();
        // A closed room from (79, 79) to (82, 82) around the origin
        open.wall_column(78, 79, 82);
        open.wall_column(82, 79, 82);
        for x in 79..=82 {
            open.add(
                grid(x, 78),
                StructureFlags::WALL | StructureFlags::BASE_TILE,
                Orientation::INSIDE_LEFT,
            );
            open.add(
                grid(x, 82),
                StructureFlags::WALL | StructureFlags::BASE_TILE,
                Orientation::INSIDE_LEFT,
            );
        }
        let reachable = open.reachable(grid(80, 80), 0);
        assert!(reachable[grid(79, 79)]);
        assert!(!reachable[grid(70, 70)]);

        // Other heights are not entered
        let mut open = Open::new();
        open.world.tiles[grid(81, 80)].height = 10;
        assert!(!open.reachable(grid(80, 80), 3)[grid(81, 80)]);

        // Exit grids block
        let mut open = Open::new();
        open.map.exit_grids.push(crate::map::ExitGrid {
            grid_no: grid(81, 80),
            dest_grid_no: grid(1, 1),
            dest_sector: Default::default(),
        });
        assert!(!open.reachable(grid(80, 80), 3)[grid(81, 80)]);
        assert!(!open.path(grid(80, 80), grid(81, 80), 3));
        assert!(open.reachable(grid(80, 80), 3)[grid(82, 80)]);
    }

    #[test]
    fn closed_doors_are_opened_unless_locked() {
        let door_tile = grid(80, 80);
        let blocked = |locked: Option<bool>| {
            let mut open = Open::new();
            // A wall across the map with a door in it, left of column 80
            open.wall_column(80, 60, 79);
            open.wall_column(80, 81, 100);
            open.add(
                door_tile,
                StructureFlags::DOOR | StructureFlags::BASE_TILE,
                Orientation::INSIDE_RIGHT,
            );
            if let Some(locked) = locked {
                open.map.doors.push(Door {
                    grid_no: door_tile,
                    locked,
                    trap: None,
                    lock: None,
                    perceived_locked: 0,
                    perceived_trapped: 0,
                    lock_damage: 0,
                });
            }
            !open.path(grid(78, 80), grid(82, 80), 10)
        };
        assert!(!blocked(None));
        assert!(!blocked(Some(false)));
        assert!(blocked(Some(true)));
    }

    #[test]
    fn tiles_behind_fences_are_passed_but_not_reachable() {
        let mut open = Open::new();
        // A fence that can be jumped north and south, on a tile with obstacles left and right
        let fence = grid(80, 78);
        open.add(
            fence,
            StructureFlags::FENCE | StructureFlags::BASE_TILE,
            Orientation::INSIDE_LEFT,
        );
        for x in [79, 81] {
            open.add(grid(x, 78), StructureFlags::GENERIC, Orientation::empty());
        }
        let costs = open.costs();
        assert_eq!(costs.cost(fence, Direction::North), TRAVELCOST_FENCE);
        assert_eq!(
            costs.cost(grid(79, 78), Direction::North),
            TRAVELCOST_OBSTACLE
        );
        let reachable = open.reachable(grid(80, 79), 2);
        // The fence tile is entered by jumping, the tiles behind it are reachable
        assert!(!reachable[fence]);
        assert!(reachable[grid(80, 77)]);
    }

    #[test]
    fn mines_of_the_player_block() {
        let mut open = Open::new();
        open.map.tiles[grid(81, 80)].player_mine = true;
        open.map.tiles[grid(79, 80)].enemy_mine = true;
        let reachable = open.reachable(grid(80, 80), 2);
        assert!(!reachable[grid(81, 80)]);
        assert!(reachable[grid(79, 80)]);
    }
}
