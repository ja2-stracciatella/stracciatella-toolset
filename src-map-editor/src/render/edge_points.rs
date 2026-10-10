//! Generating the edge points of a map, like the editor of the game does before it saves
//!
//! Port of `GenerateMapEdgepoints` and the functions it calls (`Map_Edgepoints.cc`). The quirks
//! are kept, so the result is what the game would save:
//!
//! - The tiles of each side are collected in two diagonal rows, the outer one first. Both rows
//!   only have tiles that are reachable from the first entry point (north, east, south, west,
//!   center) and, on the surface, not in a room.
//! - The secondary set has the tiles reachable from the isolated entry point, if the first test
//!   didn't reach it.
//! - Tiles that can't reach the edge of the map within 5 tiles are removed (`ValidateEdgepoints`),
//!   which moves the middle index to the next tile that is kept.
//! - With an isolated entry point, the primary tiles that are not connected to the entry point of
//!   their side within 5 tiles (250 in I14) are moved to the secondary set
//!   (`ClassifyEdgepoints`). The game counts the moved tiles of the outer row into the middle
//!   index of the secondary list, although they are added at its end.

use crate::{
    map::{
        EdgePointList, EdgePointSet, EdgePoints, EditorSmoothing, GridArray, GridNo, SectorId,
        TacticalMap,
    },
    render::{
        movement_costs::{grid_no_on_edge_of_map, MovementCosts},
        reachable::{pyth_spaces_away, PathFinder, PathRequest, TRAIL_TREE},
        world::MapWorld,
    },
};

/// The corners of the edge rows (`gsTLGridNo`, `gsTRGridNo`, `gsBLGridNo`, `gsBRGridNo`)
const TOP_LEFT: i32 = 13286;
const TOP_RIGHT: i32 = 1043;
const BOTTOM_LEFT: i32 = 24878;
const BOTTOM_RIGHT: i32 = 12635;

/// How far edge points may be from the edge of the map and from each other
/// (`EDGE_OF_MAP_SEARCH`)
const EDGE_OF_MAP_SEARCH: u8 = 5;
/// How far edge points of I14 may be from each other, the special case of `EdgepointsClose`
const I14_SEARCH: u8 = 250;

/// A diagonal row of tiles: from `start`, two steps at a time while the grid number hasn't
/// passed `end`
#[derive(Clone, Copy)]
struct Row {
    start: i32,
    end: i32,
    steps: [i32; 2],
}

impl Row {
    fn tiles(self) -> Vec<i32> {
        let mut tiles = vec![self.start];
        let mut grid_no = self.start;
        let descending = self.steps[0] + self.steps[1] < 0;
        while if descending {
            grid_no > self.end
        } else {
            grid_no < self.end
        } {
            for step in self.steps {
                grid_no += step;
                tiles.push(grid_no);
            }
        }
        tiles
    }
}

/// The outer and the inner row of each side, north, east, south, west
fn side_rows() -> [[Row; 2]; 4] {
    let row = |start, end, steps| Row { start, end, steps };
    let pair = |start, end, offset, steps| {
        [
            row(start, end, steps),
            row(start + offset, end + offset, steps),
        ]
    };
    [
        pair(TOP_LEFT, TOP_RIGHT, 161, [1, -160]),
        pair(TOP_RIGHT, BOTTOM_RIGHT, 159, [160, 1]),
        pair(BOTTOM_LEFT, BOTTOM_RIGHT, -161, [1, -160]),
        pair(TOP_LEFT, BOTTOM_LEFT, -159, [1, 160]),
    ]
}

/// The edge points of the sides that have an entry point, from the tiles of a reachable test
fn collect_edge_points(
    map: &TacticalMap,
    reachable: &GridArray<bool>,
    entries: [Option<GridNo>; 4],
) -> EdgePointSet {
    let basement = map.info.editor_smoothing == EditorSmoothing::Basement;
    let usable =
        |grid_no: GridNo| reachable[grid_no] && (map.tiles[grid_no].room.is_none() || basement);
    let mut set = EdgePointSet::default();
    for ((list, rows), entry) in set.sides_mut().into_iter().zip(side_rows()).zip(entries) {
        if entry.is_none() {
            continue;
        }
        for (index, row) in rows.into_iter().enumerate() {
            if index == 1 {
                list.middle_index = list.grid_nos.len() as u16;
            }
            list.grid_nos.extend(
                row.tiles()
                    .into_iter()
                    .filter_map(GridNo::from_i32)
                    .filter(|&grid_no| usable(grid_no)),
            );
        }
    }
    set
}

/// Generates the edge points of a map (`GenerateMapEdgepoints`)
///
/// `costs` must be up to date with the world and the exit grids of the map. `sector` is the
/// sector of the map, which only matters for I14. The secondary set is always generated, maps
/// before version 17 don't save it.
pub fn generate_edge_points(
    world: &MapWorld,
    costs: &MovementCosts,
    map: &TacticalMap,
    sector: Option<SectorId>,
) -> EdgePoints {
    let info = &map.info;
    let entries = [
        info.north_entry,
        info.east_entry,
        info.south_entry,
        info.west_entry,
    ];
    let mut edge_points = EdgePoints {
        primary: EdgePointSet::default(),
        secondary: Some(EdgePointSet::default()),
    };
    let Some(seed) = entries.into_iter().flatten().chain(info.center).next() else {
        return edge_points;
    };

    let mut finder = PathFinder::new(world, costs, map);
    let reachable = finder.global_reachable(seed);
    let mut primary = collect_edge_points(map, &reachable, entries);
    // Only if the isolated entry point is really isolated
    let mut secondary = match info.isolated.filter(|&isolated| !reachable[isolated]) {
        Some(isolated) => {
            let reachable = finder.global_reachable(isolated);
            collect_edge_points(map, &reachable, entries)
        }
        None => EdgePointSet::default(),
    };

    // Eliminates the edge points that have no path to the edge of the world
    for list in primary.sides_mut().into_iter().chain(secondary.sides_mut()) {
        validate_map_edge(&mut finder, list);
    }
    // The design flaw of the original: obstacles between normal edge points also move them to
    // the secondary list on maps with an isolated entry point
    if info.isolated.is_some() {
        let range = if sector == Some(SectorId { x: 14, y: 9, z: 0 }) {
            I14_SEARCH
        } else {
            EDGE_OF_MAP_SEARCH
        };
        let mut close = |from: GridNo, to: GridNo| {
            finder.find_path(
                &PathRequest {
                    origin: from,
                    destination: Some(to),
                    distance_limit: range,
                    trail_tree: TRAIL_TREE,
                },
                |_| {},
            )
        };
        for ((first, second), entry) in primary
            .sides_mut()
            .into_iter()
            .zip(secondary.sides_mut())
            .zip(entries)
        {
            let Some(entry) = entry else {
                continue;
            };
            if let Some(nearest) = nearest_edge_point(first, entry) {
                classify_edge_points(&mut close, nearest, first, second);
            }
        }
    }
    edge_points.primary = primary;
    edge_points.secondary = Some(secondary);
    edge_points
}

/// Whether a soldier at an edge point can reach the edge of the map within 5 tiles
/// (`VerifyEdgepoint`)
fn verify_edge_point(finder: &mut PathFinder, grid_no: GridNo) -> bool {
    let mut on_edge = false;
    let costs = finder.costs();
    let mut reached = vec![];
    finder.find_path(
        &PathRequest {
            origin: grid_no,
            destination: None,
            distance_limit: EDGE_OF_MAP_SEARCH,
            trail_tree: TRAIL_TREE,
        },
        |grid_no| reached.push(grid_no),
    );
    for grid_no in reached {
        on_edge |= grid_no_on_edge_of_map(costs, grid_no);
    }
    on_edge
}

/// Removes the edge points that can't reach the edge of the map (`ValidateMapEdge`)
///
/// The middle index moves to the next edge point that is kept.
fn validate_map_edge(finder: &mut PathFinder, list: &mut EdgePointList) {
    let mut middle = usize::from(list.middle_index);
    let mut kept = 0;
    for index in 0..list.grid_nos.len() {
        let grid_no = list.grid_nos[index];
        if verify_edge_point(finder, grid_no) {
            list.grid_nos[kept] = grid_no;
            if middle == index {
                middle = kept;
            }
            kept += 1;
        } else if middle == index {
            middle += 1;
        }
    }
    list.middle_index = middle as u16;
    list.grid_nos.truncate(kept);
}

/// The edge point of a list closest to a tile, the first one of equal distance
/// (`FindNearestEdgepointOnSpecifiedEdge`)
fn nearest_edge_point(list: &EdgePointList, grid_no: GridNo) -> Option<GridNo> {
    let mut nearest = None;
    let mut nearest_distance = i32::from(i16::MAX);
    for &edge_point in &list.grid_nos {
        let distance = pyth_spaces_away(grid_no, edge_point);
        if distance < nearest_distance {
            nearest_distance = distance;
            nearest = Some(edge_point);
        }
    }
    nearest
}

/// Moves the edge points of a side that are not connected with `grid_no` to the secondary list
/// (`InternallyClassifyEdgepoints`)
///
/// From the edge point and the closest one of the other row, the "benchmarks", the connected
/// edge points are followed in both directions of each row. Everything after the first gap is
/// moved. Where the game would read past the end of the list (the middle index can be past it),
/// the rows end at the end of the list instead.
fn classify_edge_points(
    close: &mut impl FnMut(GridNo, GridNo) -> bool,
    grid_no: GridNo,
    primary: &mut EdgePointList,
    secondary: &mut EdgePointList,
) {
    let mut points: Vec<Option<GridNo>> = primary.grid_nos.iter().copied().map(Some).collect();
    let len = points.len();
    let middle = usize::from(primary.middle_index);
    let outer_end = middle.min(len);
    let mut connected = |points: &[Option<GridNo>], a: usize, b: usize| match (points[a], points[b])
    {
        (Some(a), Some(b)) => close(a, b),
        _ => false,
    };

    let (mut first, mut second) = (None, None);
    if let Some(index) = points.iter().position(|&point| point == Some(grid_no)) {
        if index < middle {
            // in the first half of the array, find the second benchmark
            first = Some(index);
            second = (middle..len).find(|&other| connected(&points, index, other));
        } else {
            // in the second half of the array, find the first benchmark
            second = Some(index);
            first = (0..outer_end).find(|&other| connected(&points, index, other));
        }
    }

    let mut reject = |points: &mut [Option<GridNo>], index: usize, count_middle: bool| {
        if let Some(point) = points[index].take() {
            secondary.grid_nos.push(point);
        }
        if count_middle {
            secondary.middle_index = secondary.middle_index.wrapping_add(1);
        }
    };
    if let Some(benchmark) = first {
        let mut index = benchmark;
        while index > 0 {
            if !connected(&points, index, index - 1) {
                // All edge points from index 0 to index - 1 are rejected
                while index > 0 {
                    index -= 1;
                    reject(&mut points, index, true);
                }
                break;
            }
            index -= 1;
        }
        let mut index = benchmark;
        while index + 1 < outer_end {
            if !connected(&points, index, index + 1) {
                // All edge points from index + 1 to the middle index are rejected
                while index + 1 < outer_end {
                    index += 1;
                    reject(&mut points, index, true);
                }
                break;
            }
            index += 1;
        }
    }
    if let Some(benchmark) = second {
        let mut index = benchmark;
        while index > middle {
            if !connected(&points, index, index - 1) {
                // All edge points from the middle index to index - 1 are rejected
                while index > middle {
                    index -= 1;
                    reject(&mut points, index, false);
                }
                break;
            }
            index -= 1;
        }
        let mut index = benchmark;
        while index + 1 < len {
            if !connected(&points, index, index + 1) {
                // All edge points after index are rejected
                while index + 1 < len {
                    index += 1;
                    reject(&mut points, index, false);
                }
                break;
            }
            index += 1;
        }
    }

    // `CompactEdgepointArray`, which compares with the middle index while it moves it
    let mut middle = primary.middle_index;
    let mut kept = Vec::with_capacity(len);
    for (index, point) in points.into_iter().enumerate() {
        match point {
            Some(point) => kept.push(point),
            None if index < usize::from(middle) => middle -= 1,
            None => {}
        }
    }
    primary.grid_nos = kept;
    primary.middle_index = middle;
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        map::RoomNo,
        render::reachable::tests::{grid, Open},
    };

    fn generate(open: &mut Open, sector: Option<SectorId>) -> EdgePoints {
        let costs = open.costs();
        generate_edge_points(&open.world, &costs, &open.map, sector)
    }

    fn raw(list: &EdgePointList) -> Vec<usize> {
        list.grid_nos.iter().map(|g| g.index()).collect()
    }

    #[test]
    fn rows_follow_the_edges_of_the_map() {
        let [north, east, south, west] = side_rows();
        let outer = north[0].tiles();
        assert_eq!(outer.first(), Some(&TOP_LEFT));
        assert_eq!(outer.last(), Some(&TOP_RIGHT));
        assert_eq!(outer.len(), 1 + 2 * 77);
        assert_eq!(north[1].tiles()[0], TOP_LEFT + 161);
        assert_eq!(east[0].tiles().last(), Some(&BOTTOM_RIGHT));
        assert_eq!(south[0].tiles().last(), Some(&BOTTOM_RIGHT));
        assert_eq!(west[0].tiles().last(), Some(&BOTTOM_LEFT));
        assert_eq!(west[1].tiles()[0], TOP_LEFT - 159);
    }

    #[test]
    fn open_maps_have_full_edges() {
        let mut open = Open::new();
        open.map.info.north_entry = Some(grid(50, 50));
        open.map.info.east_entry = Some(grid(110, 50));
        open.map.info.south_entry = Some(grid(110, 110));
        open.map.info.west_entry = Some(grid(50, 110));
        let edge_points = generate(&mut open, None);
        let primary = &edge_points.primary;
        for list in primary.sides() {
            assert!(list.grid_nos.len() > 100, "{}", list.grid_nos.len());
            assert!(usize::from(list.middle_index) < list.grid_nos.len());
        }
        // The outer row starts at the corner
        assert_eq!(raw(&primary.north)[0], TOP_LEFT as usize);
        assert_eq!(edge_points.secondary, Some(EdgePointSet::default()));

        // Without entry points there are none
        let mut open = Open::new();
        assert_eq!(generate(&mut open, None).primary, EdgePointSet::default());
    }

    #[test]
    fn sides_without_an_entry_point_are_empty() {
        let mut open = Open::new();
        open.map.info.center = Some(grid(80, 80));
        open.map.info.north_entry = Some(grid(50, 50));
        let primary = generate(&mut open, None).primary;
        assert!(!primary.north.grid_nos.is_empty());
        assert!(primary.east.grid_nos.is_empty() && primary.east.middle_index == 0);
        assert!(primary.south.grid_nos.is_empty() && primary.west.grid_nos.is_empty());
    }

    #[test]
    fn walls_across_the_map_make_secondary_edge_points() {
        let mut open = Open::new();
        // A wall across the map between columns 80 and 81
        open.wall_column(80, 0, 159);
        open.map.info.north_entry = Some(grid(50, 50));
        open.map.info.south_entry = Some(grid(60, 110));
        open.map.info.isolated = Some(grid(110, 80));
        let edge_points = generate(&mut open, None);
        let primary = &edge_points.primary;
        let secondary = edge_points.secondary.unwrap();
        // The primary edge points are left of the wall, the secondary ones right of it
        assert!(!primary.north.grid_nos.is_empty() && !secondary.north.grid_nos.is_empty());
        assert!(primary.north.grid_nos.iter().all(|g| g.position().x <= 80));
        assert!(secondary.north.grid_nos.iter().all(|g| g.position().x > 80));
        assert!(secondary.south.grid_nos.iter().all(|g| g.position().x > 80));
        // East has no entry point
        assert!(secondary.east.grid_nos.is_empty());

        // An isolated entry point that is reachable adds nothing
        open.map.info.isolated = Some(grid(51, 51));
        let secondary = generate(&mut open, None).secondary.unwrap();
        assert_eq!(secondary.north.grid_nos.len(), 0);
    }

    #[test]
    fn classifying_moves_points_behind_gaps_to_the_secondary_list() {
        let mut open = Open::new();
        // A wall that splits the north side, the east part is still reachable around it
        open.wall_column(80, 0, 60);
        open.map.info.north_entry = Some(grid(50, 50));
        let unclassified = generate(&mut open, None).primary;
        assert!(unclassified
            .north
            .grid_nos
            .iter()
            .any(|g| g.position().x > 80));

        open.map.info.isolated = Some(grid(51, 51));
        let edge_points = generate(&mut open, None);
        let north = &edge_points.primary.north;
        let secondary = &edge_points.secondary.unwrap().north;
        assert!(north.grid_nos.iter().all(|g| g.position().x <= 80));
        assert!(secondary.grid_nos.iter().all(|g| g.position().x > 80));
        assert_eq!(
            north.grid_nos.len() + secondary.grid_nos.len(),
            unclassified.north.grid_nos.len()
        );
        // The rejects of the outer row count into the middle index of the secondary list
        assert!(secondary.middle_index > 0);
    }

    #[test]
    fn rooms_are_excluded_on_the_surface_only() {
        let mut open = Open::new();
        open.map.info.north_entry = Some(grid(50, 50));
        let corner = GridNo::from_i32(TOP_LEFT).unwrap();
        open.map.tiles[corner].room = RoomNo::new(1);
        let north = generate(&mut open, None).primary.north;
        assert!(!north.grid_nos.contains(&corner));

        open.map.info.editor_smoothing = EditorSmoothing::Basement;
        let north = generate(&mut open, None).primary.north;
        assert_eq!(north.grid_nos[0], corner);
    }

    #[test]
    fn validation_moves_the_middle_index() {
        let mut open = Open::new();
        let costs = open.costs();
        let mut finder = PathFinder::new(&open.world, &costs, &open.map);
        let edge = GridNo::from_i32(TOP_LEFT).unwrap();
        let inner = grid(80, 80);
        // The middle edge point is too far from the edge, the next one takes its place
        let mut list = EdgePointList {
            middle_index: 1,
            grid_nos: vec![edge, inner, edge],
        };
        validate_map_edge(&mut finder, &mut list);
        assert_eq!(list.grid_nos, [edge, edge]);
        assert_eq!(list.middle_index, 1);
        // When nothing follows, it points past the end
        let mut list = EdgePointList {
            middle_index: 1,
            grid_nos: vec![edge, inner, inner],
        };
        validate_map_edge(&mut finder, &mut list);
        assert_eq!(list.grid_nos, [edge]);
        assert_eq!(list.middle_index, 3);
    }

    #[test]
    fn i14_connects_edge_points_further_away() {
        let mut open = Open::new();
        // A short wall between two parts of the north edge, too long to walk around within 5
        // tiles but not within 250
        open.wall_column(80, 0, 30);
        open.map.info.north_entry = Some(grid(50, 50));
        open.map.info.isolated = Some(grid(51, 51));
        let other = generate(&mut open, SectorId::from_map_path("maps/h14.dat"));
        let i14 = generate(&mut open, SectorId::from_map_path("maps/i14.dat"));
        assert!(!other.secondary.unwrap().north.grid_nos.is_empty());
        assert!(i14.secondary.unwrap().north.grid_nos.is_empty());
    }

    /// The file at a path below `dir`, whatever the case of its components
    pub(crate) fn find_file(dir: &std::path::Path, path: &str) -> Option<std::path::PathBuf> {
        let mut found = dir.to_path_buf();
        for component in path.split('/') {
            let entry = std::fs::read_dir(&found).ok()?.flatten().find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(component)
            })?;
            found = entry.path();
        }
        Some(found)
    }

    /// The tileset of the game data in a directory
    pub(crate) fn data_tileset(
        dir: &std::path::Path,
        tilesets: &crate::file_formats::ja2_tileset::Ja2Tilesets,
        tileset_id: crate::tileset::TilesetId,
    ) -> crate::tileset::Tileset {
        use crate::file_formats::{
            ja2_stci::SubImageInfo,
            ja2_structure::Ja2StructureFile,
            ja2_tile_database::{Ja2TileDatabase, TileSurfaceInfo},
            ja2_tile_types::NUMBER_OF_TILE_TYPES,
            stci::Stci,
        };
        let tileset = tilesets.get(tileset_id.index()).unwrap();
        let surfaces = (0..NUMBER_OF_TILE_TYPES)
            .map(|tile_type| {
                let resolved = tilesets.resolve_tile_surface(tileset_id, tile_type)?;
                let file = find_file(
                    dir,
                    &format!("tilesets/{}/{}", resolved.tileset_id, resolved.filename),
                )?;
                let bytes = std::fs::read(&file).ok()?;
                let stci = Stci::from_input(&mut std::io::Cursor::new(bytes))
                    .map_err(|e| eprintln!("{}: {}", file.display(), e))
                    .ok()?;
                let jsd = resolved.filename.rsplit_once('.').unwrap().0.to_owned() + ".jsd";
                let structure =
                    find_file(dir, &format!("tilesets/{}/{}", resolved.tileset_id, jsd))
                        .and_then(|jsd| Ja2StructureFile::parse(&std::fs::read(jsd).ok()?).ok());
                Some(TileSurfaceInfo {
                    tileset_id: resolved.tileset_id,
                    filename: resolved.filename,
                    sub_images: SubImageInfo::from_stci(&stci),
                    structure,
                    terrain: tileset.terrain_id(tile_type),
                })
            })
            .collect();
        crate::tileset::Tileset::from_database(Ja2TileDatabase::build(surfaces).unwrap())
    }

    /// Generates the edge points of every map in the game data in `JA2_DATA_DIR` (or the maps
    /// whose name starts with `JA2_EDGE_POINT_MAPS`) and compares them with the saved ones
    ///
    /// The directory has `maps/*.dat`, `tilesets/<id>/*.sti|jsd` and `ja2set.dat` in
    /// `binarydata/` or `tilesets/`, in any case. Not all maps of the game match: they were saved
    /// by the original editor, some were changed afterwards (e.g. trees on saved edge points) and
    /// I14 was saved without its special case. With the vanilla maps, 222 of 261 match.
    #[test]
    fn generated_edge_points_match_the_maps_of_the_game() {
        use std::collections::HashMap;

        use crate::{
            file_formats::{ja2_map::Ja2Map, ja2_tileset::Ja2Tilesets},
            map::convert_tests::Fixture,
            tileset::TilesetId,
        };

        let Ok(dir) = std::env::var("JA2_DATA_DIR") else {
            eprintln!("JA2_DATA_DIR not set, skipping");
            return;
        };
        let prefix = std::env::var("JA2_EDGE_POINT_MAPS").unwrap_or_default();
        let dir = std::path::Path::new(&dir);
        let ja2set = find_file(dir, "binarydata/ja2set.dat")
            .or_else(|| find_file(dir, "tilesets/ja2set.dat"))
            .expect("ja2set.dat should exist");
        let tilesets = Ja2Tilesets::parse(&std::fs::read(ja2set).unwrap()).unwrap();
        let mut fixtures: HashMap<TilesetId, Fixture> = HashMap::new();
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
        let (mut matched, mut failures) = (0, vec![]);
        for path in paths {
            let Ok(file) = Ja2Map::parse(&std::fs::read(&path).unwrap()) else {
                continue;
            };
            let Some(saved) = file
                .edge_points
                .as_ref()
                .filter(|_| file.info.map_version >= 22)
            else {
                continue;
            };
            let tileset_id = TilesetId::new(file.tileset_id as u8);
            let fixture = fixtures
                .entry(tileset_id)
                .or_insert_with(|| Fixture::with_tileset(data_tileset(dir, &tilesets, tileset_id)));
            let map = fixture.convert(&file);
            let mut world = MapWorld::build(&map);
            let costs = MovementCosts::compile(&mut world, &map);
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let generated =
                generate_edge_points(&world, &costs, &map, SectorId::from_map_path(&name));
            let lists = |set: &EdgePointSet| {
                set.sides()
                    .map(|list| (list.middle_index, raw(list)))
                    .to_vec()
            };
            let saved_lists = |set: &crate::file_formats::ja2_map::Ja2EdgePointSet| {
                [&set.north, &set.east, &set.south, &set.west]
                    .map(|list| {
                        let grid_nos = list.grid_nos.iter().map(|&g| g as usize).collect();
                        (list.middle_index, grid_nos)
                    })
                    .to_vec()
            };
            let primary_ok = lists(&generated.primary) == saved_lists(&saved.primary);
            let secondary_ok = match (&generated.secondary, &saved.secondary) {
                (Some(generated), Some(saved)) => lists(generated) == saved_lists(saved),
                _ => true,
            };
            if primary_ok && secondary_ok {
                matched += 1;
            } else {
                let counts = |lists: Vec<(u16, Vec<usize>)>| {
                    lists
                        .iter()
                        .map(|(middle, list)| format!("{}/{}", middle, list.len()))
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                failures.push(format!(
                    "{name}: primary {} (saved {}), secondary {} (saved {})",
                    counts(lists(&generated.primary)),
                    counts(saved_lists(&saved.primary)),
                    counts(lists(&generated.secondary.clone().unwrap_or_default())),
                    counts(saved_lists(&saved.secondary.clone().unwrap_or_default())),
                ));
            }
        }
        eprintln!(
            "{} maps match, {} don't:\n{}",
            matched,
            failures.len(),
            failures.join("\n")
        );
        assert!(
            matched >= 4 * failures.len(),
            "too many maps don't match:\n{}",
            failures.join("\n")
        );
    }
}
