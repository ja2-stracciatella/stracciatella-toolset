//! Converts the map file into the runtime map

use std::sync::Arc;

use bevy::prelude::*;

use super::placement::{appearance, Appearance, AppearanceContext, PlacementId};
use super::save;
use super::{
    AmbientLight, Attitude, DetailedPlacement, Direction, Door, DoorTrap, DoorTrapKind,
    EdgePointList, EdgePointSet, EdgePoints, EditorSmoothing, ExitGrid, GridArray, GridNo,
    ItemHeight, ItemStack, Level, LightId, LightTime, LockId, MapContext, MapInfo, MapItem,
    MapLight, MapTile, Orders, ProfileId, RoomNo, Schedule, ScheduleAction, ScheduleEntry,
    ScheduleFlags, ScheduleId, SectorId, SoldierPlacement, SoldierStats, TacticalMap, Visibility,
    WorldItemFlags, WorldItemId, MAX_PATROL_POINTS, NUM_INV_SLOTS,
};
use crate::{
    components::map::{GridPosition, JA2_MAP_TILES},
    file_formats::{
        ja2_map::{
            Ja2DetailedSoldierPlacement, Ja2EdgePointList, Ja2EdgePointSet, Ja2ExitGrid,
            Ja2LightSprite, Ja2Map, Ja2MapElementFlags, Ja2MapInfo, Ja2Object, Ja2Schedule,
            Ja2SoldierPlacement, Ja2TileLayer, Ja2WorldItem,
        },
        ja2_tile_types::{TileType, FIRSTPOINTERS},
    },
    item::{ItemId, ItemTable},
    tileset::{Tile, Tileset, TilesetId},
};

/// Raw grid number of nothing (`NOWHERE`)
pub(super) const NOWHERE: i16 = -1;

/// Raw body type of placements whose body type is chosen at random (`BODY_RANDOM`)
pub(super) const BODY_RANDOM: i8 = -1;

/// Raw time, grid number and data of unused schedule actions
pub(super) const NO_SCHEDULE_DATA: u16 = 0xffff;

/// Ambient light of old caves maps (`ValidateAndUpdateMapVersionIfNecessary`)
const OLD_CAVES_LIGHTLEVEL: u8 = 13;

impl TacticalMap {
    /// Converts a map file, anything that can't be converted is left out with a warning
    ///
    /// Fails if the map as a whole can't be used.
    pub fn from_ja2(map: &Ja2Map, ctx: &MapContext) -> Result<TacticalMap, String> {
        let tileset = u8::try_from(map.tileset_id)
            .map(TilesetId::new)
            .map_err(|_| {
                format!(
                    "The map uses tileset {}, which does not exist",
                    map.tileset_id
                )
            })?;
        if map.map_elements.len() != JA2_MAP_TILES {
            return Err(format!(
                "The map has {} tiles instead of {}",
                map.map_elements.len(),
                JA2_MAP_TILES
            ));
        }
        let info = convert_info(&map.info);
        let (light_color, lights, skipped_lights) = convert_lights(map, ctx);
        let (soldiers, skipped_placements) = convert_placements(map, ctx);
        let (items, skipped_items) = convert_items(map, &info, ctx);
        let (exit_grids, skipped_exit_grids) = convert_exit_grids(map);
        Ok(TacticalMap {
            tileset,
            tiles: convert_tiles(map, ctx.tileset),
            ambient_light: convert_ambient_light(map),
            light_color,
            lights,
            skipped_lights,
            items,
            skipped_items,
            soldiers,
            skipped_placements,
            exit_grids,
            skipped_exit_grids,
            doors: convert_doors(map),
            edge_points: map
                .edge_points
                .as_ref()
                .map(|edge_points| EdgePoints {
                    primary: convert_edge_point_set(&edge_points.primary),
                    secondary: edge_points.secondary.as_ref().map(convert_edge_point_set),
                })
                .unwrap_or_default(),
            edge_points_stale: false,
            schedules: map.schedules.iter().filter_map(convert_schedule).collect(),
            info,
        })
    }
}

/// A grid number of the map file, `None` with a warning if it is not on the map
fn valid_grid_no(raw: i32, what: &str) -> Option<GridNo> {
    let grid_no = GridNo::from_i32(raw);
    if grid_no.is_none() {
        warn!(
            "Leaving out {}: grid number {} is not on the map",
            what, raw
        );
    }
    grid_no
}

fn convert_info(info: &Ja2MapInfo) -> MapInfo {
    let point = |raw: i16, what: &str| {
        if raw == NOWHERE {
            None
        } else {
            valid_grid_no(raw.into(), what)
        }
    };
    let editor_smoothing =
        EditorSmoothing::from_number(info.editor_smoothing_type).unwrap_or_else(|| {
            warn!(
                "Unknown editor smoothing type {}, using normal smoothing",
                info.editor_smoothing_type
            );
            EditorSmoothing::Normal
        });
    MapInfo {
        north_entry: point(info.north_grid_no, "the north entry point"),
        east_entry: point(info.east_grid_no, "the east entry point"),
        south_entry: point(info.south_grid_no, "the south entry point"),
        west_entry: point(info.west_grid_no, "the west entry point"),
        center: point(info.center_grid_no, "the center entry point"),
        isolated: point(info.isolated_grid_no, "the isolated entry point"),
        restricted_scroll: info.restricted_scroll_id != 0,
        editor_smoothing,
    }
}

/// A tile of a map file, `None` if it is not in the tileset
fn resolve_tile(tileset: &Tileset, kind: u8, sub_index: u16) -> Option<Arc<Tile>> {
    let index = tileset.index(TileType::from(kind), sub_index)?;
    tileset.tile(index).cloned()
}

/// Resolves a tile layer, counts the tiles that are not in the tileset
fn resolve_layer<'a>(
    tileset: &Tileset,
    layers: impl IntoIterator<Item = &'a Ja2TileLayer>,
    missing: &mut usize,
) -> Vec<Arc<Tile>> {
    layers
        .into_iter()
        .filter_map(|layer| {
            let tile = resolve_tile(tileset, layer.kind, layer.sub_index);
            if tile.is_none() {
                *missing += 1;
            }
            tile
        })
        .collect()
}

/// Resolves the tile layers, like the layer loading of `LoadWorldFromSGPFile`
fn convert_tiles(map: &Ja2Map, tileset: &Tileset) -> GridArray<MapTile> {
    let mut missing = 0;
    let tiles = GridArray::from_fn(|grid_no| {
        let element = &map.map_elements[grid_no.index()];
        let mut structs = vec![];
        for layer in &element.structs {
            let Some(mut tile) = resolve_tile(tileset, layer.kind, layer.sub_index) else {
                missing += 1;
                continue;
            };
            if map.version.minor <= 25 && tile.image.underflow_filler {
                // Workaround of the game for "phantom menace" struct data in old maps: skip
                // underflow fillers if there is more than one struct on the tile, otherwise use
                // the first tile of the type
                if element.structs.len() > 1 {
                    continue;
                }
                match resolve_tile(tileset, layer.kind, 1) {
                    Some(first) => tile = first,
                    None => continue,
                }
            }
            structs.push(tile);
        }
        // Objects of the pointer types and later are editor cursors, the game skips them
        let objects = element
            .objects
            .iter()
            .filter(|object| TileType::from(object.kind) < FIRSTPOINTERS);
        MapTile {
            height: element.height,
            land: resolve_layer(tileset, &element.land, &mut missing),
            objects: resolve_layer(tileset, objects, &mut missing),
            structs,
            shadows: resolve_layer(tileset, &element.shadows, &mut missing),
            roofs: resolve_layer(tileset, &element.roofs, &mut missing),
            on_roofs: resolve_layer(tileset, &element.on_roofs, &mut missing),
            room: RoomNo::new(element.room),
            enemy_mine: element
                .flags
                .contains(Ja2MapElementFlags::MAPELEMENT_ENEMY_MINE_PRESENT),
            player_mine: element
                .flags
                .contains(Ja2MapElementFlags::MAPELEMENT_PLAYER_MINE_PRESENT),
            cave: false,
        }
    });
    if missing > 0 {
        warn!(
            "{} tiles of the map are missing in the tile tileset",
            missing
        );
    }
    tiles
}

/// An item stack, `None` if it has no item (`NOTHING`)
pub(super) fn convert_object(object: &Ja2Object) -> Option<ItemStack> {
    Some(ItemStack {
        item: ItemId::new(object.item)?,
        number: object.number,
        data: object.data,
        attachments: object
            .attach_items
            .iter()
            .zip(object.attach_status)
            .filter_map(|(&item, status)| Some((ItemId::new(item)?, status)))
            .collect(),
        flags: object.flags,
        mission: object.mission,
        trap: object.trap,
        imprint_id: object.imprint_id,
        weight: object.weight,
        used: object.used,
    })
}

/// The world items, as `LoadWorldItemsFromMap` loads them in the editor, and the ones of the map
/// file that could not be converted
///
/// The items are numbered in the order of the map file.
fn convert_items(
    map: &Ja2Map,
    info: &MapInfo,
    ctx: &MapContext,
) -> (Vec<MapItem>, Vec<Ja2WorldItem>) {
    let mut missing_graphics = 0;
    let mut items = vec![];
    let mut skipped = vec![];
    for (index, item) in map.items.iter().enumerate() {
        let Some(stack) = convert_object(&item.object) else {
            warn!("Leaving out world item {}: it has no item", index);
            skipped.push(item.clone());
            continue;
        };
        // `NOWHERE` and other invalid grid numbers become the center of the map
        let on_map = GridNo::from_i32(item.grid_no.into());
        let Some(grid_no) = on_map.or(info.center) else {
            warn!(
                "Leaving out world item {}: grid number {} is not on the map, and the map has no \
                 center",
                index, item.grid_no
            );
            skipped.push(item.clone());
            continue;
        };
        let Some(visibility) = Visibility::from_number(item.visible) else {
            warn!(
                "Leaving out world item {}: unknown visibility {}",
                index, item.visible
            );
            skipped.push(item.clone());
            continue;
        };
        let graphic = item_graphic(ctx.tileset, ctx.items, stack.item);
        if graphic.is_none() {
            missing_graphics += 1;
        }
        let render_height = match item.render_z_height_above_level {
            -1 => ItemHeight::ForceGround,
            height => ItemHeight::Above(height.max(0) as u8),
        };
        let file_object =
            (save::convert_object(&stack) != item.object).then(|| item.object.clone());
        items.push(MapItem {
            id: WorldItemId(items.len() as u32),
            grid_no,
            file_grid_no: on_map.is_none().then_some(item.grid_no),
            level: Level::from_number(item.level),
            stack,
            file_object,
            graphic,
            render_height,
            visibility,
            flags: WorldItemFlags::from_bits_retain(item.flags),
            non_exist_chance: item.non_exist_chance,
        });
    }
    if missing_graphics > 0 {
        warn!("{} items of the map have no graphic", missing_graphics);
    }
    (items, skipped)
}

/// The graphic of an item on the ground, `None` if it has none
pub fn item_graphic(tileset: &Tileset, items: &ItemTable, item: ItemId) -> Option<Arc<Tile>> {
    items
        .graphic(item)
        .and_then(|(tile_type, sub_index)| tileset.index(tile_type, sub_index))
        .and_then(|tile_index| tileset.tile(tile_index))
        .cloned()
}

/// The ambient light of basements and caves, old caves maps get the level the game uses
pub fn convert_ambient_light(map: &Ja2Map) -> Option<AmbientLight> {
    map.ambient_light.as_ref().map(|ambient| AmbientLight {
        basement: ambient.basement,
        caves: ambient.caves,
        level: if ambient.caves && map.info.map_version < 25 {
            OLD_CAVES_LIGHTLEVEL
        } else {
            ambient.level
        },
    })
}

/// The light colour, the light sprites and the ones that can't be converted (`LoadMapLights`)
///
/// The lights get ids in the order of the map file.
fn convert_lights(map: &Ja2Map, ctx: &MapContext) -> ([u8; 3], Vec<MapLight>, Vec<Ja2LightSprite>) {
    let Some(lights) = &map.lights else {
        return ([0; 3], vec![], vec![]);
    };
    let light_color = lights
        .colors
        .first()
        .map_or([0; 3], |color| [color[0], color[1], color[2]]);
    let mut missing_templates = 0;
    let mut converted = vec![];
    let mut skipped = vec![];
    for (index, sprite) in lights.sprites.iter().enumerate() {
        let Some(position) = GridPosition::from_xy(sprite.x.into(), sprite.y.into()) else {
            warn!(
                "Leaving out light {}: {} x {} is not on the map",
                sprite.template_name, sprite.x, sprite.y
            );
            skipped.push(sprite.clone());
            continue;
        };
        // The game ignores lights whose template can't be loaded
        let Some(template) = ctx
            .light_templates
            .get(&sprite.template_name.to_lowercase())
        else {
            missing_templates += 1;
            skipped.push(sprite.clone());
            continue;
        };
        converted.push(MapLight {
            id: LightId(index as u32),
            position,
            time: LightTime::from_flags(sprite.flags),
            template_name: sprite.template_name.clone(),
            template: template.clone(),
            file_sprite: Some(sprite.clone()),
        });
    }
    if missing_templates > 0 {
        warn!(
            "Leaving out {} lights of the map whose template is missing",
            missing_templates
        );
    }
    (light_color, converted, skipped)
}

/// The valid placements and the invalid ones, which are left out with a warning
fn convert_placements(
    map: &Ja2Map,
    ctx: &MapContext,
) -> (Vec<SoldierPlacement>, Vec<Ja2SoldierPlacement>) {
    let mut valid = vec![];
    let mut skipped = vec![];
    for (index, placement) in map.placements.iter().enumerate() {
        match convert_placement(index, placement, ctx) {
            Ok(converted) => valid.push(converted),
            Err(error) => {
                warn!(
                    "Skipping soldier placement {} at grid number {}: {}",
                    index, placement.basic.starting_grid_no, error
                );
                skipped.push(placement.clone());
            }
        }
    }
    (valid, skipped)
}

/// Validates a placement and makes the random choices of the game
/// (`CreateDetailedPlacementGivenBasicPlacementInfo`, `TacticalCreateSoldier`)
fn convert_placement(
    index: usize,
    placement: &Ja2SoldierPlacement,
    ctx: &MapContext,
) -> Result<SoldierPlacement, String> {
    let definitions = ctx.soldiers;
    let basic = &placement.basic;
    let team = definitions
        .teams
        .validate(basic.team)
        .ok_or_else(|| format!("unknown team {}", basic.team))?;
    let soldier_class = definitions
        .classes
        .validate(basic.soldier_class)
        .ok_or_else(|| format!("unknown soldier class {}", basic.soldier_class))?;
    let given_body = match basic.body_type {
        BODY_RANDOM => None,
        raw => Some(
            definitions
                .bodies
                .validate(raw)
                .ok_or_else(|| format!("unknown body type {}", raw))?,
        ),
    };
    let grid_no = GridNo::new(basic.starting_grid_no.into())
        .ok_or_else(|| "the grid number is not on the map".to_owned())?;
    let orders = Orders::from_number(basic.orders)
        .ok_or_else(|| format!("unknown orders {}", basic.orders))?;
    let attitude = Attitude::from_number(basic.attitude)
        .ok_or_else(|| format!("unknown attitude {}", basic.attitude))?;
    // Directions >= 100 are the old way to save a fixed direction
    let mut direction = basic.direction as u8;
    if direction >= 100 {
        direction -= 100;
    }
    let direction = Direction::ALL[usize::from(direction % 8)];
    // The first entry is not a patrol point, the game sets it to where the soldier stands
    let patrol_count = usize::try_from(basic.patrol_cnt)
        .unwrap_or(0)
        .min(MAX_PATROL_POINTS);
    let patrol = basic.patrol_grid[1..=patrol_count]
        .iter()
        .filter_map(|&raw| {
            valid_grid_no(
                raw.into(),
                &format!("a patrol point of placement {}", index),
            )
        })
        .collect();
    let detailed = placement.detailed.as_ref().map(convert_detailed);
    let Appearance {
        body,
        surface,
        palette,
    } = appearance(
        index,
        grid_no,
        &team,
        &soldier_class,
        given_body.as_ref(),
        detailed.as_ref(),
        &AppearanceContext {
            items: ctx.items,
            bodies: definitions.bodies,
            palettes: ctx.palettes,
        },
    );

    Ok(SoldierPlacement {
        id: PlacementId(index as u32),
        index,
        grid_no,
        direction,
        level: if basic.on_roof {
            Level::Roof
        } else {
            Level::Ground
        },
        team,
        soldier_class,
        given_body,
        body,
        surface,
        palette,
        relative_attribute_level: basic.relative_attribute_level,
        relative_equipment_level: basic.relative_equipment_level,
        orders,
        attitude,
        patrol,
        civilian_group: basic.civilian_group,
        priority_existence: basic.priority_existance,
        has_keys: basic.has_keys,
        detailed,
    })
}

fn convert_detailed(detailed: &Ja2DetailedSoldierPlacement) -> DetailedPlacement {
    // -1 means that the game chooses the value
    let stat = |value: i8| (value != -1).then_some(value);
    let sector = (|| {
        Some(SectorId {
            x: u8::try_from(detailed.sector_x).ok()?,
            y: u8::try_from(detailed.sector_y).ok()?,
            z: u8::try_from(detailed.sector_z).ok()?,
        })
    })();
    let inventory: [Option<ItemStack>; NUM_INV_SLOTS] =
        std::array::from_fn(|slot| detailed.inventory.get(slot).and_then(convert_object));
    DetailedPlacement {
        is_static: detailed.is_static,
        profile: ProfileId::new(detailed.profile),
        copy_profile_items_over: detailed.copy_profile_items_over,
        sector,
        insertion_grid_no: GridNo::from_i32(detailed.insertion_grid_no.into()),
        stats: SoldierStats {
            life_max: stat(detailed.life_max),
            life: stat(detailed.life),
            agility: stat(detailed.agility),
            dexterity: stat(detailed.dexterity),
            exp_level: stat(detailed.exp_level),
            marksmanship: stat(detailed.marksmanship),
            medical: stat(detailed.medical),
            mechanical: stat(detailed.mechanical),
            explosive: stat(detailed.explosive),
            leadership: stat(detailed.leadership),
            strength: stat(detailed.strength),
            wisdom: stat(detailed.wisdom),
            morale: stat(detailed.morale),
            ai_morale: stat(detailed.ai_morale),
        },
        inventory,
        head_palette: detailed.head_pal.clone(),
        pants_palette: detailed.pants_pal.clone(),
        vest_palette: detailed.vest_pal.clone(),
        skin_palette: detailed.skin_pal.clone(),
        visible: detailed.visible,
        name: detailed.name.clone(),
        schedule: ScheduleId::new(detailed.schedule_id),
        vehicle_id: detailed
            .use_given_vehicle
            .then_some(detailed.use_given_vehicle_id),
    }
}

/// The exit grids of the map file, and the ones that could not be converted
fn convert_exit_grids(map: &Ja2Map) -> (Vec<ExitGrid>, Vec<Ja2ExitGrid>) {
    let mut skipped = vec![];
    let exit_grids = map
        .exit_grids
        .iter()
        .filter_map(|exit_grid| {
            let (x, y, z) = exit_grid.dest_sector;
            let grid_no = valid_grid_no(exit_grid.grid_no.into(), "an exit grid");
            let dest_grid_no = valid_grid_no(
                exit_grid.dest_grid_no.into(),
                "the destination of an exit grid",
            );
            let (Some(grid_no), Some(dest_grid_no)) = (grid_no, dest_grid_no) else {
                skipped.push(exit_grid.clone());
                return None;
            };
            Some(ExitGrid {
                grid_no,
                dest_grid_no,
                dest_sector: SectorId { x, y, z },
            })
        })
        .collect();
    (exit_grids, skipped)
}

fn convert_doors(map: &Ja2Map) -> Vec<Door> {
    map.doors
        .iter()
        .filter_map(|door| {
            let grid_no = valid_grid_no(door.grid_no.into(), "a door")?;
            let trap = match DoorTrapKind::from_number(door.trap_id) {
                Ok(kind) => kind.map(|kind| DoorTrap {
                    kind,
                    level: door.trap_level,
                }),
                Err(trap_id) => {
                    warn!(
                        "Leaving out the door at grid number {}: unknown trap {}",
                        grid_no.index(),
                        trap_id
                    );
                    return None;
                }
            };
            Some(Door {
                grid_no,
                locked: door.locked,
                trap,
                lock: LockId::new(door.lock_id),
                perceived_locked: door.perceived_locked,
                perceived_trapped: door.perceived_trapped,
                lock_damage: door.lock_damage,
            })
        })
        .collect()
}

fn convert_edge_point_list(list: &Ja2EdgePointList) -> EdgePointList {
    EdgePointList {
        middle_index: list.middle_index,
        grid_nos: list
            .grid_nos
            .iter()
            .filter_map(|&raw| valid_grid_no(raw.into(), "an edge point"))
            .collect(),
    }
}

fn convert_edge_point_set(set: &Ja2EdgePointSet) -> EdgePointSet {
    EdgePointSet {
        north: convert_edge_point_list(&set.north),
        east: convert_edge_point_list(&set.east),
        south: convert_edge_point_list(&set.south),
        west: convert_edge_point_list(&set.west),
    }
}

fn convert_schedule(schedule: &Ja2Schedule) -> Option<Schedule> {
    let Some(id) = ScheduleId::new(schedule.schedule_id) else {
        warn!("Leaving out a schedule without id");
        return None;
    };
    let actions = std::array::from_fn(|slot| {
        let data = |raw: u16, what: &str| {
            valid_grid_no(
                raw.into(),
                &format!("action {} of schedule {} ({})", slot, id.number(), what),
            )
        };
        let door = |door, destination| Some((data(door, "door")?, data(destination, "tile")?));
        let (data1, data2) = (schedule.data1[slot], schedule.data2[slot]);
        let action = match schedule.action[slot] {
            0 => return None,
            1 => {
                let (door, destination) = door(data1, data2)?;
                ScheduleAction::LockDoor { door, destination }
            }
            2 => {
                let (door, destination) = door(data1, data2)?;
                ScheduleAction::UnlockDoor { door, destination }
            }
            3 => {
                let (door, destination) = door(data1, data2)?;
                ScheduleAction::OpenDoor { door, destination }
            }
            4 => {
                let (door, destination) = door(data1, data2)?;
                ScheduleAction::CloseDoor { door, destination }
            }
            5 => ScheduleAction::GridNo(data(data1, "tile")?),
            6 => ScheduleAction::LeaveSector,
            7 => ScheduleAction::EnterSector(data(data1, "tile")?),
            8 => ScheduleAction::StayInSector,
            9 => ScheduleAction::Sleep(data(data1, "tile")?),
            10 => ScheduleAction::Wake,
            unknown => {
                warn!(
                    "Leaving out action {} of schedule {}: unknown action {}",
                    slot,
                    id.number(),
                    unknown
                );
                return None;
            }
        };
        Some(ScheduleEntry {
            time: (schedule.time[slot] != NO_SCHEDULE_DATA).then_some(schedule.time[slot]),
            action,
        })
    });
    Some(Schedule {
        id,
        soldier_id: schedule.soldier_id,
        flags: ScheduleFlags::from_bits_retain(schedule.flags),
        actions,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        file_formats::{
            ja2_map::{
                Ja2AmbientLight, Ja2BasicSoldierPlacement, Ja2LightSprite, Ja2Lights,
                Ja2MapElement, Ja2MapVersion, Ja2WorldItem,
            },
            ja2_tile_database::{
                tests::{full_surfaces, surface},
                Ja2TileDatabase,
            },
            ja2_tile_types::{FIRSTOSTRUCT, FIRSTTEXTURE, GUNS},
        },
        item::{ItemFile, ItemInfo, ItemTable},
        light::LightTemplate,
        map::{HANDPOS, LIGHT_NIGHTTIME, LIGHT_PRIMETIME},
        soldier::{
            body_type::SoldierBodyTypeId, class::SoldierClassId, palette::ResolvedSoldierPalette,
            palette_files::PaletteReplacements, team::TeamId, Registries,
        },
        tileset::tests::test_tileset,
    };

    /// Every palette name that the palette choices use
    const PALETTE_NAMES: [&str; 26] = [
        "PINKSKIN",
        "TANSKIN",
        "DARKSKIN",
        "BLACKSKIN",
        "BROWNHEAD",
        "BLACKHEAD",
        "WHITEHEAD",
        "BLONDHEAD",
        "REDHEAD",
        "YELLOWVEST",
        "GREENPANTS",
        "BLACKSHIRT",
        "BLACKPANTS",
        "REDVEST",
        "GREENVEST",
        "BEIGEPANTS",
        "JEANVEST",
        "BLUEVEST",
        "greyVEST",
        "TANPANTS",
        "GYELLOWSHIRT",
        "WHITEVEST",
        "PURPLESHIRT",
        "BROWNVEST",
        "JEANPANTS",
        "BLUEPANTS",
    ];

    /// What the conversion needs besides the map
    pub(crate) struct Fixture {
        pub(crate) tileset: Tileset,
        items: ItemTable,
        registries: Registries,
        palettes: PaletteReplacements,
        light_templates: HashMap<String, Arc<LightTemplate>>,
    }

    impl Fixture {
        pub(in crate::map) fn new() -> Self {
            Self::with_tileset(test_tileset())
        }

        pub(crate) fn with_tileset(tileset: Tileset) -> Self {
            let item = |index, graphic, hand| ItemInfo {
                index: ItemId::new(index).unwrap(),
                graphic,
                hand,
                kind: Default::default(),
            };
            let items = ItemTable::new([&ItemFile {
                items: vec![
                    item(
                        1,
                        Some((GUNS, 2)),
                        Some(crate::item::HandItem::OneHandedGun),
                    ),
                    item(2, None, None),
                ],
            }]);
            Self {
                tileset,
                items,
                registries: Registries::build(),
                palettes: PaletteReplacements::with_names(&PALETTE_NAMES),
                light_templates: HashMap::from([(
                    "l-r01.lht".to_owned(),
                    Arc::new(LightTemplate::default()),
                )]),
            }
        }

        pub(crate) fn context(&self) -> MapContext<'_> {
            MapContext {
                tileset: &self.tileset,
                items: &self.items,
                soldiers: self.registries.definitions(),
                palettes: Some(&self.palettes),
                light_templates: &self.light_templates,
            }
        }

        pub(crate) fn convert(&self, map: &Ja2Map) -> TacticalMap {
            TacticalMap::from_ja2(map, &self.context()).unwrap()
        }
    }

    pub(in crate::map) fn empty_map() -> Ja2Map {
        Ja2Map {
            version: Ja2MapVersion {
                major: 6.,
                minor: 26,
            },
            map_elements: vec![Ja2MapElement::default(); JA2_MAP_TILES],
            ..default()
        }
    }

    pub(in crate::map) fn grid(index: usize) -> GridNo {
        GridNo::new(index).unwrap()
    }

    pub(in crate::map) fn layer(kind: TileType, sub_index: u16) -> Ja2TileLayer {
        Ja2TileLayer {
            kind: kind as u8,
            sub_index,
        }
    }

    #[test]
    fn rejects_unusable_maps() {
        let fixture = Fixture::new();
        let ctx = fixture.context();
        let map = Ja2Map {
            tileset_id: -1,
            ..empty_map()
        };
        assert!(TacticalMap::from_ja2(&map, &ctx).is_err());
        assert!(TacticalMap::from_ja2(&Ja2Map::default(), &ctx).is_err());
        let map = Ja2Map {
            tileset_id: 38,
            ..empty_map()
        };
        assert_eq!(
            TacticalMap::from_ja2(&map, &ctx).unwrap().tileset,
            TilesetId::new(38)
        );
    }

    #[test]
    fn resolves_tile_layers() {
        let fixture = Fixture::new();
        let mut map = empty_map();
        map.map_elements[5] = Ja2MapElement {
            height: 4,
            land: vec![layer(FIRSTTEXTURE, 1), layer(FIRSTTEXTURE, 0)],
            objects: vec![layer(FIRSTPOINTERS, 1), layer(FIRSTOSTRUCT, 2)],
            room: 3,
            flags: Ja2MapElementFlags::MAPELEMENT_ENEMY_MINE_PRESENT
                | Ja2MapElementFlags::MAPELEMENT_REDUNDANT,
            ..default()
        };
        let converted = fixture.convert(&map);
        let tile = &converted.tiles[grid(5)];
        assert_eq!(tile.height, 4);
        // Sub index 0 is no tile, pointers are left out
        let ids = |tiles: &[Arc<Tile>]| tiles.iter().map(|tile| tile.id).collect::<Vec<_>>();
        let index = |tile_type, sub_index| fixture.tileset.index(tile_type, sub_index).unwrap();
        assert_eq!(ids(&tile.land), [index(FIRSTTEXTURE, 1)]);
        assert_eq!(ids(&tile.objects), [index(FIRSTOSTRUCT, 2)]);
        assert_eq!(tile.room.map(RoomNo::number), Some(3));
        assert!(tile.enemy_mine);
        assert!(!tile.player_mine);
        assert!(converted.tiles[grid(4)].room.is_none());
    }

    #[test]
    fn phantom_menace_workaround() {
        let mut surfaces = full_surfaces();
        surfaces[FIRSTOSTRUCT] = Some(surface("ostruct.sti", 3));
        let fixture = Fixture::with_tileset(Tileset::from_database(
            Ja2TileDatabase::build(surfaces).unwrap(),
        ));
        let first = fixture.tileset.index(FIRSTOSTRUCT, 1).unwrap();
        let filler = fixture.tileset.index(FIRSTOSTRUCT, 5).unwrap();
        let mut map = empty_map();
        map.map_elements[0].structs = vec![layer(FIRSTOSTRUCT, 5)];
        map.map_elements[1].structs = vec![layer(FIRSTOSTRUCT, 5), layer(FIRSTOSTRUCT, 2)];
        let structs = |map: &TacticalMap, index| -> Vec<_> {
            map.tiles[grid(index)]
                .structs
                .iter()
                .map(|tile| tile.id)
                .collect()
        };

        // Old maps use the first tile instead of a single filler, and skip other fillers
        map.version.minor = 25;
        let old = fixture.convert(&map);
        assert_eq!(structs(&old, 0), [first]);
        assert_eq!(structs(&old, 1), [first + 1]);
        // New maps keep them
        map.version.minor = 26;
        let new = fixture.convert(&map);
        assert_eq!(structs(&new, 0), [filler]);
        assert_eq!(structs(&new, 1), [filler, first + 1]);
    }

    #[test]
    fn entry_points() {
        let fixture = Fixture::new();
        let mut map = empty_map();
        map.info.north_grid_no = NOWHERE;
        map.info.east_grid_no = 30000;
        map.info.center_grid_no = 12880;
        map.info.restricted_scroll_id = 1;
        map.info.editor_smoothing_type = 2;
        let info = fixture.convert(&map).info;
        assert_eq!(info.north_entry, None);
        assert_eq!(info.east_entry, None);
        assert_eq!(info.center, Some(grid(12880)));
        assert!(info.restricted_scroll);
        assert_eq!(info.editor_smoothing, EditorSmoothing::Caves);
    }

    #[test]
    fn world_items() {
        let fixture = Fixture::new();
        let world_item = |item, grid_no, render_z_height_above_level| Ja2WorldItem {
            exists: true,
            grid_no,
            level: 1,
            object: Ja2Object {
                item,
                attach_items: [0, 2, 0, 0],
                attach_status: [0, 50, 0, 0],
                ..default()
            },
            render_z_height_above_level,
            visible: 1,
            ..default()
        };
        let mut map = empty_map();
        map.info.center_grid_no = 100;
        map.items = vec![
            world_item(1, 5, -1),
            // No item
            world_item(0, 5, 0),
            // `NOWHERE` is the center
            world_item(2, -1, 12),
            // Unknown visibility
            Ja2WorldItem {
                visible: 9,
                ..world_item(1, 5, 0)
            },
        ];
        let converted = fixture.convert(&map);
        let items = converted.items;
        assert_eq!(items.len(), 2);
        // Left out items are kept as they are, the others are numbered in their order
        assert_eq!(
            converted.skipped_items,
            [map.items[1].clone(), map.items[3].clone()]
        );
        assert_eq!(items.iter().map(|i| i.id.0).collect::<Vec<_>>(), [0, 1]);
        assert_eq!(items[0].file_grid_no, None);
        assert_eq!(items[1].file_grid_no, Some(NOWHERE));
        assert_eq!(items[0].grid_no, grid(5));
        assert_eq!(items[0].level, Level::Roof);
        assert_eq!(items[0].render_height, ItemHeight::ForceGround);
        assert_eq!(items[0].visibility, Visibility::Visible);
        assert_eq!(
            items[0].graphic.as_ref().map(|tile| tile.id),
            fixture.tileset.index(GUNS, 2)
        );
        assert_eq!(items[0].stack.attachments, [(ItemId::new(2).unwrap(), 50)]);
        assert_eq!(items[1].grid_no, grid(100));
        assert_eq!(items[1].render_height, ItemHeight::Above(12));
        // Items without a graphic of their own show the first gun
        assert_eq!(
            items[1].graphic.as_ref().map(|tile| tile.id),
            fixture.tileset.index(GUNS, 1)
        );

        // Without a center, items at `NOWHERE` are left out
        map.info.center_grid_no = NOWHERE;
        let converted = fixture.convert(&map);
        assert_eq!(converted.items.len(), 1);
        assert_eq!(converted.skipped_items.len(), 3);
    }

    #[test]
    fn lights() {
        let fixture = Fixture::new();
        let sprite = |x, flags, template_name: &str| Ja2LightSprite {
            x,
            y: 7,
            flags,
            template_name: template_name.to_owned(),
        };
        let mut map = empty_map();
        map.lights = Some(Ja2Lights {
            colors: vec![[1, 2, 3, 0]],
            sprites: vec![
                sprite(5, 0, "L-R01.LHT"),
                sprite(6, LIGHT_NIGHTTIME, "l-r01.lht"),
                sprite(7, LIGHT_PRIMETIME | 0x1, "l-r01.lht"),
                // Missing template and off the map
                sprite(8, 0, "missing.lht"),
                sprite(-1, 0, "l-r01.lht"),
            ],
        });
        let converted = fixture.convert(&map);
        assert_eq!(converted.light_color, [1, 2, 3]);
        let lights: Vec<_> = converted
            .lights
            .iter()
            .map(|light| (light.id, light.position, light.time))
            .collect();
        assert_eq!(
            lights,
            [
                (LightId(0), GridPosition::new(5, 7), LightTime::Always),
                (LightId(1), GridPosition::new(6, 7), LightTime::Night),
                (LightId(2), GridPosition::new(7, 7), LightTime::PrimeTime),
            ]
        );
        // The lights keep the name and the record of the map file
        let first = &converted.lights[0];
        assert_eq!(first.template_name, "L-R01.LHT");
        assert_eq!(first.radius(), Some(1));
        assert_eq!(
            first.file_sprite.as_ref(),
            map.lights.as_ref().map(|l| &l.sprites[0])
        );
        // The others are kept for saving
        let skipped: Vec<_> = converted.skipped_lights.iter().map(|s| s.x).collect();
        assert_eq!(skipped, [8, -1]);
        assert_eq!(fixture.convert(&empty_map()).light_color, [0; 3]);
    }

    #[test]
    fn old_caves_are_darker() {
        let fixture = Fixture::new();
        let mut map = empty_map();
        map.ambient_light = Some(Ja2AmbientLight {
            basement: false,
            caves: true,
            level: 5,
        });
        map.info.map_version = 24;
        assert_eq!(
            fixture.convert(&map).ambient_light.map(|a| a.level),
            Some(OLD_CAVES_LIGHTLEVEL)
        );
        map.info.map_version = 25;
        assert_eq!(
            fixture.convert(&map).ambient_light.map(|a| a.level),
            Some(5)
        );
    }

    #[test]
    fn schedules_and_doors() {
        let fixture = Fixture::new();
        let mut map = empty_map();
        map.schedules = vec![
            Ja2Schedule {
                time: [60, 120, 0, 0],
                data1: [300, 400, 0, 0],
                data2: [301, 0, 0, 0],
                action: [1, 5, 0, 42],
                schedule_id: 1,
                ..default()
            },
            Ja2Schedule::default(),
        ];
        map.doors = vec![
            crate::file_formats::ja2_map::Ja2Door {
                grid_no: 10,
                trap_id: 2,
                trap_level: 4,
                lock_id: 0,
                ..default()
            },
            crate::file_formats::ja2_map::Ja2Door {
                grid_no: 11,
                trap_id: 99,
                ..default()
            },
        ];
        let converted = fixture.convert(&map);
        assert_eq!(converted.schedules.len(), 1);
        let schedule = &converted.schedules[0];
        assert_eq!(schedule.id, ScheduleId::new(1).unwrap());
        assert_eq!(
            schedule.actions,
            [
                Some(ScheduleEntry {
                    time: Some(60),
                    action: ScheduleAction::LockDoor {
                        door: grid(300),
                        destination: grid(301)
                    }
                }),
                Some(ScheduleEntry {
                    time: Some(120),
                    action: ScheduleAction::GridNo(grid(400))
                }),
                None,
                None,
            ]
        );
        assert_eq!(converted.doors.len(), 1);
        assert_eq!(
            converted.doors[0].trap,
            Some(DoorTrap {
                kind: DoorTrapKind::Electric,
                level: 4
            })
        );
        assert_eq!(converted.doors[0].lock, None);
    }

    pub(in crate::map) fn placement(
        team: i8,
        soldier_class: u8,
        body_type: i8,
    ) -> Ja2SoldierPlacement {
        Ja2SoldierPlacement {
            basic: Ja2BasicSoldierPlacement {
                team,
                soldier_class,
                body_type,
                ..default()
            },
            detailed: None,
        }
    }

    pub(in crate::map) fn detailed() -> Ja2DetailedSoldierPlacement {
        Ja2DetailedSoldierPlacement {
            profile: 200,
            inventory: vec![Ja2Object::default(); NUM_INV_SLOTS],
            ..default()
        }
    }

    #[test]
    fn validates_placements() {
        let fixture = Fixture::new();
        let ctx = fixture.context();
        let convert = |placement: &Ja2SoldierPlacement| convert_placement(0, placement, &ctx);
        let valid = convert(&placement(4, 8, 27)).unwrap();
        assert_eq!(valid.team.id, TeamId::CIV);
        assert_eq!(valid.soldier_class.id, SoldierClassId::MINER);
        assert_eq!(valid.body.id, SoldierBodyTypeId::JEEP);
        assert!(convert(&placement(1, 0, -1)).is_ok());
        assert!(convert(&placement(1, 0, 28))
            .unwrap_err()
            .contains("body type 28"));
        assert!(convert(&placement(1, 0, -2)).is_err());
        assert!(convert(&placement(6, 0, 0)).unwrap_err().contains("team 6"));
        assert!(convert(&placement(-1, 0, 0)).is_err());
        assert!(convert(&placement(1, 9, 0))
            .unwrap_err()
            .contains("soldier class 9"));
        let mut off_map = placement(1, 0, 0);
        off_map.basic.starting_grid_no = JA2_MAP_TILES as u16;
        assert!(convert(&off_map).is_err());
        let mut orders = placement(1, 0, 0);
        orders.basic.orders = 8;
        assert!(convert(&orders).unwrap_err().contains("orders 8"));
    }

    #[test]
    fn resolves_placement_values() {
        let fixture = Fixture::new();
        let ctx = fixture.context();
        let mut raw = placement(1, 0, 0);
        raw.basic.starting_grid_no = 500;
        raw.basic.direction = 103;
        raw.basic.on_roof = true;
        raw.basic.orders = 4;
        raw.basic.attitude = 7;
        raw.basic.patrol_grid = [500, -1, 700, 800, 0, 0, 0, 0, 0, 0];
        raw.basic.patrol_cnt = 4;
        let mut detailed = detailed();
        detailed.profile = 57;
        detailed.life = -1;
        detailed.agility = 80;
        detailed.inventory[HANDPOS].item = 1;
        detailed.schedule_id = 2;
        raw.detailed = Some(detailed);
        let converted = convert_placement(0, &raw, &ctx).unwrap();
        assert_eq!(converted.grid_no, grid(500));
        assert_eq!(converted.direction, Direction::SouthEast);
        assert_eq!(converted.level, Level::Roof);
        assert_eq!(converted.orders, Orders::PointPatrol);
        assert_eq!(converted.attitude, Attitude::AttackSlayOnly);
        // The first entry is not a patrol point
        assert_eq!(converted.patrol, [grid(700), grid(800), grid(0)]);
        // Holding a pistol
        assert_eq!(
            converted.surface.id,
            crate::soldier::standing_surface::StandingSurfaceId::REGMALE_PISTOL
        );
        let detailed = converted.detailed.unwrap();
        assert_eq!(detailed.profile.map(ProfileId::number), Some(57));
        assert_eq!(detailed.stats.life, None);
        assert_eq!(detailed.stats.agility, Some(80));
        assert_eq!(detailed.schedule, ScheduleId::new(2));
        assert!(detailed.inventory[0].is_none());

        let mut no_profile = placement(1, 0, 0);
        no_profile.detailed = Some(self::detailed());
        let converted = convert_placement(0, &no_profile, &ctx).unwrap();
        assert_eq!(converted.detailed.unwrap().profile, None);
    }

    #[test]
    fn skips_invalid_placements() {
        let fixture = Fixture::new();
        let map = Ja2Map {
            placements: vec![placement(1, 2, 0), placement(9, 2, 0), placement(4, 0, -1)],
            ..empty_map()
        };
        let converted = fixture.convert(&map);
        let valid = converted.soldiers;
        let indices: Vec<_> = valid.iter().map(|placement| placement.index).collect();
        assert_eq!(indices, [0, 2]);
        assert_eq!(valid[1].team.id, TeamId::CIV);
        // Saving keeps them
        assert_eq!(converted.skipped_placements, [placement(9, 2, 0)]);
    }

    /// The body types chosen for `count` placements of `team`, `soldier_class` and `body_type`
    fn body_types(
        fixture: &Fixture,
        team: i8,
        soldier_class: u8,
        body_type: i8,
        count: usize,
    ) -> Vec<SoldierBodyTypeId> {
        let ctx = fixture.context();
        (0..count)
            .map(|index| {
                let raw = placement(team, soldier_class, body_type);
                convert_placement(index, &raw, &ctx).unwrap().body.id
            })
            .collect()
    }

    #[test]
    fn chooses_body_types() {
        use SoldierBodyTypeId as B;
        let fixture = Fixture::new();
        let body_types = |team, soldier_class, body_type, count| {
            body_types(&fixture, team, soldier_class, body_type, count)
        };
        assert_eq!(body_types(1, 0, 27, 1), [B::JEEP]);
        assert_eq!(body_types(4, 0, 3, 1), [B::REGFEMALE]);
        assert_eq!(body_types(1, 2, 3, 1), [B::REGFEMALE]);
        // Enemies that are not elite are never female, even with a given body type
        assert!(!body_types(1, 0, 3, 50).contains(&B::REGFEMALE));
        assert_eq!(body_types(2, 7, -1, 3), [B::REGMALE; 3]);
        let enemies = body_types(1, 3, -1, 200);
        assert!(!enemies.contains(&B::REGFEMALE));
        assert!(enemies.contains(&B::STOCKYMALE));
        let elites = body_types(1, 2, -1, 200);
        assert!(elites.contains(&B::REGFEMALE));
        let miners = body_types(4, 8, -1, 200);
        assert!(miners
            .iter()
            .all(|body| [B::REGMALE, B::BIGMALE, B::MANCIV].contains(body)));
        let civilians = body_types(4, 0, -1, 200);
        assert!(civilians
            .iter()
            .all(|body| (11..=16).contains(&body.index())));
        assert!(civilians.contains(&B::KIDCIV));
        // The same placement always gets the same body type
        assert_eq!(body_types(3, 4, -1, 20), body_types(3, 4, -1, 20));
    }

    /// The palettes of `count` placements, generated palettes as their replacement names
    fn palettes(
        fixture: &Fixture,
        team: i8,
        soldier_class: u8,
        body_type: SoldierBodyTypeId,
        detailed: Option<Ja2DetailedSoldierPlacement>,
        count: usize,
    ) -> Vec<ResolvedSoldierPalette> {
        let ctx = fixture.context();
        (0..count)
            .map(|index| {
                let raw = Ja2SoldierPlacement {
                    detailed: detailed.clone(),
                    ..placement(team, soldier_class, body_type.index() as i8)
                };
                convert_placement(index, &raw, &ctx).unwrap().palette
            })
            .collect()
    }

    fn names<'a>(fixture: &'a Fixture, palette: &ResolvedSoldierPalette) -> Vec<&'a str> {
        match palette {
            ResolvedSoldierPalette::Generated(ids) => ids
                .iter()
                .map(|&id| fixture.palettes.name(id).unwrap())
                .collect(),
            _ => panic!("expected a generated palette, got {:?}", palette),
        }
    }

    #[test]
    fn palettes_of_body_types() {
        use SoldierBodyTypeId as B;
        let fixture = Fixture::new();
        assert_eq!(
            palettes(&fixture, 2, 7, B::QUEENMONSTER, None, 1),
            [ResolvedSoldierPalette::Own]
        );
        assert_eq!(
            palettes(&fixture, 2, 7, B::AM_MONSTER, None, 1),
            [ResolvedSoldierPalette::Substitution(
                "anims/monsters/monster.col"
            )]
        );
        let larvae = palettes(&fixture, 2, 7, B::LARVAE_MONSTER, None, 1);
        assert_eq!(names(&fixture, &larvae[0]).len(), 4);
    }

    #[test]
    fn classes_wear_their_uniform() {
        let fixture = Fixture::new();
        for palette in palettes(&fixture, 1, 2, SoldierBodyTypeId::REGMALE, None, 50) {
            let [head, vest, pants, skin] = names(&fixture, &palette)[..] else {
                panic!("expected 4 replacements");
            };
            assert_eq!((vest, pants), ("BLACKSHIRT", "BLACKPANTS"));
            if ["DARKSKIN", "BLACKSKIN"].contains(&skin) {
                assert!(["BLACKHEAD", "BROWNHEAD"].contains(&head));
            }
        }
    }

    #[test]
    fn visible_placements_keep_their_clothes() {
        let fixture = Fixture::new();
        let detailed = Ja2DetailedSoldierPlacement {
            visible: true,
            vest_pal: "PURPLESHIRT".to_owned(),
            pants_pal: "TANPANTS".to_owned(),
            ..self::detailed()
        };
        let visible = palettes(
            &fixture,
            4,
            0,
            SoldierBodyTypeId::MANCIV,
            Some(detailed.clone()),
            20,
        );
        // The replacements are applied in the order of the game
        assert!(visible
            .iter()
            .all(|palette| names(&fixture, palette)[1..3] == ["PURPLESHIRT", "TANPANTS"]));
        let hidden = Ja2DetailedSoldierPlacement {
            visible: false,
            ..detailed
        };
        let hidden = palettes(&fixture, 4, 0, SoldierBodyTypeId::MANCIV, Some(hidden), 20);
        assert!(hidden
            .iter()
            .any(|palette| names(&fixture, palette)[1] != "PURPLESHIRT"));
    }

    #[test]
    fn palettes_without_replacements_file() {
        let fixture = Fixture::new();
        let ctx = MapContext {
            palettes: None,
            ..fixture.context()
        };
        let raw = placement(1, 2, 0);
        assert_eq!(
            convert_placement(0, &raw, &ctx).unwrap().palette,
            ResolvedSoldierPalette::Generated(vec![])
        );
        // Unknown names are left out
        let palettes = PaletteReplacements::with_names(&["PINKSKIN", "BLACKSHIRT"]);
        let ctx = MapContext {
            palettes: Some(&palettes),
            ..fixture.context()
        };
        let ResolvedSoldierPalette::Generated(ids) =
            convert_placement(0, &raw, &ctx).unwrap().palette
        else {
            panic!("expected a generated palette");
        };
        assert!(ids.contains(&palettes.find("BLACKSHIRT").unwrap()));
        assert!(ids.len() < 4);
    }

    /// Converts every `.dat` file in the directory given by `JA2_MAPS_DIR` with the test tileset
    #[test]
    fn convert_maps_from_dir() {
        let Ok(dir) = std::env::var("JA2_MAPS_DIR") else {
            eprintln!("JA2_MAPS_DIR not set, skipping");
            return;
        };
        let fixture = Fixture::new();
        let ctx = fixture.context();
        let mut converted = 0;
        let mut failures = vec![];
        for entry in std::fs::read_dir(&dir).expect("JA2_MAPS_DIR should be readable") {
            let path = entry.expect("dir entry should be readable").path();
            let is_dat = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"));
            if !is_dat {
                continue;
            }
            let bytes = std::fs::read(&path).expect("map should be readable");
            let Ok(map) = Ja2Map::parse(&bytes) else {
                continue;
            };
            match TacticalMap::from_ja2(&map, &ctx) {
                Ok(converted_map) => {
                    converted += 1;
                    // An unchanged map saves the same world items
                    let mut saved = map.clone();
                    converted_map.apply_items(&mut saved);
                    if saved.items != map.items {
                        failures.push(format!("{}: the items changed", path.display()));
                    }
                    // And the same lights, the skipped ones last
                    converted_map.apply_lights(&mut saved);
                    let sprites = |lights: &Option<Ja2Lights>| {
                        let mut sprites = lights
                            .as_ref()
                            .map(|lights| lights.sprites.clone())
                            .unwrap_or_default();
                        sprites.sort_by_key(|s| (s.x, s.y, s.flags, s.template_name.clone()));
                        sprites
                    };
                    let colors = |lights: &Option<Ja2Lights>| {
                        lights
                            .as_ref()
                            .and_then(|lights| lights.colors.first().copied())
                    };
                    if sprites(&saved.lights) != sprites(&map.lights)
                        || (map.lights.is_some() && colors(&saved.lights) != colors(&map.lights))
                        || saved.ambient_light != map.ambient_light
                    {
                        failures.push(format!("{}: the lights changed", path.display()));
                    }
                }
                Err(e) => failures.push(format!("{}: {}", path.display(), e)),
            }
        }
        assert!(
            failures.is_empty(),
            "failed to convert maps:\n{}",
            failures.join("\n")
        );
        assert!(converted > 0, "no maps found in {}", dir);
    }
}
