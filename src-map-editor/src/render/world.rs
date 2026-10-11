//! The level nodes of a loaded map, like the game keeps them in `gpWorldLevelData`
//!
//! Port of the layer loading in `LoadWorldFromSGPFile` (`WorldDef.cc`), the node list functions in
//! `WorldMan.cc` that it uses, the structure placement of `AddStructureToWorld` (`Structure.cc`)
//! and `OptimizeMapForShadows`. The node lists are kept in the same order as the game's linked
//! lists, because the renderer draws them in that order.

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use bevy::prelude::*;

use crate::{
    components::map::GridPosition,
    map::{Direction, GridArray, GridNo, Level, TacticalMap},
    render::{
        items::{add_item, item_placement},
        palette::DEFAULT_SHADE_LEVEL,
        soldiers::Soldier,
    },
    tileset::{Orientation, StructureFlags, StructureTile, Tile, TileStructure, PROFILE_Z_SIZE},
};

/// Structure ids below this are soldiers and corpses (`FIRST_AVAILABLE_STRUCTURE_ID`)
const FIRST_AVAILABLE_STRUCTURE_ID: u16 = 256;

bitflags::bitflags! {
    /// Flags of a level node (a subset of `LevelnodeFlags`)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct NodeFlags: u8 {
        /// A shadow that belongs to a struct on the same tile
        const BUDDY_SHADOW = 0x01;
        /// An item of the world item pool
        const ITEM = 0x02;
    }
}

/// The light of a level node
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeLight {
    /// Shade level the node is drawn with (`ubShadeLevel`)
    pub shade: u8,
    /// Shade level without lights (`ubNaturalShadeLevel`)
    pub natural_shade: u8,
    /// Strongest light that reaches the node (`ubMaxLights`)
    pub max_lights: u8,
}

impl Default for NodeLight {
    fn default() -> Self {
        Self {
            shade: DEFAULT_SHADE_LEVEL,
            natural_shade: DEFAULT_SHADE_LEVEL,
            max_lights: 0,
        }
    }
}

/// An element of one of the tile level lists of a tile (`LEVELNODE`)
#[derive(Clone, Debug)]
pub struct LevelNode {
    /// The tile of the node (`usIndex`)
    pub tile: Arc<Tile>,
    pub flags: NodeFlags,
    pub light: NodeLight,
    /// Structure that the node placed (`pStructureData`), the id of the structure
    pub structure_id: Option<u16>,
    /// How far an item is drawn above the ground (`bRenderZHeightAboveLevel`), for items
    pub item_height: i8,
    /// Index of the item in `TacticalMap::items`, for items
    pub item: Option<u16>,
}

impl LevelNode {
    pub fn new(tile: Arc<Tile>) -> Self {
        Self {
            tile,
            flags: NodeFlags::empty(),
            light: NodeLight::default(),
            structure_id: None,
            item_height: 0,
            item: None,
        }
    }
}

/// An element of the merc list of a tile (`LEVELNODE` with `pSoldier`)
#[derive(Clone, Debug)]
pub struct MercNode {
    pub soldier: Arc<Soldier>,
    pub light: NodeLight,
}

/// One tile of a structure that has been placed in the world (`STRUCTURE`)
#[derive(Clone, Debug)]
pub struct Structure {
    /// Shared by all tiles of the structure (`usStructureID`)
    pub id: u16,
    pub flags: StructureFlags,
    pub wall_orientation: Orientation,
    /// Height level of the tile, 0 or 4 (`sCubeOffset`)
    pub cube_offset: i16,
    pub grid_no: GridNo,
    pub base_grid_no: GridNo,
    /// Tile of the node that placed the structure
    pub tile: Arc<Tile>,
    /// Index of the structure tile in `TileStructure::tiles`
    pub structure_tile_index: u8,
    /// Height in cubes once the game computed it, 0 before (`ubStructureHeight`). The game only
    /// computes it on demand, and the lighting reads the cached value.
    pub structure_height: u8,
}

impl Structure {
    /// The structure tile of the tile data
    pub fn structure_tile(&self) -> Option<&StructureTile> {
        self.tile
            .structure
            .as_ref()?
            .tiles
            .get(usize::from(self.structure_tile_index))
    }
}

/// The level lists of a map tile (`MAP_ELEMENT`)
///
/// The lists are in the order of the game's linked lists, from head to tail.
#[derive(Clone, Debug, Default)]
pub struct WorldTile {
    /// Terrain height in pixels (`sHeight`)
    pub height: i16,
    pub land: Vec<LevelNode>,
    /// The topmost full land tile (`pLandStart`), rendering goes from it back to the head
    pub land_start: Option<usize>,
    pub objects: Vec<LevelNode>,
    pub structs: Vec<LevelNode>,
    pub shadows: Vec<LevelNode>,
    pub roofs: Vec<LevelNode>,
    pub on_roofs: Vec<LevelNode>,
    pub topmost: Vec<LevelNode>,
    /// Soldiers (`pMercHead`)
    pub mercs: Vec<MercNode>,
    /// Structures on this tile (`pStructureHead`)
    pub structures: Vec<Structure>,
    /// Soldiers on this tile should not burn through structs (`MAPELEMENT_EXT_NOBURN_STRUCT`)
    pub no_burn_struct: bool,
}

impl WorldTile {
    /// Adds a land node to the head of the land list, a full tile becomes the new start
    fn add_land_to_head(&mut self, node: LevelNode, full_tile: bool) {
        self.land.insert(0, node);
        match self.land_start {
            _ if full_tile => self.land_start = Some(0),
            Some(start) => self.land_start = Some(start + 1),
            None => {}
        }
    }

    /// First structure with any of the flags (`FindStructure`)
    pub fn find_structure(&self, flags: StructureFlags) -> Option<&Structure> {
        self.structures.iter().find(|s| s.flags.intersects(flags))
    }

    pub fn find_structure_by_id(&self, id: u16) -> Option<&Structure> {
        self.structures.iter().find(|s| s.id == id)
    }
}

/// Why a node could not be added to the world
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailedToAddNode;

/// The level nodes of the loaded map
#[derive(Resource, Clone, Debug)]
pub struct MapWorld {
    pub tiles: GridArray<WorldTile>,
    /// Scrolling restriction of the map, changes the z values (`ubRestrictedScrollID`)
    pub restricted_scroll: bool,
    /// Next id for structures (`gusNextAvailableStructureID`)
    next_structure_id: u16,
}

impl MapWorld {
    /// An empty world
    pub fn new(restricted_scroll: bool) -> Self {
        Self {
            tiles: GridArray::default(),
            restricted_scroll,
            next_structure_id: FIRST_AVAILABLE_STRUCTURE_ID,
        }
    }

    /// Builds the level lists of a map, like `LoadWorldFromSGPFile` does
    ///
    /// The layers are added one after the other for the whole map, like in the game.
    pub fn build(map: &TacticalMap) -> Self {
        let mut world = MapWorld::new(map.info.restricted_scroll);
        for (grid_no, tile) in map.tiles.iter() {
            world.tiles[grid_no].height = i16::from(tile.height);
        }

        for (grid_no, tile) in map.tiles.iter() {
            for land in &tile.land {
                world.add_land_to_head(grid_no, land);
            }
        }
        for (grid_no, tile) in map.tiles.iter() {
            for object in &tile.objects {
                world.tiles[grid_no]
                    .objects
                    .push(LevelNode::new(object.clone()));
            }
        }
        let mut failed = 0;
        for (grid_no, tile) in map.tiles.iter() {
            for structure in &tile.structs {
                // The game ignores structs that overlap others, some maps are defective
                if world.add_struct_to_tail(grid_no, structure).is_err() {
                    failed += 1;
                }
            }
        }
        for (grid_no, tile) in map.tiles.iter() {
            for shadow in &tile.shadows {
                world.tiles[grid_no]
                    .shadows
                    .push(LevelNode::new(shadow.clone()));
            }
        }
        for (grid_no, tile) in map.tiles.iter() {
            for roof in &tile.roofs {
                // The game fails to load the map in this case
                match world.add_node_to_world(grid_no, roof, Level::Roof) {
                    Ok(node) => world.tiles[grid_no].roofs.push(node),
                    Err(FailedToAddNode) => failed += 1,
                }
            }
        }
        for (grid_no, tile) in map.tiles.iter() {
            for on_roof in &tile.on_roofs {
                match world.add_node_to_world(grid_no, on_roof, Level::Roof) {
                    Ok(node) => world.tiles[grid_no].on_roofs.push(node),
                    Err(FailedToAddNode) => failed += 1,
                }
            }
        }
        if failed > 0 {
            info!(
                "{} tiles of the map overlap other structures and are left out, like in the game",
                failed
            );
        }

        world.optimize_map_for_shadows();
        world
    }

    pub fn tile(&self, position: GridPosition) -> &WorldTile {
        &self.tiles[GridNo::from(position)]
    }

    /// Adds a land tile to the head of the land list (`AddLandToHead`)
    pub fn add_land_to_head(&mut self, grid_no: GridNo, tile: &Arc<Tile>) {
        let full_tile = tile.is_full_land();
        self.tiles[grid_no].add_land_to_head(LevelNode::new(tile.clone()), full_tile);
    }

    /// Adds a struct and its structure, and a buddy shadow outdoors (`AddStructToTail`)
    pub fn add_struct_to_tail(
        &mut self,
        grid_no: GridNo,
        tile: &Arc<Tile>,
    ) -> Result<(), FailedToAddNode> {
        let node = self.add_node_to_world(grid_no, tile, Level::Ground)?;
        self.tiles[grid_no].structs.push(node);

        if let Some(buddy) = &tile.shadow_buddy {
            if !self.grid_no_indoors(grid_no) {
                let mut shadow = LevelNode::new(buddy.clone());
                shadow.flags |= NodeFlags::BUDDY_SHADOW;
                self.tiles[grid_no].shadows.insert(0, shadow);
            }
        }
        // Special flag to stop soldiers burning through structs on the same tile
        if let Some(structure) = &tile.structure {
            let tile = &mut self.tiles[grid_no];
            tile.no_burn_struct = tile.find_structure(StructureFlags::WALLSTUFF).is_none()
                && structure.tiles.len() == 1;
        }
        Ok(())
    }

    /// Adds a node to the head of the on roof list and places its structure (`AddOnRoofToHead`)
    pub fn add_on_roof_to_head(
        &mut self,
        grid_no: GridNo,
        tile: &Arc<Tile>,
    ) -> Result<(), FailedToAddNode> {
        let node = self.add_node_to_world(grid_no, tile, Level::Roof)?;
        self.tiles[grid_no].on_roofs.insert(0, node);
        Ok(())
    }

    /// Whether a tile is inside of a building (`GridNoIndoors`, the map is never a basement yet
    /// when the structs are loaded)
    fn grid_no_indoors(&self, grid_no: GridNo) -> bool {
        self.tiles[grid_no]
            .land
            .iter()
            .any(|node| node.tile.is_floor())
    }

    /// Creates a node and places its structure, if it has one (`AddNodeToWorld`)
    pub fn add_node_to_world(
        &mut self,
        grid_no: GridNo,
        tile: &Arc<Tile>,
        level: Level,
    ) -> Result<LevelNode, FailedToAddNode> {
        let mut node = LevelNode::new(tile.clone());
        node.structure_id = self.add_structure_to_world(grid_no, level, tile)?;
        Ok(node)
    }

    /// Places the structure of a tile on all tiles it covers (`AddStructureToWorld`)
    ///
    /// Returns the id of the new structure, `None` if the tile has no structure.
    fn add_structure_to_world(
        &mut self,
        base_grid_no: GridNo,
        level: Level,
        node_tile: &Arc<Tile>,
    ) -> Result<Option<u16>, FailedToAddNode> {
        let Some(db_structure) = &node_tile.structure else {
            return Ok(None);
        };
        if db_structure.tiles.is_empty() {
            return Err(FailedToAddNode);
        }
        if !self.okay_to_add_structure_to_world(base_grid_no, level, db_structure) {
            return Err(FailedToAddNode);
        }

        let number_of_tiles = db_structure.tiles.len();
        let mut structures = Vec::with_capacity(number_of_tiles);
        for (i, tile) in db_structure.tiles.iter().enumerate() {
            // Fix garage doors not having a flag of their own separate from sliding doors
            let mut flags = if db_structure.flags.contains(StructureFlags::SLIDINGDOOR)
                && number_of_tiles == 2
            {
                (db_structure.flags - StructureFlags::SLIDINGDOOR) | StructureFlags::GARAGEDOOR
            } else {
                // The old `STRUCTURE_HIDDEN` flag had the same value as the garage door flag
                db_structure.flags - StructureFlags::GARAGEDOOR
            };
            if tile.pos_rel_to_base == 0 {
                flags |= StructureFlags::BASE_TILE;
            }
            // The non-base tiles of multi-tile walls get no orientation
            let wall_orientation = if !flags.contains(StructureFlags::WALL)
                || flags.contains(StructureFlags::BASE_TILE)
            {
                db_structure.wall_orientation
            } else {
                Orientation::empty()
            };
            let on_roof = tile.on_roof;
            let cube_offset =
                (i16::from(level.number()) + i16::from(on_roof)) * PROFILE_Z_SIZE as i16;
            if tile.passable {
                flags |= StructureFlags::PASSABLE;
            }
            if db_structure.flags.contains(StructureFlags::CORPSE) {
                flags |= StructureFlags::CORPSE;
                if number_of_tiles < 10 {
                    flags |= StructureFlags::PASSABLE;
                    flags -= StructureFlags::BLOCKSMOVES;
                } else {
                    flags -= StructureFlags::TRANSPARENT;
                }
            }
            // The tiles are on the map, `okay_to_add_structure_to_world` checked them
            let grid_no = base_grid_no
                .offset(i32::from(tile.pos_rel_to_base))
                .ok_or(FailedToAddNode)?;
            structures.push(Structure {
                id: 0,
                flags,
                wall_orientation,
                cube_offset,
                grid_no,
                base_grid_no,
                tile: node_tile.clone(),
                structure_tile_index: i as u8,
                structure_height: 0,
            });
        }

        self.next_structure_id = self.next_structure_id.wrapping_add(1);
        if self.next_structure_id == 0 {
            self.next_structure_id = FIRST_AVAILABLE_STRUCTURE_ID;
        }
        let id = self.next_structure_id;

        let base_height = self.tiles[base_grid_no].height;
        if structures
            .iter()
            .any(|s| self.tiles[s.grid_no].height != base_height)
        {
            // Not level ground
            return Err(FailedToAddNode);
        }
        for mut structure in structures {
            structure.id = id;
            self.tiles[structure.grid_no].structures.push(structure);
        }
        Ok(Some(id))
    }

    /// Whether a structure is not blocked by the existing ones (`InternalOkayToAddStructureToWorld`)
    pub(crate) fn okay_to_add_structure_to_world(
        &self,
        base_grid_no: GridNo,
        level: Level,
        db_structure: &TileStructure,
    ) -> bool {
        for (i, tile) in db_structure.tiles.iter().enumerate() {
            let cube_offset = if !tile.on_roof {
                i16::from(level.number()) * PROFILE_Z_SIZE as i16
            } else if level == Level::Ground {
                PROFILE_Z_SIZE as i16
            } else {
                return false;
            };
            if !self.okay_to_add_structure_to_tile(base_grid_no, cube_offset, db_structure, i) {
                return false;
            }
        }
        true
    }

    /// Port of `OkayToAddStructureToTile` for structures of tiles (no soldiers or exclusions)
    fn okay_to_add_structure_to_tile(
        &self,
        base_grid_no: GridNo,
        cube_offset: i16,
        db_structure: &TileStructure,
        structure_tile: usize,
    ) -> bool {
        let tiles = &db_structure.tiles;
        let Some(grid_no) = base_grid_no.offset(i32::from(tiles[structure_tile].pos_rel_to_base))
        else {
            return false;
        };
        if self.tiles[base_grid_no].height != self.tiles[grid_no].height {
            // Uneven terrain, one portion on top of cliff and another not
            return false;
        }
        let flags = db_structure.flags;
        let covers = |other: GridNo| {
            tiles
                .iter()
                .any(|tile| base_grid_no.offset(i32::from(tile.pos_rel_to_base)) == Some(other))
        };
        // The grid numbers on the other side of a wall with the given orientation
        let other_sides = |grid_no: GridNo, orientation: Orientation| {
            (1..4u8).map(move |step| {
                let direction = if orientation.intersects(Orientation::LEFT) {
                    Direction::from_number(step + 2)
                } else if orientation.intersects(Orientation::RIGHT) {
                    Direction::from_number(step)
                } else {
                    Direction::SouthEast
                };
                grid_no.step(direction)
            })
        };

        for existing in &self.tiles[grid_no].structures {
            if cube_offset != existing.cube_offset {
                continue;
            }
            if flags.contains(StructureFlags::MOBILE)
                && existing.flags.contains(StructureFlags::PASSABLE)
            {
                continue;
            }

            if flags.intersects(StructureFlags::OBSTACLE) {
                if existing.flags.intersects(StructureFlags::OBSTACLE)
                    && !flags.contains(StructureFlags::MOBILE)
                {
                    // Two obstacles are only allowed if the existing one is passable
                    if !existing.flags.contains(StructureFlags::PASSABLE)
                        || existing.flags.contains(StructureFlags::MOBILE)
                    {
                        return false;
                    }
                } else if tiles.len() > 1 && existing.flags.intersects(StructureFlags::WALLSTUFF) {
                    let open_door = existing.flags.intersects(StructureFlags::ANYDOOR)
                        && existing.flags.contains(StructureFlags::OPEN);
                    // A multi-tile obstacle must not be on both sides of a wall
                    if !open_door && other_sides(grid_no, existing.wall_orientation).any(&covers) {
                        return false;
                    }
                }
            } else if flags.intersects(StructureFlags::WALLSTUFF) {
                if existing.flags.intersects(StructureFlags::WALLSTUFF)
                    && existing.wall_orientation == db_structure.wall_orientation
                {
                    // Two walls with the same alignment aren't allowed in the same tile
                    return false;
                } else if !existing
                    .flags
                    .intersects(StructureFlags::CORPSE | StructureFlags::PERSON)
                {
                    // The wall must not be on top of a multi-tile obstacle
                    let straddles =
                        other_sides(grid_no, db_structure.wall_orientation).any(|other| {
                            self.tiles[other]
                                .find_structure_by_id(existing.id)
                                .is_some_and(|s| !s.flags.contains(StructureFlags::WALL))
                        });
                    if straddles {
                        return false;
                    }
                }
            }

            if flags.contains(StructureFlags::OPENABLE)
                && existing.flags.contains(StructureFlags::OPENABLE)
            {
                // Two openable structures in the same tile would break the interface
                return false;
            }
        }
        true
    }

    /// Removes the shadows of trees that are surrounded by structures (`OptimizeMapForShadows`)
    fn optimize_map_for_shadows(&mut self) {
        for grid_no in GridNo::all() {
            self.optimize_tile_for_shadows(grid_no);
        }
    }

    /// `OptimizeMapForShadows` for one tile, which depends on the structures of its neighbours
    fn optimize_tile_for_shadows(&mut self, grid_no: GridNo) {
        const DIRECTIONS: [Direction; 5] = [
            Direction::West,
            Direction::SouthWest,
            Direction::South,
            Direction::SouthEast,
            Direction::East,
        ];
        if self.tiles[grid_no]
            .find_structure(StructureFlags::TREE)
            .is_none()
        {
            return;
        }
        let surrounded = DIRECTIONS
            .iter()
            .all(|&direction| !self.tiles[grid_no.step(direction)].structures.is_empty());
        if surrounded {
            self.tiles[grid_no].shadows.clear();
        }
    }
}

/// The tiles that a structure placed at `base_grid_no` covers
fn structure_footprint(base_grid_no: GridNo, tile: &Tile) -> impl Iterator<Item = GridNo> + '_ {
    tile.structure
        .iter()
        .flat_map(|structure| &structure.tiles)
        .filter_map(move |tile| base_grid_no.offset(i32::from(tile.pos_rel_to_base)))
}

/// The layers of a tile whose nodes place structures, in the order `MapWorld::build` adds them
const STRUCTURE_LAYERS: usize = 3;
/// How far the tiles of a structure can be from its base tile, in tiles in each direction
const MAX_STRUCTURE_REACH: i32 = 10;

impl WorldTile {
    fn structure_layer(&self, layer: usize) -> &Vec<LevelNode> {
        match layer {
            0 => &self.structs,
            1 => &self.roofs,
            _ => &self.on_roofs,
        }
    }

    /// Removes the nodes of items, returns their indices in `TacticalMap::items`
    fn take_items(&mut self) -> Vec<u16> {
        let mut items = vec![];
        for nodes in [&mut self.structs, &mut self.on_roofs] {
            nodes.retain(|node| match node.item {
                Some(item) if node.flags.contains(NodeFlags::ITEM) => {
                    items.push(item);
                    false
                }
                _ => true,
            });
        }
        items
    }
}

impl MapWorld {
    /// Updates the level nodes after the layers of the `changed` tiles of the map changed
    ///
    /// The result is the same as building the world again (`build`, then `add_items`), except for
    /// the structure ids, with one exception: the structs of the changed tiles are placed after
    /// the structures of the other tiles. If they overlap, the game would have kept the one of
    /// the lower grid number. The editor never adds overlapping structs.
    ///
    /// Returns the tiles whose level nodes or structures may have changed. The lights of the
    /// new nodes are not set.
    pub fn sync_tiles(
        &mut self,
        map: &TacticalMap,
        changed: &BTreeSet<GridNo>,
    ) -> BTreeSet<GridNo> {
        let mut is_changed = GridArray::filled(false);
        for &grid_no in changed {
            is_changed[grid_no] = true;
        }
        let mut touched = is_changed.clone();

        // Whether a struct is placed depends on the structs placed before it, so the structs of
        // the tiles whose structs overlap those of the changed tiles are placed again as well
        let restacked = self.overlapping_struct_tiles(map, changed);

        // The structures that the nodes of these tiles placed, on all tiles they cover
        for &grid_no in &restacked {
            let layers = if is_changed[grid_no] {
                STRUCTURE_LAYERS
            } else {
                1
            };
            let tile = &self.tiles[grid_no];
            let placed: Vec<(u16, Arc<Tile>)> = (0..layers)
                .flat_map(|layer| tile.structure_layer(layer))
                .filter(|node| !node.flags.contains(NodeFlags::ITEM))
                .filter_map(|node| Some((node.structure_id?, node.tile.clone())))
                .collect();
            for (id, node_tile) in placed {
                for covered in structure_footprint(grid_no, &node_tile) {
                    self.tiles[covered].structures.retain(|s| s.id != id);
                    touched[covered] = true;
                }
            }
            touched[grid_no] = true;
        }

        // Items are placed again later, on their own tiles or on those of the structures they
        // lie on
        let mut removed_items = BTreeSet::new();
        for grid_no in touched.flagged() {
            removed_items.extend(self.take_items_from(grid_no));
        }
        let cleared = touched.clone();

        // The layers of the changed tiles, in the order of `build`
        for &grid_no in changed {
            let tile = &mut self.tiles[grid_no];
            tile.height = i16::from(map.tiles[grid_no].height);
            tile.land.clear();
            tile.land_start = None;
            tile.objects.clear();
            tile.structs.clear();
            tile.shadows.clear();
            tile.roofs.clear();
            tile.on_roofs.clear();
            tile.no_burn_struct = false;
        }
        for &grid_no in changed {
            for land in &map.tiles[grid_no].land {
                self.add_land_to_head(grid_no, land);
            }
        }
        for &grid_no in changed {
            for object in &map.tiles[grid_no].objects {
                self.tiles[grid_no]
                    .objects
                    .push(LevelNode::new(object.clone()));
            }
        }
        for &grid_no in &restacked {
            let tile = &mut self.tiles[grid_no];
            tile.structs.clear();
            tile.no_burn_struct = false;
        }
        for &grid_no in &restacked {
            for structure in &map.tiles[grid_no].structs {
                if self.add_struct_to_tail(grid_no, structure).is_ok() {
                    for covered in structure_footprint(grid_no, structure) {
                        touched[covered] = true;
                    }
                }
            }
        }
        for &grid_no in changed {
            for shadow in &map.tiles[grid_no].shadows {
                self.tiles[grid_no]
                    .shadows
                    .push(LevelNode::new(shadow.clone()));
            }
        }
        for &grid_no in changed {
            for roof in &map.tiles[grid_no].roofs {
                if let Ok(node) = self.add_node_to_world(grid_no, roof, Level::Roof) {
                    self.tiles[grid_no].roofs.push(node);
                    for covered in structure_footprint(grid_no, roof) {
                        touched[covered] = true;
                    }
                }
            }
        }
        for &grid_no in changed {
            for on_roof in &map.tiles[grid_no].on_roofs {
                if let Ok(node) = self.add_node_to_world(grid_no, on_roof, Level::Roof) {
                    self.tiles[grid_no].on_roofs.push(node);
                    for covered in structure_footprint(grid_no, on_roof) {
                        touched[covered] = true;
                    }
                }
            }
        }
        for grid_no in touched.flagged() {
            self.sort_structures(grid_no);
        }

        self.replace_items(map, &mut touched, cleared, removed_items);

        // Shadows of trees next to changed structures, `OptimizeMapForShadows` might have
        // removed them or would remove them now
        let mut is_neighbour = GridArray::filled(false);
        for grid_no in touched.flagged() {
            is_neighbour[grid_no] = true;
            for direction in Direction::ALL {
                is_neighbour[grid_no.step(direction)] = true;
            }
        }
        for grid_no in is_neighbour.flagged() {
            if !is_changed[grid_no] {
                let before = shadow_ids(&self.tiles[grid_no]);
                self.restore_shadows(map, grid_no);
                self.optimize_tile_for_shadows(grid_no);
                if shadow_ids(&self.tiles[grid_no]) != before {
                    touched[grid_no] = true;
                }
            } else {
                self.optimize_tile_for_shadows(grid_no);
            }
        }
        touched.flagged().collect()
    }

    /// The changed tiles and the tiles whose structs could overlap those of the changed tiles,
    /// or those of other tiles in the result
    ///
    /// Both the structs of the map and the placed ones count, so the result also contains tiles
    /// whose structs were left out because they overlap others.
    fn overlapping_struct_tiles(
        &self,
        map: &TacticalMap,
        changed: &BTreeSet<GridNo>,
    ) -> BTreeSet<GridNo> {
        let map_footprints = |grid_no: GridNo| {
            map.tiles[grid_no]
                .structs
                .iter()
                .flat_map(move |tile| structure_footprint(grid_no, tile))
        };
        // Dense flags instead of set lookups, the scan below checks hundreds of candidates for
        // every covered tile
        let mut in_result = GridArray::filled(false);
        for &grid_no in changed {
            in_result[grid_no] = true;
        }
        let mut covered = GridArray::filled(false);
        let cover = |covered: &mut GridArray<bool>, tile: GridNo, new: &mut Vec<GridNo>| {
            if !std::mem::replace(&mut covered[tile], true) {
                new.push(tile);
            }
        };
        let mut new: Vec<GridNo> = vec![];
        for &grid_no in changed {
            let placed = self.tiles[grid_no]
                .structs
                .iter()
                .flat_map(|node| structure_footprint(grid_no, &node.tile));
            for covered_tile in map_footprints(grid_no).chain(placed) {
                cover(&mut covered, covered_tile, &mut new);
            }
        }
        let mut added: Vec<GridNo> = vec![];
        while let Some(covered_tile) = new.pop() {
            let reach = MAX_STRUCTURE_REACH;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let Some(candidate) = covered_tile.offset(dy * 160 + dx) else {
                        continue;
                    };
                    if in_result[candidate]
                        || map.tiles[candidate].structs.is_empty()
                        || !map_footprints(candidate).any(|tile| covered[tile])
                    {
                        continue;
                    }
                    in_result[candidate] = true;
                    added.push(candidate);
                    for tile in map_footprints(candidate) {
                        cover(&mut covered, tile, &mut new);
                    }
                }
            }
        }
        let mut result = changed.clone();
        result.extend(added);
        result
    }

    /// Updates the item nodes after the world items of the map changed, returns the tiles whose
    /// nodes may have changed
    ///
    /// The nodes know their item by its index in `TacticalMap::items`, which changes for all
    /// later items when one is removed, so all items are placed again.
    pub fn sync_items(&mut self, map: &TacticalMap) -> BTreeSet<GridNo> {
        let holding: Vec<GridNo> = self
            .tiles
            .iter()
            .filter(|(_, tile)| {
                tile.structs
                    .iter()
                    .chain(&tile.on_roofs)
                    .any(|node| node.flags.contains(NodeFlags::ITEM))
            })
            .map(|(grid_no, _)| grid_no)
            .collect();
        let mut touched = GridArray::filled(false);
        for grid_no in holding {
            self.take_items_from(grid_no);
            touched[grid_no] = true;
        }
        let all = (0..map.items.len()).map(|index| index as u16).collect();
        let cleared = touched.clone();
        self.replace_items(map, &mut touched, cleared, all);
        touched.flagged().collect()
    }

    /// Removes the item nodes of a tile together with their structures
    fn take_items_from(&mut self, grid_no: GridNo) -> Vec<u16> {
        let item_structures: Vec<(u16, Arc<Tile>)> =
            [&self.tiles[grid_no].structs, &self.tiles[grid_no].on_roofs]
                .into_iter()
                .flatten()
                .filter(|node| node.flags.contains(NodeFlags::ITEM))
                .filter_map(|node| Some((node.structure_id?, node.tile.clone())))
                .collect();
        for (id, tile) in item_structures {
            for covered in structure_footprint(grid_no, &tile) {
                self.tiles[covered].structures.retain(|s| s.id != id);
            }
        }
        self.tiles[grid_no].take_items()
    }

    /// Places the removed items again, the items of the touched tiles and the items that now
    /// are or were on the same tiles
    ///
    /// The items of a tile are in the order of the map, so all items of a tile are placed again
    /// once one of them is. `cleared` are the tiles whose items were already removed.
    fn replace_items(
        &mut self,
        map: &TacticalMap,
        touched: &mut GridArray<bool>,
        mut cleared: GridArray<bool>,
        mut removed: BTreeSet<u16>,
    ) {
        // Structures were placed onto these tiles since the items were removed
        for grid_no in touched.flagged() {
            if !std::mem::replace(&mut cleared[grid_no], true) {
                removed.extend(self.take_items_from(grid_no));
            }
        }
        // Where the items are now
        let mut item_tiles: HashMap<u16, GridNo> = HashMap::new();
        for (grid_no, tile) in self.tiles.iter() {
            for node in tile.structs.iter().chain(&tile.on_roofs) {
                if let (Some(item), true) = (node.item, node.flags.contains(NodeFlags::ITEM)) {
                    item_tiles.insert(item, grid_no);
                }
            }
        }
        let mut targets: Vec<Option<GridNo>> = Vec::with_capacity(map.items.len());
        for item in &map.items {
            targets.push(
                item.graphic
                    .as_ref()
                    .map(|_| item_placement(self, item).grid_no),
            );
        }

        loop {
            let mut more = BTreeSet::new();
            for (index, target) in targets.iter().enumerate() {
                let Some(target) = *target else {
                    continue;
                };
                let index = index as u16;
                let current = item_tiles.get(&index).copied();
                let affected = removed.contains(&index)
                    || touched[target]
                    || current.is_some_and(|current| touched[current]);
                if !affected {
                    continue;
                }
                removed.insert(index);
                for grid_no in [Some(target), current].into_iter().flatten() {
                    if !cleared[grid_no] {
                        more.insert(grid_no);
                    }
                }
            }
            if more.is_empty() {
                break;
            }
            for grid_no in more {
                for item in self.take_items_from(grid_no) {
                    item_tiles.remove(&item);
                    removed.insert(item);
                }
                cleared[grid_no] = true;
                touched[grid_no] = true;
            }
        }
        for &index in &removed {
            item_tiles.remove(&index);
        }
        for index in removed {
            add_item(self, map, usize::from(index));
        }
    }

    /// Sorts the structures of a tile in the order `build` and `add_items` place them
    fn sort_structures(&mut self, grid_no: GridNo) {
        let mut structures = std::mem::take(&mut self.tiles[grid_no].structures);
        structures.sort_by_cached_key(|structure| {
            let base = &self.tiles[structure.base_grid_no];
            let node = (0..STRUCTURE_LAYERS).find_map(|layer| {
                let nodes = base.structure_layer(layer);
                let index = nodes
                    .iter()
                    .position(|node| node.structure_id == Some(structure.id))?;
                let node = &nodes[index];
                Some(match node.item {
                    // Items come last, in the order of the map
                    Some(item) if node.flags.contains(NodeFlags::ITEM) => {
                        (STRUCTURE_LAYERS, 0, usize::from(item))
                    }
                    _ => (layer, structure.base_grid_no.index(), index),
                })
            });
            node.unwrap_or((usize::MAX, 0, 0))
        });
        self.tiles[grid_no].structures = structures;
    }

    /// Sets the shadows of a tile to the ones `build` adds: the buddy shadows of the structs
    /// (which `add_struct_to_tail` adds to the head), then the shadows of the map
    fn restore_shadows(&mut self, map: &TacticalMap, grid_no: GridNo) {
        let indoors = self.grid_no_indoors(grid_no);
        let tile = &mut self.tiles[grid_no];
        let mut shadows: Vec<LevelNode> = if indoors {
            vec![]
        } else {
            tile.structs
                .iter()
                .rev()
                .filter_map(|node| node.tile.shadow_buddy.clone())
                .map(|buddy| {
                    let mut shadow = LevelNode::new(buddy);
                    shadow.flags |= NodeFlags::BUDDY_SHADOW;
                    shadow
                })
                .collect()
        };
        shadows.extend(
            map.tiles[grid_no]
                .shadows
                .iter()
                .map(|shadow| LevelNode::new(shadow.clone())),
        );
        tile.shadows = shadows;
    }
}

/// The tiles of the shadows of a tile, to see whether they changed
fn shadow_ids(tile: &WorldTile) -> Vec<(u16, NodeFlags)> {
    tile.shadows
        .iter()
        .map(|node| (node.tile.id, node.flags))
        .collect()
}

/// Height of a structure tile in cubes, 1 to 4, from its profile (`StructureHeight`)
pub fn structure_height(structure: &Structure) -> u8 {
    let Some(tile) = structure.structure_tile() else {
        return 0;
    };
    // Highest cube level of any column (`AtHeight`)
    let highest = tile
        .shape
        .iter()
        .flatten()
        .filter_map(|&column| (0..PROFILE_Z_SIZE).rev().find(|&z| column & (1 << z) != 0))
        .max();
    highest.map_or(0, |z| z as u8 + 1)
}

impl MapWorld {
    /// Height of a structure, computed and cached like `StructureHeight` does
    pub fn cache_structure_height(&mut self, grid_no: GridNo, structure: usize) -> u8 {
        let structure = &mut self.tiles[grid_no].structures[structure];
        if structure.structure_height == 0 {
            structure.structure_height = structure_height(structure);
        }
        structure.structure_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tileset::tests::{test_node, test_tileset, FIRSTTEXTURE};

    #[test]
    fn land_start_follows_the_topmost_full_tile() {
        let tileset = test_tileset();
        let node = |sub_index| test_node(&tileset, FIRSTTEXTURE, sub_index);
        let mut tile = WorldTile::default();
        tile.add_land_to_head(node(1), false);
        assert_eq!(tile.land_start, None);
        tile.add_land_to_head(node(2), true);
        tile.add_land_to_head(node(3), false);
        tile.add_land_to_head(node(4), false);
        assert_eq!(tile.land_start, Some(2));
        assert_eq!(tile.land[2].tile.id, node(2).tile.id);
        assert_eq!(
            tile.land
                .iter()
                .map(|node| node.tile.id)
                .collect::<Vec<_>>(),
            [4, 3, 2, 1].map(|sub_index| node(sub_index).tile.id)
        );
    }
}
