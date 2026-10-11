//! What the host is told about the world items and how it edits them
//!
//! Whenever the selected tile, its items or the number of placed items change, the host gets the
//! items of the selected tile (`item`) and how many items of each choice of the palette the map
//! has. The host sends the edited values of an item back with `set_item` and everything else with
//! `item_command`. Failed edits are reported with a `notice`.

use std::collections::BTreeMap;

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    host::report_editor_event,
    item::{ItemTable, IC_KEY},
    map::{
        item_data::KeyData, Level, MapItem, TacticalMap, Visibility, WorldItemFlags, WorldItemId,
    },
    render::{world::MapWorld, PendingSceneUpdate, RenderOptions},
    states::loading::MapToLoad,
    systems::pixel_view::PixelView,
};

use super::{
    document::{EditContext, EditorDocument},
    items::{center_view, matches_choice, ItemChoice, ItemPools, ItemSelection, NUMBER_TRIGGERS},
    soldier_events::{ItemStackValue, NoticeEvent},
    soldiers::edit_once,
    MapEdit,
};

/// A world item as the host shows it
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemValue {
    pub id: u32,
    pub grid_no: u16,
    pub on_roof: bool,
    pub stack: ItemStackValue,
    pub visibility: Visibility,
    /// `WorldItemFlags`, e.g. `REALISTIC_ONLY` (0x100) and `SCIFI_ONLY` (0x80)
    pub flags: u16,
    /// Chance of the item not existing in the game, in percent
    pub non_exist_chance: u8,
}

impl ItemValue {
    pub fn of(item: &MapItem) -> Self {
        Self {
            id: item.id.0,
            grid_no: item.grid_no.index() as u16,
            on_roof: item.level == Level::Roof,
            stack: ItemStackValue::of(&item.stack),
            visibility: item.visibility,
            flags: item.flags.bits(),
            non_exist_chance: item.non_exist_chance,
        }
    }
}

/// The values of an item sent with `set_item`
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEdit {
    pub id: u32,
    pub stack: ItemStackValue,
    pub visibility: Visibility,
    pub flags: u16,
    pub non_exist_chance: u8,
}

/// How often the map has an item, and how many items its stacks hold
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ItemCount {
    pub item: u16,
    pub placed: u16,
    pub quantity: u32,
}

/// How often the map has the key of a lock
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCount {
    pub lock_id: u8,
    pub placed: u16,
}

/// How many items of each choice of the palette the map has
/// (`CountNumberOfEditorPlacementsInWorld`)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ItemCounts {
    /// By item, in the order of the items
    pub items: Vec<ItemCount>,
    /// By trigger slot
    pub triggers: Vec<u16>,
    /// By lock, in the order of the locks
    pub keys: Vec<KeyCount>,
}

impl ItemCounts {
    pub fn of(map: &TacticalMap, items: Option<&ItemTable>) -> Self {
        let mut by_item: BTreeMap<u16, ItemCount> = BTreeMap::new();
        let mut by_lock: BTreeMap<u8, u16> = BTreeMap::new();
        let mut triggers = vec![0; usize::from(NUMBER_TRIGGERS)];
        for item in &map.items {
            let stack = &item.stack;
            let count = by_item.entry(stack.item.index()).or_insert(ItemCount {
                item: stack.item.index(),
                placed: 0,
                quantity: 0,
            });
            count.placed += 1;
            count.quantity += u32::from(stack.number);
            let kind = items.and_then(|items| items.info(stack.item));
            if kind.is_some_and(|kind| kind.class == IC_KEY) {
                *by_lock
                    .entry(KeyData::read(&stack.data).key_id)
                    .or_default() += 1;
            }
            for (slot, count) in triggers.iter_mut().enumerate() {
                if matches_choice(stack, kind, ItemChoice::Trigger { slot: slot as u8 }) {
                    *count += 1;
                }
            }
        }
        Self {
            items: by_item.into_values().collect(),
            triggers,
            keys: by_lock
                .into_iter()
                .map(|(lock_id, placed)| KeyCount { lock_id, placed })
                .collect(),
        }
    }
}

/// The selected tile and its items
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "item", rename_all = "camelCase")]
pub struct ItemEvent {
    pub map: String,
    /// The selected tile, `null` if none is
    pub tile: Option<u16>,
    /// The items shown on the selected tile, in the order of the map
    pub items: Vec<ItemValue>,
    pub counts: ItemCounts,
}

/// What the host asks of the items besides editing them
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemCommand {
    /// Removes an item of the selected tile
    Delete { id: u32 },
    /// Centers the view on the selected tile
    Center,
}

/// What item commands need besides the map and the view
#[derive(SystemParam)]
pub struct ItemCommandParams<'w> {
    pub selection: ResMut<'w, ItemSelection>,
    pub items: Option<Res<'w, ItemTable>>,
}

/// Edits an item with the values of the host, ignored if the item is gone
pub fn update_item(context: &mut EditContext, items: &ItemTable, edit: &ItemEdit) {
    let Some(stack) = edit.stack.stack() else {
        return;
    };
    context.change_item(
        items,
        WorldItemId(edit.id),
        stack,
        edit.visibility,
        WorldItemFlags::from_bits_retain(edit.flags),
        edit.non_exist_chance,
    );
}

/// Runs an item command of the host
pub fn run_item_command(
    command: ItemCommand,
    edit: &mut MapEdit,
    params: &mut ItemCommandParams,
    view: Option<&mut PixelView>,
) {
    let selection = &mut *params.selection;
    match command {
        ItemCommand::Delete { id } => {
            edit_once(edit, |context| context.delete_item(WorldItemId(id)))
        }
        ItemCommand::Center => {
            if let Some(tile) = selection.selected {
                center_view(view, &edit.map, tile);
            }
        }
    }
}

/// The item event that was reported last, `None` for a new map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReportedItem(Option<ItemEvent>);

pub fn reset_reported_item(mut reported: ResMut<ReportedItem>) {
    reported.0 = None;
}

/// Tells the host about the selected tile when it or its items change, and about failed edits
///
/// The selection is dropped here when its tile has no items on the floor any more, after the
/// world caught up with the edits of the frame. The pools are only computed again when one of
/// the resources that the event is made of changed.
pub fn report_item(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    world: Res<MapWorld>,
    options: Res<RenderOptions>,
    items: Option<Res<ItemTable>>,
    mut selection: ResMut<ItemSelection>,
    mut reported: ResMut<ReportedItem>,
) {
    // The item table is removed and inserted again together with the map
    let unchanged = reported.0.is_some()
        && !map_to_load.is_changed()
        && !map.is_changed()
        && !world.is_changed()
        && !options.is_changed()
        && !items.as_ref().is_some_and(|items| items.is_changed())
        && !selection.is_changed();
    if unchanged {
        return;
    }
    if let Some(message) = selection.notice.take() {
        report_editor_event(&NoticeEvent {
            map: map_to_load.0.clone(),
            message,
        });
    }
    let pools = ItemPools::new(&map, &world, options.floor);
    selection.validate(&pools);
    let pool = selection
        .selected
        .map(|tile| pools.pool(tile))
        .unwrap_or_default();
    let event = ItemEvent {
        map: map_to_load.0.clone(),
        tile: selection.selected.map(|tile| tile.index() as u16),
        items: pool
            .into_iter()
            .map(|index| ItemValue::of(&map.items[index]))
            .collect(),
        counts: ItemCounts::of(&map, items.as_deref()),
    };
    if reported.0.as_ref() != Some(&event) {
        report_editor_event(&event);
        reported.0 = Some(event);
    }
}

/// Brings the item nodes of the world up to date after the world items changed, the scene
/// catches up with the tiles they were and are on
pub fn sync_item_scene(
    mut document: ResMut<EditorDocument>,
    map: Res<TacticalMap>,
    mut world: ResMut<MapWorld>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    if !document.items_dirty {
        return;
    }
    document.items_dirty = false;
    let touched = world.sync_items(&map);
    pending.tiles.extend(touched);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        document::tests::grid,
        items::{
            tests::{mode, ItemFixture},
            world_item, ItemMode,
        },
    };

    #[test]
    fn commands_are_read_from_json() {
        let parse = |json: &str| serde_json::from_str::<ItemCommand>(json).unwrap();
        assert_eq!(
            parse(r#"{"kind":"delete","id":4}"#),
            ItemCommand::Delete { id: 4 }
        );
        assert_eq!(parse(r#"{"kind":"center"}"#), ItemCommand::Center);
        assert!(serde_json::from_str::<ItemCommand>(r#"{"kind":"nextPool"}"#).is_err());
    }

    #[test]
    fn the_host_edits_the_selected_item() {
        let mut fixture = ItemFixture::new();
        let placed = mode(r#"{"mode":"items","choice":{"kind":"item","item":202}}"#);
        let id = fixture
            .edit(|context, items| context.place_item(items, &placed, grid(30, 30), Level::Ground))
            .unwrap();
        let value = ItemValue::of(world_item(&fixture.fixture.map, id).unwrap());
        let mut json = serde_json::to_value(&value).unwrap();
        assert_eq!(json["gridNo"], 30 * 160 + 30);
        assert_eq!(json["onRoof"], false);
        assert_eq!(json["visibility"], "invisible");
        assert_eq!(json["stack"]["item"], 202);

        // The host sends the item back with its changes
        json["visibility"] = "hiddenItem".into();
        json["flags"] = 0x80.into();
        json["nonExistChance"] = 25.into();
        json["stack"]["data"][0] = 55.into();
        let edit: ItemEdit = serde_json::from_value(json).unwrap();
        fixture.edit(|context, items| update_item(context, items, &edit));
        let edited = world_item(&fixture.fixture.map, id).unwrap();
        assert_eq!(edited.visibility, Visibility::HiddenItem);
        assert_eq!(edited.flags, WorldItemFlags::SCIFI_ONLY);
        assert_eq!(edited.non_exist_chance, 25);
        assert_eq!(edited.stack.data[0], 55);
        assert_eq!(fixture.fixture.document.undo.depth(), 2);
        // Edits of other items are ignored
        let other = ItemEdit { id: 99, ..edit };
        fixture.edit(|context, items| update_item(context, items, &other));
        assert_eq!(fixture.fixture.document.undo.depth(), 2);

        // Removing a single item of the tile
        fixture.edit(|context, _| context.delete_item(WorldItemId(id.0)));
        assert!(fixture.fixture.map.items.is_empty());
        fixture.undo();
        assert!(world_item(&fixture.fixture.map, id).is_some());
    }

    #[test]
    fn counts_have_the_choices_of_the_palette() {
        let mut fixture = ItemFixture::new();
        let place = |fixture: &mut ItemFixture, choice: ItemChoice, x| {
            let mode = ItemMode {
                choice: Some(choice),
                ..Default::default()
            };
            fixture
                .edit(|context, items| context.place_item(items, &mode, grid(x, 30), Level::Ground))
                .unwrap();
        };
        place(&mut fixture, ItemChoice::Item { item: 71 }, 10);
        place(&mut fixture, ItemChoice::Item { item: 71 }, 11);
        place(&mut fixture, ItemChoice::Trigger { slot: 3 }, 12);
        place(&mut fixture, ItemChoice::Trigger { slot: 26 }, 13);
        let key = ItemChoice::Key {
            lock_id: 5,
            item: 271,
        };
        place(&mut fixture, key, 14);
        let counts = ItemCounts::of(&fixture.fixture.map, Some(&fixture.items));
        assert_eq!(
            counts.items[0],
            ItemCount {
                item: 71,
                placed: 2,
                quantity: 2
            }
        );
        assert_eq!(counts.triggers.len(), 27);
        assert_eq!(counts.triggers[3], 1);
        assert_eq!(counts.triggers[26], 1);
        assert_eq!(counts.triggers.iter().sum::<u16>(), 2);
        assert_eq!(
            counts.keys,
            [KeyCount {
                lock_id: 5,
                placed: 1
            }]
        );
        let first = ItemValue::of(&fixture.fixture.map.items[0]);
        let json = serde_json::to_value(&ItemEvent {
            map: "maps/a9.dat".to_owned(),
            tile: Some(30 * 160 + 10),
            items: vec![first],
            counts,
        })
        .unwrap();
        assert_eq!(json["type"], "item");
        assert_eq!(json["tile"], 30 * 160 + 10);
        assert_eq!(json["items"][0]["stack"]["item"], 71);
        assert_eq!(json["counts"]["keys"][0]["lockId"], 5);
    }
}
