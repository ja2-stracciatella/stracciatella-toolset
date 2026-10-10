//! Puts the edits of the runtime map back into the map file, the inverse of the parts of
//! `convert.rs` that the editor changes
//!
//! The map file keeps everything else, e.g. the tiles and items that could not be converted, so
//! it is patched instead of written from scratch.

use std::{collections::HashMap, sync::Arc};

use super::{
    convert::{convert_ambient_light, BODY_RANDOM, NOWHERE, NO_SCHEDULE_DATA},
    DetailedPlacement, Door, EdgePointList, EdgePointSet, ExitGrid, GridNo, ItemStack, LightTime,
    MapItem, MapLight, MapTile, Schedule, ScheduleAction, ScheduleFlags, ScheduleId,
    SoldierPlacement, TacticalMap, LIGHT_NIGHTTIME, LIGHT_PRIMETIME, LIGHT_SPR_ACTIVE,
    LIGHT_SPR_ON, MAX_PATROL_POINTS,
};
use crate::{
    components::map::JA2_MAP_TILES,
    file_formats::{
        ja2_map::{
            Ja2AmbientLight, Ja2BasicSoldierPlacement, Ja2DetailedSoldierPlacement, Ja2Door,
            Ja2EdgePointList, Ja2EdgePointSet, Ja2EdgePoints, Ja2ExitGrid, Ja2LightSprite,
            Ja2Lights, Ja2Map, Ja2MapElement, Ja2MapElementFlags, Ja2MapFlags, Ja2Object,
            Ja2Schedule, Ja2SoldierPlacement, Ja2TileLayer, Ja2WorldItem,
        },
        ja2_tile_types::TILE_TYPE_NAMES,
    },
    tileset::Tile,
};

/// Most soldiers of a map (`MAX_NUM_SOLDIERS`)
pub const MAX_NUM_SOLDIERS: usize = 148;
/// Most schedules that are saved (`SaveSchedules`)
pub const MAX_SCHEDULES: usize = 32;
/// Profile of soldiers without a profile (`NO_PROFILE`)
const NO_PROFILE: u8 = 200;
/// Flag of items that are not dropped when the soldier dies (`OBJECT_UNDROPPABLE`)
pub const OBJECT_UNDROPPABLE: i8 = 0x01;

impl TacticalMap {
    /// Overwrites the tiles, the doors, the editor smoothing, the scroll restriction (other ids
    /// than 0 and 1 are kept as long as it stays restricted), the entry points, the exit grids,
    /// the world items, the lights, the soldiers and the schedules of the map file with the ones
    /// of this map, and the edge points if they were generated again
    pub fn apply_to_ja2(&self, raw: &mut Ja2Map) -> Result<(), String> {
        if raw.map_elements.len() != JA2_MAP_TILES {
            return Err(format!(
                "The map has {} tiles instead of {}",
                raw.map_elements.len(),
                JA2_MAP_TILES
            ));
        }
        for ((grid_no, tile), element) in self.tiles.iter().zip(&mut raw.map_elements) {
            apply_tile(tile, element)
                .map_err(|error| format!("Tile {}: {}", grid_no.index(), error))?;
        }
        raw.doors = self.doors.iter().map(convert_door).collect();
        if !raw.doors.is_empty() {
            raw.flags |= Ja2MapFlags::MAP_DOORTABLE_SAVED;
        }
        raw.info.editor_smoothing_type = self.info.editor_smoothing.number();
        if self.info.restricted_scroll != (raw.info.restricted_scroll_id != 0) {
            raw.info.restricted_scroll_id = u8::from(self.info.restricted_scroll);
        }
        self.apply_entry_points(raw);
        self.apply_exit_grids(raw);
        self.apply_edge_points(raw);
        self.apply_items(raw);
        self.apply_lights(raw);
        self.apply_soldiers(raw)
    }

    /// Writes the light colour and the light sprites, the converted ones first
    /// (`SaveMapLights`), and the ambient light if it changed
    ///
    /// Extra light colours of the map file are kept. The ambient light is compared with the one
    /// the map file has, so the level of old caves maps is only written once it was changed.
    /// Maps above ground have none, like in the original.
    pub(super) fn apply_lights(&self, raw: &mut Ja2Map) {
        let mut colors = raw
            .lights
            .as_ref()
            .map(|lights| lights.colors.clone())
            .unwrap_or_default();
        let [r, g, b] = self.light_color;
        match colors.first_mut() {
            Some(first) => *first = [r, g, b, first[3]],
            None => colors.push([r, g, b, 0]),
        }
        raw.lights = Some(Ja2Lights {
            colors,
            sprites: self
                .lights
                .iter()
                .map(convert_light)
                .chain(self.skipped_lights.iter().cloned())
                .collect(),
        });
        raw.flags |= Ja2MapFlags::MAP_WORLDLIGHTS_SAVED;

        match self.ambient_light {
            Some(ambient) => {
                if convert_ambient_light(raw) != Some(ambient) {
                    raw.ambient_light = Some(Ja2AmbientLight {
                        basement: ambient.basement,
                        caves: ambient.caves,
                        level: ambient.level,
                    });
                    raw.flags |= Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED;
                }
            }
            None => {
                raw.ambient_light = None;
                raw.flags.remove(Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED);
            }
        }
    }

    /// Writes the entry points, `NOWHERE` for the ones that are not set
    pub(super) fn apply_entry_points(&self, raw: &mut Ja2Map) {
        let raw_grid_no = |grid_no: Option<GridNo>| grid_no.map_or(NOWHERE, |g| g.index() as i16);
        let info = &mut raw.info;
        info.north_grid_no = raw_grid_no(self.info.north_entry);
        info.east_grid_no = raw_grid_no(self.info.east_entry);
        info.south_grid_no = raw_grid_no(self.info.south_entry);
        info.west_grid_no = raw_grid_no(self.info.west_entry);
        info.center_grid_no = raw_grid_no(self.info.center);
        info.isolated_grid_no = raw_grid_no(self.info.isolated);
    }

    /// Writes the exit grids and the ones that could not be converted, sorted by their grid
    /// numbers like the game keeps them (`SaveExitGrids`)
    ///
    /// The game always saves them, a map without them keeps not saving them. A skipped exit grid
    /// is dropped once its tile got a new one, the game would load the one that comes last.
    pub(super) fn apply_exit_grids(&self, raw: &mut Ja2Map) {
        let converted: Vec<Ja2ExitGrid> = self.exit_grids.iter().map(convert_exit_grid).collect();
        let skipped = self
            .skipped_exit_grids
            .iter()
            .filter(|skipped| converted.iter().all(|grid| grid.grid_no != skipped.grid_no))
            .cloned();
        let mut exit_grids: Vec<Ja2ExitGrid> = converted.iter().cloned().chain(skipped).collect();
        exit_grids.sort_by_key(|grid| grid.grid_no);
        if !exit_grids.is_empty() {
            raw.flags |= Ja2MapFlags::MAP_EXITGRIDS_SAVED;
        }
        raw.exit_grids = exit_grids;
    }

    /// Writes the edge points if they were generated again (`SaveMapEdgepoints`), maps before
    /// version 17 only have the primary ones
    pub(super) fn apply_edge_points(&self, raw: &mut Ja2Map) {
        if !self.edge_points_stale {
            return;
        }
        let secondary = (raw.info.map_version >= 17).then(|| {
            convert_edge_point_set(&self.edge_points.secondary.clone().unwrap_or_default())
        });
        raw.edge_points = Some(Ja2EdgePoints {
            primary: convert_edge_point_set(&self.edge_points.primary),
            secondary,
        });
        raw.flags |= Ja2MapFlags::MAP_EDGEPOINTS_SAVED;
    }

    /// Writes the world items, the converted ones first (`SaveWorldItemsToMap`)
    pub(super) fn apply_items(&self, raw: &mut Ja2Map) {
        raw.items = self
            .items
            .iter()
            .map(convert_world_item)
            .chain(self.skipped_items.iter().cloned())
            .collect();
        raw.flags |= Ja2MapFlags::MAP_WORLDITEMS_SAVED;
    }

    /// Writes the placements, the converted ones first, and the schedules they use
    /// (`SaveSoldiersToMap`, `OptimizeSchedules`, `SaveSchedules`)
    fn apply_soldiers(&self, raw: &mut Ja2Map) -> Result<(), String> {
        let count = self.soldiers.len() + self.skipped_placements.len();
        if count > MAX_NUM_SOLDIERS {
            return Err(format!(
                "The map has {} soldiers, at most {} can be saved",
                count, MAX_NUM_SOLDIERS
            ));
        }
        let schedules = optimize_schedules(&self.schedules, &self.soldiers);
        raw.placements = self
            .soldiers
            .iter()
            .map(|placement| {
                let schedule = placement
                    .detailed
                    .as_ref()
                    .and_then(|detailed| detailed.schedule)
                    .and_then(|id| schedules.ids.get(&id).copied());
                convert_placement(placement, schedule)
            })
            .chain(self.skipped_placements.iter().cloned())
            .collect();
        raw.info.num_individuals = count as u8;
        raw.flags |= Ja2MapFlags::MAP_FULLSOLDIER_SAVED;
        raw.schedules = schedules.saved;
        if !raw.schedules.is_empty() {
            raw.flags |= Ja2MapFlags::MAP_NPCSCHEDULES_SAVED;
        }
        Ok(())
    }
}

/// The schedules as they are saved, and their new ids by their old ones
struct OptimizedSchedules {
    saved: Vec<Ja2Schedule>,
    ids: HashMap<ScheduleId, u8>,
}

/// Leaves out the temporary schedules, numbers the others from 1 in their order and sorts their
/// actions by time (`OptimizeSchedules`, `SortSchedule`, `SaveSchedules`)
///
/// The schedules know the soldier that uses them by its index in the map file.
fn optimize_schedules(schedules: &[Schedule], soldiers: &[SoldierPlacement]) -> OptimizedSchedules {
    let mut users = HashMap::new();
    for (index, placement) in soldiers.iter().enumerate() {
        if let Some(id) = placement.detailed.as_ref().and_then(|d| d.schedule) {
            users.entry(id).or_insert(index as u8);
        }
    }
    let mut saved = vec![];
    let mut ids = HashMap::new();
    for schedule in schedules {
        if schedule.flags.contains(ScheduleFlags::TEMPORARY) || saved.len() == MAX_SCHEDULES {
            continue;
        }
        let id = saved.len() as u8 + 1;
        ids.insert(schedule.id, id);
        let soldier_id = users
            .get(&schedule.id)
            .copied()
            .unwrap_or(schedule.soldier_id);
        saved.push(convert_schedule(&sort_schedule(schedule), id, soldier_id));
    }
    OptimizedSchedules { saved, ids }
}

/// The flags of a slot of a schedule
fn slot_flags(slot: usize) -> ScheduleFlags {
    let variance = [
        ScheduleFlags::VARIANCE1,
        ScheduleFlags::VARIANCE2,
        ScheduleFlags::VARIANCE3,
        ScheduleFlags::VARIANCE4,
    ];
    let active = [
        ScheduleFlags::ACTIVE1,
        ScheduleFlags::ACTIVE2,
        ScheduleFlags::ACTIVE3,
        ScheduleFlags::ACTIVE4,
    ];
    variance[slot] | active[slot]
}

/// The actions in the order of their times, actions without a time and unused slots last
/// (`SortSchedule`)
///
/// Unlike the game, the variance of an action moves with it.
pub fn sort_schedule(schedule: &Schedule) -> Schedule {
    let mut slots: Vec<usize> = (0..schedule.actions.len()).collect();
    slots.sort_by_key(|&slot| {
        let entry = schedule.actions[slot];
        (
            entry.is_none(),
            entry.and_then(|e| e.time).unwrap_or(NO_SCHEDULE_DATA),
        )
    });
    let all_slots = (0..4).fold(ScheduleFlags::empty(), |all, slot| all | slot_flags(slot));
    let mut flags = schedule.flags - all_slots;
    for (to, &from) in slots.iter().enumerate() {
        if schedule.flags.intersects(slot_flags(from)) {
            let moved = (schedule.flags & slot_flags(from)).bits() >> from << to;
            flags |= ScheduleFlags::from_bits_retain(moved);
        }
    }
    Schedule {
        flags,
        actions: std::array::from_fn(|to| schedule.actions[slots[to]]),
        ..schedule.clone()
    }
}

fn convert_schedule(schedule: &Schedule, id: u8, soldier_id: u8) -> Ja2Schedule {
    let mut raw = Ja2Schedule {
        time: [NO_SCHEDULE_DATA; 4],
        data1: [NO_SCHEDULE_DATA; 4],
        data2: [NO_SCHEDULE_DATA; 4],
        action: [0; 4],
        schedule_id: id,
        soldier_id,
        flags: schedule.flags.bits(),
    };
    for (slot, entry) in schedule.actions.iter().enumerate() {
        let Some(entry) = entry else {
            continue;
        };
        let raw_grid = |grid_no: super::GridNo| grid_no.index() as u16;
        let (action, data1, data2) = match entry.action {
            ScheduleAction::LockDoor { door, destination } => (1, Some(door), Some(destination)),
            ScheduleAction::UnlockDoor { door, destination } => (2, Some(door), Some(destination)),
            ScheduleAction::OpenDoor { door, destination } => (3, Some(door), Some(destination)),
            ScheduleAction::CloseDoor { door, destination } => (4, Some(door), Some(destination)),
            ScheduleAction::GridNo(grid_no) => (5, Some(grid_no), None),
            ScheduleAction::LeaveSector => (6, None, None),
            ScheduleAction::EnterSector(grid_no) => (7, Some(grid_no), None),
            ScheduleAction::StayInSector => (8, None, None),
            ScheduleAction::Sleep(grid_no) => (9, Some(grid_no), None),
            ScheduleAction::Wake => (10, None, None),
        };
        raw.time[slot] = entry.time.unwrap_or(NO_SCHEDULE_DATA);
        raw.action[slot] = action;
        raw.data1[slot] = data1.map_or(NO_SCHEDULE_DATA, raw_grid);
        raw.data2[slot] = data2.map_or(NO_SCHEDULE_DATA, raw_grid);
    }
    raw
}

/// The placement of the map file, the inverse of `convert_placement`
fn convert_placement(placement: &SoldierPlacement, schedule: Option<u8>) -> Ja2SoldierPlacement {
    let team = placement.team.id.index() as i8;
    let body_type = placement
        .given_body
        .as_ref()
        .map_or(BODY_RANDOM, |body| body.id.index() as i8);
    let direction = placement.direction as u8 as i8;
    let on_roof = placement.level == super::Level::Roof;
    // The first entry is where the soldier stands
    let mut patrol_grid = [0; 10];
    patrol_grid[0] = placement.grid_no.index() as i16;
    let patrol_count = placement.patrol.len().min(MAX_PATROL_POINTS);
    for (to, grid_no) in patrol_grid[1..].iter_mut().zip(&placement.patrol) {
        *to = grid_no.index() as i16;
    }
    let basic = Ja2BasicSoldierPlacement {
        detailed_placement: placement.detailed.is_some(),
        starting_grid_no: placement.grid_no.index() as u16,
        team,
        relative_attribute_level: placement.relative_attribute_level,
        relative_equipment_level: placement.relative_equipment_level,
        direction,
        orders: placement.orders.number(),
        attitude: placement.attitude.number(),
        body_type,
        patrol_grid,
        patrol_cnt: patrol_count as i8,
        on_roof,
        soldier_class: placement.soldier_class.id.index() as u8,
        civilian_group: placement.civilian_group,
        priority_existance: placement.priority_existence,
        has_keys: placement.has_keys,
    };
    let detailed = placement
        .detailed
        .as_ref()
        .map(|detailed| convert_detailed(detailed, &basic, schedule));
    Ja2SoldierPlacement { basic, detailed }
}

/// The detailed placement, which repeats the values of the basic one
fn convert_detailed(
    detailed: &DetailedPlacement,
    basic: &Ja2BasicSoldierPlacement,
    schedule: Option<u8>,
) -> Ja2DetailedSoldierPlacement {
    // -1 means that the game chooses the value
    let stat = |value: Option<i8>| value.unwrap_or(-1);
    let stats = &detailed.stats;
    let sector = detailed.sector.unwrap_or_default();
    Ja2DetailedSoldierPlacement {
        is_static: detailed.is_static,
        profile: detailed
            .profile
            .map_or(NO_PROFILE, |profile| profile.number()),
        copy_profile_items_over: detailed.copy_profile_items_over,
        sector_x: sector.x.into(),
        sector_y: sector.y.into(),
        direction: basic.direction,
        insertion_grid_no: detailed
            .insertion_grid_no
            .map_or(NOWHERE, |grid_no| grid_no.index() as i16),
        team: basic.team,
        body_type: basic.body_type,
        attitude: basic.attitude,
        orders: basic.orders,
        life_max: stat(stats.life_max),
        life: stat(stats.life),
        agility: stat(stats.agility),
        dexterity: stat(stats.dexterity),
        exp_level: stat(stats.exp_level),
        marksmanship: stat(stats.marksmanship),
        medical: stat(stats.medical),
        mechanical: stat(stats.mechanical),
        explosive: stat(stats.explosive),
        leadership: stat(stats.leadership),
        strength: stat(stats.strength),
        wisdom: stat(stats.wisdom),
        morale: stat(stats.morale),
        ai_morale: stat(stats.ai_morale),
        inventory: detailed
            .inventory
            .iter()
            .map(|slot| slot.as_ref().map_or_else(empty_slot, convert_object))
            .collect(),
        head_pal: detailed.head_palette.clone(),
        pants_pal: detailed.pants_palette.clone(),
        vest_pal: detailed.vest_palette.clone(),
        skin_pal: detailed.skin_palette.clone(),
        patrol_grid: basic.patrol_grid,
        patrol_cnt: basic.patrol_cnt,
        visible: detailed.visible,
        name: detailed.name.clone(),
        soldier_class: basic.soldier_class,
        on_roof: basic.on_roof,
        sector_z: sector.z as i8,
        civilian_group: basic.civilian_group,
        schedule_id: schedule.unwrap_or(0),
        use_given_vehicle: detailed.vehicle_id.is_some(),
        use_given_vehicle_id: detailed.vehicle_id.unwrap_or(0),
        has_keys: basic.has_keys,
    }
}

/// An empty inventory slot, like the editor creates them
/// (`CreateStaticDetailedPlacementGivenBasicPlacementInfo`)
fn empty_slot() -> Ja2Object {
    Ja2Object {
        flags: OBJECT_UNDROPPABLE,
        ..Default::default()
    }
}

/// The item stack of the map file, the inverse of `convert_object`
pub(super) fn convert_object(stack: &ItemStack) -> Ja2Object {
    let mut attach_items = [0; 4];
    let mut attach_status = [0; 4];
    for (slot, &(item, status)) in stack.attachments.iter().take(4).enumerate() {
        attach_items[slot] = item.index();
        attach_status[slot] = status;
    }
    Ja2Object {
        item: stack.item.index(),
        number: stack.number,
        data: stack.data,
        attach_items,
        attach_status,
        flags: stack.flags,
        mission: stack.mission,
        trap: stack.trap,
        imprint_id: stack.imprint_id,
        weight: stack.weight,
        used: stack.used,
    }
}

/// The light sprite of the map file: the one it was loaded from while it is unchanged, else
/// the flags of that one with the new time, or those of a new light (`PlaceLight`)
fn convert_light(light: &MapLight) -> Ja2LightSprite {
    let x = i16::from(light.position.x);
    let y = i16::from(light.position.y);
    let time_flags = LIGHT_PRIMETIME | LIGHT_NIGHTTIME;
    let flags = match &light.file_sprite {
        Some(sprite)
            if sprite.x == x
                && sprite.y == y
                && sprite.template_name == light.template_name
                && LightTime::from_flags(sprite.flags) == light.time =>
        {
            return sprite.clone();
        }
        Some(sprite) => sprite.flags & !time_flags,
        None => LIGHT_SPR_ACTIVE | LIGHT_SPR_ON,
    };
    Ja2LightSprite {
        x,
        y,
        flags: flags | light.time.flags(),
        template_name: light.template_name.clone(),
    }
}

/// The world item of the map file, the inverse of `convert_items`
fn convert_world_item(item: &MapItem) -> Ja2WorldItem {
    Ja2WorldItem {
        exists: true,
        grid_no: item
            .file_grid_no
            .unwrap_or_else(|| item.grid_no.index() as i16),
        level: item.level.number(),
        object: match &item.file_object {
            Some(object)
                if super::convert::convert_object(object).as_ref() == Some(&item.stack) =>
            {
                object.clone()
            }
            _ => convert_object(&item.stack),
        },
        flags: item.flags.bits(),
        render_z_height_above_level: item.render_height.number(),
        visible: item.visibility.number(),
        non_exist_chance: item.non_exist_chance,
    }
}

fn apply_tile(tile: &MapTile, element: &mut Ja2MapElement) -> Result<(), String> {
    element.height = tile.height;
    element.land = convert_layer(&tile.land)?;
    element.objects = convert_layer(&tile.objects)?;
    element.structs = convert_layer(&tile.structs)?;
    element.shadows = convert_layer(&tile.shadows)?;
    element.roofs = convert_layer(&tile.roofs)?;
    element.on_roofs = convert_layer(&tile.on_roofs)?;
    element.room = tile.room.map_or(0, |room| room.number());
    element.flags.set(
        Ja2MapElementFlags::MAPELEMENT_ENEMY_MINE_PRESENT,
        tile.enemy_mine,
    );
    element.flags.set(
        Ja2MapElementFlags::MAPELEMENT_PLAYER_MINE_PRESENT,
        tile.player_mine,
    );
    Ok(())
}

fn convert_layer(tiles: &[Arc<Tile>]) -> Result<Vec<Ja2TileLayer>, String> {
    tiles
        .iter()
        .map(|tile| {
            let kind = TILE_TYPE_NAMES
                .iter()
                .position(|&name| name == tile.type_name)
                .ok_or_else(|| format!("unknown tile type {}", tile.type_name))?;
            let kind = u8::try_from(kind)
                .map_err(|_| format!("tile type {} does not fit the map", tile.type_name))?;
            Ok(Ja2TileLayer {
                kind,
                sub_index: tile.sub_index,
            })
        })
        .collect()
}

fn convert_exit_grid(grid: &ExitGrid) -> Ja2ExitGrid {
    let sector = grid.dest_sector;
    Ja2ExitGrid {
        grid_no: grid.grid_no.index() as u16,
        dest_grid_no: grid.dest_grid_no.index() as u16,
        dest_sector: (sector.x, sector.y, sector.z),
    }
}

fn convert_edge_point_list(list: &EdgePointList) -> Ja2EdgePointList {
    Ja2EdgePointList {
        middle_index: list.middle_index,
        grid_nos: list.grid_nos.iter().map(|g| g.index() as i16).collect(),
    }
}

fn convert_edge_point_set(set: &EdgePointSet) -> Ja2EdgePointSet {
    Ja2EdgePointSet {
        north: convert_edge_point_list(&set.north),
        east: convert_edge_point_list(&set.east),
        south: convert_edge_point_list(&set.south),
        west: convert_edge_point_list(&set.west),
    }
}

fn convert_door(door: &Door) -> Ja2Door {
    Ja2Door {
        grid_no: door.grid_no.index() as i16,
        locked: door.locked,
        trap_level: door.trap.map_or(0, |trap| trap.level),
        trap_id: door.trap.map_or(0, |trap| trap.kind.number()),
        lock_id: door.lock.map_or(0, |lock| lock.number()),
        perceived_locked: door.perceived_locked,
        perceived_trapped: door.perceived_trapped,
        lock_damage: door.lock_damage,
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::default;

    use super::*;
    use crate::{
        file_formats::ja2_tile_types::{FIRSTOSTRUCT, FIRSTTEXTURE},
        map::{
            convert::tests::{detailed, empty_map, grid, layer, placement, Fixture},
            DoorTrap, DoorTrapKind, EditorSmoothing, LockId, RoomNo, ScheduleEntry, HANDPOS,
        },
    };

    #[test]
    fn edits_survive_a_round_trip() {
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.map_elements[5].land = vec![layer(FIRSTTEXTURE, 1)];
        raw.map_elements[5].flags = Ja2MapElementFlags::MAPELEMENT_REDUNDANT;
        let mut map = fixture.convert(&raw);

        let tile = |tile_type, sub_index| {
            let index = fixture.tileset.index(tile_type, sub_index).unwrap();
            fixture.tileset.tile(index).unwrap().clone()
        };
        let edited = &mut map.tiles[grid(5)];
        edited.height = 7;
        edited.land.push(tile(FIRSTTEXTURE, 2));
        edited.structs = vec![tile(FIRSTOSTRUCT, 2), tile(FIRSTOSTRUCT, 1)];
        edited.room = RoomNo::new(4);
        edited.player_mine = true;
        map.info.editor_smoothing = EditorSmoothing::Basement;
        map.info.restricted_scroll = true;
        map.doors = vec![Door {
            grid_no: grid(5),
            locked: true,
            trap: Some(DoorTrap {
                kind: DoorTrapKind::Siren,
                level: 2,
            }),
            lock: LockId::new(3),
            perceived_locked: 1,
            perceived_trapped: 0,
            lock_damage: 0,
        }];

        map.apply_to_ja2(&mut raw).unwrap();
        let element = &raw.map_elements[5];
        assert_eq!(
            element.land,
            [layer(FIRSTTEXTURE, 1), layer(FIRSTTEXTURE, 2)]
        );
        // Other element flags are kept
        assert!(element
            .flags
            .contains(Ja2MapElementFlags::MAPELEMENT_REDUNDANT));
        assert!(raw.flags.contains(Ja2MapFlags::MAP_DOORTABLE_SAVED));
        assert_eq!(raw.info.editor_smoothing_type, 1);
        assert_eq!(raw.info.restricted_scroll_id, 1);
        assert_eq!(raw.doors[0].trap_id, 3);

        let reloaded = fixture.convert(&raw);
        let ids = |tiles: &[Arc<Tile>]| tiles.iter().map(|tile| tile.id).collect::<Vec<_>>();
        let (before, after) = (&map.tiles[grid(5)], &reloaded.tiles[grid(5)]);
        assert_eq!(after.height, 7);
        assert_eq!(ids(&after.land), ids(&before.land));
        assert_eq!(ids(&after.structs), ids(&before.structs));
        assert_eq!(after.room, before.room);
        assert!(after.player_mine && !after.enemy_mine);
        assert_eq!(reloaded.doors, map.doors);
        assert_eq!(reloaded.info, map.info);

        // The patched map can be written and read again
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        assert!(Ja2Map::parse(&bytes).unwrap() == raw);
    }

    #[test]
    fn rejects_tiles_that_the_map_file_cannot_hold() {
        let fixture = Fixture::new();
        let mut raw = empty_map();
        let mut map = fixture.convert(&raw);
        let mut unknown = (*fixture
            .tileset
            .tile(fixture.tileset.index(FIRSTTEXTURE, 1).unwrap())
            .unwrap()
            .clone())
        .clone();
        unknown.type_name = "NOT A TYPE".to_owned();
        map.tiles[grid(0)].land = vec![Arc::new(unknown)];
        assert!(map.apply_to_ja2(&mut raw).is_err());
        assert!(map.apply_to_ja2(&mut Ja2Map { ..default() }).is_err());
    }

    fn schedule(id: u8, flags: ScheduleFlags) -> Ja2Schedule {
        Ja2Schedule {
            time: [120, 60, NO_SCHEDULE_DATA, NO_SCHEDULE_DATA],
            data1: [300, NO_SCHEDULE_DATA, NO_SCHEDULE_DATA, NO_SCHEDULE_DATA],
            data2: [NO_SCHEDULE_DATA; 4],
            action: [5, 6, 0, 0],
            schedule_id: id,
            soldier_id: 99,
            flags: flags.bits(),
        }
    }

    #[test]
    fn soldiers_and_schedules_survive_a_round_trip() {
        let fixture = Fixture::new();
        let mut raw = empty_map();
        let mut enemy = placement(1, 3, BODY_RANDOM);
        enemy.basic.starting_grid_no = 100;
        enemy.basic.direction = 3;
        enemy.basic.orders = 4;
        enemy.basic.patrol_grid = [100, 200, 300, 0, 0, 0, 0, 0, 0, 0];
        enemy.basic.patrol_cnt = 2;
        let unknown_team = placement(9, 0, 0);
        let mut civilian = placement(4, 0, 12);
        civilian.basic.starting_grid_no = 500;
        civilian.basic.detailed_placement = true;
        let mut civilian_detailed = detailed();
        civilian_detailed.schedule_id = 7;
        civilian_detailed.life = 50;
        civilian_detailed.agility = -1;
        civilian_detailed.insertion_grid_no = 500;
        civilian_detailed.inventory[HANDPOS] = Ja2Object {
            item: 1,
            number: 1,
            data: [90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            attach_items: [2, 0, 0, 0],
            attach_status: [40, 0, 0, 0],
            flags: -128,
            ..Default::default()
        };
        civilian.detailed = Some(civilian_detailed);
        raw.placements = vec![enemy, unknown_team.clone(), civilian];
        raw.info.num_individuals = 3;
        raw.flags |= Ja2MapFlags::MAP_FULLSOLDIER_SAVED | Ja2MapFlags::MAP_NPCSCHEDULES_SAVED;
        raw.schedules = vec![
            schedule(3, ScheduleFlags::TEMPORARY),
            schedule(7, ScheduleFlags::VARIANCE1),
        ];
        let map = fixture.convert(&raw);
        assert_eq!(map.soldiers.len(), 2);
        assert_eq!(map.schedules.len(), 2);

        map.apply_to_ja2(&mut raw).unwrap();
        // The converted placements first, then the skipped ones
        assert_eq!(raw.info.num_individuals, 3);
        assert_eq!(raw.placements[2], unknown_team);
        let enemy = &raw.placements[0];
        // A random body stays random
        assert_eq!(enemy.basic.body_type, BODY_RANDOM);
        assert_eq!(enemy.basic.patrol_cnt, 2);
        assert_eq!(enemy.basic.patrol_grid[..3], [100, 200, 300]);
        assert!(enemy.detailed.is_none());
        let civilian = raw.placements[1].detailed.as_ref().unwrap();
        assert_eq!(civilian.life, 50);
        assert_eq!(civilian.agility, -1);
        assert_eq!(civilian.body_type, 12);
        assert_eq!(civilian.team, 4);
        assert_eq!(civilian.inventory[HANDPOS].item, 1);
        assert_eq!(civilian.inventory[HANDPOS].attach_status, [40, 0, 0, 0]);
        assert_eq!(civilian.inventory[0].item, 0);
        assert_eq!(civilian.inventory[0].flags, OBJECT_UNDROPPABLE);
        // The temporary schedule is left out, the other one is numbered again and sorted, its
        // variance moves with the action
        assert_eq!(civilian.schedule_id, 1);
        assert_eq!(raw.schedules.len(), 1);
        let saved = &raw.schedules[0];
        assert_eq!(saved.schedule_id, 1);
        assert_eq!(saved.soldier_id, 1);
        assert_eq!(saved.time, [60, 120, NO_SCHEDULE_DATA, NO_SCHEDULE_DATA]);
        assert_eq!(saved.action, [6, 5, 0, 0]);
        assert_eq!(saved.data1[1], 300);
        assert_eq!(saved.flags, ScheduleFlags::VARIANCE2.bits());

        let reloaded = fixture.convert(&raw);
        let (before, after) = (&map.soldiers[0], &reloaded.soldiers[0]);
        assert_eq!(after.grid_no, before.grid_no);
        assert_eq!(after.patrol, before.patrol);
        assert_eq!(after.body.id, before.body.id);
        assert_eq!(after.direction, before.direction);
        let (before, after) = (&map.soldiers[1], &reloaded.soldiers[1]);
        let mut expected = before.detailed.clone().unwrap();
        expected.schedule = ScheduleId::new(1);
        expected.inventory[0] = None;
        assert_eq!(after.detailed.as_ref(), Some(&expected));
        assert_eq!(reloaded.skipped_placements, [unknown_team]);
        assert_eq!(
            reloaded.schedules[0].actions[0],
            Some(ScheduleEntry {
                time: Some(60),
                action: ScheduleAction::LeaveSector
            })
        );

        // The patched map can be written and read again
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        assert!(Ja2Map::parse(&bytes).unwrap() == raw);
    }

    #[test]
    fn items_survive_a_round_trip() {
        use crate::{
            file_formats::ja2_map::Ja2WorldItem,
            map::{ItemHeight, Level, Visibility, WorldItemFlags},
        };
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.info.center_grid_no = 100;
        let world_item = |item, grid_no, visible| Ja2WorldItem {
            exists: true,
            grid_no,
            level: 0,
            object: Ja2Object {
                item,
                number: 1,
                data: [80, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                attach_items: [2, 0, 0, 0],
                attach_status: [60, 0, 0, 0],
                flags: 8,
                trap: 3,
                imprint_id: 200,
                ..Default::default()
            },
            flags: (WorldItemFlags::ARMED_BOMB | WorldItemFlags::SCIFI_ONLY).bits(),
            render_z_height_above_level: -1,
            visible,
            non_exist_chance: 30,
        };
        let unknown_visibility = world_item(1, 5, 9);
        // The attachment is in the third slot, the stack keeps it in the first
        let mut third_slot = world_item(1, 5, 1);
        third_slot.object.attach_items = [0, 0, 2, 0];
        third_slot.object.attach_status = [0, 0, 60, 0];
        raw.items = vec![
            third_slot,
            Ja2WorldItem {
                level: 1,
                render_z_height_above_level: 12,
                ..world_item(2, 7, -3)
            },
            // `NOWHERE` stays `NOWHERE`
            world_item(2, NOWHERE, -1),
            unknown_visibility.clone(),
        ];
        raw.flags |= Ja2MapFlags::MAP_WORLDITEMS_SAVED;
        let original = raw.items.clone();
        let map = fixture.convert(&raw);
        assert_eq!(map.items.len(), 3);
        assert_eq!(map.items[2].grid_no, grid(100));

        // An unchanged map saves the same items
        map.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(raw.items, original);

        // Edited items are saved with their changes, moved ones lose their file grid number
        let mut edited = map.clone();
        edited.items[0].visibility = Visibility::Buried;
        edited.items[0].render_height = ItemHeight::Above(4);
        edited.items[0].stack.number = 3;
        edited.items[1].level = Level::Ground;
        edited.items[2].grid_no = grid(300);
        edited.items[2].file_grid_no = None;
        edited.items.remove(1);
        edited.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(raw.items.len(), 3);
        assert_eq!(raw.items[0].visible, -3);
        assert_eq!(raw.items[0].render_z_height_above_level, 4);
        assert_eq!(raw.items[0].object.number, 3);
        // Changed stacks are saved as they are now
        assert_eq!(raw.items[0].object.attach_items, [2, 0, 0, 0]);
        assert_eq!(raw.items[1].grid_no, 300);
        assert_eq!(raw.items[2], unknown_visibility);

        // Without items the map still says that it saved them
        let mut empty = map.clone();
        empty.items.clear();
        empty.skipped_items.clear();
        raw.flags -= Ja2MapFlags::MAP_WORLDITEMS_SAVED;
        empty.apply_to_ja2(&mut raw).unwrap();
        assert!(raw.items.is_empty());
        assert!(raw.flags.contains(Ja2MapFlags::MAP_WORLDITEMS_SAVED));

        // The patched map can be written and read again
        edited.apply_to_ja2(&mut raw).unwrap();
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        let reloaded = Ja2Map::parse(&bytes).unwrap();
        assert!(reloaded == raw);
        let reloaded = fixture.convert(&reloaded);
        assert_eq!(reloaded.items.len(), 2);
        assert_eq!(reloaded.items[0].stack, edited.items[0].stack);
        assert_eq!(reloaded.items[0].flags, edited.items[0].flags);
        assert_eq!(reloaded.items[0].non_exist_chance, 30);
    }

    #[test]
    fn lights_survive_a_round_trip() {
        use crate::{
            file_formats::ja2_map::{Ja2LightSprite, Ja2Lights},
            map::LightId,
        };
        let fixture = Fixture::new();
        let mut raw = empty_map();
        let sprite = |x, flags, name: &str| Ja2LightSprite {
            x,
            y: 9,
            flags,
            template_name: name.to_owned(),
        };
        let night = sprite(2, LIGHT_SPR_ACTIVE | LIGHT_NIGHTTIME | 0x40, "L-R01.LHT");
        let missing = sprite(4, LIGHT_SPR_ACTIVE, "missing.lht");
        let always = sprite(3, LIGHT_SPR_ACTIVE | LIGHT_SPR_ON, "l-r01.lht");
        raw.lights = Some(Ja2Lights {
            colors: vec![[1, 2, 3, 9], [4, 5, 6, 7]],
            sprites: vec![night.clone(), missing.clone(), always.clone()],
        });
        raw.flags |= Ja2MapFlags::MAP_WORLDLIGHTS_SAVED;
        let map = fixture.convert(&raw);
        assert_eq!(map.lights.len(), 2);

        // An unchanged map saves the same lights, the converted ones first
        map.apply_to_ja2(&mut raw).unwrap();
        let lights = raw.lights.clone().unwrap();
        assert_eq!(lights.colors, [[1, 2, 3, 9], [4, 5, 6, 7]]);
        assert_eq!(
            lights.sprites,
            [night.clone(), always.clone(), missing.clone()]
        );

        // Edited lights keep their other flags, new ones are on
        let mut edited = map.clone();
        edited.light_color = [7, 8, 9];
        edited.lights[0].time = LightTime::PrimeTime;
        edited.lights.remove(1);
        edited.lights.push(MapLight {
            id: LightId(5),
            position: crate::components::map::GridPosition::new(10, 11),
            time: LightTime::Night,
            template_name: "L-R01.LHT".to_owned(),
            template: map.lights[0].template.clone(),
            file_sprite: None,
        });
        edited.apply_to_ja2(&mut raw).unwrap();
        let lights = raw.lights.clone().unwrap();
        // Only the first colour is the light colour
        assert_eq!(lights.colors, [[7, 8, 9, 9], [4, 5, 6, 7]]);
        assert_eq!(
            lights.sprites,
            [
                sprite(2, LIGHT_SPR_ACTIVE | LIGHT_PRIMETIME | 0x40, "L-R01.LHT"),
                Ja2LightSprite {
                    x: 10,
                    y: 11,
                    flags: LIGHT_SPR_ACTIVE | LIGHT_SPR_ON | LIGHT_NIGHTTIME,
                    template_name: "L-R01.LHT".to_owned(),
                },
                missing.clone(),
            ]
        );

        // Maps without lights get a light colour
        let mut no_lights = empty_map();
        fixture
            .convert(&no_lights)
            .apply_to_ja2(&mut no_lights)
            .unwrap();
        assert_eq!(no_lights.lights.as_ref().unwrap().colors, [[0, 0, 0, 0]]);
        assert!(no_lights.lights.as_ref().unwrap().sprites.is_empty());
        assert!(no_lights.flags.contains(Ja2MapFlags::MAP_WORLDLIGHTS_SAVED));

        // The patched map can be written and read again
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        let reloaded = Ja2Map::parse(&bytes).unwrap();
        assert!(reloaded == raw);
        let reloaded = fixture.convert(&reloaded);
        assert_eq!(reloaded.light_color, [7, 8, 9]);
        let times: Vec<_> = reloaded.lights.iter().map(|l| l.time).collect();
        assert_eq!(times, [LightTime::PrimeTime, LightTime::Night]);
    }

    #[test]
    fn the_ambient_light_is_only_written_when_it_changed() {
        use crate::file_formats::ja2_map::Ja2AmbientLight;
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.info.map_version = 24;
        raw.ambient_light = Some(Ja2AmbientLight {
            basement: false,
            caves: true,
            level: 5,
        });
        raw.flags |= Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED;
        let map = fixture.convert(&raw);
        // Old caves maps use another level than the saved one, which stays
        assert_ne!(map.ambient_light.unwrap().level, 5);
        map.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(raw.ambient_light.as_ref().unwrap().level, 5);

        raw.info.map_version = 25;
        let mut edited = fixture.convert(&raw);
        edited.ambient_light.as_mut().unwrap().level = 9;
        edited.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(
            raw.ambient_light,
            Some(Ja2AmbientLight {
                basement: false,
                caves: true,
                level: 9
            })
        );
        assert_eq!(fixture.convert(&raw).ambient_light, edited.ambient_light);

        // Maps that are above ground now lose it
        edited.ambient_light = None;
        edited.apply_to_ja2(&mut raw).unwrap();
        assert!(raw.ambient_light.is_none());
        assert!(!raw.flags.contains(Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED));

        // Maps without an ambient light don't get one
        let mut outdoors = empty_map();
        fixture
            .convert(&outdoors)
            .apply_to_ja2(&mut outdoors)
            .unwrap();
        assert!(outdoors.ambient_light.is_none());
        assert!(!outdoors
            .flags
            .contains(Ja2MapFlags::MAP_AMBIENTLIGHTLEVEL_SAVED));
    }

    #[test]
    fn rejects_too_many_soldiers() {
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.placements = vec![placement(1, 3, BODY_RANDOM); MAX_NUM_SOLDIERS + 1];
        raw.info.num_individuals = raw.placements.len() as u8;
        raw.flags |= Ja2MapFlags::MAP_FULLSOLDIER_SAVED;
        let map = fixture.convert(&raw);
        assert!(map
            .apply_to_ja2(&mut raw)
            .unwrap_err()
            .contains("149 soldiers"));
    }

    #[test]
    fn only_32_schedules_are_saved() {
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.schedules = (1..=40)
            .map(|id| schedule(id, ScheduleFlags::empty()))
            .collect();
        let map = fixture.convert(&raw);
        map.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(raw.schedules.len(), MAX_SCHEDULES);
        assert_eq!(raw.schedules[31].schedule_id, 32);
        assert!(raw.flags.contains(Ja2MapFlags::MAP_NPCSCHEDULES_SAVED));
    }

    #[test]
    fn exit_grids_and_entry_points_survive_a_round_trip() {
        use crate::{
            file_formats::ja2_map::Ja2ExitGrid,
            map::{EntryPoint, ExitGrid, SectorId},
        };
        let fixture = Fixture::new();
        let mut raw = empty_map();
        let exit_grid = |grid_no, dest_grid_no| Ja2ExitGrid {
            grid_no,
            dest_grid_no,
            dest_sector: (9, 1, 1),
        };
        // The second leads off the map, the third is off the map
        raw.exit_grids = vec![
            exit_grid(300, 50),
            exit_grid(200, 30000),
            exit_grid(26000, 1),
        ];
        raw.flags |= Ja2MapFlags::MAP_EXITGRIDS_SAVED;
        raw.info.north_grid_no = 1000;
        raw.info.center_grid_no = NOWHERE;
        raw.info.isolated_grid_no = 2000;
        let map = fixture.convert(&raw);
        assert_eq!(map.exit_grids.len(), 1);
        assert_eq!(map.skipped_exit_grids.len(), 2);

        // An unchanged map saves the same exit grids, sorted by grid number
        map.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(
            raw.exit_grids,
            [
                exit_grid(200, 30000),
                exit_grid(300, 50),
                exit_grid(26000, 1)
            ]
        );
        assert_eq!(
            (
                raw.info.north_grid_no,
                raw.info.center_grid_no,
                raw.info.isolated_grid_no
            ),
            (1000, NOWHERE, 2000)
        );

        let mut edited = map.clone();
        edited.exit_grids.push(ExitGrid {
            grid_no: grid(100),
            dest_grid_no: grid(7),
            dest_sector: SectorId { x: 11, y: 10, z: 0 },
        });
        // A new exit grid replaces the skipped one of its tile
        edited.exit_grids.push(ExitGrid {
            grid_no: grid(200),
            dest_grid_no: grid(8),
            dest_sector: SectorId { x: 1, y: 2, z: 3 },
        });
        *edited.info.entry_mut(EntryPoint::North) = None;
        *edited.info.entry_mut(EntryPoint::West) = Some(grid(4000));
        edited.apply_to_ja2(&mut raw).unwrap();
        let saved: Vec<u16> = raw.exit_grids.iter().map(|grid| grid.grid_no).collect();
        assert_eq!(saved, [100, 200, 300, 26000]);
        assert_eq!(raw.exit_grids[0].dest_sector, (11, 10, 0));
        assert_eq!(raw.exit_grids[1].dest_grid_no, 8);
        assert_eq!(
            (raw.info.north_grid_no, raw.info.west_grid_no),
            (NOWHERE, 4000)
        );

        // The patched map can be written and read again
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        let reloaded = Ja2Map::parse(&bytes).unwrap();
        assert!(reloaded == raw);
        let reloaded = fixture.convert(&reloaded);
        assert_eq!(reloaded.exit_grids.len(), 3);
        assert_eq!(reloaded.info.west_entry, Some(grid(4000)));
        assert_eq!(reloaded.info.north_entry, None);

        // Maps without exit grids don't start saving them
        let mut no_grids = empty_map();
        fixture
            .convert(&no_grids)
            .apply_to_ja2(&mut no_grids)
            .unwrap();
        assert!(!no_grids.flags.contains(Ja2MapFlags::MAP_EXITGRIDS_SAVED));
    }

    #[test]
    fn only_generated_edge_points_are_saved() {
        use crate::{
            file_formats::ja2_map::{Ja2EdgePointList, Ja2EdgePointSet, Ja2EdgePoints},
            map::{EdgePointList, EdgePointSet, EdgePoints},
        };
        let fixture = Fixture::new();
        let mut raw = empty_map();
        raw.info.map_version = 25;
        // Invalid grid numbers and a middle index past the end are kept as they are
        let list = |grid_nos: Vec<i16>, middle_index| Ja2EdgePointList {
            middle_index,
            grid_nos,
        };
        let saved = Ja2EdgePoints {
            primary: Ja2EdgePointSet {
                north: list(vec![1, -5, 3], 7),
                ..Default::default()
            },
            secondary: Some(Ja2EdgePointSet {
                west: list(vec![9], 0),
                ..Default::default()
            }),
        };
        raw.edge_points = Some(saved.clone());
        raw.flags |= Ja2MapFlags::MAP_EDGEPOINTS_SAVED;
        let map = fixture.convert(&raw);
        assert_eq!(map.edge_points.primary.north.middle_index, 7);
        map.apply_to_ja2(&mut raw).unwrap();
        assert_eq!(raw.edge_points.as_ref(), Some(&saved));

        let mut generated = map.clone();
        generated.edge_points = EdgePoints {
            primary: EdgePointSet {
                east: EdgePointList {
                    middle_index: 1,
                    grid_nos: vec![grid(1043), grid(1203)],
                },
                ..Default::default()
            },
            secondary: Some(EdgePointSet::default()),
        };
        generated.edge_points_stale = true;
        generated.apply_to_ja2(&mut raw).unwrap();
        let written = raw.edge_points.clone().unwrap();
        assert_eq!(written.primary.east, list(vec![1043, 1203], 1));
        assert_eq!(written.primary.north, list(vec![], 0));
        assert_eq!(written.secondary, Some(Ja2EdgePointSet::default()));
        raw.info.map_version = raw.version.minor;
        raw.russian_unknown = Some([0; 148]);
        let bytes = raw.to_bytes().unwrap();
        assert!(Ja2Map::parse(&bytes).unwrap() == raw);

        // Maps before version 17 have no secondary edge points
        let mut old = empty_map();
        old.version.major = 5.;
        old.version.minor = 16;
        old.info.map_version = 16;
        generated.apply_to_ja2(&mut old).unwrap();
        assert!(old.flags.contains(Ja2MapFlags::MAP_EDGEPOINTS_SAVED));
        assert_eq!(old.edge_points.as_ref().unwrap().secondary, None);
        old.to_bytes().unwrap();
    }
}
