//! Painting the ground textures (`PasteTexture` and `TerrainFill` of the original editor)

use std::collections::VecDeque;

use crate::{
    map::{Direction, GridArray, GridNo},
    random::Random,
    render::movement_costs::grid_no_on_visible_world_tile,
    tileset::{Tile, TileCategory},
};

use super::{
    document::EditContext,
    smooth::{smooth_all_terrain_type_radius, smooth_terrain_radius},
};

/// Sub index of the tile a texture starts with before it is smoothed (`REQUIRES_SMOOTHING_TILE`)
const REQUIRES_SMOOTHING_TILE: u16 = 19;

/// Which textures the ground brush paints
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroundSelection {
    /// Name of the tile type that is painted while no texture has a weight
    pub selected: Option<String>,
    /// The tile types with a weight, in the order of the tile types
    pub weights: Vec<(String, u8)>,
}

impl GroundSelection {
    /// The selection of the host, without what is not a texture of the tileset
    pub fn new(
        context: &EditContext,
        selected: Option<String>,
        weights: impl IntoIterator<Item = (String, u8)>,
    ) -> Self {
        let mut weights: Vec<(String, u8)> = weights
            .into_iter()
            .filter(|(type_name, weight)| *weight > 0 && is_texture(context, type_name))
            .collect();
        weights.sort_by_key(|(type_name, _)| context.tileset.type_start(type_name));
        Self {
            selected: selected.filter(|type_name| is_texture(context, type_name)),
            weights,
        }
    }
}

/// Whether the ground brush can paint a tile type
pub fn is_texture(context: &EditContext, type_name: &str) -> bool {
    context
        .category_of(type_name)
        .is_some_and(TileCategory::is_texture)
}

/// Picks the texture to paint (`ChooseWeightedTerrainTile`)
///
/// Unlike the original, which gives the first texture with a weight one more chance, every
/// texture is picked in proportion to its weight.
pub fn choose_weighted_texture<'a>(
    random: &mut Random,
    selection: &'a GroundSelection,
) -> Option<&'a str> {
    let total: u32 = selection.weights.iter().map(|&(_, w)| u32::from(w)).sum();
    if total == 0 {
        return selection.selected.as_deref();
    }
    let mut pick = random.next(total);
    for (type_name, weight) in &selection.weights {
        let weight = u32::from(*weight);
        if pick < weight {
            return Some(type_name);
        }
        pick -= weight;
    }
    selection.selected.as_deref()
}

fn is_floor(tile: &Tile) -> bool {
    tile.category == Some(TileCategory::Floor)
}

/// Paints a texture on a tile and smooths it with the tiles around it (`PasteTextureCommon`)
pub fn paste_texture_common(context: &mut EditContext, grid_no: GridNo, paste: &str) {
    if context.category_of(paste) == Some(TileCategory::DeepWater) {
        // Deep water only goes onto full regular water
        let on_water = context
            .find_category_in_land(grid_no, TileCategory::Water)
            .is_some_and(|index| context.land(grid_no)[index].is_full_land());
        if !on_water {
            return;
        }
    }
    // Don't draw over floors
    if context.land_exists(grid_no, is_floor) {
        return;
    }
    paste_texture_ex(context, grid_no, paste);
    smooth_terrain_radius(context, grid_no, paste, 1, true);
}

/// `PasteTextureEx`
fn paste_texture_ex(context: &mut EditContext, grid_no: GridNo, paste: &str) {
    match context.find_type_in_land(grid_no, paste) {
        Some(index) => {
            // The texture is below the head, it becomes the head
            if context.land_level(grid_no, paste) != Some(0) {
                let tile = context.land(grid_no)[index].clone();
                paste_existing_texture(context, grid_no, tile);
            }
        }
        None => {
            // Smoothing picks the right tile later
            if let Some(tile) = context.tile(paste, REQUIRES_SMOOTHING_TILE) {
                context.set_land_index(grid_no, tile);
            }
        }
    }
}

/// Makes a texture that is below the head the head (`PasteExistingTexture`): the land is replaced
/// by the background and the texture, and everything around is smoothed again
fn paste_existing_texture(context: &mut EditContext, grid_no: GridNo, tile: std::sync::Arc<Tile>) {
    context.delete_all_land(grid_no);
    if let Some(background) = context.background_tile() {
        context.add_land_to_head(grid_no, background);
    }
    context.set_land_index(grid_no, tile);
    smooth_all_terrain_type_radius(context, grid_no, 2, true);
}

/// Removes the topmost texture of a tile, unless it is the only one (`EraseMapTile` for ground)
pub fn erase_ground(context: &mut EditContext, grid_no: GridNo) {
    let land = context.land(grid_no);
    if land.len() < 2 {
        return;
    }
    let type_name = land[land.len() - 1].type_name.clone();
    context.remove_land(grid_no, land.len() - 1);
    smooth_terrain_radius(context, grid_no, &type_name, 1, true);
}

/// Paints the connected area of tiles with the same topmost texture (`TerrainFill`)
///
/// The original fills recursively and never ends if the texture can't be painted, e.g. deep water
/// onto land. Here every tile is visited once.
pub fn terrain_fill(context: &mut EditContext, start: GridNo, paste: &str) {
    let Some(search) = context.land_head_type(start).map(str::to_owned) else {
        return;
    };
    if search == paste {
        return;
    }
    let restricted_scroll = context.world.restricted_scroll;
    let mut visited = GridArray::filled(false);
    let mut queue = VecDeque::from([start]);
    visited[start] = true;
    while let Some(grid_no) = queue.pop_front() {
        if !grid_no_on_visible_world_tile(grid_no, restricted_scroll)
            || context.land_head_type(grid_no) != Some(search.as_str())
        {
            continue;
        }
        paste_texture_common(context, grid_no, paste);
        let position = grid_no.position();
        let neighbours = [
            (position.y > 0, Direction::North),
            (position.y < 159, Direction::South),
            (position.x > 0, Direction::West),
            (position.x < 159, Direction::East),
        ];
        for (on_map, direction) in neighbours {
            let neighbour = grid_no.step(direction);
            if on_map && !visited[neighbour] {
                visited[neighbour] = true;
                queue.push_back(neighbour);
            }
        }
    }
}

/// Replaces the textures of the whole map by the background texture (`SetBackgroundTexture`),
/// nothing happens if the tile type is not a texture
pub fn set_background_texture(context: &mut EditContext, background: &str) {
    if !is_texture(context, background) {
        return;
    }
    let previous = std::mem::replace(&mut context.document.background, background.to_owned());
    context.document.undo.record_background(&previous);
    for grid_no in GridNo::all() {
        context.remove_land_where(grid_no, |tile| {
            tile.category.is_some_and(TileCategory::is_texture)
        });
        let Some(tile) = context.background_tile() else {
            continue;
        };
        if context.land_exists(grid_no, is_floor) {
            // Show the land below the floor
            context.add_land_to_tail(grid_no, tile);
        } else {
            context.add_land_to_head(grid_no, tile);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::document::tests::{grid, Fixture};

    fn selection(
        context: &EditContext,
        selected: Option<&str>,
        weights: &[(&str, u8)],
    ) -> GroundSelection {
        let weights = weights.iter().map(|&(name, w)| (name.to_owned(), w));
        GroundSelection::new(context, selected.map(str::to_owned), weights)
    }

    fn weights(selection: &GroundSelection) -> Vec<(&str, u8)> {
        let weights = selection.weights.iter();
        weights.map(|(name, w)| (name.as_str(), *w)).collect()
    }

    #[test]
    fn weighted_textures_are_picked_in_proportion() {
        let mut fixture = Fixture::new();
        let context = fixture.context();
        let mut random = Random::new(3);
        let selection = selection(&context, Some("TEXTURE4"), &[]);
        assert_eq!(
            choose_weighted_texture(&mut random, &selection),
            Some("TEXTURE4")
        );

        let selection = self::selection(
            &context,
            Some("TEXTURE4"),
            &[("WATER1", 3), ("TEXTURE2", 1), ("TEXTURE3", 0)],
        );
        assert_eq!(weights(&selection), [("TEXTURE2", 1), ("WATER1", 3)]);
        let mut counts = [0; 2];
        for _ in 0..4000 {
            match choose_weighted_texture(&mut random, &selection) {
                Some("TEXTURE2") => counts[0] += 1,
                Some("WATER1") => counts[1] += 1,
                other => panic!("{other:?}"),
            }
        }
        assert!((800..1200).contains(&counts[0]), "{counts:?}");
    }

    #[test]
    fn only_textures_are_selected() {
        let mut fixture = Fixture::new();
        let context = fixture.context();
        let selection = selection(
            &context,
            Some("FLOOR1"),
            &[("FLOOR1", 2), ("ROCKS", 1), ("DEEP WATER", 1)],
        );
        assert_eq!(selection.selected, None);
        assert_eq!(weights(&selection), [("DEEP WATER", 1)]);
        assert_eq!(
            choose_weighted_texture(&mut Random::new(1), &GroundSelection::default()),
            None
        );
    }

    #[test]
    fn pasting_a_new_texture_smooths_it() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        let mut context = fixture.context();
        paste_texture_common(&mut context, center, "TEXTURE2");
        context.finish();
        // Alone, so the edge tile for all sides
        assert_eq!(fixture.land(center), [("TEXTURE1", 1), ("TEXTURE2", 20)]);

        // A second tile next to it changes both
        let mut context = fixture.context();
        paste_texture_common(&mut context, grid(51, 50), "TEXTURE2");
        context.finish();
        assert_eq!(fixture.land(center).last(), Some(&("TEXTURE2", 22)));
        assert_eq!(fixture.land(grid(51, 50)).last(), Some(&("TEXTURE2", 19)));
    }

    #[test]
    fn a_buried_texture_becomes_the_head() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        let mut context = fixture.context();
        paste_texture_common(&mut context, center, "TEXTURE2");
        // Grass is below the sand now
        paste_texture_common(&mut context, center, "TEXTURE1");
        context.finish();
        let land = fixture.land(center);
        assert_eq!(land.len(), 1, "{land:?}");
        assert_eq!(land[0].0, "TEXTURE1");
    }

    #[test]
    fn deep_water_only_goes_onto_full_water() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        let mut context = fixture.context();
        paste_texture_common(&mut context, center, "DEEP WATER");
        assert_eq!(context.land(center).len(), 1);
        // Not onto the shore
        let shore = context.tile("WATER1", 11).unwrap();
        context.set_land_index(center, shore);
        paste_texture_common(&mut context, center, "DEEP WATER");
        assert_eq!(context.land_head_type(center), Some("WATER1"));

        let water = context.tile("WATER1", 1).unwrap();
        context.set_land_index(center, water);
        paste_texture_common(&mut context, center, "DEEP WATER");
        assert_eq!(context.land_head_type(center), Some("DEEP WATER"));
    }

    #[test]
    fn floors_are_not_painted_over() {
        let mut fixture = Fixture::new();
        let center = grid(50, 50);
        let mut context = fixture.context();
        let floor = context.tile("FLOOR1", 1).unwrap();
        context.add_land_to_head(center, floor);
        paste_texture_common(&mut context, center, "TEXTURE2");
        context.finish();
        assert_eq!(fixture.land(center), [("TEXTURE1", 1), ("FLOOR1", 1)]);
    }

    #[test]
    fn erasing_removes_the_head_and_smooths_the_rest() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        paste_texture_common(&mut context, grid(50, 50), "TEXTURE2");
        paste_texture_common(&mut context, grid(51, 50), "TEXTURE2");
        erase_ground(&mut context, grid(51, 50));
        // The only land is kept
        erase_ground(&mut context, grid(51, 50));
        context.finish();
        assert_eq!(fixture.land(grid(51, 50)), [("TEXTURE1", 1)]);
        assert_eq!(fixture.land(grid(50, 50)).last(), Some(&("TEXTURE2", 20)));
    }

    #[test]
    fn fill_paints_the_connected_area_and_ends() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        // A ring of sand around (50, 50) separates it from the rest of the grass
        for (x, y) in [(49, 49), (50, 49), (51, 49), (49, 50), (51, 50), (49, 51)] {
            paste_texture_common(&mut context, grid(x, y), "TEXTURE2");
        }
        for (x, y) in [(50, 51), (51, 51)] {
            paste_texture_common(&mut context, grid(x, y), "TEXTURE2");
        }
        terrain_fill(&mut context, grid(50, 50), "WATER1");
        assert_eq!(context.land_head_type(grid(50, 50)), Some("WATER1"));
        assert_eq!(context.land_head_type(grid(60, 60)), Some("TEXTURE1"));

        // Deep water can't be painted onto grass, the fill still ends
        terrain_fill(&mut context, grid(60, 60), "DEEP WATER");
        assert_eq!(context.land_head_type(grid(60, 60)), Some("TEXTURE1"));
    }

    #[test]
    fn the_background_replaces_all_textures() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        paste_texture_common(&mut context, grid(50, 50), "TEXTURE2");
        let floor = context.tile("FLOOR1", 1).unwrap();
        context.add_land_to_head(grid(60, 60), floor);
        set_background_texture(&mut context, "TEXTURE2");
        let touched = context.finish();
        assert_eq!(touched.len(), GridNo::all().len());
        for grid_no in [grid(0, 0), grid(50, 50)] {
            let land = fixture.land(grid_no);
            assert_eq!(land.len(), 1);
            assert_eq!(land[0].0, "TEXTURE2");
            assert!((1..=10).contains(&land[0].1));
        }
        let land = fixture.land(grid(60, 60));
        assert_eq!((land[0].0, land[1]), ("TEXTURE2", ("FLOOR1", 1)));

        let command = fixture.document.undo.pop().unwrap();
        assert_eq!(command.background.as_deref(), Some("TEXTURE1"));
        assert_eq!(command.tiles.len(), GridNo::all().len());

        // Only textures can be the background
        let mut context = fixture.context();
        set_background_texture(&mut context, "FLOOR1");
        assert!(context.finish().is_empty());
        assert_eq!(fixture.document.background, "TEXTURE2");
        assert_eq!(fixture.document.undo.depth(), 0);
    }

    #[test]
    fn restoring_returns_the_command_that_redoes_it() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        set_background_texture(&mut context, "TEXTURE2");
        context.finish();
        let undo = fixture.document.undo.pop().unwrap();

        let mut context = fixture.context();
        let redo = context.restore(undo);
        context.finish();
        assert_eq!(fixture.document.background, "TEXTURE1");
        assert_eq!(fixture.land(grid(50, 50))[0].0, "TEXTURE1");
        assert_eq!(redo.background.as_deref(), Some("TEXTURE2"));
        assert_eq!(redo.tiles.len(), GridNo::all().len());

        let mut context = fixture.context();
        let undo = context.restore(redo);
        context.finish();
        assert_eq!(fixture.document.background, "TEXTURE2");
        assert_eq!(fixture.land(grid(50, 50))[0].0, "TEXTURE2");
        assert_eq!(undo.background.as_deref(), Some("TEXTURE1"));
        // Restoring is not recorded
        assert_eq!(fixture.document.undo.depth(), 0);
    }
}
