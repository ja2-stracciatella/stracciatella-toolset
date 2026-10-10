//! Updating the world, the movement costs, the lights and the draws after an edit gives the same
//! result as building them again

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use bevy::prelude::*;

use crate::editor::{
    building::{add_building_section, kill_building, remove_building_section, Materials, TileRect},
    building_mode::paste_single,
    caves::{add_cave_section, remove_cave_section},
    document::{EditContext, EditorDocument},
    layout::paste_building,
    placement::SelectionList,
    soldiers::{
        tests::{mode, SoldierFixture},
        SoldierTeam,
    },
};
use crate::{
    components::map::GridPosition,
    editor::light_events::changed_light_tiles,
    file_formats::ja2_tile_types::{FENCESTRUCT, SECONDOSTRUCT},
    item::ItemId,
    light::{LightNode, LightTemplate, LIGHT_NEW_RAY},
    map::{
        Direction, GridNo, ItemHeight, ItemStack, Level, LightId, LightTime, MapItem, MapLight,
        TacticalMap, Visibility, WorldItemFlags, WorldItemId,
    },
    random::Random,
    render::{
        draw_list::{
            Draw, DrawCache, DrawContext, DrawListOptions, PaletteKey, SpriteGeometry, SpriteKey,
            VisibleTeams,
        },
        items::add_items,
        lighting::{light_all, relight_area, LightingMode},
        movement_costs::MovementCosts,
        soldiers::{add_soldiers, sync_soldiers, SoldierFiles},
        world::{MapWorld, WorldTile},
        MapSoldiers,
    },
    soldier::class::SoldierClassId,
    tileset::{
        tests::{
            building_tileset, structure_tileset, FIRSTFULLSTRUCT, FIRSTOSTRUCT, FIRSTTEXTURE, GUNS,
        },
        IndexedImages, StciSheet, SubImage, Tile, TileIndex, Tileset, TilesetId,
    },
};

const MODE: LightingMode = LightingMode::Day;
/// The edits happen in this part of the map, which is on the visible map
const AREA: std::ops::Range<u8> = 70..84;

fn sprites(_: SpriteKey) -> Option<SpriteGeometry> {
    Some(SpriteGeometry {
        offset: IVec2::new(-5, -30),
        size: UVec2::new(50, 50),
    })
}

fn draw_options() -> DrawListOptions {
    DrawListOptions {
        layers: Default::default(),
        floor: Level::Roof,
        shadows: true,
    }
}

fn tile(tileset: &Tileset, tile_type: usize, sub_index: u16) -> Arc<Tile> {
    tileset
        .tile(tileset.index(tile_type, sub_index).unwrap())
        .unwrap()
        .clone()
}

fn random_grid_no(random: &mut Random) -> GridNo {
    let len = u32::from(AREA.end - AREA.start);
    let x = AREA.start + random.next(len) as u8;
    let y = AREA.start + random.next(len) as u8;
    GridNo::from(GridPosition::new(x, y))
}

/// A light that reaches 3 tiles in all 8 directions
fn light_template() -> Arc<LightTemplate> {
    light_template_of(3)
}

/// A light that reaches `reach` tiles in all 8 directions
fn light_template_of(reach: i16) -> Arc<LightTemplate> {
    let mut lights = vec![LightNode {
        dx: 0,
        dy: 0,
        light: 4,
    }];
    let mut rays = vec![LIGHT_NEW_RAY, 0];
    for direction in Direction::ALL {
        let increment = direction.grid_no_increment();
        let (dx, dy) = match increment {
            -160 => (0, -1),
            160 => (0, 1),
            1 => (1, 0),
            -1 => (-1, 0),
            -159 => (1, -1),
            -161 => (-1, -1),
            159 => (-1, 1),
            _ => (1, 1),
        };
        rays.push(LIGHT_NEW_RAY);
        for step in 1..=reach {
            rays.push(lights.len() as u16);
            lights.push(LightNode {
                dx: dx * step,
                dy: dy * step,
                light: 4 - step as u8,
            });
        }
    }
    Arc::new(LightTemplate { lights, rays })
}

/// A light that is always on
fn light(id: u32, grid_no: GridNo, template: &Arc<LightTemplate>) -> MapLight {
    MapLight {
        id: LightId(id),
        position: grid_no.position(),
        time: LightTime::Always,
        template_name: "L-R03.LHT".to_owned(),
        template: template.clone(),
        file_sprite: None,
    }
}

fn item(tileset: &Tileset, id: u32, grid_no: GridNo, height: ItemHeight) -> MapItem {
    MapItem {
        id: WorldItemId(id),
        grid_no,
        file_grid_no: None,
        level: Level::Ground,
        stack: ItemStack {
            item: ItemId::new(1).unwrap(),
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
        graphic: Some(tile(tileset, GUNS, 1)),
        render_height: height,
        visibility: Visibility::Visible,
        flags: WorldItemFlags::empty(),
        non_exist_chance: 0,
    }
}

/// Everything that is built from the map
struct Built {
    world: MapWorld,
    costs: MovementCosts,
    draws: DrawCache,
    soldiers: MapSoldiers,
}

impl Built {
    fn new(map: &TacticalMap, tileset: &Tileset) -> Self {
        Self::with_soldiers(map, tileset, None)
    }

    /// Builds everything, with the soldiers of the shown teams
    fn with_soldiers(
        map: &TacticalMap,
        tileset: &Tileset,
        soldier_files: Option<(&SoldierFiles, &VisibleTeams)>,
    ) -> Self {
        let mut world = MapWorld::build(map);
        add_items(&mut world, map);
        let costs = MovementCosts::compile(&mut world, map);
        let mut soldiers = MapSoldiers::default();
        if let Some((files, visible)) = soldier_files {
            add_soldiers(
                &mut world,
                &map.soldiers,
                |p| visible.shows(p.team.id),
                files,
                &mut soldiers,
            );
        }
        light_all(&mut world, map, None, &costs, MODE);
        let draws = DrawCache::build(&DrawContext {
            world: &world,
            tileset,
            sprites,
            options: draw_options(),
        });
        Self {
            world,
            costs,
            draws,
            soldiers,
        }
    }

    /// Updates everything after the given tiles of the map changed, like `EditContext::commit`
    /// and `SceneBuilder::update`
    fn update(&mut self, map: &TacticalMap, tileset: &Tileset, changed: &BTreeSet<GridNo>) {
        let touched = self.world.sync_tiles(map, changed);
        self.refresh(map, tileset, &touched);
    }

    /// Updates the items after the world items changed, like `sync_item_scene`
    fn update_items(&mut self, map: &TacticalMap, tileset: &Tileset) {
        let touched = self.world.sync_items(map);
        self.refresh(map, tileset, &touched);
    }

    /// Updates the soldiers after the placements changed, like `sync_soldier_scene`
    fn update_soldiers(
        &mut self,
        map: &TacticalMap,
        tileset: &Tileset,
        files: &SoldierFiles,
        visible: &VisibleTeams,
    ) {
        let touched = sync_soldiers(
            &mut self.world,
            &map.soldiers,
            |p| visible.shows(p.team.id),
            files,
            &mut self.soldiers,
        );
        self.refresh(map, tileset, &touched);
    }

    /// Updates the scene after the nodes of the touched tiles changed, like `SceneBuilder::update`
    fn refresh(&mut self, map: &TacticalMap, tileset: &Tileset, touched: &BTreeSet<GridNo>) {
        let mut redraw = touched.clone();
        redraw.extend(self.costs.recompile_area(&mut self.world, map, touched));
        redraw.extend(relight_area(
            &mut self.world,
            map,
            None,
            &self.costs,
            MODE,
            &redraw.clone(),
        ));
        let context = DrawContext {
            world: &self.world,
            tileset,
            sprites,
            options: draw_options(),
        };
        for grid_no in redraw {
            self.draws.replace(grid_no, context.tile_draws(grid_no));
        }
    }
}

/// Everything about a tile that is not an id of the world, the soldiers with their palettes
fn tile_state(tile: &WorldTile, soldiers: &MapSoldiers) -> String {
    let nodes = |nodes: &[crate::render::world::LevelNode]| {
        nodes
            .iter()
            .map(|node| {
                format!(
                    "{}/{:?}/{:?}/{}/{:?}/{}",
                    node.tile.id,
                    node.flags,
                    node.light,
                    node.item_height,
                    node.item,
                    node.structure_id.is_some()
                )
            })
            .collect::<Vec<_>>()
    };
    let structures: Vec<_> = tile
        .structures
        .iter()
        .map(|s| {
            format!(
                "{:?}/{:?}/{}/{:?}/{:?}/{}/{}/{}",
                s.flags,
                s.wall_orientation,
                s.cube_offset,
                s.grid_no,
                s.base_grid_no,
                s.tile.id,
                s.structure_tile_index,
                s.structure_height
            )
        })
        .collect();
    let mercs: Vec<_> = tile
        .mercs
        .iter()
        .map(|node| {
            let soldier = &node.soldier;
            format!(
                "{:?}/{:?}/{:?}/{}/{}/{}/{:?}/{:?}",
                soldier.grid_no,
                soldier.surface.id,
                soldier.level,
                soldier.frame,
                soldier.hostile,
                soldier.vehicle,
                node.light,
                soldiers.palettes[usize::from(soldier.id)]
            )
        })
        .collect();
    format!(
        "height {} land {:?} start {:?} objects {:?} structs {:?} shadows {:?} roofs {:?} \
         on roofs {:?} topmost {:?} no burn {} structures {:?} mercs {:?}",
        tile.height,
        nodes(&tile.land),
        tile.land_start,
        nodes(&tile.objects),
        nodes(&tile.structs),
        nodes(&tile.shadows),
        nodes(&tile.roofs),
        nodes(&tile.on_roofs),
        nodes(&tile.topmost),
        tile.no_burn_struct,
        structures,
        mercs
    )
}

/// The draws with the palettes of the soldiers numbered by their colours, so that draws of the
/// same palettes compare equal whatever their index in `MapSoldiers` is
fn canonical_draws(built: &Built) -> Vec<Draw> {
    // The palettes of the soldiers of the world, palettes of soldiers that are gone stay in
    // `MapSoldiers`
    let ids: BTreeSet<u16> = built
        .world
        .tiles
        .values()
        .flat_map(|tile| tile.mercs.iter().map(|node| node.soldier.id))
        .collect();
    let palette = |id: u16| built.soldiers.palettes[usize::from(id)];
    let mut palettes: Vec<_> = ids.iter().map(|&id| palette(id)).collect();
    palettes.sort();
    palettes.dedup();
    let canonical: HashMap<u16, u16> = ids
        .iter()
        .map(|&id| {
            let index = palettes.binary_search(&palette(id)).unwrap();
            (id, index as u16)
        })
        .collect();
    built
        .draws
        .all()
        .into_iter()
        .map(|mut draw| {
            draw.palette = match draw.palette {
                PaletteKey::Soldier(id) => PaletteKey::Soldier(canonical[&id]),
                PaletteKey::SoldierUnbiased(id) => PaletteKey::SoldierUnbiased(canonical[&id]),
                other => other,
            };
            draw
        })
        .collect()
}

fn assert_same(step: usize, incremental: &Built, full: &Built) {
    for grid_no in GridNo::all() {
        assert_eq!(
            tile_state(&incremental.world.tiles[grid_no], &incremental.soldiers),
            tile_state(&full.world.tiles[grid_no], &full.soldiers),
            "step {step}, tile {grid_no:?}"
        );
        for direction in Direction::ALL {
            assert_eq!(
                incremental.costs.cost(grid_no, direction),
                full.costs.cost(grid_no, direction),
                "step {step}, cost of {grid_no:?} {direction:?}"
            );
        }
        assert_eq!(
            incremental.costs.terrain(grid_no),
            full.costs.terrain(grid_no),
            "step {step}, terrain of {grid_no:?}"
        );
    }
    assert!(
        canonical_draws(incremental) == canonical_draws(full),
        "step {step}: the draws differ"
    );
}

/// Structs that the edits add
fn struct_tiles(tileset: &Tileset) -> Vec<Arc<Tile>> {
    let mut tiles: Vec<_> = (1..=4).map(|i| tile(tileset, FIRSTOSTRUCT, i)).collect();
    tiles.extend((1..=3).map(|i| tile(tileset, FIRSTFULLSTRUCT, i)));
    tiles.push(tile(tileset, SECONDOSTRUCT, 1));
    tiles.push(tile(tileset, FENCESTRUCT, 1));
    // Without structure data
    tiles.push(tile(tileset, FIRSTOSTRUCT, 7));
    tiles
}

/// A random edit of the map, returns the changed tiles
fn edit(
    map: &mut TacticalMap,
    world: &MapWorld,
    tileset: &Tileset,
    random: &mut Random,
) -> BTreeSet<GridNo> {
    let structs = struct_tiles(tileset);
    let mut changed = BTreeSet::new();
    for _ in 0..=random.next(4) {
        let grid_no = random_grid_no(random);
        let map_tile = &mut map.tiles[grid_no];
        match random.next(7) {
            0 => {
                let sub_index = 1 + random.next(35) as TileIndex;
                map_tile.land.push(tile(
                    tileset,
                    FIRSTTEXTURE + random.next(3) as usize,
                    sub_index,
                ));
            }
            1 => {
                if map_tile.land.len() > 1 {
                    map_tile.land.pop();
                }
            }
            2 => {
                let tile = structs[random.next(structs.len() as u32) as usize].clone();
                // The editor never adds structs that overlap others
                let fits = tile.structure.as_ref().is_none_or(|structure| {
                    world.okay_to_add_structure_to_world(grid_no, Level::Ground, structure)
                        && !changed.iter().any(|&other: &GridNo| {
                            let (a, b) = (grid_no.position(), other.position());
                            a.x.abs_diff(b.x) < 3 && a.y.abs_diff(b.y) < 3
                        })
                });
                if fits {
                    map_tile.structs.insert(0, tile);
                }
            }
            3 => {
                if !map_tile.structs.is_empty() {
                    let index = random.next(map_tile.structs.len() as u32) as usize;
                    map_tile.structs.remove(index);
                }
            }
            4 => map_tile.objects.push(tile(tileset, FIRSTOSTRUCT, 9)),
            5 => map_tile.objects.clear(),
            _ => map_tile.shadows.push(tile(
                tileset,
                crate::tileset::tests::FIRSTSHADOW,
                1 + random.next(3) as TileIndex,
            )),
        }
        changed.insert(grid_no);
    }
    changed
}

#[test]
fn incremental_updates_equal_full_builds() {
    for seed in 0..8 {
        incremental_updates_equal_full_builds_with(seed);
    }
}

fn incremental_updates_equal_full_builds_with(seed: u64) {
    let tileset = structure_tileset();
    let mut random = Random::new(seed);
    let mut map = TacticalMap::empty(TilesetId::new(0));
    for y in AREA.start - 5..AREA.end + 5 {
        for x in AREA.start - 5..AREA.end + 5 {
            let grid_no = GridNo::from(GridPosition::new(x, y));
            map.tiles[grid_no].land = vec![tile(&tileset, FIRSTTEXTURE, 1)];
        }
    }
    for _ in 0..12 {
        let grid_no = random_grid_no(&mut random);
        let height = if random.next(2) == 0 {
            ItemHeight::ForceGround
        } else {
            ItemHeight::Above(3)
        };
        map.items
            .push(item(&tileset, map.items.len() as u32, grid_no, height));
    }
    let template = light_template();
    for _ in 0..4 {
        let id = map.lights.len() as u32;
        map.lights
            .push(light(id, random_grid_no(&mut random), &template));
    }

    let mut incremental = Built::new(&map, &tileset);
    for step in 0..40 {
        let changed = edit(&mut map, &incremental.world, &tileset, &mut random);
        incremental.update(&map, &tileset, &changed);
        let full = Built::new(&map, &tileset);
        assert_same(step, &incremental, &full);
    }
    // The edits placed and removed structures, items moved onto them
    assert!(incremental
        .world
        .tiles
        .values()
        .any(|tile| !tile.structures.is_empty()));
}

#[test]
fn item_edits_equal_full_builds() {
    for seed in 0..6 {
        item_edits_equal_full_builds_with(seed);
    }
}

fn item_edits_equal_full_builds_with(seed: u64) {
    let tileset = structure_tileset();
    let mut random = Random::new(seed);
    let mut map = TacticalMap::empty(TilesetId::new(0));
    for y in AREA.start - 5..AREA.end + 5 {
        for x in AREA.start - 5..AREA.end + 5 {
            let grid_no = GridNo::from(GridPosition::new(x, y));
            map.tiles[grid_no].land = vec![tile(&tileset, FIRSTTEXTURE, 1)];
        }
    }
    let mut next_id = 0;
    let mut new_item = |random: &mut Random, map: &TacticalMap| {
        next_id += 1;
        let height = if random.next(2) == 0 {
            ItemHeight::ForceGround
        } else {
            ItemHeight::Above(3)
        };
        // Items pile up on few tiles
        let grid_no = if random.next(2) == 0 && !map.items.is_empty() {
            map.items[random.next(map.items.len() as u32) as usize].grid_no
        } else {
            random_grid_no(random)
        };
        item(&tileset, next_id, grid_no, height)
    };
    for _ in 0..8 {
        let added = new_item(&mut random, &map);
        map.items.push(added);
    }
    let mut incremental = Built::new(&map, &tileset);
    for step in 0..40 {
        match random.next(5) {
            0 | 1 => {
                let added = new_item(&mut random, &map);
                map.items.push(added);
                incremental.update_items(&map, &tileset);
            }
            2 => {
                if !map.items.is_empty() {
                    let index = random.next(map.items.len() as u32) as usize;
                    map.items.remove(index);
                }
                incremental.update_items(&map, &tileset);
            }
            3 => {
                if !map.items.is_empty() {
                    let index = random.next(map.items.len() as u32) as usize;
                    map.items[index].grid_no = random_grid_no(&mut random);
                }
                incremental.update_items(&map, &tileset);
            }
            _ => {
                // Structures that items move onto
                let changed = edit(&mut map, &incremental.world, &tileset, &mut random);
                incremental.update(&map, &tileset, &changed);
            }
        }
        let full = Built::new(&map, &tileset);
        assert_same(step, &incremental, &full);
    }
}

#[test]
fn light_edits_equal_full_builds() {
    for seed in 0..6 {
        light_edits_equal_full_builds_with(seed);
    }
}

fn light_edits_equal_full_builds_with(seed: u64) {
    let tileset = structure_tileset();
    let mut random = Random::new(seed);
    let mut map = TacticalMap::empty(TilesetId::new(0));
    for y in AREA.start - 5..AREA.end + 5 {
        for x in AREA.start - 5..AREA.end + 5 {
            let grid_no = GridNo::from(GridPosition::new(x, y));
            map.tiles[grid_no].land = vec![tile(&tileset, FIRSTTEXTURE, 1)];
        }
    }
    let templates = [
        ("L-R02.LHT", light_template_of(2)),
        ("L-R03.LHT", light_template()),
    ];
    let mut next_id = 0;
    let mut new_light = |random: &mut Random| {
        next_id += 1;
        let (name, template) = &templates[random.next(2) as usize];
        MapLight {
            template_name: (*name).to_owned(),
            ..light(next_id, random_grid_no(random), template)
        }
    };
    for _ in 0..3 {
        let added = new_light(&mut random);
        map.lights.push(added);
    }
    // Structures that block the lights
    let mut incremental = Built::new(&map, &tileset);
    for _ in 0..4 {
        let changed = edit(&mut map, &incremental.world, &tileset, &mut random);
        incremental.update(&map, &tileset, &changed);
    }
    for step in 0..40 {
        let old = map.lights.clone();
        match random.next(4) {
            0 => {
                let added = new_light(&mut random);
                map.lights.push(added);
            }
            1 => {
                if !map.lights.is_empty() {
                    let index = random.next(map.lights.len() as u32) as usize;
                    map.lights.remove(index);
                }
            }
            2 => {
                // Another template, or a time that is off at day
                if !map.lights.is_empty() {
                    let index = random.next(map.lights.len() as u32) as usize;
                    let changed = &mut map.lights[index];
                    if random.next(2) == 0 {
                        let (name, template) = &templates[random.next(2) as usize];
                        changed.template_name = (*name).to_owned();
                        changed.template = template.clone();
                    } else {
                        changed.time = match changed.time {
                            LightTime::Always => LightTime::Night,
                            _ => LightTime::Always,
                        };
                    }
                }
            }
            _ => {
                let changed = edit(&mut map, &incremental.world, &tileset, &mut random);
                incremental.update(&map, &tileset, &changed);
            }
        }
        // Like `sync_light_scene`
        let tiles = changed_light_tiles(&old, &map.lights);
        incremental.refresh(&map, &tileset, &tiles);
        let full = Built::new(&map, &tileset);
        assert_same(step, &incremental, &full);
    }
}

#[test]
fn tree_shadows_follow_the_structures_around_them() {
    let tileset = structure_tileset();
    let mut map = TacticalMap::empty(TilesetId::new(0));
    let tree = GridNo::from(GridPosition::new(80, 80));
    for y in 75..85 {
        for x in 75..85 {
            let grid_no = GridNo::from(GridPosition::new(x, y));
            map.tiles[grid_no].land = vec![tile(&tileset, FIRSTTEXTURE, 1)];
        }
    }
    map.tiles[tree].structs = vec![tile(&tileset, FIRSTFULLSTRUCT, 1)];
    let neighbours = [
        Direction::West,
        Direction::SouthWest,
        Direction::South,
        Direction::SouthEast,
        Direction::East,
    ]
    .map(|direction| tree.step(direction));
    for &grid_no in &neighbours {
        map.tiles[grid_no].structs = vec![tile(&tileset, FIRSTOSTRUCT, 1)];
    }
    let mut incremental = Built::new(&map, &tileset);
    // Surrounded, so the game removes the shadow
    assert!(incremental.world.tiles[tree].shadows.is_empty());

    for step in 0..4 {
        let neighbour = neighbours[step / 2];
        if step % 2 == 0 {
            map.tiles[neighbour].structs.clear();
        } else {
            map.tiles[neighbour].structs = vec![tile(&tileset, FIRSTOSTRUCT, 1)];
        }
        incremental.update(&map, &tileset, &BTreeSet::from([neighbour]));
        assert_eq!(
            incremental.world.tiles[tree].shadows.is_empty(),
            step % 2 == 1,
            "step {step}"
        );
        assert_same(step, &incremental, &Built::new(&map, &tileset));
    }
}

/// A random rectangle in the edit area, up to 5 tiles wide and high
fn random_rect(random: &mut Random) -> TileRect {
    let start = random_grid_no(random).position();
    let (left, top) = (i32::from(start.x), i32::from(start.y));
    let end = |start: i32, random: &mut Random| {
        (start + random.next(5) as i32).min(i32::from(AREA.end) - 1)
    };
    TileRect {
        left,
        top,
        right: end(left, random),
        bottom: end(top, random),
    }
}

/// A random building edit of the editor
fn building_edit(context: &mut EditContext, random: &mut Random) {
    let materials = |random: &mut Random| {
        let tile_id = |name: &str| context_tile_id(context, name);
        let list = SelectionList::new(
            context.tileset,
            [
                (tile_id(["WALL1", "WALL2"][random.next(2) as usize]), 1),
                (tile_id("FLOOR1"), 1),
                // Slant roofs without a flat roof
                (
                    tile_id(["ROOF1", "ROOF3", "SROOF1"][random.next(3) as usize]),
                    1,
                ),
            ],
        );
        Materials::new(list)
    };
    let rect = random_rect(random);
    let first = GridNo::from(GridPosition::new(rect.left as u8, rect.top as u8));
    match random.next(9) {
        0..=2 => add_building_section(context, rect, &materials(random)),
        3 => remove_building_section(context, rect, &materials(random)),
        4 => kill_building(context, first),
        5 => add_cave_section(context, rect),
        6 => remove_cave_section(context, rect),
        7 => {
            let target = random_grid_no(random);
            paste_building(context, first, target, random.next(2) == 0);
        }
        _ => {
            let (name, sub_index) = [("DOOR1", 1), ("DOOR2", 16), ("WALL1", 40), ("WALL1", 30)]
                [random.next(4) as usize];
            let tile = context.tile(name, sub_index).unwrap();
            paste_single(context, first, tile);
        }
    }
}

fn context_tile_id(context: &EditContext, name: &str) -> TileIndex {
    context.tileset.tile_of(name, 1).unwrap()
}

#[test]
fn building_edits_equal_full_builds() {
    for seed in 0..4 {
        building_edits_equal_full_builds_with(seed);
    }
}

fn building_edits_equal_full_builds_with(seed: u64) {
    let tileset = building_tileset();
    let mut random = Random::new(seed);
    let mut map = TacticalMap::empty(TilesetId::new(0));
    for y in AREA.start - 5..AREA.end + 5 {
        for x in AREA.start - 5..AREA.end + 5 {
            let grid_no = GridNo::from(GridPosition::new(x, y));
            map.tiles[grid_no].land = vec![tile(&tileset, FIRSTTEXTURE, 1)];
        }
    }
    let mut edit_world = MapWorld::build(&map);
    let mut document = EditorDocument::new(seed);
    let mut incremental = Built::new(&map, &tileset);
    for step in 0..16 {
        document.undo.begin();
        let mut context = EditContext::new(&mut map, &mut edit_world, &tileset, &mut document);
        building_edit(&mut context, &mut random);
        context.finish();
        // The tiles the edit changed are the ones it recorded for undo
        let changed: BTreeSet<GridNo> = document
            .undo
            .pop()
            .map(|command| command.tiles.keys().copied().collect())
            .unwrap_or_default();
        incremental.update(&map, &tileset, &changed);
        let full = Built::new(&map, &tileset);
        assert_same(step, &incremental, &full);
    }
    // The edits built something
    assert!(map.tiles.values().any(|tile| !tile.structs.is_empty()));
}

/// A sheet whose 8 directions have one 2×2 frame each
fn soldier_sheet() -> StciSheet {
    let sub_image = SubImage {
        offset: (-1, -2),
        dimensions: (2, 2),
    };
    StciSheet {
        sub_images: vec![sub_image; 8],
        frames_per_direction: 1,
        indexed: Some(IndexedImages {
            palette: Box::new([[7; 3]; 256]),
            pixels: vec![vec![10, 12, 14, 16]; 8],
        }),
        database_info: vec![],
    }
}

#[test]
fn soldier_edits_equal_full_builds() {
    for seed in 0..4 {
        soldier_edits_equal_full_builds_with(seed);
    }
}

fn soldier_edits_equal_full_builds_with(seed: u64) {
    let mut fixture = SoldierFixture::new();
    let mut random = Random::new(seed);
    let template = light_template();
    for _ in 0..3 {
        let id = fixture.fixture.map.lights.len() as u32;
        let grid_no = random_grid_no(&mut random);
        fixture
            .fixture
            .map
            .lights
            .push(light(id, grid_no, &template));
    }
    let sheet = soldier_sheet();
    let palettes = fixture.palettes.clone();
    let files = SoldierFiles {
        surfaces: fixture
            .registries
            .surfaces
            .iter()
            .map(|_| Some(&sheet))
            .collect(),
        replacements: Some(&palettes),
        monster_palettes: vec![],
    };
    let mut visible = VisibleTeams::default();
    let tileset = fixture.fixture.tileset.clone();
    let mut incremental =
        Built::with_soldiers(&fixture.fixture.map, &tileset, Some((&files, &visible)));
    let mut placements = 0;
    for step in 0..30 {
        let soldiers: Vec<_> = fixture.map().soldiers.iter().map(|p| p.id).collect();
        let some_soldier = |random: &mut Random| {
            (!soldiers.is_empty()).then(|| soldiers[random.next(soldiers.len() as u32) as usize])
        };
        match random.next(6) {
            0..=1 => {
                let team = SoldierTeam::ALL[random.next(4) as usize];
                let mode = mode(&format!(
                    r#"{{"mode":"soldiers","team":"{}"}}"#,
                    serde_json::to_value(team).unwrap().as_str().unwrap()
                ));
                let grid_no = random_grid_no(&mut random);
                let placed = fixture
                    .edit(|context, sc| context.place_soldier(sc, &mode, grid_no, Level::Ground));
                placements += usize::from(placed.is_ok());
            }
            2 => {
                if let Some(id) = some_soldier(&mut random) {
                    let grid_no = random_grid_no(&mut random);
                    let _ = fixture.edit(|context, _| context.move_soldier(id, grid_no));
                }
            }
            3 => {
                if let Some(id) = some_soldier(&mut random) {
                    fixture.edit(|context, _| context.delete_soldier(id));
                }
            }
            4 => {
                if let Some(id) = some_soldier(&mut random) {
                    let direction = Direction::ALL[random.next(8) as usize];
                    let class =
                        [SoldierClassId::ARMY, SoldierClassId::ELITE][random.next(2) as usize];
                    fixture.edit(|context, sc| {
                        context.change_soldier(id, |p| {
                            p.direction = direction;
                            p.soldier_class = sc.definitions.classes.get(class).clone();
                            p.resolve_appearance(&sc.appearance);
                        })
                    });
                }
            }
            _ => {
                let team = SoldierTeam::ALL[random.next(4) as usize];
                match team {
                    SoldierTeam::Enemy => visible.enemy = !visible.enemy,
                    SoldierTeam::Creature => visible.creature = !visible.creature,
                    SoldierTeam::Rebel => visible.rebel = !visible.rebel,
                    SoldierTeam::Civilian => visible.civilian = !visible.civilian,
                }
            }
        }
        incremental.update_soldiers(fixture.map(), &tileset, &files, &visible);
        let full = Built::with_soldiers(fixture.map(), &tileset, Some((&files, &visible)));
        assert_same(step, &incremental, &full);
    }
    // The edits placed soldiers with different palettes
    assert!(placements > 3);
    assert!(incremental.soldiers.palettes.len() > 1);
}
