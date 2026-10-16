//! World items: placing, erasing, selecting, moving and deleting them with the items mode (the
//! Items tab of the original editor, `EditorItems.cc`)
//!
//! The `place` tool places the chosen item, trigger, action item or key with a left click, with
//! the defaults of the original (`AddSelectedItemToWorld`). With `erase` its brush removes the
//! items shown on the tiles instead. The `select` tool selects the clicked tile with all items
//! shown on it (its pool, `ITEM_POOL`). Only the items on the floor of the render options are
//! edited, on the roof floor items are placed on flat roofs. Dragging the selected tile moves its items, dragging it
//! while Ctrl (Cmd on macOS) was held on the press copies them. Delete removes all items of the
//! selected tile and Esc deselects it. Every change can be undone. The host edits the items of
//! the selected tile with `set_item` and removes single ones with `item_command`, see
//! `item_events.rs`.
//!
//! Unlike the original, selecting an item doesn't change whether it is hidden, the status of a
//! new item doesn't overwrite the amount of money or the values of bombs, keys and ownership, new
//! items always exist (there is no default exist chance) and items that sink can't be placed in
//! water.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    item::{
        ItemId, ItemKind, ItemTable, IC_AMMO, IC_ARMOUR, IC_BLADE, IC_BOMB, IC_FACE, IC_GRENADE,
        IC_GUN, IC_KEY, IC_KIT, IC_LAUNCHER, IC_MEDKIT, IC_MISC, IC_MONEY, IC_PUNCH,
        IC_THROWING_KNIFE, IC_THROWN,
    },
    map::{
        item_data::{BombData, GunData, KeyData, MoneyData, OwnerData, BOMB_PRESSURE, BOMB_REMOTE},
        item_graphic, GridNo, ItemHeight, ItemStack, Level, MapItem, TacticalMap, Visibility,
        WorldItemFlags, WorldItemId,
    },
    render::{
        items::item_placement,
        world::{MapWorld, NodeFlags},
        RenderOptions,
    },
    systems::{cursor::CursorInfo, pixel_view::PixelView},
    tileset::{Terrain, TilesetId},
};

use super::{
    brush::{Brush, BrushArea},
    commands::CurrentMode,
    document::EditContext,
    mode::EditorMode,
    soldier_events::tile_center,
    soldiers::{edit_once, has_level, NO_FLAT_ROOF},
    MapEdit,
};

/// The trigger slots of the editor: pairs of a panic trigger and its action, then pairs of a
/// trigger and its action, the last one is the pressure action (`NUMBER_TRIGGERS`)
pub const NUMBER_TRIGGERS: u8 = 27;
/// The slot of the pressure action (`PRESSURE_ACTION_ID`)
pub const PRESSURE_ACTION_SLOT: u8 = NUMBER_TRIGGERS - 1;
/// Frequencies of the panic triggers (`PANIC_FREQUENCY` and so on), from 125 the trigger can be
/// an alarm trigger
pub const PANIC_FREQUENCY_3: i8 = 125;
/// Frequency of the first trigger the map places (`FIRST_MAP_PLACED_FREQUENCY`)
const FIRST_MAP_PLACED_FREQUENCY: i8 = 50;

/// Profile of items that nobody owns (`NO_PROFILE`)
pub const NO_PROFILE: u8 = 200;
/// Flags of item stacks (`OBJECT_UNDROPPABLE`, `OBJECT_ARMED_BOMB`)
const OBJECT_UNDROPPABLE: i8 = 0x01;
const OBJECT_ARMED_BOMB: i8 = 0x08;
/// The side that placed the bombs of the map (`ubBombOwner`)
const MAP_BOMB_OWNER: u8 = 1;
/// The most money of a stack (`MAX_MONEY_PER_SLOT`)
pub const MAX_MONEY: u32 = 20000;
/// The highest trap level
pub const MAX_TRAP_LEVEL: i8 = 20;
/// The highest tolerance of a panic trigger
const MAX_TOLERANCE: u8 = 99;
/// Number of civilian groups an ownership item can name (`NUM_CIV_GROUPS`)
const NUM_CIV_GROUPS: u8 = super::soldiers::NUM_CIV_GROUPS;

/// What an action item does (`ItemActionType`), `ACTION_ITEM_BLOW_UP` explodes its bomb item
const ACTION_ITEM_BLOW_UP: u8 = 3;
const ACTION_ITEM_SMALL_PIT: u8 = 4;
const ACTION_ITEM_LARGE_PIT: u8 = 5;

/// The actions the editor offers (`ACTIONITEM_*`, `ChangeActionItem`): the item that explodes, or
/// the action of the action item
pub const ACTION_ITEMS: [(Option<u16>, u8); 34] = [
    // Klaxon mine, flare mine, teargas, stun, smoke, mustard gas, land mine
    (Some(144), ACTION_ITEM_BLOW_UP),
    (Some(143), ACTION_ITEM_BLOW_UP),
    (Some(132), ACTION_ITEM_BLOW_UP),
    (Some(131), ACTION_ITEM_BLOW_UP),
    (Some(151), ACTION_ITEM_BLOW_UP),
    (Some(133), ACTION_ITEM_BLOW_UP),
    (Some(141), ACTION_ITEM_BLOW_UP),
    // Open door, close door, 3x3 pit, 5x5 pit
    (None, 1),
    (None, 2),
    (None, ACTION_ITEM_SMALL_PIT),
    (None, ACTION_ITEM_LARGE_PIT),
    // Small (hand grenade), medium (TNT) and large (C4) explosion
    (Some(135), ACTION_ITEM_BLOW_UP),
    (Some(137), ACTION_ITEM_BLOW_UP),
    (Some(142), ACTION_ITEM_BLOW_UP),
    // Toggle door, toggle actions 1 to 4, enter and exit brothel, Kingpin alarm, sex, reveal
    // room, local and global alarm, klaxon, unlock door, toggle lock, untrap door, toggle
    // pressure items, museum alarm, bloodcat alarm
    (None, 6),
    (None, 7),
    (None, 8),
    (None, 9),
    (None, 10),
    (None, 11),
    (None, 12),
    (None, 13),
    (None, 14),
    (None, 15),
    (None, 16),
    (None, 17),
    (None, 18),
    (None, 19),
    (None, 20),
    (None, 21),
    (None, 22),
    (None, 23),
    (None, 24),
    // Big teargas
    (Some(157), ACTION_ITEM_BLOW_UP),
];
/// The action of new action items in the original (`ACTIONITEM_MEDIUM`)
const DEFAULT_ACTION: u8 = 12;

/// What a click places
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemChoice {
    /// An item of the weapons, ammo, armour, explosives or equipment
    Item { item: u16 },
    /// A slot of the triggers: even slots are triggers, odd ones and the last one actions
    Trigger { slot: u8 },
    /// A key of a lock of `Locks.bin`, `item` is the key item of the lock
    Key { lock_id: u8, item: u16 },
}

/// What a click in the items mode does
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemTool {
    /// Selects the items of a tile, moves, copies and deletes them
    #[default]
    Select,
    /// Places items, or erases them with the brush
    Place,
}

/// The items mode (`TASK_ITEMS` of the original editor)
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ItemMode {
    pub tool: ItemTool,
    /// Removes the items of the brush instead of placing them
    pub erase: bool,
    /// The brush that erases items
    pub brush: Brush,
    pub choice: Option<ItemChoice>,
    /// Trap level of new action items (`gbDefaultBombTrapLevel`)
    pub trap_level: i8,
    /// What new action items do, an index of [`ACTION_ITEMS`] (`gbActionItemIndex`)
    pub action: u8,
}

impl Default for ItemMode {
    fn default() -> Self {
        Self {
            tool: ItemTool::Select,
            erase: false,
            brush: Brush::default(),
            choice: None,
            trap_level: 9,
            action: DEFAULT_ACTION,
        }
    }
}

impl ItemMode {
    /// The brush that erases items, `None` while placing or selecting, which use clicks
    pub fn brush(&self) -> Option<Brush> {
        match (self.tool, self.erase, self.brush) {
            (ItemTool::Place, true, Brush::Fill) => Some(Brush::default()),
            (ItemTool::Place, true, brush) => Some(brush),
            _ => None,
        }
    }

    /// Erases the items on the floor shown on the tiles of the brush
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea, floor: Level) {
        if self.brush().is_some() {
            context.erase_items(area, floor);
        }
    }
}

/// The frequency of the triggers and actions of a slot (`CalcItemFrequency`)
pub fn trigger_frequency(slot: u8) -> i8 {
    match slot {
        0..=1 => 127,
        2..=3 => 126,
        4..=5 => PANIC_FREQUENCY_3,
        _ => FIRST_MAP_PLACED_FREQUENCY + ((slot - 4) / 2) as i8,
    }
}

/// Whether the editor offers items of a class (`InitEditorItemsInfo`)
pub fn is_editable_class(class: u32) -> bool {
    [
        IC_GUN,
        IC_BLADE,
        IC_THROWING_KNIFE,
        IC_LAUNCHER,
        IC_THROWN,
        IC_PUNCH,
        IC_AMMO,
        IC_ARMOUR,
        IC_GRENADE,
        IC_BOMB,
        IC_MEDKIT,
        IC_KIT,
        IC_FACE,
        IC_MISC,
        IC_MONEY,
        IC_KEY,
    ]
    .contains(&class)
}

/// Whether the land of a tile is water, which sinking items are lost in (`Water`)
pub fn is_water(world: &MapWorld, tileset: TilesetId, grid_no: GridNo) -> bool {
    let tile = &world.tiles[grid_no];
    let object = tile.objects.first();
    if object.is_some_and(|node| tileset.is_pool_without_terrain(node.tile.id)) {
        return true;
    }
    let terrain = object
        .map(|node| node.tile.terrain)
        .filter(|&terrain| terrain != Terrain::NoTerrain)
        .or_else(|| tile.land.first().map(|node| node.tile.terrain));
    matches!(
        terrain,
        Some(Terrain::LowWater | Terrain::MedWater | Terrain::DeepWater)
    )
}

/// Sets what an action item does (`ChangeActionItem`)
pub fn change_action_item(data: &mut [u8; 12], action: u8) {
    let (bomb_item, action_value) = ACTION_ITEMS
        .get(usize::from(action))
        .copied()
        .unwrap_or(ACTION_ITEMS[usize::from(DEFAULT_ACTION)]);
    let mut bomb = BombData::read(data);
    bomb.bomb_item = bomb_item.unwrap_or(0);
    bomb.action_value = action_value;
    bomb.write(data);
}

/// The pit an action item digs, its size in tiles
pub fn pit_size(stack: &ItemStack) -> Option<u8> {
    if stack.item != ItemId::ACTION_ITEM {
        return None;
    }
    match BombData::read(&stack.data).action_value {
        ACTION_ITEM_SMALL_PIT => Some(3),
        ACTION_ITEM_LARGE_PIT => Some(5),
        _ => None,
    }
}

/// An empty stack of an item
fn empty_stack(item: ItemId) -> ItemStack {
    ItemStack {
        item,
        number: 1,
        data: [0; 12],
        attachments: vec![],
        flags: 0,
        mission: 0,
        trap: 0,
        imprint_id: 0,
        weight: 0,
        used: 0,
    }
}

/// A new item stack like the original creates it with the status 100 (`CreateItem`): guns are
/// loaded with their default magazine, magazines are full
pub fn create_item(items: &ItemTable, item: ItemId, kind: &ItemKind) -> ItemStack {
    let mut stack = empty_stack(item);
    match kind.class {
        IC_GUN => {
            stack.imprint_id = NO_PROFILE;
            let mut gun = GunData {
                status: 100,
                ammo_status: 100,
                ..default()
            };
            if item == ItemId::ROCKET_LAUNCHER {
                gun.shots_left = 1;
            } else if item == ItemId::TANK_CANNON {
                gun.shots_left = 0;
            } else if let Some(magazine) = items.default_magazine(item) {
                let mag = items.info(magazine);
                gun.ammo_item = magazine.index();
                gun.ammo_type = mag.map_or(0, |mag| mag.ammo_type);
                gun.shots_left = mag.map_or(kind.mag_size, |mag| mag.capacity);
            } else {
                // Monster spit and guns without magazines
                gun.shots_left = kind.mag_size;
            }
            gun.write(&mut stack.data);
        }
        IC_AMMO => stack.data[0] = kind.capacity,
        IC_MONEY => MoneyData {
            status: 100,
            amount: 100 * 50,
        }
        .write(&mut stack.data),
        _ => stack.data[0] = 100,
    }
    if kind.default_undroppable {
        stack.flags |= OBJECT_UNDROPPABLE;
    }
    stack
}

/// The index of an item in `TacticalMap::items`
pub fn item_index(map: &TacticalMap, id: WorldItemId) -> Option<usize> {
    map.items.iter().position(|item| item.id == id)
}

/// The item with an id
pub fn world_item(map: &TacticalMap, id: WorldItemId) -> Option<&MapItem> {
    map.items.iter().find(|item| item.id == id)
}

/// Whether an item is what a choice of the palette places, for counting them
/// (`CountNumberOfEditorPlacementsInWorld`)
pub fn matches_choice(stack: &ItemStack, kind: Option<&ItemKind>, choice: ItemChoice) -> bool {
    match choice {
        ItemChoice::Item { item } => stack.item.index() == item,
        ItemChoice::Trigger { slot } => {
            let bomb = BombData::read(&stack.data);
            if slot >= PRESSURE_ACTION_SLOT {
                stack.item == ItemId::ACTION_ITEM && bomb.detonator == BOMB_PRESSURE
            } else {
                let item = if slot % 2 == 0 {
                    ItemId::SWITCH
                } else {
                    ItemId::ACTION_ITEM
                };
                stack.item == item && bomb.frequency == trigger_frequency(slot)
            }
        }
        ItemChoice::Key { lock_id, .. } => {
            kind.is_some_and(|kind| kind.class == IC_KEY)
                && KeyData::read(&stack.data).key_id == lock_id
        }
    }
}

/// The tiles the items of a level are shown on: their own tile or the base tile of the structure
/// they lie on, by their index in `TacticalMap::items`
///
/// Items of the same tile form a pool, in the order of the map (`ITEM_POOL`). The items of the
/// other level have no tile.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemPools {
    level: Level,
    tiles: Vec<Option<GridNo>>,
}

impl ItemPools {
    /// The pools of the items on a level of the map, after the world was synced with them
    pub fn new(map: &TacticalMap, world: &MapWorld, level: Level) -> Self {
        let mut tiles: Vec<Option<GridNo>> = map
            .items
            .iter()
            .map(|item| (item.level == level).then_some(item.grid_no))
            .collect();
        for (grid_no, tile) in world.tiles.iter() {
            for node in tile.structs.iter().chain(&tile.on_roofs) {
                if let (Some(index), true) = (node.item, node.flags.contains(NodeFlags::ITEM)) {
                    if let Some(Some(shown)) = tiles.get_mut(usize::from(index)) {
                        *shown = grid_no;
                    }
                }
            }
        }
        Self { level, tiles }
    }

    /// The level of the items
    pub fn level(&self) -> Level {
        self.level
    }

    /// The tile an item is shown on, by its index, `None` for the items of the other level
    pub fn tile(&self, index: usize) -> Option<GridNo> {
        self.tiles.get(index).copied().flatten()
    }

    /// The indices of the items shown on a tile
    pub fn pool(&self, grid_no: GridNo) -> Vec<usize> {
        (0..self.tiles.len())
            .filter(|&index| self.tiles[index] == Some(grid_no))
            .collect()
    }

    /// The tiles with items, in the order of their grid numbers (`BuildItemPoolList`)
    pub fn pool_tiles(&self) -> Vec<GridNo> {
        let mut tiles: Vec<GridNo> = self.tiles.iter().copied().flatten().collect();
        tiles.sort();
        tiles.dedup();
        tiles
    }
}

impl EditContext<'_> {
    /// Places what the mode chose on a level of a tile, on the roof only on a flat roof, returns
    /// the new item (`AddSelectedItemToWorld`)
    pub fn place_item(
        &mut self,
        items: &ItemTable,
        mode: &ItemMode,
        grid_no: GridNo,
        level: Level,
    ) -> Result<WorldItemId, String> {
        let Some(choice) = mode.choice else {
            return Err("Choose an item to place first".to_owned());
        };
        if !has_level(self.world, grid_no, level) {
            return Err(NO_FLAT_ROOF.to_owned());
        }
        let mut visibility = Visibility::Invisible;
        let mut flags = WorldItemFlags::empty();
        let (stack, kind) = match choice {
            ItemChoice::Item { item } => {
                let id = ItemId::new(item).ok_or("Nothing can't be placed")?;
                let kind = items.info(id).ok_or("The item doesn't exist")?;
                if !is_editable_class(kind.class)
                    || kind.not_editor
                    || id == ItemId::SWITCH
                    || id == ItemId::ACTION_ITEM
                {
                    return Err("The editor can't place this item".to_owned());
                }
                let mut stack = create_item(items, id, kind);
                match kind.class {
                    IC_MONEY => {
                        let amount = 100 + self.random(19901);
                        MoneyData {
                            status: 100,
                            amount,
                        }
                        .write(&mut stack.data);
                    }
                    _ if id == ItemId::OWNERSHIP => {
                        OwnerData {
                            profile: NO_PROFILE,
                            civ_group: 0,
                        }
                        .write(&mut stack.data);
                        visibility = Visibility::Buried;
                    }
                    _ => self.randomize_status(&mut stack, kind),
                }
                (stack, kind)
            }
            ItemChoice::Trigger { slot } => {
                let slot = slot.min(PRESSURE_ACTION_SLOT);
                let is_trigger = slot < PRESSURE_ACTION_SLOT && slot % 2 == 0;
                let id = if is_trigger {
                    ItemId::SWITCH
                } else {
                    ItemId::ACTION_ITEM
                };
                let kind = items.info(id).ok_or("The item doesn't exist")?;
                let mut stack = empty_stack(id);
                let mut bomb = BombData {
                    status: 100,
                    owner: MAP_BOMB_OWNER,
                    ..default()
                };
                if is_trigger {
                    // Restricted to one trigger per tile
                    let taken = self.map.items.iter().any(|item| {
                        item.grid_no == grid_no
                            && item.level == level
                            && item.stack.item == ItemId::SWITCH
                    });
                    if taken {
                        return Err("There already is a trigger on this tile".to_owned());
                    }
                    bomb.frequency = trigger_frequency(slot);
                } else {
                    stack.trap = mode.trap_level.clamp(0, MAX_TRAP_LEVEL);
                    if slot < PRESSURE_ACTION_SLOT {
                        bomb.detonator = BOMB_REMOTE;
                        bomb.frequency = trigger_frequency(slot);
                    } else {
                        bomb.detonator = BOMB_PRESSURE;
                    }
                    stack.flags |= OBJECT_ARMED_BOMB;
                }
                bomb.write(&mut stack.data);
                if !is_trigger {
                    change_action_item(&mut stack.data, mode.action);
                }
                visibility = Visibility::Buried;
                flags |= WorldItemFlags::ARMED_BOMB;
                (stack, kind)
            }
            ItemChoice::Key { lock_id, item } => {
                let id = ItemId::new(item).ok_or("The lock has no key")?;
                let kind = items.info(id).ok_or("The key doesn't exist")?;
                let mut stack = create_item(items, id, kind);
                KeyData {
                    status: 100,
                    key_id: lock_id,
                }
                .write(&mut stack.data);
                self.randomize_status(&mut stack, kind);
                (stack, kind)
            }
        };
        if kind.sinks && is_water(self.world, self.map.tileset, grid_no) {
            return Err("The item would sink in the water".to_owned());
        }
        let id = self.document.new_item_id();
        let graphic = item_graphic(self.tileset, items, stack.item);
        self.modify_items().push(MapItem {
            id,
            grid_no,
            file_grid_no: None,
            level,
            stack,
            file_object: None,
            graphic,
            render_height: ItemHeight::Above(0),
            visibility,
            flags,
            non_exist_chance: 0,
        });
        Ok(id)
    }

    /// The random status of a new item, 70 to 95, only in the status of its class: magazines get
    /// a random number of rounds and guns random rounds (`AddSelectedItemToWorld`)
    fn randomize_status(&mut self, stack: &mut ItemStack, kind: &ItemKind) {
        if kind.class == IC_AMMO {
            let capacity = u32::from(kind.capacity);
            stack.data[0] = if self.random(2) == 1 {
                kind.capacity
            } else {
                self.random(capacity.max(1)) as u8
            };
            return;
        }
        stack.data[0] = (70 + self.random(26)) as u8;
        if kind.class == IC_GUN {
            let mut gun = GunData::read(&stack.data);
            gun.shots_left = if stack.item == ItemId::ROCKET_LAUNCHER {
                1
            } else {
                self.random(u32::from(kind.mag_size).max(1)) as u8
            };
            gun.write(&mut stack.data);
        }
    }

    /// Removes an item (`DeleteSelectedItem`)
    pub fn delete_item(&mut self, id: WorldItemId) {
        if let Some(index) = item_index(self.map, id) {
            self.modify_items().remove(index);
        }
    }

    /// Removes the items on a level shown on the tiles of the brush, only recorded if there are
    /// any
    pub fn erase_items(&mut self, area: &BrushArea, level: Level) {
        let pools = ItemPools::new(self.map, self.world, level);
        let erased: Vec<WorldItemId> = (0..self.map.items.len())
            .filter(|&index| pools.tile(index).is_some_and(|t| area.tiles.contains(&t)))
            .map(|index| self.map.items[index].id)
            .collect();
        if erased.is_empty() {
            return;
        }
        self.modify_items()
            .retain(|item| !erased.contains(&item.id));
    }

    /// Removes the items shown on a tile, only recorded if there are any (`DeleteSelectedItem`)
    pub fn delete_pool(&mut self, pools: &ItemPools, grid_no: GridNo) {
        let deleted: Vec<WorldItemId> = pools
            .pool(grid_no)
            .into_iter()
            .filter_map(|index| self.map.items.get(index).map(|item| item.id))
            .collect();
        if deleted.is_empty() {
            return;
        }
        self.modify_items()
            .retain(|item| !deleted.contains(&item.id));
    }

    /// Moves the items on a level shown on a tile to another tile, or copies them there with new
    /// ids, they keep their level. Returns the tile the first of them is shown on afterwards,
    /// `None` if there is nothing to move.
    ///
    /// Nothing is moved if an item would sink in water, the tile would get a second trigger or
    /// the items are on the roof and the tile has no flat roof.
    pub fn move_items(
        &mut self,
        items: &ItemTable,
        level: Level,
        from: GridNo,
        to: GridNo,
        copy: bool,
    ) -> Result<Option<GridNo>, String> {
        let pool = ItemPools::new(self.map, self.world, level).pool(from);
        if from == to || pool.is_empty() {
            return Ok(None);
        }
        if !has_level(self.world, to, level) {
            return Err(NO_FLAT_ROOF.to_owned());
        }
        let water = is_water(self.world, self.map.tileset, to);
        let switch = |item: &MapItem| item.level == level && item.stack.item == ItemId::SWITCH;
        let mut switches = 0;
        for (index, item) in self.map.items.iter().enumerate() {
            let moved = pool.contains(&index);
            if moved && water && items.info(item.stack.item).is_some_and(|kind| kind.sinks) {
                return Err("The items would sink in the water".to_owned());
            }
            if switch(item) && (moved || item.grid_no == to) {
                switches += 1;
            }
        }
        if switches > 1 {
            return Err("There already is a trigger on this tile".to_owned());
        }
        let mut moved = Vec::with_capacity(pool.len());
        for &index in &pool {
            let mut item = self.map.items[index].clone();
            item.grid_no = to;
            // The tile and the height of the map file were where the item was
            item.file_grid_no = None;
            if let ItemHeight::Above(_) = item.render_height {
                item.render_height = ItemHeight::Above(0);
            }
            if copy {
                item.id = self.document.new_item_id();
            }
            moved.push((index, item));
        }
        let shown = match &moved[0].1 {
            item if item.graphic.is_some() => item_placement(self.world, item).grid_no,
            item => item.grid_no,
        };
        let map_items = self.modify_items();
        for (index, item) in moved {
            if copy {
                map_items.push(item);
            } else {
                map_items[index] = item;
            }
        }
        Ok(Some(shown))
    }

    /// Changes an item, it is recorded for undo if the change changes it
    pub fn change_world_item(
        &mut self,
        id: WorldItemId,
        change: impl FnOnce(&mut MapItem),
    ) -> bool {
        let Some(index) = item_index(self.map, id) else {
            return false;
        };
        let mut changed = self.map.items[index].clone();
        change(&mut changed);
        if changed == self.map.items[index] {
            return false;
        }
        self.modify_items()[index] = changed;
        true
    }

    /// Sets the values the host edited, clamped like the original does
    /// (`ExtractAndUpdate...GUI` of `Item_Statistics.cc`), ignored if they change nothing
    pub fn change_item(
        &mut self,
        items: &ItemTable,
        id: WorldItemId,
        mut stack: ItemStack,
        visibility: Visibility,
        flags: WorldItemFlags,
        non_exist_chance: u8,
    ) -> bool {
        let Some(current) = world_item(self.map, id) else {
            return false;
        };
        // The item itself can't be changed
        stack.item = current.stack.item;
        let kind = items.info(stack.item).cloned().unwrap_or_default();
        if stack.number != current.stack.number {
            stack.number = stack.number.clamp(1, kind.per_pocket.max(1));
        }
        stack.trap = stack.trap.clamp(0, MAX_TRAP_LEVEL);
        stack.attachments.truncate(4);
        for (_, status) in &mut stack.attachments {
            *status = (*status).clamp(1, 100);
        }
        let clamp_status = |data: &mut [u8; 12]| data[0] = (data[0] as i8).clamp(0, 100) as u8;
        let mut non_exist_chance = non_exist_chance.min(99);
        if stack.item == ItemId::OWNERSHIP {
            let mut owner = OwnerData::read(&stack.data);
            owner.civ_group = owner.civ_group.min(NUM_CIV_GROUPS - 1);
            owner.write(&mut stack.data);
            non_exist_chance = 0;
        } else if stack.item == ItemId::SWITCH {
            let mut bomb = BombData::read(&stack.data);
            bomb.tolerance = bomb.tolerance.min(MAX_TOLERANCE);
            bomb.write(&mut stack.data);
        } else if stack.item == ItemId::ACTION_ITEM {
            clamp_status(&mut stack.data);
        } else {
            match kind.class {
                IC_MONEY => {
                    let mut money = MoneyData::read(&stack.data);
                    money.amount = money.amount.clamp(1, MAX_MONEY);
                    money.status = 100;
                    money.write(&mut stack.data);
                }
                IC_GUN => {
                    clamp_status(&mut stack.data);
                    let mut gun = GunData::read(&stack.data);
                    if kind.mag_size > 0 {
                        gun.shots_left = gun.shots_left.min(kind.mag_size);
                    }
                    gun.write(&mut stack.data);
                }
                IC_AMMO => {}
                _ => clamp_status(&mut stack.data),
            }
        }
        self.change_world_item(id, |item| {
            item.stack = stack;
            item.visibility = visibility;
            item.flags = flags;
            item.non_exist_chance = non_exist_chance;
        })
    }
}

/// A drag of the selected tile in progress
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ItemDrag {
    /// Where the mouse was pressed
    from: GridNo,
    /// Whether Ctrl or Cmd was held on the press, which copies the items instead of moving them
    copy: bool,
    /// The level of the items
    level: Level,
}

/// The selected tile, whose items the host edits, and what the host is told after a failed edit
#[derive(Resource, Clone, Debug, Default)]
pub struct ItemSelection {
    /// The tile the selected items are shown on
    pub selected: Option<GridNo>,
    pub notice: Option<String>,
    drag: Option<ItemDrag>,
}

impl ItemSelection {
    /// Drops the selection when the tile has no items any more, e.g. after undo
    pub fn validate(&mut self, pools: &ItemPools) {
        if self
            .selected
            .is_some_and(|tile| pools.pool(tile).is_empty())
        {
            self.clear();
        }
    }

    /// Drops the selection and the drag
    pub fn clear(&mut self) {
        self.selected = None;
        self.drag = None;
    }

    /// Whether the selected items are being dragged
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Selects a tile with items and starts dragging them, or clears the selection if the tile
    /// has none
    pub fn press(&mut self, pools: &ItemPools, tile: GridNo, copy: bool) {
        if pools.pool(tile).is_empty() {
            self.clear();
            return;
        }
        self.selected = Some(tile);
        self.drag = Some(ItemDrag {
            from: tile,
            copy,
            level: pools.level(),
        });
    }

    /// Ends the drag at a tile, moving or copying the items there and selecting the tile they
    /// are shown on
    pub fn release(&mut self, context: &mut EditContext, items: &ItemTable, to: GridNo) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        match context.move_items(items, drag.level, drag.from, to, drag.copy) {
            Ok(Some(shown)) => self.selected = Some(shown),
            Ok(None) => {}
            Err(notice) => self.notice = Some(notice),
        }
    }

    /// Removes all items of the selected tile (`DeleteSelectedItem`)
    pub fn delete(&mut self, context: &mut EditContext, pools: &ItemPools) {
        if let Some(selected) = self.selected {
            context.delete_pool(pools, selected);
        }
        self.clear();
    }
}

pub fn reset_item_selection(mut selection: ResMut<ItemSelection>) {
    *selection = ItemSelection::default();
}

/// The items mode, `None` for the others
pub fn item_mode(mode: &CurrentMode) -> Option<&ItemMode> {
    match &mode.0 {
        EditorMode::Items(mode) => Some(mode),
        _ => None,
    }
}

/// Centers the view on a tile
pub fn center_view(view: Option<&mut PixelView>, map: &TacticalMap, grid_no: GridNo) {
    if let Some(view) = view {
        view.position = tile_center(map, grid_no);
    }
}

/// The mouse and the keys of the items mode, erasing is done by `paint` with the brush
///
/// The selection is checked by `report_item`, once the world caught up with the edits.
pub fn edit_items(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    items: Option<Res<ItemTable>>,
    mut selection: ResMut<ItemSelection>,
    mut edit: MapEdit,
) {
    let Some(mode) = item_mode(&mode) else {
        // Leaving the items mode deselects the tile
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        return;
    };
    let Some(items) = items else {
        return;
    };
    let selection = &mut *selection;
    let floor = options.floor;
    let tile = cursor.tile.map(GridNo::from);
    if mode.tool != ItemTool::Select {
        // Only the select tool has a selection
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        let (Some(tile), false) = (tile, mode.erase) else {
            return;
        };
        if mouse.just_pressed(MouseButton::Left) {
            let mut result = Ok(());
            edit_once(&mut edit, |context| {
                result = context.place_item(&items, mode, tile, floor).map(|_| ())
            });
            if let Err(notice) = result {
                selection.notice = Some(notice);
            }
        }
        return;
    }
    if let Some(tile) = tile.filter(|_| mouse.just_pressed(MouseButton::Left)) {
        // Cmd on macOS, where Ctrl+click is a right click
        let copy = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
        selection.press(&ItemPools::new(&edit.map, &edit.world, floor), tile, copy);
    }
    if selection.drag.is_some() {
        if cursor.world_position.is_none() {
            // Leaving the viewer cancels the drag, like the area brushes
            selection.drag = None;
        } else if !mouse.pressed(MouseButton::Left) {
            match tile {
                Some(to) => edit_once(&mut edit, |context| selection.release(context, &items, to)),
                None => selection.drag = None,
            }
        }
    }
    if selection.drag.is_none() && keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace]) {
        let pools = ItemPools::new(&edit.map, &edit.world, floor);
        edit_once(&mut edit, |context| selection.delete(context, &pools));
    }
    if keys.just_pressed(KeyCode::Escape) {
        selection.clear();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        file_formats::ja2_tile_types::FIRSTTEXTURE,
        item::ItemFile,
        map::item_data::BOMB_REMOTE,
        tileset::tests::structure_tileset,
    };

    /// A map to place items on, with a few items of every kind
    pub(crate) struct ItemFixture {
        pub fixture: Fixture,
        pub items: ItemTable,
    }

    pub(crate) fn id(index: u16) -> ItemId {
        ItemId::new(index).unwrap()
    }

    impl ItemFixture {
        pub(crate) fn new() -> Self {
            let weapons = br#"[
                { "itemIndex": 2, "internalType": "M_PISTOL", "calibre": "AMMO9", "ubMagSize": 15,
                  "ubPerPocket": 1, "bSinks": true, "tileGraphic": { "type": "GUNS", "subIndex": 3 } },
                { "itemIndex": 51, "internalType": "LAW", "ubMagSize": 1 },
                { "itemIndex": 43, "internalType": "BLADE", "bNotEditor": true }
            ]"#;
            let magazines = br#"[
                { "itemIndex": 71, "calibre": "AMMO9", "capacity": 15, "ammoType": "AMMO_HP",
                  "ubPerPocket": 8 }
            ]"#;
            let explosives = br#"[
                { "itemIndex": 135, "itemClass": "GRENADE", "ubPerPocket": 4 },
                { "itemIndex": 137, "itemClass": "BOMB", "ubPerPocket": 4 }
            ]"#;
            let items = br#"[
                { "itemIndex": 202, "usItemClass": 4096, "bDefaultUndroppable": true },
                { "itemIndex": 219, "usItemClass": 536870912, "ubPerPocket": 1 },
                { "itemIndex": 233, "usItemClass": 268435456, "ubPerPocket": 99 },
                { "itemIndex": 234, "usItemClass": 268435456, "ubPerPocket": 99 },
                { "itemIndex": 240, "usItemClass": 268435456 },
                { "itemIndex": 271, "usItemClass": 65536, "ubClassIndex": 0, "ubPerPocket": 8 },
                { "itemIndex": 300, "usItemClass": 1 }
            ]"#;
            let items = ItemTable::new(&[
                ItemFile::parse(weapons, "weapons.json").unwrap(),
                ItemFile::parse(magazines, "magazines.json").unwrap(),
                ItemFile::parse(explosives, "explosives.json").unwrap(),
                ItemFile::parse(items, "items.json").unwrap(),
            ]);
            Self {
                fixture: Fixture::with_tileset(structure_tileset()),
                items,
            }
        }

        /// Edits the map as one undo command, the world is synced with the items afterwards
        pub(crate) fn edit<R>(
            &mut self,
            edit: impl FnOnce(&mut EditContext, &ItemTable) -> R,
        ) -> R {
            self.fixture.document.undo.begin();
            let mut context = self.fixture.context();
            let result = edit(&mut context, &self.items);
            context.finish();
            self.fixture.document.undo.end();
            self.sync();
            result
        }

        pub(crate) fn sync(&mut self) {
            if std::mem::take(&mut self.fixture.document.items_dirty) {
                self.fixture.world.sync_items(&self.fixture.map);
            }
        }

        pub(crate) fn place(
            &mut self,
            mode: &ItemMode,
            grid_no: GridNo,
        ) -> Result<MapItem, String> {
            self.place_on(mode, grid_no, Level::Ground)
        }

        pub(crate) fn place_on(
            &mut self,
            mode: &ItemMode,
            grid_no: GridNo,
            level: Level,
        ) -> Result<MapItem, String> {
            let id = self.edit(|context, items| context.place_item(items, mode, grid_no, level))?;
            Ok(world_item(&self.fixture.map, id).unwrap().clone())
        }

        /// The pools of the ground
        pub(crate) fn pools(&self) -> ItemPools {
            self.pools_on(Level::Ground)
        }

        pub(crate) fn pools_on(&self, level: Level) -> ItemPools {
            ItemPools::new(&self.fixture.map, &self.fixture.world, level)
        }

        pub(crate) fn undo(&mut self) {
            let command = self.fixture.document.undo.pop().unwrap();
            let mut context = self.fixture.context();
            context.restore(command);
            context.finish();
            self.sync();
        }
    }

    pub(crate) fn mode(json: &str) -> ItemMode {
        match serde_json::from_str(json).unwrap() {
            EditorMode::Items(mode) => mode,
            _ => panic!("not the items mode"),
        }
    }

    fn choose(choice: ItemChoice) -> ItemMode {
        ItemMode {
            choice: Some(choice),
            ..default()
        }
    }

    fn item(item: u16) -> ItemMode {
        choose(ItemChoice::Item { item })
    }

    #[test]
    fn modes_are_read_from_json() {
        let read = mode(r#"{"mode":"items","choice":{"kind":"key","lockId":4,"item":271}}"#);
        assert_eq!(
            read.choice,
            Some(ItemChoice::Key {
                lock_id: 4,
                item: 271
            })
        );
        // Missing values are the defaults of the original
        assert_eq!((read.trap_level, read.action), (9, 12));
        let read = mode(r#"{"mode":"items","choice":{"kind":"trigger","slot":26}}"#);
        assert_eq!(read.choice, Some(ItemChoice::Trigger { slot: 26 }));
        assert_eq!(mode(r#"{"mode":"items"}"#), ItemMode::default());

        let erasing = mode(
            r#"{"mode":"items","tool":"place","erase":true,
                "brush":{"kind":"circle","size":5}}"#,
        );
        assert_eq!(erasing.brush(), Some(Brush::Circle { size: 5 }));
        let defaults = mode(r#"{"mode":"items"}"#);
        assert_eq!(
            (defaults.tool, defaults.erase, defaults.brush),
            (ItemTool::Select, false, Brush::default())
        );
        // Only erasing uses the brush
        assert_eq!(defaults.brush(), None);
        let select = mode(r#"{"mode":"items","tool":"select","erase":true}"#);
        assert_eq!(select.brush(), None);
        let place = mode(r#"{"mode":"items","tool":"place"}"#);
        assert_eq!(place.brush(), None);
        let fill = mode(r#"{"mode":"items","tool":"place","erase":true,"brush":{"kind":"fill"}}"#);
        assert_eq!(fill.brush(), Some(Brush::default()));
    }

    #[test]
    fn new_items_get_the_defaults_of_the_original() {
        let mut fixture = ItemFixture::new();
        let gun = fixture.place(&item(2), grid(50, 50)).unwrap();
        assert_eq!(gun.visibility, Visibility::Invisible);
        // New items always exist
        assert_eq!(gun.non_exist_chance, 0);
        assert_eq!(gun.level, Level::Ground);
        assert!(gun.graphic.is_some());
        assert_eq!(gun.stack.imprint_id, NO_PROFILE);
        let data = GunData::read(&gun.stack.data);
        assert!((70..=95).contains(&data.status));
        assert!(data.shots_left < 15);
        // Loaded with the default magazine
        assert_eq!(
            (data.ammo_item, data.ammo_type, data.ammo_status),
            (71, 1, 100)
        );
        let law = fixture.place(&item(51), grid(51, 50)).unwrap();
        assert_eq!(GunData::read(&law.stack.data).shots_left, 1);

        let ammo = fixture.place(&item(71), grid(52, 50)).unwrap();
        assert!(ammo.stack.data[0] <= 15);
        assert_eq!(ammo.stack.data[1..], [0; 11]);

        // The status doesn't overwrite the money, the owner, the bombs and the keys
        let money = fixture.place(&item(219), grid(53, 50)).unwrap();
        let amount = MoneyData::read(&money.stack.data);
        assert_eq!(amount.status, 100);
        assert!((100..=20000).contains(&amount.amount));
        let owner = fixture.place(&item(240), grid(54, 50)).unwrap();
        assert_eq!(owner.visibility, Visibility::Buried);
        assert_eq!(owner.non_exist_chance, 0);
        assert_eq!(OwnerData::read(&owner.stack.data).profile, NO_PROFILE);
        let key = fixture
            .place(
                &choose(ItemChoice::Key {
                    lock_id: 7,
                    item: 271,
                }),
                grid(55, 50),
            )
            .unwrap();
        let key_data = KeyData::read(&key.stack.data);
        assert_eq!(key_data.key_id, 7);
        assert!((70..=95).contains(&key_data.status));
        let kit = fixture.place(&item(202), grid(56, 50)).unwrap();
        assert_eq!(kit.stack.flags, OBJECT_UNDROPPABLE);

        // Triggers and actions are buried, armed bombs of the map
        let trigger = fixture
            .place(&choose(ItemChoice::Trigger { slot: 2 }), grid(57, 50))
            .unwrap();
        assert_eq!(trigger.stack.item, ItemId::SWITCH);
        assert_eq!(trigger.visibility, Visibility::Buried);
        assert_eq!(trigger.flags, WorldItemFlags::ARMED_BOMB);
        let bomb = BombData::read(&trigger.stack.data);
        assert_eq!((bomb.status, bomb.owner, bomb.frequency), (100, 1, 126));
        let mut action_mode = choose(ItemChoice::Trigger { slot: 9 });
        action_mode.trap_level = 4;
        let action = fixture.place(&action_mode, grid(58, 50)).unwrap();
        assert_eq!(action.stack.item, ItemId::ACTION_ITEM);
        assert_eq!(action.stack.trap, 4);
        assert_eq!(action.stack.flags, OBJECT_ARMED_BOMB);
        let bomb = BombData::read(&action.stack.data);
        assert_eq!(bomb.detonator, BOMB_REMOTE);
        assert_eq!(bomb.frequency, 52);
        // A medium explosion by default
        assert_eq!(
            (bomb.bomb_item, bomb.action_value),
            (137, ACTION_ITEM_BLOW_UP)
        );
        let mut pressure_mode = choose(ItemChoice::Trigger { slot: 26 });
        pressure_mode.action = 9;
        let pressure = fixture.place(&pressure_mode, grid(59, 50)).unwrap();
        let bomb = BombData::read(&pressure.stack.data);
        assert_eq!((bomb.detonator, bomb.bomb_item), (BOMB_PRESSURE, 0));
        assert_eq!(pit_size(&pressure.stack), Some(3));
        assert_eq!(fixture.fixture.document.undo.depth(), 10);
        assert_eq!(fixture.fixture.map.items.len(), 10);
    }

    #[test]
    fn some_items_can_not_be_placed() {
        let mut fixture = ItemFixture::new();
        let g = grid(50, 50);
        assert!(fixture.place(&ItemMode::default(), g).is_err());
        // Items of no class, items not for the editor and unknown items
        for index in [300, 43, 999, 233] {
            assert!(fixture.place(&item(index), g).is_err(), "{}", index);
        }
        // One trigger per tile, actions can share it
        let trigger = choose(ItemChoice::Trigger { slot: 0 });
        fixture.place(&trigger, g).unwrap();
        assert!(fixture.place(&trigger, g).is_err());
        fixture
            .place(&choose(ItemChoice::Trigger { slot: 1 }), g)
            .unwrap();

        // Items that sink are not placed in water
        let water = grid(60, 60);
        let mut tile = (**fixture
            .fixture
            .tileset
            .tile(fixture.fixture.tileset.index(FIRSTTEXTURE, 1).unwrap())
            .unwrap())
        .clone();
        tile.terrain = Terrain::LowWater;
        fixture.fixture.map.tiles[water].land = vec![Arc::new(tile)];
        fixture.fixture.world = MapWorld::build(&fixture.fixture.map);
        assert!(fixture.place(&item(2), water).unwrap_err().contains("sink"));
        fixture.place(&item(71), water).unwrap();
        assert_eq!(fixture.fixture.map.items.len(), 3);
    }

    #[test]
    fn pressing_selects_the_tile() {
        let mut fixture = ItemFixture::new();
        let (a, b) = (grid(50, 50), grid(40, 60));
        for g in [a, b, a, a] {
            fixture.place(&item(71), g).unwrap();
        }
        let pools = fixture.pools();
        assert_eq!(pools.pool(a), [0, 2, 3]);
        assert_eq!(pools.pool_tiles(), [a, b]);

        let mut selection = ItemSelection::default();
        selection.press(&pools, a, false);
        assert_eq!(selection.selected, Some(a));
        assert!(selection.dragging());
        // Releasing where it was pressed changes nothing
        let depth = fixture.fixture.document.undo.depth();
        release(&mut fixture, &mut selection, a);
        assert_eq!(selection.selected, Some(a));
        assert!(!selection.dragging());
        assert_eq!(fixture.fixture.document.undo.depth(), depth);
        selection.press(&pools, b, false);
        assert_eq!(selection.selected, Some(b));
        // Tiles without items deselect
        selection.press(&pools, grid(10, 10), false);
        assert_eq!(selection.selected, None);
        assert!(!selection.dragging());
    }

    #[test]
    fn items_on_structures_are_in_the_pool_of_the_structure() {
        let mut fixture = ItemFixture::new();
        let g = grid(50, 50);
        // A 2×2 obstacle, its base tile is the one south east
        let obstacle = fixture
            .fixture
            .context()
            .tile("OSTRUCT1", 3)
            .expect("the 2x2 obstacle");
        let base = grid(51, 51);
        fixture.edit(|context, _| context.add_struct_to_tail(base, obstacle));
        let placed = fixture.place(&item(71), g).unwrap();
        let pools = fixture.pools();
        let shown = pools.tile(0).unwrap();
        assert_eq!(placed.grid_no, g);
        let mut selection = ItemSelection::default();
        selection.press(&pools, shown, false);
        assert_eq!(selection.selected, Some(shown));

        // Moving items onto the structure selects the tile they are shown on
        let other = fixture.place(&item(2), grid(60, 60)).unwrap();
        selection.press(&fixture.pools(), grid(60, 60), false);
        release(&mut fixture, &mut selection, g);
        assert_eq!(
            world_item(&fixture.fixture.map, other.id).unwrap().grid_no,
            g
        );
        assert_eq!(selection.selected, Some(shown));
        selection.validate(&fixture.pools());
        assert_eq!(selection.selected, Some(shown));
    }

    /// Releases the drag of the selection as one undo command
    fn release(fixture: &mut ItemFixture, selection: &mut ItemSelection, to: GridNo) {
        fixture.edit(|context, items| selection.release(context, items, to));
    }

    fn ids(fixture: &ItemFixture) -> Vec<WorldItemId> {
        fixture.fixture.map.items.iter().map(|i| i.id).collect()
    }

    #[test]
    fn dragging_moves_or_copies_the_items_of_the_tile() {
        let mut fixture = ItemFixture::new();
        let (a, b, c) = (grid(50, 50), grid(55, 50), grid(60, 50));
        let gun = fixture.place(&item(2), a).unwrap();
        fixture.place(&item(71), a).unwrap();
        // Items of the map file keep the tile and the height of the file until they are moved
        fixture.fixture.map.items[1].file_grid_no = Some(-1);
        fixture.fixture.map.items[1].render_height = ItemHeight::Above(12);
        let roof = fixture.fixture.map.items[1].clone();
        let staying = fixture.place(&item(71), b).unwrap();
        let mut selection = ItemSelection::default();

        selection.press(&fixture.pools(), a, false);
        release(&mut fixture, &mut selection, b);
        assert_eq!(selection.selected, Some(b));
        assert_eq!(ids(&fixture), [gun.id, roof.id, staying.id]);
        let moved = &fixture.fixture.map.items;
        assert_eq!((moved[0].grid_no, moved[1].grid_no), (b, b));
        assert_eq!(moved[1].file_grid_no, None);
        assert_eq!(moved[1].render_height, ItemHeight::Above(0));
        assert_eq!(moved[0].stack, gun.stack);
        assert_eq!(fixture.pools().pool(b), [0, 1, 2]);

        // Copying copies the whole pool with new ids
        selection.press(&fixture.pools(), b, true);
        release(&mut fixture, &mut selection, c);
        assert_eq!(selection.selected, Some(c));
        assert_eq!(fixture.fixture.map.items.len(), 6);
        assert_eq!(fixture.pools().pool(c), [3, 4, 5]);
        assert_eq!(fixture.pools().pool(b), [0, 1, 2]);
        let copies = &fixture.fixture.map.items[3..];
        assert!(copies
            .iter()
            .all(|copy| ![gun.id, roof.id, staying.id].contains(&copy.id)));
        assert_eq!(copies[0].stack, gun.stack);

        // Each drag is undone as a whole
        fixture.undo();
        assert_eq!(ids(&fixture), [gun.id, roof.id, staying.id]);
        selection.validate(&fixture.pools());
        assert_eq!(selection.selected, None);
        fixture.undo();
        assert_eq!(fixture.fixture.map.items[0], gun);
        assert_eq!(fixture.fixture.map.items[1], roof);
    }

    #[test]
    fn items_are_edited_on_their_floor() {
        let mut fixture = ItemFixture::new();
        let (a, b, c) = (grid(50, 50), grid(55, 50), grid(60, 50));
        let tileset = &fixture.fixture.tileset;
        let roof = tileset
            .tile(
                tileset
                    .tile_of("FIRSTROOF", 1)
                    .or_else(|| tileset.tile_of("ROOF1", 1))
                    .unwrap(),
            )
            .unwrap()
            .clone();
        assert!(roof.is_flat_roof());
        fixture.edit(|context, _| {
            context.add_roof_to_head(a, roof.clone());
            context.add_roof_to_head(b, roof);
        });
        let ground = fixture.place(&item(71), a).unwrap();
        let above = fixture.place_on(&item(71), a, Level::Roof).unwrap();
        assert_eq!(above.level, Level::Roof);
        // Not on the roof without a flat roof
        assert_eq!(
            fixture.place_on(&item(71), c, Level::Roof),
            Err(NO_FLAT_ROOF.to_owned())
        );
        // A trigger per tile and level
        let trigger = choose(ItemChoice::Trigger { slot: 2 });
        fixture.place(&trigger, a).unwrap();
        fixture.place_on(&trigger, a, Level::Roof).unwrap();
        assert!(fixture.place_on(&trigger, a, Level::Roof).is_err());

        // Each floor only has its own pools
        assert_eq!(fixture.pools().pool(a), [0, 2]);
        assert_eq!(fixture.pools_on(Level::Roof).pool(a), [1, 3]);
        assert_eq!(fixture.pools_on(Level::Roof).tile(0), None);
        let mut selection = ItemSelection::default();
        selection.press(&fixture.pools_on(Level::Roof), a, false);
        assert_eq!(selection.selected, Some(a));

        // Roof items only move onto flat roofs
        release(&mut fixture, &mut selection, c);
        assert_eq!(selection.notice.take(), Some(NO_FLAT_ROOF.to_owned()));
        selection.press(&fixture.pools_on(Level::Roof), a, false);
        release(&mut fixture, &mut selection, b);
        assert_eq!(fixture.pools_on(Level::Roof).pool(b), [1, 3]);
        assert_eq!(fixture.fixture.map.items[1].level, Level::Roof);
        assert_eq!(fixture.pools().pool(a), [0, 2]);

        // Erasing only removes the items of the floor
        let erase = mode(r#"{"mode":"items","tool":"place","erase":true}"#);
        let area = erase.brush().unwrap().region(None, b.position());
        fixture.edit(|context, _| erase.apply(context, &area, Level::Ground));
        assert_eq!(fixture.fixture.map.items.len(), 4);
        fixture.edit(|context, _| erase.apply(context, &area, Level::Roof));
        let left = &fixture.fixture.map.items;
        assert_eq!(left.len(), 2);
        assert_eq!(left[0].id, ground.id);
        assert!(left.iter().all(|item| item.level == Level::Ground));
    }

    #[test]
    fn items_are_not_moved_into_water_or_onto_a_second_trigger() {
        let mut fixture = ItemFixture::new();
        let (a, b, c) = (grid(50, 50), grid(55, 50), grid(60, 50));
        let water = grid(60, 60);
        let mut tile = (**fixture
            .fixture
            .tileset
            .tile(fixture.fixture.tileset.index(FIRSTTEXTURE, 1).unwrap())
            .unwrap())
        .clone();
        tile.terrain = Terrain::LowWater;
        fixture.fixture.map.tiles[water].land = vec![Arc::new(tile)];
        fixture.fixture.world = MapWorld::build(&fixture.fixture.map);

        let trigger = choose(ItemChoice::Trigger { slot: 0 });
        fixture.place(&trigger, a).unwrap();
        fixture.place(&item(2), a).unwrap();
        fixture.place(&trigger, b).unwrap();
        fixture.place(&item(71), c).unwrap();
        let before = fixture.fixture.map.items.clone();
        let depth = fixture.fixture.document.undo.depth();

        let mut selection = ItemSelection::default();
        // The gun sinks
        selection.press(&fixture.pools(), a, false);
        release(&mut fixture, &mut selection, water);
        assert!(selection.notice.take().unwrap().contains("sink"));
        assert_eq!(selection.selected, Some(a));
        // Both tiles have a trigger, also when copying
        for copy in [false, true] {
            selection.press(&fixture.pools(), a, copy);
            release(&mut fixture, &mut selection, b);
            assert!(selection.notice.take().unwrap().contains("trigger"));
        }
        assert_eq!(fixture.fixture.map.items, before);
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        // Magazines don't sink, triggers can go where none is
        selection.press(&fixture.pools(), c, false);
        release(&mut fixture, &mut selection, water);
        assert_eq!(selection.selected, Some(water));
        selection.press(&fixture.pools(), b, false);
        release(&mut fixture, &mut selection, c);
        assert_eq!(selection.notice, None);
        assert_eq!(fixture.fixture.map.items[2].grid_no, c);
    }

    #[test]
    fn deleting_removes_the_items_of_the_tile_and_can_be_undone() {
        let mut fixture = ItemFixture::new();
        let (a, b) = (grid(50, 50), grid(40, 60));
        for g in [a, b, a] {
            fixture.place(&item(71), g).unwrap();
        }
        let placed = ids(&fixture);
        let mut selection = ItemSelection::default();
        selection.press(&fixture.pools(), a, false);
        release(&mut fixture, &mut selection, a);
        let pools = fixture.pools();
        fixture.edit(|context, _| selection.delete(context, &pools));
        assert_eq!(selection.selected, None);
        assert_eq!(ids(&fixture), [placed[1]]);
        // Nothing selected deletes nothing
        let depth = fixture.fixture.document.undo.depth();
        let pools = fixture.pools();
        fixture.edit(|context, _| selection.delete(context, &pools));
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        fixture.undo();
        assert_eq!(ids(&fixture), placed);
        assert_eq!(fixture.pools().pool(a), [0, 2]);
    }

    #[test]
    fn erasing_removes_the_items_of_the_brush() {
        let mut fixture = ItemFixture::new();
        fixture.place(&item(71), grid(50, 50)).unwrap();
        fixture.place(&item(2), grid(51, 49)).unwrap();
        let outside = fixture.place(&item(71), grid(60, 50)).unwrap();
        let erase = ItemMode {
            tool: ItemTool::Place,
            erase: true,
            brush: Brush::Square { size: 3 },
            ..default()
        };
        let area = erase.brush().unwrap().region(None, grid(50, 50).position());
        fixture.edit(|context, _| erase.apply(context, &area, Level::Ground));
        assert_eq!(ids(&fixture), [outside.id]);
        // Nothing to erase is not recorded
        let depth = fixture.fixture.document.undo.depth();
        fixture.edit(|context, _| erase.apply(context, &area, Level::Ground));
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        fixture.undo();
        assert_eq!(fixture.fixture.map.items.len(), 3);
    }

    #[test]
    fn edits_are_clamped_like_the_original() {
        let mut fixture = ItemFixture::new();
        let gun = fixture.place(&item(2), grid(50, 50)).unwrap();
        let mut stack = gun.stack.clone();
        stack.item = id(71);
        stack.trap = 40;
        stack.number = 5;
        let mut data = GunData::read(&stack.data);
        data.status = 120;
        data.shots_left = 40;
        data.write(&mut stack.data);
        let changed = fixture.edit(|context, items| {
            context.change_item(
                items,
                gun.id,
                stack.clone(),
                Visibility::HiddenItem,
                WorldItemFlags::REALISTIC_ONLY,
                150,
            )
        });
        assert!(changed);
        let edited = world_item(&fixture.fixture.map, gun.id).unwrap();
        // The item stays a gun
        assert_eq!(edited.stack.item, id(2));
        assert_eq!(edited.stack.trap, 20);
        assert_eq!(edited.stack.number, 1);
        let data = GunData::read(&edited.stack.data);
        assert_eq!((data.status, data.shots_left), (100, 15));
        assert_eq!(edited.visibility, Visibility::HiddenItem);
        assert_eq!(edited.flags, WorldItemFlags::REALISTIC_ONLY);
        assert_eq!(edited.non_exist_chance, 99);
        let depth = fixture.fixture.document.undo.depth();

        // The same values change nothing
        let same = edited.clone();
        let changed = fixture.edit(|context, items| {
            context.change_item(
                items,
                same.id,
                same.stack.clone(),
                same.visibility,
                same.flags,
                same.non_exist_chance,
            )
        });
        assert!(!changed);
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        let money = fixture.place(&item(219), grid(51, 50)).unwrap();
        let mut stack = money.stack.clone();
        MoneyData {
            status: 3,
            amount: 50000,
        }
        .write(&mut stack.data);
        fixture.edit(|context, items| {
            context.change_item(items, money.id, stack, money.visibility, money.flags, 0)
        });
        let edited = world_item(&fixture.fixture.map, money.id).unwrap();
        assert_eq!(
            MoneyData::read(&edited.stack.data),
            MoneyData {
                status: 100,
                amount: MAX_MONEY
            }
        );

        let trigger = fixture
            .place(&choose(ItemChoice::Trigger { slot: 4 }), grid(52, 50))
            .unwrap();
        let mut stack = trigger.stack.clone();
        stack.data[7] = 200;
        fixture.edit(|context, items| {
            context.change_item(
                items,
                trigger.id,
                stack,
                trigger.visibility,
                trigger.flags,
                0,
            )
        });
        let edited = world_item(&fixture.fixture.map, trigger.id).unwrap();
        assert_eq!(BombData::read(&edited.stack.data).tolerance, 99);
    }

    #[test]
    fn trigger_slots_have_the_frequencies_of_the_original() {
        let frequencies: Vec<i8> = (0..PRESSURE_ACTION_SLOT).map(trigger_frequency).collect();
        assert_eq!(frequencies[..8], [127, 127, 126, 126, 125, 125, 51, 51]);
        assert_eq!(frequencies[25], 60);
        let mut data = [0; 12];
        change_action_item(&mut data, 33);
        assert_eq!(BombData::read(&data).bomb_item, 157);
        change_action_item(&mut data, 7);
        let bomb = BombData::read(&data);
        assert_eq!((bomb.bomb_item, bomb.action_value), (0, 1));
    }
}
