//! World items of the map as level nodes
//!
//! Port of `LoadWorldItemsFromMap` (`World_Items.cc`), `InternalAddItemToPool` and
//! `AddItemGraphicToWorld` (`Handle_Items.cc`) as the editor runs them: every item is shown, and
//! none of the random or game-only replacements happen. Items that sink in water are kept, because
//! the terrain is not known yet when the game loads the items.

use std::sync::Arc;

use crate::{
    item::ItemId,
    map::{GridNo, ItemHeight, Level, MapItem, TacticalMap},
    render::world::{MapWorld, NodeFlags},
    tileset::{StructureFlags, Tile, PROFILE_Z_SIZE},
};

/// Height of a structure level in pixels (`CONVERT_INDEX_TO_PIXELS`)
fn index_to_pixels(index: i32) -> i8 {
    const MAX_STRUCTURE_HEIGHT: i32 = 50;
    const HEIGHT_UNITS_PER_INDEX: i32 = 64;
    const HEIGHT_UNITS: i32 = 256;
    (index * MAX_STRUCTURE_HEIGHT * HEIGHT_UNITS_PER_INDEX / HEIGHT_UNITS) as i8
}

/// Adds the items of the map to the struct and on roof lists
pub fn add_items(world: &mut MapWorld, map: &TacticalMap) {
    for index in 0..map.items.len() {
        add_item(world, map, index);
    }
}

/// Adds an item of the map to the world, like `add_items` does
///
/// Items without a graphic are left out, the conversion warned about them.
pub fn add_item(world: &mut MapWorld, map: &TacticalMap, index: usize) {
    let item = &map.items[index];
    if let Some(graphic) = &item.graphic {
        let placement = item_placement(world, item);
        add_item_graphic(world, index, graphic, placement);
    }
}

/// Where an item is shown
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemPlacement {
    /// The tile of the item, or the base tile of the structure it lies on
    pub grid_no: GridNo,
    pub level: Level,
    /// How far the item is drawn above the ground (`bRenderZHeightAboveLevel`)
    pub render_height: i8,
}

/// Port of `InternalAddItemToPool` for the placement of an item
///
/// Only depends on the roofs and structures of the tile of the item.
pub fn item_placement(world: &mut MapWorld, item: &MapItem) -> ItemPlacement {
    let mut grid_no = item.grid_no;

    // Items on roofs without a roof are put on the ground (`FlatRoofAboveGridNo`)
    let flat_roof = world.tiles[grid_no]
        .roofs
        .iter()
        .any(|node| node.tile.is_flat_roof());
    let level = if !flat_roof {
        Level::Ground
    } else {
        item.level
    };

    let (force_on_ground, mut render_height) = match item.render_height {
        ItemHeight::ForceGround => (true, 0),
        ItemHeight::Above(height) => (false, height as i8),
    };

    if item.stack.item != ItemId::OWNERSHIP && item.stack.item != ItemId::ACTION_ITEM {
        let desired_level = match level {
            Level::Ground => 0,
            Level::Roof => PROFILE_Z_SIZE as i16,
        };
        let tile = grid_no;
        for index in 0..world.tiles[tile].structures.len() {
            let structure = &world.tiles[tile].structures[index];
            if !structure.flags.intersects(StructureFlags::BLOCKSMOVES)
                || structure
                    .flags
                    .intersects(StructureFlags::PERSON | StructureFlags::CORPSE)
                || structure.cube_offset != desired_level
            {
                continue;
            }
            let Some(shape) = structure.structure_tile().map(|tile| tile.shape) else {
                continue;
            };
            // Lowest cube level of the structure, 1 to 4 (`StructureBottomLevel`)
            let bottom_level = shape
                .iter()
                .flatten()
                .filter_map(|&column| (0..PROFILE_Z_SIZE).find(|&z| column & (1 << z) != 0))
                .min()
                .map_or(PROFILE_Z_SIZE + 1, |z| z + 1);
            if bottom_level != 1 && force_on_ground {
                break;
            }
            // The item moves to the base tile of the structure
            grid_no = structure.base_grid_no;
            let flags = structure.flags;
            if flags.contains(StructureFlags::OPENABLE) {
                let height = world.cache_structure_height(tile, index);
                render_height = index_to_pixels(i32::from(height));
                break;
            } else if flags.contains(StructureFlags::GENERIC) {
                // On top of the most dense level (`StructureDensity`)
                let density: [usize; PROFILE_Z_SIZE] = std::array::from_fn(|z| {
                    shape
                        .iter()
                        .flatten()
                        .filter(|&&column| column & (1 << z) != 0)
                        .count()
                });
                render_height = 0;
                let mut max = 0;
                for z in (0..PROFILE_Z_SIZE).rev() {
                    if density[z] > max {
                        max = density[z];
                        render_height = index_to_pixels(z as i32 + 1);
                    }
                }
                break;
            }
        }
    }
    ItemPlacement {
        grid_no,
        level,
        render_height,
    }
}

/// Adds the graphic of the item with the given index in the map (`AddItemGraphicToWorld`)
fn add_item_graphic(
    world: &mut MapWorld,
    index: usize,
    graphic: &Arc<Tile>,
    placement: ItemPlacement,
) {
    let grid_no = placement.grid_no;
    let node = if placement.level == Level::Ground {
        if world.add_struct_to_tail(grid_no, graphic).is_err() {
            return;
        }
        world.tiles[grid_no].structs.last_mut()
    } else {
        match world.add_on_roof_to_head(grid_no, graphic) {
            Ok(()) => world.tiles[grid_no].on_roofs.first_mut(),
            Err(_) => return,
        }
    };
    if let Some(node) = node {
        node.flags |= NodeFlags::ITEM;
        node.item = Some(index as u16);
        node.item_height = placement.render_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_levels_in_pixels() {
        assert_eq!([1, 2, 3, 4].map(index_to_pixels), [12, 25, 37, 50]);
    }
}
