//! What the items mode draws over the map: a ring on every tile with items (`SELRING1` of the
//! original editor), the selected tile, the areas of the pits its action items dig and the tile
//! under the cursor, which depends on the tool

use bevy::prelude::*;

use crate::{
    item::{ItemId, ItemTable},
    map::{GridNo, Level, TacticalMap},
    render::{world::MapWorld, RenderOptions},
    systems::cursor::CursorInfo,
};

use super::{
    commands::CurrentMode,
    document::EditorDocument,
    input::{area_outline, raised},
    items::{is_water, item_mode, pit_size, ItemChoice, ItemPools, ItemSelection, ItemTool},
    soldier_events::tile_center,
    soldier_overlay::{cursor_outline, tile_outline},
    soldiers::has_level,
};

/// The tiles of a square around a tile, `size` tiles wide, that are on the map
fn square(grid_no: GridNo, size: u8) -> Vec<GridNo> {
    let reach = i32::from(size / 2);
    let mut tiles = vec![];
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            if let Some(tile) = grid_no.offset(dy * 160 + dx) {
                tiles.push(tile);
            }
        }
    }
    tiles
}

/// A small ring in the middle of a tile, a diamond of a third of its size
pub fn ring(map: &TacticalMap, grid_no: GridNo) -> Vec<(Vec2, Vec2)> {
    let center = tile_center(map, grid_no);
    tile_outline(map, grid_no, Level::Ground)
        .into_iter()
        .map(|(from, to)| (center + (from - center) / 3., center + (to - center) / 3.))
        .collect()
}

/// Whether a click on a level of a tile can place the chosen item, without the random choices
pub fn can_place(
    map: &TacticalMap,
    world: &MapWorld,
    items: &ItemTable,
    choice: Option<ItemChoice>,
    grid_no: GridNo,
    level: Level,
) -> bool {
    if !has_level(world, grid_no, level) {
        return false;
    }
    let item = match choice {
        None => return false,
        Some(ItemChoice::Item { item } | ItemChoice::Key { item, .. }) => ItemId::new(item),
        Some(ItemChoice::Trigger { slot }) => {
            let trigger = slot % 2 == 0 && slot < super::items::PRESSURE_ACTION_SLOT;
            if trigger
                && map.items.iter().any(|i| {
                    i.grid_no == grid_no && i.level == level && i.stack.item == ItemId::SWITCH
                })
            {
                return false;
            }
            Some(if trigger {
                ItemId::SWITCH
            } else {
                ItemId::ACTION_ITEM
            })
        }
    };
    let Some(kind) = item.and_then(|item| items.info(item)) else {
        return false;
    };
    !(kind.sinks && is_water(world, map.tileset, grid_no))
}

/// The tiles with items on the floor, kept until the items, the world or the floor change
#[derive(Default)]
pub struct PoolTiles {
    revision: Option<u64>,
    pools: ItemPools,
}

/// Draws the rings of the pools of the floor in white, the selected tile in yellow with the areas
/// of the pits of its action items in red, and the tile under the cursor on the floor: for
/// placing green where a click places the item, red where it can't, for selecting green where the
/// selected items are dragged to and faintly a tile with items that a click selects. The brush
/// that erases is drawn by `draw_brush_cursor`.
pub fn draw_item_markers(
    mut gizmos: Gizmos,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    selection: Res<ItemSelection>,
    document: Res<EditorDocument>,
    map: Res<TacticalMap>,
    world: Res<MapWorld>,
    items: Option<Res<ItemTable>>,
    mut cache: Local<PoolTiles>,
) {
    let Some(mode) = item_mode(&mode) else {
        return;
    };
    let floor = options.floor;
    if cache.revision != Some(document.revision)
        || map.is_changed()
        || world.is_changed()
        || cache.pools.level() != floor
    {
        cache.revision = Some(document.revision);
        cache.pools = ItemPools::new(&map, &world, floor);
    }
    let mut lines = |lines: Vec<(Vec2, Vec2)>, color: Srgba| {
        for (from, to) in lines {
            gizmos.line_2d(from, to, color);
        }
    };
    let pool_tiles = cache.pools.pool_tiles();
    for &tile in &pool_tiles {
        lines(raised(ring(&map, tile), floor), Srgba::WHITE);
    }
    if let Some(selected) = selection.selected {
        for index in cache.pools.pool(selected) {
            let item = &map.items[index];
            if let Some(size) = pit_size(&item.stack) {
                lines(area_outline(&square(item.grid_no, size), &map), Srgba::RED);
            }
        }
        lines(
            tile_outline(&map, selected, floor),
            Srgba::rgb(1., 0.85, 0.),
        );
    }
    let Some(tile) = cursor.tile.map(GridNo::from) else {
        return;
    };
    match mode.tool {
        ItemTool::Place if mode.erase => {}
        ItemTool::Place => {
            let can_place = items
                .is_some_and(|items| can_place(&map, &world, &items, mode.choice, tile, floor));
            let color = if can_place { Srgba::GREEN } else { Srgba::RED };
            lines(cursor_outline(&map, tile, floor), color);
        }
        ItemTool::Select if selection.dragging() => {
            if selection.selected != Some(tile) {
                lines(cursor_outline(&map, tile, floor), Srgba::GREEN);
            }
        }
        ItemTool::Select => {
            if pool_tiles.binary_search(&tile).is_ok() && selection.selected != Some(tile) {
                lines(
                    cursor_outline(&map, tile, floor),
                    Srgba::new(1., 1., 1., 0.35),
                );
            }
        }
    }
}
