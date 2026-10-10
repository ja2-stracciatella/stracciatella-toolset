//! Placing debris and structures (`PasteDebris` and `PasteStructure` in `Edit_Sys.cc`)

use std::sync::Arc;

use crate::{
    map::{GridNo, Level},
    random::Random,
    tileset::{Tile, TileCategory, TileIndex, Tileset},
};

use super::document::EditContext;

/// Whether a tile of the objects is debris, which placing debris replaces and erasing debris
/// removes
pub fn is_debris(tile: &Tile) -> bool {
    tile.category == Some(TileCategory::Debris)
}

/// Whether a tile of the structs is removed by erasing structures
pub fn is_erased_structure(tile: &Tile) -> bool {
    tile.category.is_some_and(TileCategory::is_structure)
}

/// Whether a tile of the objects is removed by erasing structures, like the original
/// `DEBRIS2MISC` and `ANOTHERDEBRIS`
pub fn is_erased_with_structures(tile: &Tile) -> bool {
    tile.erased_with_structures
}

/// Tiles to place and how often each is picked (`Selections`)
#[derive(Clone, Debug, Default)]
pub struct SelectionList(Vec<(Arc<Tile>, u32)>);

impl SelectionList {
    /// The tiles with the given ids and counts, unknown tiles are left out
    pub fn new(tileset: &Tileset, entries: impl IntoIterator<Item = (TileIndex, u32)>) -> Self {
        Self(
            entries
                .into_iter()
                .filter(|&(_, count)| count > 0)
                .filter_map(|(id, count)| Some((tileset.tile(id)?.clone(), count)))
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Picks a tile, the higher its count the more often (`GetRandomSelection`)
    pub fn pick(&self, random: &mut Random) -> Option<&Arc<Tile>> {
        self.pick_where(random, |_| true)
    }

    /// Picks one of the tiles that match like [`SelectionList::pick`], `None` if none does
    /// (`GetRandomIndexByRange`, which ignores the counts)
    pub fn pick_where(
        &self,
        random: &mut Random,
        matches: impl Fn(&Tile) -> bool,
    ) -> Option<&Arc<Tile>> {
        let candidates = || self.0.iter().filter(|(tile, _)| matches(tile));
        let total: u32 = candidates().map(|(_, count)| count).sum();
        if total == 0 {
            return None;
        }
        let mut pick = random.next(total);
        for (tile, count) in candidates() {
            if pick < *count {
                return Some(tile);
            }
            pick -= count;
        }
        None
    }

    /// Whether any of the tiles matches
    pub fn contains(&self, matches: impl Fn(&Tile) -> bool) -> bool {
        self.0.iter().any(|(tile, _)| matches(tile))
    }
}

/// Places a tile of the objects palette, debris like [`paste_debris`] and everything else like
/// [`paste_structure`]
pub fn paste_tile(context: &mut EditContext, grid_no: GridNo, tile: Arc<Tile>) {
    if is_debris(&tile) {
        paste_debris(context, grid_no, tile);
    } else {
        paste_structure(context, grid_no, tile);
    }
}

/// Places debris at the tail of the objects, replacing the debris of the tile (`PasteDebris`)
///
/// The original removes all objects from `ANOTHERDEBRIS` on, including e.g. roads. Only debris is
/// removed here.
pub fn paste_debris(context: &mut EditContext, grid_no: GridNo, tile: Arc<Tile>) {
    context.remove_objects_where(grid_no, is_debris);
    context.add_object_to_tail(grid_no, tile);
}

/// Places a structure at the head of the structs, if it fits (`PasteStructureCommon`)
///
/// The world is synced right away, so the next structure sees this one.
pub fn paste_structure(context: &mut EditContext, grid_no: GridNo, tile: Arc<Tile>) {
    if let Some(structure) = &tile.structure {
        if !context
            .world
            .okay_to_add_structure_to_world(grid_no, Level::Ground, structure)
        {
            return;
        }
    }
    context.add_struct_to_head(grid_no, tile);
    context.commit();
}

/// Removes the debris of a tile (`EraseMapTile` for debris)
pub fn erase_debris(context: &mut EditContext, grid_no: GridNo) {
    context.remove_objects_where(grid_no, is_debris);
}

/// Removes the structures of a tile (`EraseMapTile` for structures), and like the original also
/// `DEBRIS2MISC` and `ANOTHERDEBRIS`
pub fn erase_structures(context: &mut EditContext, grid_no: GridNo) {
    context.remove_structs_where(grid_no, is_erased_structure);
    context.remove_objects_where(grid_no, is_erased_with_structures);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        file_formats::ja2_tile_types::{DEBRISROCKS, FIRSTFULLSTRUCT, FIRSTOSTRUCT},
        tileset::tests::{structure_tileset, test_tileset},
    };

    #[test]
    fn picks_follow_the_counts() {
        let tileset = test_tileset();
        let id = |sub_index| tileset.index(DEBRISROCKS, sub_index).unwrap();
        let list = SelectionList::new(&tileset, [(id(1), 1), (id(2), 0), (id(3), 3)]);
        let mut random = Random::new(7);
        let mut counts = [0; 4];
        for _ in 0..4000 {
            counts[usize::from(list.pick(&mut random).unwrap().sub_index)] += 1;
        }
        assert_eq!(counts[2], 0);
        assert!((800..1200).contains(&counts[1]), "{counts:?}");
        assert!(SelectionList::new(&tileset, []).pick(&mut random).is_none());

        // The same seed picks the same tiles
        let picks = |seed| {
            let mut random = Random::new(seed);
            (0..20)
                .map(|_| list.pick(&mut random).unwrap().sub_index)
                .collect::<Vec<_>>()
        };
        assert_eq!(picks(9), picks(9));
    }

    #[test]
    fn debris_only_replaces_debris() {
        let mut fixture = Fixture::new();
        let g = grid(50, 50);
        let rock = fixture.tileset.index(DEBRISROCKS, 2).unwrap();
        let rock = fixture.tileset.tile(rock).unwrap().clone();
        let mut context = fixture.context();
        let road = context.tile("ROADPIECES", 1).unwrap();
        let old = context.tile("ANOTHERDEBRIS", 1).unwrap();
        context.add_object_to_head(g, road);
        context.add_object_to_tail(g, old);
        paste_debris(&mut context, g, rock.clone());
        paste_debris(&mut context, g, rock);
        context.finish();
        let objects: Vec<_> = fixture.map.tiles[g]
            .objects
            .iter()
            .map(|tile| (tile.type_name.as_str(), tile.sub_index))
            .collect();
        assert_eq!(objects, [("ROADPIECES", 1), ("ROCKS", 2)]);

        let mut context = fixture.context();
        erase_debris(&mut context, g);
        context.finish();
        assert_eq!(fixture.map.tiles[g].objects.len(), 1);
    }

    #[test]
    fn structures_do_not_overlap() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        // A 2×2 obstacle
        let big = fixture.tileset.index(FIRSTOSTRUCT, 3).unwrap();
        let big = fixture.tileset.tile(big).unwrap().clone();
        let mut context = fixture.context();
        for (x, y) in [(50, 50), (51, 50), (50, 51), (52, 52)] {
            paste_structure(&mut context, grid(x, y), big.clone());
        }
        // Off the map
        paste_structure(&mut context, grid(159, 159), big);
        context.finish();
        let placed: Vec<GridNo> = GridNo::all()
            .filter(|&g| !fixture.map.tiles[g].structs.is_empty())
            .collect();
        assert_eq!(placed, [grid(50, 50), grid(52, 52)]);
        assert_eq!(fixture.world.tiles[grid(51, 51)].structures.len(), 1);
    }

    #[test]
    fn erasing_structures_removes_their_shadows() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        let g = grid(50, 50);
        let tree = fixture.tileset.index(FIRSTFULLSTRUCT, 1).unwrap();
        assert!(fixture.tileset.tile(tree).unwrap().shadow_buddy.is_some());
        let tree = fixture.tileset.tile(tree).unwrap().clone();
        let mut context = fixture.context();
        paste_structure(&mut context, g, tree);
        let junk = context.tile("MISC2", 1).unwrap();
        context.add_object_to_tail(g, junk);
        context.finish();
        assert_eq!(fixture.world.tiles[g].shadows.len(), 1);
        assert_eq!(fixture.world.tiles[g].structures.len(), 1);

        let mut context = fixture.context();
        erase_structures(&mut context, g);
        context.finish();
        assert!(fixture.map.tiles[g].structs.is_empty());
        assert!(fixture.map.tiles[g].objects.is_empty());
        assert!(fixture.world.tiles[g].shadows.is_empty());
        assert!(fixture.world.tiles[g].structures.is_empty());
    }
}
