//! The map being edited and the node list helpers of the original editor
//!
//! The editor changes the tile layers of the [`TacticalMap`], which is the document. Every change
//! goes the same way: the tile is recorded for undo, changed, and once a change is committed,
//! [`MapWorld::sync_tiles`] updates the level nodes of the world, which the scene is built from.
//!
//! The helpers are ports of the ones in `WorldMan.cc` and `TileDef.cc` that the editor uses, see
//! [`MapTile`] for the order of the layers. The game keeps a logical height per tile type, but it
//! sets all of them to 1 before the editor runs (`gTileTypeLogicalHeight`), so "higher" land never
//! exists and those code paths are left out.

use std::{collections::BTreeSet, sync::Arc};

use bevy::prelude::Resource;

use crate::{
    map::{
        AmbientLight, EditorSmoothing, EntryPoint, ExitGrid, GridNo, Level, LightId, MapInfo,
        MapItem, MapLight, MapTile, PlacementId, RoomNo, Schedule, SoldierPlacement, TacticalMap,
        WorldItemId,
    },
    random::Random,
    render::world::MapWorld,
    tileset::{Tile, TileCategory, Tileset},
};

use super::{
    lights::SURFACE_AMBIENT_LEVEL,
    undo::{UndoCommand, UndoStack},
};

/// The state of the editor that belongs to the loaded map
#[derive(Resource, Clone, Debug)]
pub struct EditorDocument {
    pub random: Random,
    pub undo: UndoStack,
    /// Name of the tile type of the land below everything else (`gCurrentBackground`)
    pub background: String,
    /// The door the host is asked to edit, see `doors.rs`
    pub door_request: Option<GridNo>,
    /// Counts the changes of the map, so the host can tell whether it was changed since it was
    /// saved
    pub revision: u64,
    /// Whether the soldiers changed since the world was last synced with them
    pub soldiers_dirty: bool,
    /// Whether the world items changed since the world was last synced with them
    pub items_dirty: bool,
    /// Whether the lights, the light colour or the ambient light changed since the scene was
    /// last synced with them
    pub lights_dirty: bool,
    /// Whether the exit grids changed since the scene was last synced with them
    pub traversal_dirty: bool,
    /// Whether the map changed in a way that can move the edge points since they were last
    /// generated: the tiles, the exit grids, the entry points, the map type or the door locks
    pub edge_points_dirty: bool,
    /// Whether the scroll restriction changed since the world was last synced with it
    pub restricted_scroll_dirty: bool,
    /// The id of the next new placement, ids are never used twice
    next_placement_id: u32,
    /// The id of the next new world item
    next_item_id: u32,
    /// The id of the next new light
    next_light_id: u32,
}

impl EditorDocument {
    pub fn new(seed: u64) -> Self {
        Self {
            random: Random::new(seed),
            undo: UndoStack::default(),
            background: "TEXTURE1".to_owned(),
            door_request: None,
            revision: 0,
            soldiers_dirty: false,
            items_dirty: false,
            lights_dirty: false,
            traversal_dirty: false,
            edge_points_dirty: false,
            restricted_scroll_dirty: false,
            next_placement_id: 0,
            next_item_id: 0,
            next_light_id: 0,
        }
    }

    /// Starts editing a map: new placements, items and lights get ids after the ones of the map
    pub fn for_map(seed: u64, map: &TacticalMap) -> Self {
        let next_placement_id = map
            .soldiers
            .iter()
            .map(|placement| placement.id.0 + 1)
            .max()
            .unwrap_or(0);
        let next_item_id = map
            .items
            .iter()
            .map(|item| item.id.0 + 1)
            .max()
            .unwrap_or(0);
        let next_light_id = map
            .lights
            .iter()
            .map(|light| light.id.0 + 1)
            .max()
            .unwrap_or(0);
        Self {
            next_placement_id,
            next_item_id,
            next_light_id,
            ..Self::new(seed)
        }
    }

    /// A new placement id
    pub fn new_placement_id(&mut self) -> PlacementId {
        let id = PlacementId(self.next_placement_id);
        self.next_placement_id += 1;
        id
    }
}

impl EditorDocument {
    /// A new world item id
    pub fn new_item_id(&mut self) -> WorldItemId {
        let id = WorldItemId(self.next_item_id);
        self.next_item_id += 1;
        id
    }

    /// A new light id
    /// The id that the next new light gets, without taking it
    pub fn next_light_id(&self) -> LightId {
        LightId(self.next_light_id)
    }

    pub fn new_light_id(&mut self) -> LightId {
        let id = LightId(self.next_light_id);
        self.next_light_id += 1;
        id
    }
}

impl Default for EditorDocument {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Changes the map and keeps the world in sync with it
pub struct EditContext<'a> {
    pub map: &'a mut TacticalMap,
    pub world: &'a mut MapWorld,
    pub tileset: &'a Tileset,
    pub document: &'a mut EditorDocument,
    /// Tiles changed since the last commit
    changed: BTreeSet<GridNo>,
    /// Tiles whose level nodes changed with the commits so far
    touched: BTreeSet<GridNo>,
}

impl<'a> EditContext<'a> {
    pub fn new(
        map: &'a mut TacticalMap,
        world: &'a mut MapWorld,
        tileset: &'a Tileset,
        document: &'a mut EditorDocument,
    ) -> Self {
        Self {
            map,
            world,
            tileset,
            document,
            changed: BTreeSet::new(),
            touched: BTreeSet::new(),
        }
    }

    /// Updates the world with the changed tiles, so that e.g. the structures placed so far block
    /// the next ones
    pub fn commit(&mut self) {
        if self.changed.is_empty() {
            return;
        }
        let changed = std::mem::take(&mut self.changed);
        self.touched
            .extend(self.world.sync_tiles(self.map, &changed));
    }

    /// Commits the changes, returns the tiles whose level nodes changed
    pub fn finish(mut self) -> BTreeSet<GridNo> {
        self.commit();
        self.touched
    }

    /// A number in `0..range` (`Random`)
    pub fn random(&mut self, range: u32) -> u32 {
        self.document.random.next(range)
    }

    /// A tile of the tileset, `None` if it does not exist
    pub fn tile(&self, type_name: &str, sub_index: u16) -> Option<Arc<Tile>> {
        let index = self.tileset.tile_of(type_name, sub_index)?;
        self.tileset.tile(index).cloned()
    }

    /// What the tiles of a type are to the editor, `None` if the type has no tiles
    pub fn category_of(&self, type_name: &str) -> Option<TileCategory> {
        let tile = self.tile(type_name, 1)?;
        if tile.type_name != type_name {
            return None;
        }
        tile.category
    }

    /// A random full tile of the background texture
    pub fn background_tile(&mut self) -> Option<Arc<Tile>> {
        let sub_index = 1 + self.random(10) as u16;
        self.tile(&self.document.background, sub_index)
    }

    /// The tile, to be changed: it is recorded for undo and synced with the next commit
    pub fn modify(&mut self, grid_no: GridNo) -> &mut MapTile {
        self.document
            .undo
            .record_tile(grid_no, &self.map.tiles[grid_no]);
        self.document.edge_points_dirty = true;
        self.changed.insert(grid_no);
        &mut self.map.tiles[grid_no]
    }

    /// Restores the tiles and the background of an undo command, without recording it, returns
    /// the command that restores what it replaced (to redo an undo and the other way round)
    ///
    /// The edge points are generated again afterwards, whatever the command changed.
    pub fn restore(&mut self, command: UndoCommand) -> UndoCommand {
        self.document.edge_points_dirty = true;
        let mut inverse = UndoCommand::default();
        for (grid_no, tile) in command.tiles {
            let previous = std::mem::replace(&mut self.map.tiles[grid_no], tile);
            inverse.tiles.push(grid_no, previous);
            self.changed.insert(grid_no);
        }
        if let Some(background) = command.background {
            inverse.background = Some(std::mem::replace(&mut self.document.background, background));
        }
        if let Some(smoothing) = command.smoothing {
            inverse.smoothing = Some(std::mem::replace(
                &mut self.map.info.editor_smoothing,
                smoothing,
            ));
        }
        if let Some(restricted_scroll) = command.restricted_scroll {
            inverse.restricted_scroll = Some(std::mem::replace(
                &mut self.map.info.restricted_scroll,
                restricted_scroll,
            ));
            self.document.restricted_scroll_dirty = true;
        }
        if let Some(doors) = command.doors {
            inverse.doors = Some(std::mem::replace(&mut self.map.doors, doors));
        }
        if let Some((soldiers, schedules)) = command.soldiers {
            inverse.soldiers = Some((
                std::mem::replace(&mut self.map.soldiers, soldiers),
                std::mem::replace(&mut self.map.schedules, schedules),
            ));
            self.document.soldiers_dirty = true;
        }
        if let Some(items) = command.items {
            inverse.items = Some(std::mem::replace(&mut self.map.items, items));
            self.document.items_dirty = true;
        }
        if let Some((lights, color, ambient)) = command.lights {
            inverse.lights = Some((
                std::mem::replace(&mut self.map.lights, lights),
                std::mem::replace(&mut self.map.light_color, color),
                std::mem::replace(&mut self.map.ambient_light, ambient),
            ));
            self.document.lights_dirty = true;
        }
        if let Some(exit_grids) = command.exit_grids {
            inverse.exit_grids = Some(std::mem::replace(&mut self.map.exit_grids, exit_grids));
            self.document.traversal_dirty = true;
        }
        if let Some(info) = command.entry_points {
            inverse.entry_points = Some(self.map.info.clone());
            for point in EntryPoint::ALL {
                *self.map.info.entry_mut(point) = info.entry(point);
            }
        }
        inverse
    }

    /// The exit grids, to be changed: they are recorded for undo, the movement costs of their
    /// tiles are compiled again and the edge points are generated again
    pub fn modify_exit_grids(&mut self) -> &mut Vec<ExitGrid> {
        self.document.undo.record_exit_grids(&self.map.exit_grids);
        self.document.traversal_dirty = true;
        self.document.edge_points_dirty = true;
        &mut self.map.exit_grids
    }

    /// The map info, to change its entry points: they are recorded for undo and the edge points
    /// are generated again
    pub fn modify_map_info(&mut self) -> &mut MapInfo {
        self.document.undo.record_entry_points(&self.map.info);
        self.document.edge_points_dirty = true;
        &mut self.map.info
    }

    /// Records the lights, the light colour and the ambient light for undo, the scene is synced
    /// with them
    fn record_lights(&mut self) {
        self.document.undo.record_lights(
            &self.map.lights,
            self.map.light_color,
            self.map.ambient_light,
        );
        self.document.lights_dirty = true;
    }

    /// The lights, to be changed: they are recorded for undo and the scene is synced with them
    pub fn modify_lights(&mut self) -> &mut Vec<MapLight> {
        self.record_lights();
        &mut self.map.lights
    }

    /// The light colour and the ambient light, to be changed: they are recorded for undo and
    /// the scene is rebuilt with them
    pub fn modify_light_settings(&mut self) -> (&mut [u8; 3], &mut Option<AmbientLight>) {
        self.record_lights();
        (&mut self.map.light_color, &mut self.map.ambient_light)
    }

    /// The world items, to be changed: they are recorded for undo and the world is synced with
    /// them
    pub fn modify_items(&mut self) -> &mut Vec<MapItem> {
        self.document.undo.record_items(&self.map.items);
        self.document.items_dirty = true;
        &mut self.map.items
    }

    /// The soldier placements and the schedules, to be changed: they are recorded for undo and
    /// the world is synced with them
    pub fn modify_soldiers(&mut self) -> (&mut Vec<SoldierPlacement>, &mut Vec<Schedule>) {
        self.document
            .undo
            .record_soldiers(&self.map.soldiers, &self.map.schedules);
        self.document.soldiers_dirty = true;
        (&mut self.map.soldiers, &mut self.map.schedules)
    }

    /// Sets how the editor smooths the map, it is recorded for undo
    ///
    /// Like `SetEditorSmoothingMode`, it also decides whether the map is a basement or caves for
    /// the ambient light: only those have a light level, which they keep when they switch.
    pub fn set_smoothing(&mut self, smoothing: EditorSmoothing) {
        let previous = self.map.info.editor_smoothing;
        if previous != smoothing {
            self.document.undo.record_smoothing(previous);
            // Rooms on the edges of basements have edge points
            self.document.edge_points_dirty = true;
            self.map.info.editor_smoothing = smoothing;
        }
        let ambient = match smoothing {
            EditorSmoothing::Normal => None,
            EditorSmoothing::Basement | EditorSmoothing::Caves => Some(AmbientLight {
                basement: smoothing == EditorSmoothing::Basement,
                caves: smoothing == EditorSmoothing::Caves,
                level: self
                    .map
                    .ambient_light
                    .map_or(SURFACE_AMBIENT_LEVEL, |ambient| ambient.level),
            }),
        };
        if ambient != self.map.ambient_light {
            *self.modify_light_settings().1 = ambient;
        }
    }

    /// Sets whether the game only scrolls over the middle of the map (`ubRestrictedScrollID`),
    /// it is recorded for undo
    ///
    /// The visible tiles depend on it, so the scene is rebuilt and the edge points are generated
    /// again.
    pub fn set_restricted_scroll(&mut self, restricted_scroll: bool) {
        let previous = self.map.info.restricted_scroll;
        if previous == restricted_scroll {
            return;
        }
        self.document.undo.record_restricted_scroll(previous);
        self.document.restricted_scroll_dirty = true;
        self.document.edge_points_dirty = true;
        self.map.info.restricted_scroll = restricted_scroll;
    }

    pub fn land(&self, grid_no: GridNo) -> &[Arc<Tile>] {
        &self.map.tiles[grid_no].land
    }

    /// Index in `MapTile::land` of the land of a type closest to the head (`FindTypeInLandLayer`)
    pub fn find_type_in_land(&self, grid_no: GridNo, type_name: &str) -> Option<usize> {
        self.land(grid_no)
            .iter()
            .rposition(|tile| tile.type_name == type_name)
    }

    /// Index in `MapTile::land` of the land of a category closest to the head
    pub fn find_category_in_land(&self, grid_no: GridNo, category: TileCategory) -> Option<usize> {
        self.land(grid_no)
            .iter()
            .rposition(|tile| tile.category == Some(category))
    }

    /// Whether any land matches (`TypeRangeExistsInLandLayer`)
    pub fn land_exists(&self, grid_no: GridNo, matches: impl Fn(&Tile) -> bool) -> bool {
        self.land(grid_no).iter().any(|tile| matches(tile))
    }

    /// Type of the topmost land tile
    pub fn land_head_type(&self, grid_no: GridNo) -> Option<&str> {
        self.land(grid_no)
            .last()
            .map(|tile| tile.type_name.as_str())
    }

    /// How far a type is below the head, 0 for the head (`GetTypeLandLevel`)
    pub fn land_level(&self, grid_no: GridNo, type_name: &str) -> Option<usize> {
        let index = self.find_type_in_land(grid_no, type_name)?;
        Some(self.land(grid_no).len() - 1 - index)
    }

    /// Replaces the land of the type of the tile, or adds the tile to the head (`SetLandIndex`)
    pub fn set_land_index(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        match self.find_type_in_land(grid_no, &tile.type_name) {
            Some(index) => {
                // `ReplaceLandIndex`
                self.modify(grid_no).land[index] = tile;
                self.adjust_for_full_tile(grid_no);
            }
            None => self.add_land_to_head(grid_no, tile),
        }
    }

    /// `AddLandToHead`
    pub fn add_land_to_head(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).land.push(tile);
    }

    /// `AddLandToTail`
    pub fn add_land_to_tail(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).land.insert(0, tile);
    }

    /// Removes a land tile by its index in `MapTile::land` (`RemoveLand`)
    pub fn remove_land(&mut self, grid_no: GridNo, index: usize) {
        self.modify(grid_no).land.remove(index);
        self.adjust_for_full_tile(grid_no);
    }

    /// Adds a background tile to the tail if no land covers the whole tile (`AdjustForFullTile`)
    fn adjust_for_full_tile(&mut self, grid_no: GridNo) {
        if self.land(grid_no).iter().any(|tile| tile.is_full_land()) {
            return;
        }
        if let Some(tile) = self.background_tile() {
            self.add_land_to_tail(grid_no, tile);
        }
    }

    /// `DeleteAllLandLayers`
    pub fn delete_all_land(&mut self, grid_no: GridNo) {
        self.modify(grid_no).land.clear();
    }

    /// Removes all land that matches, without adjusting for a full tile
    /// (`RemoveAllLandsOfTypeRange`), returns whether there was any
    pub fn remove_land_where(&mut self, grid_no: GridNo, matches: impl Fn(&Tile) -> bool) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.land)
    }

    /// Whether the tile has a floor (`FloorAtGridNo`)
    pub fn has_floor(&self, grid_no: GridNo) -> bool {
        self.land_exists(grid_no, Tile::is_floor)
    }

    /// Removes the objects that match (`RemoveAllObjectsOfTypeRange`), returns whether there were
    /// any
    pub fn remove_objects_where(
        &mut self,
        grid_no: GridNo,
        matches: impl Fn(&Tile) -> bool,
    ) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.objects)
    }

    /// Removes the structs that match (`RemoveAllStructsOfTypeRange`), returns whether there were
    /// any. Their buddy shadows go with them when the world is synced.
    pub fn remove_structs_where(
        &mut self,
        grid_no: GridNo,
        matches: impl Fn(&Tile) -> bool,
    ) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.structs)
    }

    /// `AddObjectToHead`
    pub fn add_object_to_head(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).objects.insert(0, tile);
    }

    /// `AddObjectToTail`
    pub fn add_object_to_tail(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).objects.push(tile);
    }

    /// `AddStructToHead`, the world adds the structure and the buddy shadow when it is synced
    pub fn add_struct_to_head(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).structs.insert(0, tile);
    }

    /// `AddStructToTail`
    pub fn add_struct_to_tail(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).structs.push(tile);
    }

    pub fn structs(&self, grid_no: GridNo) -> &[Arc<Tile>] {
        &self.map.tiles[grid_no].structs
    }

    /// Removes a struct by its index in `MapTile::structs` (`RemoveStruct`)
    pub fn remove_struct(&mut self, grid_no: GridNo, index: usize) {
        self.modify(grid_no).structs.remove(index);
    }

    /// Whether the structure of a tile can be placed on the ground level of a tile, the world is
    /// synced with the changes so far first
    pub fn struct_fits(&mut self, grid_no: GridNo, tile: &Tile) -> bool {
        let Some(structure) = &tile.structure else {
            return true;
        };
        self.commit();
        self.world
            .okay_to_add_structure_to_world(grid_no, Level::Ground, structure)
    }

    /// Adds a wall, door or wall decoration in the order the game draws them: before the walls
    /// of a lower orientation value at right angles, else before a roof or at the tail
    /// (`AddWallToStructLayer`)
    ///
    /// A wall of the same alignment is replaced if `replace` is set, otherwise nothing is added.
    /// Like the game, nothing is added if the structure doesn't fit. Returns whether the tile
    /// was added.
    pub fn add_wall_to_struct_layer(
        &mut self,
        grid_no: GridNo,
        tile: Arc<Tile>,
        replace: bool,
    ) -> bool {
        let orientation = tile.wall_orientation();
        let mut insert_found = false;
        let mut roof_level = None;
        for (level, existing) in self.structs(grid_no).iter().enumerate() {
            let check = existing.wall_orientation();
            // A wall at right angles to the new one
            if check.bits() > orientation.bits()
                && ((orientation.is_vertical() && check.is_horizontal())
                    || (orientation.is_horizontal() && check.is_vertical()))
            {
                insert_found = true;
            }
            if existing.building.is_some_and(|b| b.piece.is_flat_roof()) {
                roof_level = Some(level);
            }
            // Parallel walls, e.g. inside and outside left, can't coexist
            if (orientation.is_vertical() && check.is_vertical())
                || (orientation.is_horizontal() && check.is_horizontal())
            {
                if !replace {
                    return false;
                }
                // `ReplaceStructIndex`
                self.remove_struct(grid_no, level);
                return self.add_wall_to_struct_layer(grid_no, tile, false);
            }
        }
        if !self.struct_fits(grid_no, &tile) {
            return false;
        }
        let index = if insert_found {
            0
        } else {
            roof_level.unwrap_or(self.structs(grid_no).len())
        };
        self.modify(grid_no).structs.insert(index, tile);
        true
    }

    /// Replaces the struct at an index by a wall piece (`ReplaceStructIndex`), which is added
    /// like [`EditContext::add_wall_to_struct_layer`] without replacing
    pub fn replace_struct(&mut self, grid_no: GridNo, index: usize, tile: Arc<Tile>) {
        self.remove_struct(grid_no, index);
        self.add_wall_to_struct_layer(grid_no, tile, false);
    }

    /// Adds a shadow to the head, unless the tile already has it (`AddExclusiveShadow`)
    pub fn add_exclusive_shadow(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        if self.map.tiles[grid_no]
            .shadows
            .iter()
            .any(|shadow| shadow.id == tile.id)
        {
            return;
        }
        self.modify(grid_no).shadows.insert(0, tile);
    }

    /// Removes the shadows that match (`RemoveAllShadowsOfTypeRange`), returns whether there
    /// were any
    pub fn remove_shadows_where(
        &mut self,
        grid_no: GridNo,
        matches: impl Fn(&Tile) -> bool,
    ) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.shadows)
    }

    pub fn roofs(&self, grid_no: GridNo) -> &[Arc<Tile>] {
        &self.map.tiles[grid_no].roofs
    }

    /// `AddRoofToHead`
    pub fn add_roof_to_head(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).roofs.insert(0, tile);
    }

    /// `AddRoofToTail`
    pub fn add_roof_to_tail(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).roofs.push(tile);
    }

    /// Removes the roofs that match (`RemoveAllRoofsOfTypeRange`), returns whether there were
    /// any
    pub fn remove_roofs_where(&mut self, grid_no: GridNo, matches: impl Fn(&Tile) -> bool) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.roofs)
    }

    /// `AddOnRoofToTail`
    pub fn add_on_roof_to_tail(&mut self, grid_no: GridNo, tile: Arc<Tile>) {
        self.modify(grid_no).on_roofs.push(tile);
    }

    /// Removes the tiles on the roofs that match (`RemoveAllOnRoofsOfTypeRange`), returns whether
    /// there were any
    pub fn remove_on_roofs_where(
        &mut self,
        grid_no: GridNo,
        matches: impl Fn(&Tile) -> bool,
    ) -> bool {
        remove_where(self, grid_no, matches, |tile| &mut tile.on_roofs)
    }

    /// Sets the room of a tile, it is only recorded if it changes
    pub fn set_room(&mut self, grid_no: GridNo, room: Option<RoomNo>) {
        if self.map.tiles[grid_no].room != room {
            self.modify(grid_no).room = room;
        }
    }

    /// The first wall or door along the edge to the next column (`GetVerticalWall`)
    pub fn vertical_wall(&self, grid_no: GridNo) -> Option<WallNode> {
        self.wall_node(grid_no, |tile| tile.wall_orientation().is_vertical())
    }

    /// The first wall or door along the edge to the next row (`GetHorizontalWall`)
    pub fn horizontal_wall(&self, grid_no: GridNo) -> Option<WallNode> {
        self.wall_node(grid_no, |tile| tile.wall_orientation().is_horizontal())
    }

    fn wall_node(&self, grid_no: GridNo, aligned: impl Fn(&Tile) -> bool) -> Option<WallNode> {
        wall_node(&self.map.tiles[grid_no], aligned)
    }
}

/// The first wall or door of a map tile with an alignment
pub fn wall_node(tile: &MapTile, aligned: impl Fn(&Tile) -> bool) -> Option<WallNode> {
    let index = tile.structs.iter().position(|tile| {
        tile.building
            .is_some_and(|b| b.piece.is_wall() || b.piece.is_door())
            && aligned(tile)
    })?;
    Some(WallNode {
        index,
        tile: tile.structs[index].clone(),
    })
}

/// A wall or door of the structs of a tile
#[derive(Clone, Debug)]
pub struct WallNode {
    /// Index in `MapTile::structs`
    pub index: usize,
    pub tile: Arc<Tile>,
}

/// Removes the tiles of a layer that match, the tile is only recorded for undo if any do
fn remove_where(
    context: &mut EditContext,
    grid_no: GridNo,
    matches: impl Fn(&Tile) -> bool,
    layer: impl Fn(&mut MapTile) -> &mut Vec<Arc<Tile>>,
) -> bool {
    let tile = &mut context.map.tiles[grid_no];
    if !layer(tile).iter().any(|tile| matches(tile)) {
        return false;
    }
    layer(context.modify(grid_no)).retain(|tile| !matches(tile));
    true
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        file_formats::ja2_tile_types::FIRSTTEXTURE,
        tileset::{tests::test_tileset, TilesetId},
    };

    /// Everything an `EditContext` needs, with a map covered by `FIRSTTEXTURE` 1
    pub(crate) struct Fixture {
        pub map: TacticalMap,
        pub world: MapWorld,
        pub tileset: Tileset,
        pub document: EditorDocument,
    }

    impl Fixture {
        pub(crate) fn new() -> Self {
            Self::with_tileset(test_tileset())
        }

        pub(crate) fn with_tileset(tileset: Tileset) -> Self {
            let mut map = TacticalMap::empty(TilesetId::new(0));
            let grass = tileset
                .tile(tileset.index(FIRSTTEXTURE, 1).unwrap())
                .unwrap()
                .clone();
            for tile in map.tiles.values_mut() {
                tile.land = vec![grass.clone()];
            }
            let world = MapWorld::build(&map);
            Self {
                map,
                world,
                tileset,
                document: EditorDocument::new(1),
            }
        }

        pub(crate) fn context(&mut self) -> EditContext<'_> {
            EditContext::new(
                &mut self.map,
                &mut self.world,
                &self.tileset,
                &mut self.document,
            )
        }

        /// `(type, sub index)` of the land of a tile, from the tail to the head
        pub(crate) fn land(&self, grid_no: GridNo) -> Vec<(&str, u16)> {
            self.map.tiles[grid_no]
                .land
                .iter()
                .map(|tile| (tile.type_name.as_str(), tile.sub_index))
                .collect()
        }
    }

    pub(crate) fn grid(x: u8, y: u8) -> GridNo {
        GridNo::from(GridPosition::new(x, y))
    }

    #[test]
    fn land_helpers_follow_the_order_of_the_land_list() {
        let mut fixture = Fixture::new();
        let g = grid(10, 10);
        let mut context = fixture.context();
        let sand = context.tile("TEXTURE2", 19).unwrap();
        context.set_land_index(g, sand);
        assert_eq!(context.land_head_type(g), Some("TEXTURE2"));
        assert_eq!(context.land_level(g, "TEXTURE2"), Some(0));
        assert_eq!(context.land_level(g, "TEXTURE1"), Some(1));

        // Replacing keeps the position
        let grass = context.tile("TEXTURE1", 12).unwrap();
        context.set_land_index(g, grass);
        let touched = context.finish();
        assert!(touched.contains(&g));
        let land = fixture.land(g);
        assert_eq!(land[1..], [("TEXTURE1", 12), ("TEXTURE2", 19)]);
        // No full tile is left, so a background tile is added to the tail
        assert_eq!(land[0].0, "TEXTURE1");
        assert!((1..=10).contains(&land[0].1));
    }

    #[test]
    fn removing_the_only_full_tile_adds_a_background_tile() {
        let mut fixture = Fixture::new();
        let g = grid(10, 10);
        let mut context = fixture.context();
        let edge = context.tile("TEXTURE2", 19).unwrap();
        context.add_land_to_head(g, edge);
        context.remove_land(g, 0);
        context.finish();
        let land = fixture.land(g);
        assert_eq!(land.len(), 2);
        assert_eq!(land[1], ("TEXTURE2", 19));
        assert_eq!(land[0].0, "TEXTURE1");
        assert!((1..=10).contains(&land[0].1));
        // The sync updated the world
        assert_eq!(fixture.world.tiles[g].land.len(), 2);
    }

    #[test]
    fn restoring_an_undo_command_brings_back_the_tiles() {
        let mut fixture = Fixture::new();
        let g = grid(10, 10);
        let mut context = fixture.context();
        let rock = context.tile("ROCKS", 1).unwrap();
        context.add_object_to_head(g, rock.clone());
        context.add_object_to_head(g, rock);
        context.finish();
        assert_eq!(fixture.world.tiles[g].objects.len(), 2);

        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        let touched = context.finish();
        assert!(touched.contains(&g));
        assert!(fixture.map.tiles[g].objects.is_empty());
        assert!(fixture.world.tiles[g].objects.is_empty());
        // Restoring is not recorded
        assert_eq!(fixture.document.undo.depth(), 0);
    }

    #[test]
    fn the_smoothing_can_be_undone() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        context.set_smoothing(EditorSmoothing::Caves);
        context.set_smoothing(EditorSmoothing::Basement);
        context.finish();
        assert_eq!(fixture.map.info.editor_smoothing, EditorSmoothing::Basement);
        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert_eq!(fixture.map.info.editor_smoothing, EditorSmoothing::Normal);
        // Setting it again changes nothing
        let mut context = fixture.context();
        context.set_smoothing(EditorSmoothing::Normal);
        context.finish();
        assert_eq!(fixture.document.undo.depth(), 0);
    }

    #[test]
    fn the_scroll_restriction_can_be_undone() {
        let mut fixture = Fixture::new();
        let mut context = fixture.context();
        context.set_restricted_scroll(true);
        context.finish();
        assert!(fixture.map.info.restricted_scroll);
        assert!(fixture.document.restricted_scroll_dirty && fixture.document.edge_points_dirty);
        fixture.document.restricted_scroll_dirty = false;
        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert!(!fixture.map.info.restricted_scroll);
        assert!(fixture.document.restricted_scroll_dirty);
        // Setting it again changes nothing
        let mut context = fixture.context();
        context.set_restricted_scroll(false);
        context.finish();
        assert_eq!(fixture.document.undo.depth(), 0);
    }

    #[test]
    fn the_smoothing_decides_the_ambient_light() {
        let mut fixture = Fixture::new();
        assert_eq!(fixture.map.ambient_light, None);
        // Maps above ground become basements with the level of the original editor
        let mut context = fixture.context();
        context.set_smoothing(EditorSmoothing::Basement);
        context.finish();
        let basement = AmbientLight {
            basement: true,
            caves: false,
            level: SURFACE_AMBIENT_LEVEL,
        };
        assert_eq!(fixture.map.ambient_light, Some(basement));
        assert!(fixture.document.lights_dirty);

        // Caves keep the level
        fixture.map.ambient_light = Some(AmbientLight {
            level: 9,
            ..basement
        });
        let mut context = fixture.context();
        context.set_smoothing(EditorSmoothing::Caves);
        context.finish();
        assert_eq!(
            fixture.map.ambient_light,
            Some(AmbientLight {
                basement: false,
                caves: true,
                level: 9
            })
        );

        // Maps above ground have none, undo brings it back with the smoothing
        fixture.document.undo.begin();
        let mut context = fixture.context();
        context.set_smoothing(EditorSmoothing::Normal);
        context.finish();
        assert_eq!(fixture.map.ambient_light, None);
        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert_eq!(fixture.map.info.editor_smoothing, EditorSmoothing::Caves);
        assert_eq!(fixture.map.ambient_light.map(|a| a.level), Some(9));
    }

    #[test]
    fn undo_restores_the_items() {
        use crate::{
            item::ItemId,
            map::{ItemHeight, ItemStack, MapItem, Visibility, WorldItemFlags},
        };
        let mut fixture = Fixture::new();
        let item = MapItem {
            id: WorldItemId(0),
            grid_no: grid(10, 10),
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
            graphic: None,
            render_height: ItemHeight::Above(0),
            visibility: Visibility::Invisible,
            flags: WorldItemFlags::empty(),
            non_exist_chance: 0,
        };
        fixture.map.items.push(item.clone());
        fixture.document = EditorDocument::for_map(1, &fixture.map);
        assert_eq!(fixture.document.new_item_id(), WorldItemId(1));

        let mut context = fixture.context();
        context.modify_items().clear();
        context.finish();
        assert!(fixture.map.items.is_empty());
        assert!(fixture.document.items_dirty);
        fixture.document.items_dirty = false;

        let command = fixture.document.undo.pop().unwrap();
        let mut context = fixture.context();
        context.restore(command);
        context.finish();
        assert_eq!(fixture.map.items, [item]);
        assert!(fixture.document.items_dirty);
    }

    #[test]
    fn removing_by_type_only_records_real_changes() {
        let mut fixture = Fixture::new();
        let g = grid(10, 10);
        let mut context = fixture.context();
        let is_rock = |tile: &Tile| tile.type_name == "ROCKS";
        assert!(!context.remove_objects_where(g, is_rock));
        let rock = context.tile("ROCKS", 1).unwrap();
        let other = context.tile("OSTRUCT1", 1).unwrap();
        context.add_object_to_tail(g, rock);
        context.add_object_to_head(g, other);
        assert!(context.remove_objects_where(g, is_rock));
        context.finish();
        let objects: Vec<_> = fixture.map.tiles[g]
            .objects
            .iter()
            .map(|tile| tile.type_name.as_str())
            .collect();
        assert_eq!(objects, ["OSTRUCT1"]);
        assert_eq!(fixture.document.undo.depth(), 1);
    }
}
