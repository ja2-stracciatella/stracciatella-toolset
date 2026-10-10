//! Soldier placements: placing, erasing, selecting, moving, copying and deleting them with the
//! soldiers mode (the Mercs tab of the original editor, `EditorMercs.cc`)
//!
//! Only the soldiers on the floor of the render options are edited. The `place` tool places a
//! soldier of the team of the mode with the defaults of the mode with a left click, on the roof
//! floor only on flat roofs. With `erase` its brush removes the shown soldiers of the floor on
//! the tiles instead. The `select` tool selects the soldier of the clicked tile, clicking it again
//! selects the next soldier of the tile. Dragging the selected soldier moves it, dragging it while
//! Ctrl (Cmd on macOS) was held on the press copies it, both keep its level. Delete removes the selected soldier, Esc deselects it and 1 to 9 set its
//! patrol points at the cursor (Shift removes them). Every change can be undone, the original
//! could not undo them. The host edits the selected soldier with the soldier commands, see
//! `soldier_events.rs`.

use std::sync::Arc;

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::Deserialize;

use crate::{
    item::ItemTable,
    map::{
        AppearanceContext, Attitude, DetailedPlacement, Direction, GridNo, Level, Orders,
        PlacementId, ProfileId, SoldierPlacement, SoldierStats, TacticalMap, MAX_NUM_SOLDIERS,
        MAX_PATROL_POINTS,
    },
    render::{draw_list::VisibleTeams, soldiers::flat_roof_above, world::MapWorld, RenderOptions},
    soldier::{
        body_type::{SoldierBodyType, SoldierBodyTypeId},
        class::SoldierClassId,
        palette_files::PaletteReplacements,
        team::TeamId,
        SoldierDefinitions, SoldierRegistries,
    },
    states::loading::MapAssets,
    systems::cursor::CursorInfo,
    tileset::{StructureFlags, TilesetId, PROFILE_Z_SIZE},
};

use super::{
    brush::{Brush, BrushArea},
    commands::CurrentMode,
    document::EditContext,
    mode::EditorMode,
    schedules::ScheduleEditing,
    MapEdit,
};

/// Why a soldier could not be placed, moved or copied
const CANT_STAND: &str = "A soldier can't stand there";
/// Why a soldier or an item can't go onto the roof of a tile
pub const NO_FLAT_ROOF: &str = "There is no flat roof here";

/// Most soldiers of a team (the team sizes of `g_default_team_info`)
pub const MAX_TEAM_SOLDIERS: usize = 32;
/// Most soldiers of a map (`MAX_NUM_SOLDIERS`)
pub const MAX_SOLDIERS: usize = MAX_NUM_SOLDIERS;
/// Number of civilian groups (`NUM_CIV_GROUPS`)
pub const NUM_CIV_GROUPS: u8 = 20;
/// The tileset of the mines, whose civilians are miners (`CAVES_1`)
const CAVES_TILESET: TilesetId = TilesetId::new(1);

/// The teams the editor places soldiers of (the draw modes `DRAW_MODE_ENEMY` and so on)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoldierTeam {
    #[default]
    Enemy,
    Creature,
    Rebel,
    Civilian,
}

impl SoldierTeam {
    pub const ALL: [Self; 4] = [Self::Enemy, Self::Creature, Self::Rebel, Self::Civilian];

    pub fn team_id(self) -> TeamId {
        match self {
            Self::Enemy => TeamId::ENEMY,
            Self::Creature => TeamId::CREATURE,
            Self::Rebel => TeamId::MILITIA,
            Self::Civilian => TeamId::CIV,
        }
    }

    /// The team of the editor of a team of the game, `None` for the player's teams
    pub fn of(team: TeamId) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.team_id() == team)
    }

    /// The number of the first soldier of the team (`bFirstID`), the soldiers of the editor are
    /// numbered like the game numbers them
    pub fn first_slot(self) -> usize {
        // Our team has 20 soldiers
        20 + MAX_TEAM_SOLDIERS * self as usize
    }

    /// The body types the editor offers, `None` is a random one (`bEnemyArray` and so on)
    pub fn body_types(self) -> &'static [Option<SoldierBodyTypeId>] {
        use SoldierBodyTypeId as B;
        match self {
            Self::Enemy => &[
                None,
                Some(B::REGMALE),
                Some(B::BIGMALE),
                Some(B::STOCKYMALE),
                Some(B::REGFEMALE),
                Some(B::TANK_NW),
                Some(B::TANK_NE),
            ],
            Self::Creature => &[
                Some(B::BLOODCAT),
                Some(B::LARVAE_MONSTER),
                Some(B::INFANT_MONSTER),
                Some(B::YAF_MONSTER),
                Some(B::YAM_MONSTER),
                Some(B::ADULTFEMALEMONSTER),
                Some(B::AM_MONSTER),
                Some(B::QUEENMONSTER),
            ],
            Self::Rebel => &[
                None,
                Some(B::FATCIV),
                Some(B::MANCIV),
                Some(B::REGMALE),
                Some(B::BIGMALE),
                Some(B::STOCKYMALE),
                Some(B::REGFEMALE),
            ],
            Self::Civilian => &[
                None,
                Some(B::FATCIV),
                Some(B::MANCIV),
                Some(B::MINICIV),
                Some(B::DRESSCIV),
                Some(B::HATKIDCIV),
                Some(B::KIDCIV),
                Some(B::REGMALE),
                Some(B::BIGMALE),
                Some(B::STOCKYMALE),
                Some(B::REGFEMALE),
                Some(B::HUMVEE),
                Some(B::ELDORADO),
                Some(B::ICECREAMTRUCK),
                Some(B::JEEP),
                Some(B::CRIPPLECIV),
                Some(B::ROBOTNOWEAPON),
                Some(B::COW),
            ],
        }
    }
}

/// The classes the editor offers for enemies, with their names (`SetEnemyColorCode`)
pub const ENEMY_CLASSES: [(SoldierClassId, &str); 3] = [
    (SoldierClassId::ARMY, "Army"),
    (SoldierClassId::ADMINISTRATOR, "Admin"),
    (SoldierClassId::ELITE, "Elite"),
];

/// What new soldiers get, the original keeps the values last set for a soldier
/// (`gbDefaultOrders` and so on)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SoldierDefaults {
    pub orders: Orders,
    pub attitude: Attitude,
    pub direction: Direction,
    /// 0 to 4, from bad to great
    pub relative_equipment_level: i8,
    pub relative_attribute_level: i8,
    /// The class of new enemies (`gubSoldierClass`)
    pub soldier_class: u8,
    /// The civilian group of new civilians (`gubCivGroup`)
    pub civilian_group: u8,
    /// The body type of new creatures (`gbCurrCreature`)
    pub creature_body: u8,
}

impl Default for SoldierDefaults {
    fn default() -> Self {
        Self {
            orders: Orders::Stationary,
            attitude: Attitude::Defensive,
            direction: Direction::NorthWest,
            relative_equipment_level: 2,
            relative_attribute_level: 2,
            soldier_class: SoldierClassId::ARMY.index() as u8,
            civilian_group: 0,
            creature_body: SoldierBodyTypeId::BLOODCAT.index() as u8,
        }
    }
}

/// What a click in the soldiers mode does
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoldierTool {
    /// Selects, moves, copies and deletes soldiers
    #[default]
    Select,
    /// Places soldiers, or erases them with the brush
    Place,
}

/// The soldiers mode (`TASK_MERCS` of the original editor)
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SoldierMode {
    pub tool: SoldierTool,
    /// Removes the shown soldiers of the brush instead of placing them
    pub erase: bool,
    /// The brush that erases soldiers
    pub brush: Brush,
    /// The team of new soldiers
    pub team: SoldierTeam,
    pub defaults: SoldierDefaults,
}

impl SoldierMode {
    /// The brush that erases soldiers, `None` while placing or selecting, which use clicks
    pub fn brush(&self) -> Option<Brush> {
        match (self.tool, self.erase, self.brush) {
            (SoldierTool::Place, true, Brush::Fill) => Some(Brush::default()),
            (SoldierTool::Place, true, brush) => Some(brush),
            _ => None,
        }
    }

    /// Erases the soldiers of the shown teams of the floor on the tiles of the brush
    pub fn apply(
        &self,
        context: &mut EditContext,
        area: &BrushArea,
        visible: &VisibleTeams,
        floor: Level,
    ) {
        if self.brush().is_some() {
            context.erase_soldiers(area, visible, floor);
        }
    }
}

/// What edits of soldiers need besides the map
#[derive(Clone, Copy)]
pub struct SoldierContext<'a> {
    pub definitions: SoldierDefinitions<'a>,
    pub appearance: AppearanceContext<'a>,
}

/// The soldier definitions, the item table and the palettes as a system parameter
#[derive(SystemParam)]
pub struct SoldierSources<'w> {
    registries: SoldierRegistries<'w>,
    items: Option<Res<'w, ItemTable>>,
    map_assets: Option<Res<'w, MapAssets>>,
    replacements: Res<'w, Assets<PaletteReplacements>>,
}

impl SoldierSources<'_> {
    /// `None` until a map is loaded
    pub fn context(&self) -> Option<SoldierContext<'_>> {
        let definitions = self.registries.definitions();
        Some(SoldierContext {
            definitions,
            appearance: AppearanceContext {
                items: self.items.as_deref()?,
                bodies: definitions.bodies,
                palettes: self
                    .map_assets
                    .as_ref()
                    .and_then(|assets| self.replacements.get(&assets.soldier_palettes)),
            },
        })
    }
}

/// Whether the soldier of a placement is shown with these teams visible
pub fn is_shown(placement: &SoldierPlacement, visible: &VisibleTeams) -> bool {
    placement.team.placed_by_editor && visible.shows(placement.team.id)
}

/// Whether a tile has a level that soldiers and items can be on: the ground always, the roof
/// only with a flat roof (`FlatRoofAboveGridNo`)
pub fn has_level(world: &MapWorld, grid_no: GridNo, level: Level) -> bool {
    level == Level::Ground || flat_roof_above(world, grid_no)
}

/// Whether a soldier can stand on a level of a tile: the level exists, no structure blocks it
/// and, unless `people` is false, no other soldier stands there (`IsLocationSittable`,
/// `IsLocationSittableExcludingPeople`)
pub fn is_sittable(
    map: &TacticalMap,
    world: &MapWorld,
    grid_no: GridNo,
    level: Level,
    people: bool,
) -> bool {
    if !has_level(world, grid_no, level) {
        return false;
    }
    if people
        && map
            .soldiers
            .iter()
            .any(|placement| placement.grid_no == grid_no && placement.level == level)
    {
        return false;
    }
    let cube_offset = i16::from(level.number()) * PROFILE_Z_SIZE as i16;
    !world.tiles[grid_no].structures.iter().any(|structure| {
        structure.flags.intersects(StructureFlags::BLOCKSMOVES)
            && !structure.flags.contains(StructureFlags::PASSABLE)
            && structure.cube_offset == cube_offset
    })
}

/// The first shown soldier on a level of a tile (`IsMercHere`)
pub fn soldier_at(
    map: &TacticalMap,
    grid_no: GridNo,
    visible: &VisibleTeams,
    level: Level,
) -> Option<PlacementId> {
    soldiers_on(map, grid_no, visible, level).first().copied()
}

/// The placement with an id
pub fn placement(map: &TacticalMap, id: PlacementId) -> Option<&SoldierPlacement> {
    map.soldiers.iter().find(|placement| placement.id == id)
}

/// The shown soldiers on a level of a tile, in the order of the placements
fn soldiers_on(
    map: &TacticalMap,
    grid_no: GridNo,
    visible: &VisibleTeams,
    level: Level,
) -> Vec<PlacementId> {
    map.soldiers
        .iter()
        .filter(|p| p.grid_no == grid_no && p.level == level && is_shown(p, visible))
        .map(|p| p.id)
        .collect()
}

/// Why a soldier can't stand on a level of a tile, `None` if it can
fn standing_error(
    map: &TacticalMap,
    world: &MapWorld,
    grid_no: GridNo,
    level: Level,
) -> Option<String> {
    if !has_level(world, grid_no, level) {
        Some(NO_FLAT_ROOF.to_owned())
    } else if !is_sittable(map, world, grid_no, level, true) {
        Some(CANT_STAND.to_owned())
    } else {
        None
    }
}

/// The slot of the game that a soldier gets, its number when the editor creates the soldiers
/// team by team (`ubID`, shown as `Slot #`)
pub fn soldier_slot(map: &TacticalMap, id: PlacementId) -> Option<usize> {
    let placement = placement(map, id)?;
    let team = SoldierTeam::of(placement.team.id)?;
    let index = map
        .soldiers
        .iter()
        .filter(|p| p.team.id == placement.team.id)
        .position(|p| p.id == id)?;
    Some(team.first_slot() + index)
}

/// Why a soldier could not be added
pub fn can_add_soldier(map: &TacticalMap, team: TeamId) -> Result<(), String> {
    if map.soldiers.len() + map.skipped_placements.len() >= MAX_SOLDIERS {
        return Err(format!("The map already has {} soldiers", MAX_SOLDIERS));
    }
    let in_team = map.soldiers.iter().filter(|p| p.team.id == team).count();
    if in_team >= MAX_TEAM_SOLDIERS {
        return Err(format!(
            "The team already has {} soldiers",
            MAX_TEAM_SOLDIERS
        ));
    }
    Ok(())
}

/// The class and body type that soldiers of the team of the mode get, and the civilian group
/// (`AddMercToWorld`)
fn team_values(
    context: &EditContext,
    sc: &SoldierContext,
    mode: &SoldierMode,
) -> (SoldierClassId, Option<Arc<SoldierBodyType>>, u8) {
    let defaults = &mode.defaults;
    match mode.team {
        SoldierTeam::Enemy => {
            let class = ENEMY_CLASSES
                .iter()
                .map(|&(id, _)| id)
                .find(|id| id.index() == usize::from(defaults.soldier_class))
                .unwrap_or(SoldierClassId::ARMY);
            (class, None, 0)
        }
        SoldierTeam::Creature => {
            let body = SoldierTeam::Creature
                .body_types()
                .iter()
                .flatten()
                .find(|id| id.index() == usize::from(defaults.creature_body))
                .unwrap_or(&SoldierBodyTypeId::BLOODCAT);
            (
                SoldierClassId::NONE,
                Some(sc.definitions.bodies.get(*body).clone()),
                0,
            )
        }
        SoldierTeam::Rebel => (SoldierClassId::NONE, None, 0),
        SoldierTeam::Civilian => {
            let class = if context.map.tileset == CAVES_TILESET {
                SoldierClassId::MINER
            } else {
                SoldierClassId::NONE
            };
            (class, None, defaults.civilian_group.min(NUM_CIV_GROUPS - 1))
        }
    }
}

/// A new placement of the team of the mode, with its defaults (`AddMercToWorld`)
fn new_placement(
    context: &mut EditContext,
    sc: &SoldierContext,
    mode: &SoldierMode,
    grid_no: GridNo,
    level: Level,
) -> SoldierPlacement {
    let defaults = &mode.defaults;
    let (class, given_body, civilian_group) = team_values(context, sc, mode);
    let id = context.document.new_placement_id();
    let team = sc.definitions.teams.get(mode.team.team_id()).clone();
    let soldier_class = sc.definitions.classes.get(class).clone();
    let mut placement = SoldierPlacement {
        id,
        index: id.0 as usize,
        grid_no,
        direction: defaults.direction,
        level,
        // Replaced by `resolve_appearance`
        body: given_body.clone().unwrap_or_else(|| {
            sc.definitions
                .bodies
                .get(SoldierBodyTypeId::REGMALE)
                .clone()
        }),
        surface: sc
            .definitions
            .surfaces
            .iter()
            .next()
            .expect("there are standing surfaces")
            .clone(),
        palette: crate::soldier::palette::ResolvedSoldierPalette::Own,
        team,
        soldier_class,
        given_body,
        relative_attribute_level: defaults.relative_attribute_level.clamp(0, 4),
        relative_equipment_level: defaults.relative_equipment_level.clamp(0, 4),
        orders: defaults.orders,
        attitude: defaults.attitude,
        patrol: vec![],
        civilian_group,
        priority_existence: false,
        has_keys: false,
        detailed: None,
    };
    placement.resolve_appearance(&sc.appearance);
    placement
}

/// A detailed placement with nothing fixed yet, the soldier keeps what the game generates
/// (`CreateStaticDetailedPlacementGivenBasicPlacementInfo`)
pub fn static_detailed_placement(placement: &SoldierPlacement) -> DetailedPlacement {
    DetailedPlacement {
        is_static: true,
        profile: None,
        copy_profile_items_over: false,
        sector: None,
        insertion_grid_no: Some(placement.grid_no),
        stats: SoldierStats::default(),
        inventory: std::array::from_fn(|_| None),
        head_palette: String::new(),
        pants_palette: String::new(),
        vest_palette: String::new(),
        skin_palette: String::new(),
        visible: false,
        name: String::new(),
        schedule: None,
        vehicle_id: None,
    }
}

impl EditContext<'_> {
    /// Changes a placement, it is recorded for undo if the change changes it
    pub fn change_soldier(
        &mut self,
        id: PlacementId,
        change: impl FnOnce(&mut SoldierPlacement),
    ) -> bool {
        let Some(index) = self.map.soldiers.iter().position(|p| p.id == id) else {
            return false;
        };
        let mut changed = self.map.soldiers[index].clone();
        change(&mut changed);
        if changed == self.map.soldiers[index] {
            return false;
        }
        self.modify_soldiers().0[index] = changed;
        true
    }

    /// Places a new soldier of the team of the mode on a level of a tile where a soldier can
    /// stand, returns it (`AddMercToWorld`)
    pub fn place_soldier(
        &mut self,
        sc: &SoldierContext,
        mode: &SoldierMode,
        grid_no: GridNo,
        level: Level,
    ) -> Result<PlacementId, String> {
        can_add_soldier(self.map, mode.team.team_id())?;
        if let Some(error) = standing_error(self.map, self.world, grid_no, level) {
            return Err(error);
        }
        let placement = new_placement(self, sc, mode, grid_no, level);
        let id = placement.id;
        self.modify_soldiers().0.push(placement);
        Ok(id)
    }

    /// Copies a soldier where a soldier can stand on its level, returns the copy
    /// (`CopyMercPlacement`, `PasteMercPlacement`)
    ///
    /// Unlike the original, the copy keeps the team, the class, the civilian group and the body
    /// type. It doesn't get the schedule, two soldiers can't follow the same one.
    pub fn copy_soldier(
        &mut self,
        sc: &SoldierContext,
        id: PlacementId,
        grid_no: GridNo,
    ) -> Result<PlacementId, String> {
        let Some(copy) = placement(self.map, id) else {
            return Err("The soldier is gone".to_owned());
        };
        can_add_soldier(self.map, copy.team.id)?;
        if let Some(error) = standing_error(self.map, self.world, grid_no, copy.level) {
            return Err(error);
        }
        let mut placement = copy.clone();
        let id = self.document.new_placement_id();
        placement.id = id;
        placement.index = id.0 as usize;
        placement.grid_no = grid_no;
        if let Some(detailed) = &mut placement.detailed {
            detailed.insertion_grid_no = Some(grid_no);
            detailed.schedule = None;
        }
        placement.resolve_appearance(&sc.appearance);
        self.modify_soldiers().0.push(placement);
        Ok(id)
    }

    /// Moves a soldier where a soldier can stand on its level (`HandleRightClickOnMerc`)
    pub fn move_soldier(&mut self, id: PlacementId, grid_no: GridNo) -> Result<(), String> {
        let Some(level) = placement(self.map, id).map(|p| p.level) else {
            return Ok(());
        };
        if let Some(error) = standing_error(self.map, self.world, grid_no, level) {
            return Err(error);
        }
        self.change_soldier(id, |placement| {
            placement.grid_no = grid_no;
            if let Some(detailed) = &mut placement.detailed {
                detailed.insertion_grid_no = Some(grid_no);
            }
        });
        Ok(())
    }

    /// Removes the soldiers that match and their schedules, only recorded if any match
    fn remove_soldiers(&mut self, matches: impl Fn(&SoldierPlacement) -> bool) {
        if !self.map.soldiers.iter().any(&matches) {
            return;
        }
        let (soldiers, schedules) = self.modify_soldiers();
        let mut removed = vec![];
        soldiers.retain(|placement| {
            if !matches(placement) {
                return true;
            }
            removed.extend(placement.detailed.as_ref().and_then(|d| d.schedule));
            false
        });
        schedules.retain(|schedule| !removed.contains(&schedule.id));
    }

    /// Removes a soldier and its schedule (`DeleteSelectedMerc`)
    pub fn delete_soldier(&mut self, id: PlacementId) {
        self.remove_soldiers(|placement| placement.id == id);
    }

    /// Removes the shown soldiers on a level of the tiles of the brush and their schedules, only
    /// recorded if there are any
    pub fn erase_soldiers(&mut self, area: &BrushArea, visible: &VisibleTeams, level: Level) {
        self.remove_soldiers(|placement| {
            is_shown(placement, visible)
                && placement.level == level
                && area.tiles.contains(&placement.grid_no)
        });
    }

    /// Sets patrol point `number` (1 to 9) to a tile, the points before it that are missing get
    /// the same tile (`AddMercWaypoint`)
    pub fn set_patrol_point(&mut self, id: PlacementId, number: usize, grid_no: GridNo) {
        if !(1..=MAX_PATROL_POINTS).contains(&number) {
            return;
        }
        self.change_soldier(id, |placement| {
            let patrol = &mut placement.patrol;
            if number > patrol.len() {
                patrol.resize(number, grid_no);
            } else {
                patrol[number - 1] = grid_no;
            }
        });
    }

    /// Removes patrol point `number` (1 to 9), the later ones move up (`EraseMercWaypoint`)
    pub fn remove_patrol_point(&mut self, id: PlacementId, number: usize) {
        self.change_soldier(id, |placement| {
            if (1..=placement.patrol.len()).contains(&number) {
                placement.patrol.remove(number - 1);
            }
        });
    }

    /// Gives a soldier a detailed placement with nothing fixed, or removes it
    /// (`InitDetailedPlacementForMerc`, `KillDetailedPlacementForMerc`)
    pub fn set_detailed(&mut self, sc: &SoldierContext, id: PlacementId, detailed: bool) {
        let Some(placement) = placement(self.map, id) else {
            return;
        };
        if placement.detailed.is_some() == detailed {
            return;
        }
        let schedule = placement.detailed.as_ref().and_then(|d| d.schedule);
        self.change_soldier(id, |placement| {
            placement.detailed = detailed.then(|| static_detailed_placement(placement));
            if detailed {
                placement.priority_existence = false;
            }
            placement.resolve_appearance(&sc.appearance);
        });
        if let Some(schedule) = schedule {
            self.modify_soldiers().1.retain(|s| s.id != schedule);
        }
    }

    /// Makes a detailed placement a named character or not. A profile brings its items and
    /// makes the soldier always exist (`ExtractAndUpdateMercProfile`).
    pub fn set_profile(
        &mut self,
        sc: &SoldierContext,
        id: PlacementId,
        profile: Option<ProfileId>,
    ) {
        self.change_soldier(id, |placement| {
            let Some(detailed) = &mut placement.detailed else {
                return;
            };
            detailed.profile = profile;
            detailed.copy_profile_items_over = profile.is_some();
            if profile.is_some() {
                placement.priority_existence = true;
            }
            placement.resolve_appearance(&sc.appearance);
        });
    }
}

/// A drag of the selected soldier in progress
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SoldierDrag {
    /// Where the mouse was pressed
    from: GridNo,
    /// Whether Ctrl or Cmd was held on the press, which copies the soldier instead of moving it
    copy: bool,
    /// Whether the soldier was selected before the press, releasing on the same tile then
    /// selects the next soldier of the tile
    was_selected: bool,
}

/// The selected soldier and what the mouse does next
#[derive(Resource, Clone, Debug, Default)]
pub struct SoldierSelection {
    pub selected: Option<PlacementId>,
    /// What the host is told after a failed edit
    pub notice: Option<String>,
    /// The schedule of the selected soldier
    pub schedule: ScheduleEditing,
    drag: Option<SoldierDrag>,
}

impl SoldierSelection {
    /// Drops the selection when the soldier is gone, e.g. after undo, and keeps the schedule
    /// draft up to date
    pub fn validate(&mut self, map: &TacticalMap) {
        if self.selected.is_some_and(|id| placement(map, id).is_none()) {
            self.selected = None;
            self.drag = None;
        }
        self.schedule.sync(map, self.selected);
    }

    /// Drops the selection, the drag and the schedule draft
    pub fn clear(&mut self) {
        self.selected = None;
        self.drag = None;
        self.schedule = ScheduleEditing::default();
    }

    /// Whether the selected soldier is being dragged
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Selects the first shown soldier on the floor of a tile, or keeps the selected one if it
    /// is there, and starts dragging it, or clears the selection if the tile has no shown
    /// soldiers on the floor
    pub fn press(
        &mut self,
        map: &TacticalMap,
        visible: &VisibleTeams,
        floor: Level,
        tile: GridNo,
        copy: bool,
    ) {
        let on_tile = soldiers_on(map, tile, visible, floor);
        let was_selected = self.selected.is_some_and(|id| on_tile.contains(&id));
        if !was_selected {
            let Some(first) = soldier_at(map, tile, visible, floor) else {
                self.selected = None;
                self.drag = None;
                return;
            };
            self.selected = Some(first);
        }
        self.drag = Some(SoldierDrag {
            from: tile,
            copy,
            was_selected,
        });
    }

    /// Ends the drag at a tile, moving or copying the soldier there and selecting it. Releasing
    /// where the selected soldier already was selects the next shown soldier on the floor of the
    /// tile.
    pub fn release(
        &mut self,
        context: &mut EditContext,
        sc: &SoldierContext,
        visible: &VisibleTeams,
        floor: Level,
        to: GridNo,
    ) {
        let (Some(drag), Some(selected)) = (self.drag.take(), self.selected) else {
            return;
        };
        if to != drag.from {
            let result = if drag.copy {
                context.copy_soldier(sc, selected, to)
            } else {
                context.move_soldier(selected, to).map(|()| selected)
            };
            match result {
                Ok(id) => self.selected = Some(id),
                Err(notice) => self.notice = Some(notice),
            }
            return;
        }
        if drag.was_selected {
            let on_tile = soldiers_on(context.map, to, visible, floor);
            if let Some(position) = on_tile.iter().position(|&id| id == selected) {
                self.selected = Some(on_tile[(position + 1) % on_tile.len()]);
            }
        }
    }

    /// Removes the selected soldier
    pub fn delete(&mut self, context: &mut EditContext) {
        if let Some(selected) = self.selected.take() {
            context.delete_soldier(selected);
        }
        self.drag = None;
    }
}

pub fn reset_soldier_selection(mut selection: ResMut<SoldierSelection>) {
    *selection = SoldierSelection::default();
}

/// The soldiers mode, `None` for the others
pub fn soldier_mode(mode: &CurrentMode) -> Option<&SoldierMode> {
    match &mode.0 {
        EditorMode::Soldiers(mode) => Some(mode),
        _ => None,
    }
}

const DIGITS: [KeyCode; MAX_PATROL_POINTS] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Edits the map with one undo command
pub fn edit_once(edit: &mut MapEdit, change: impl FnOnce(&mut EditContext)) {
    edit.document().undo.begin();
    edit.edit(change);
    edit.document().undo.end();
}

/// The mouse and the keys of the soldiers mode, erasing is done by `paint` with the brush
pub fn edit_soldiers(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    sources: SoldierSources,
    mut selection: ResMut<SoldierSelection>,
    mut edit: MapEdit,
) {
    let Some(mode) = soldier_mode(&mode) else {
        // Leaving the soldiers mode deselects the soldier
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        return;
    };
    let Some(sc) = sources.context() else {
        return;
    };
    let selection = &mut *selection;
    let floor = options.floor;
    let tile = cursor.tile.map(GridNo::from);
    if mode.tool != SoldierTool::Select {
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
                result = context.place_soldier(&sc, mode, tile, floor).map(|_| ())
            });
            if let Err(notice) = result {
                selection.notice = Some(notice);
            }
        }
        return;
    }
    // Cmd on macOS, where Ctrl+click is a right click
    let ctrl = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let visible = &options.layers.soldier_teams;
    if let Some(tile) = tile.filter(|_| mouse.just_pressed(MouseButton::Left)) {
        // The click picks a tile of the schedule first
        let mut picked = false;
        edit_once(&mut edit, |context| {
            picked = selection.schedule.click(context, tile)
        });
        if !picked {
            selection.press(&edit.map, visible, floor, tile, ctrl);
        }
    }
    if selection.drag.is_some() {
        if cursor.world_position.is_none() {
            // Leaving the viewer cancels the drag, like the area brushes
            selection.drag = None;
        } else if !mouse.pressed(MouseButton::Left) {
            match tile {
                Some(to) => edit_once(&mut edit, |context| {
                    selection.release(context, &sc, visible, floor, to)
                }),
                None => selection.drag = None,
            }
        }
    }
    if let (Some(tile), Some(selected)) = (tile, selection.selected.filter(|_| !ctrl)) {
        if let Some(number) = DIGITS.iter().position(|&key| keys.just_pressed(key)) {
            edit_once(&mut edit, |context| {
                if shift {
                    context.remove_patrol_point(selected, number + 1);
                } else {
                    context.set_patrol_point(selected, number + 1, tile);
                }
            });
        }
    }
    if selection.drag.is_none() && keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace]) {
        edit_once(&mut edit, |context| selection.delete(context));
    }
    if keys.just_pressed(KeyCode::Escape) {
        if selection.schedule.pick.is_some() {
            edit_once(&mut edit, |context| selection.schedule.cancel_pick(context));
        } else {
            selection.selected = None;
            selection.drag = None;
        }
    }
    selection.validate(&edit.map);
    // Hidden soldiers and those on the other floor can't stay selected
    // (`SetMercTeamVisibility`)
    if selection
        .selected
        .and_then(|id| placement(&edit.map, id))
        .is_some_and(|p| !is_shown(p, visible) || p.level != floor)
    {
        selection.selected = None;
        selection.drag = None;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        editor::document::tests::{grid, Fixture},
        file_formats::ja2_tile_types::FIRSTFULLSTRUCT,
        item::{HandItem, ItemFile, ItemInfo, ItemTable},
        map::{Schedule, ScheduleFlags, ScheduleId},
        soldier::Registries,
        tileset::tests::structure_tileset,
    };

    /// A map to edit soldiers on, with the soldier definitions
    pub(crate) struct SoldierFixture {
        pub fixture: Fixture,
        pub registries: Registries,
        items: ItemTable,
        pub palettes: PaletteReplacements,
    }

    impl SoldierFixture {
        pub(crate) fn new() -> Self {
            let items = ItemTable::new([&ItemFile {
                items: vec![ItemInfo {
                    index: crate::item::ItemId::new(1).unwrap(),
                    graphic: None,
                    hand: Some(HandItem::OneHandedGun),
                    kind: Default::default(),
                }],
            }]);
            Self {
                fixture: Fixture::with_tileset(structure_tileset()),
                registries: Registries::build(),
                items,
                palettes: PaletteReplacements::coloured(&[
                    "PINKSKIN",
                    "TANSKIN",
                    "DARKSKIN",
                    "BLACKSKIN",
                    "BROWNHEAD",
                    "BLACKHEAD",
                    "WHITEHEAD",
                    "BLONDHEAD",
                    "REDHEAD",
                    "REDVEST",
                    "YELLOWVEST",
                    "BLACKSHIRT",
                    "WHITEVEST",
                    "GREENPANTS",
                    "BLACKPANTS",
                    "TANPANTS",
                    "BLUEPANTS",
                ]),
            }
        }

        /// Edits the map as one undo command
        pub(crate) fn edit<R>(
            &mut self,
            edit: impl FnOnce(&mut EditContext, &SoldierContext) -> R,
        ) -> R {
            let sc = SoldierContext {
                definitions: self.registries.definitions(),
                appearance: AppearanceContext {
                    items: &self.items,
                    bodies: &self.registries.bodies,
                    palettes: Some(&self.palettes),
                },
            };
            self.fixture.document.undo.begin();
            let mut context = self.fixture.context();
            let result = edit(&mut context, &sc);
            context.finish();
            self.fixture.document.undo.end();
            result
        }

        pub(crate) fn map(&self) -> &TacticalMap {
            &self.fixture.map
        }

        pub(crate) fn soldier(&self, id: PlacementId) -> &SoldierPlacement {
            placement(self.map(), id).unwrap()
        }

        pub(crate) fn undo(&mut self) {
            let command = self.fixture.document.undo.pop().unwrap();
            let mut context = self.fixture.context();
            context.restore(command);
            context.finish();
        }
    }

    pub(crate) fn mode(json: &str) -> SoldierMode {
        match serde_json::from_str(json).unwrap() {
            EditorMode::Soldiers(mode) => mode,
            _ => panic!("not the soldiers mode"),
        }
    }

    #[test]
    fn modes_are_read_from_json() {
        let mode = mode(
            r#"{"mode":"soldiers","team":"civilian",
                "defaults":{"orders":"pointPatrol","attitude":"cunningAid","direction":"south",
                    "relativeEquipmentLevel":4,"civilianGroup":3,"creatureBody":8}}"#,
        );
        assert_eq!(mode.team, SoldierTeam::Civilian);
        let defaults = mode.defaults;
        assert_eq!(defaults.orders, Orders::PointPatrol);
        assert_eq!(defaults.attitude, Attitude::CunningAid);
        assert_eq!(defaults.direction, Direction::South);
        assert_eq!(defaults.relative_equipment_level, 4);
        // Missing values are the defaults of the original
        assert_eq!(defaults.relative_attribute_level, 2);
        assert_eq!(defaults.soldier_class, 3);
        let default = super::tests::mode(r#"{"mode":"soldiers"}"#);
        assert_eq!(default, SoldierMode::default());
        assert_eq!(default.defaults.direction, Direction::NorthWest);
        assert_eq!(
            (default.tool, default.erase, default.brush),
            (SoldierTool::Select, false, Brush::default())
        );
        // Only erasing uses the brush
        assert_eq!(default.brush(), None);
        let erasing = super::tests::mode(
            r#"{"mode":"soldiers","tool":"place","erase":true,
                "brush":{"kind":"circle","size":5}}"#,
        );
        assert_eq!(erasing.brush(), Some(Brush::Circle { size: 5 }));
        let select = super::tests::mode(r#"{"mode":"soldiers","tool":"select","erase":true}"#);
        assert_eq!(select.brush(), None);
        let fill = super::tests::mode(
            r#"{"mode":"soldiers","tool":"place","erase":true,"brush":{"kind":"fill"}}"#,
        );
        assert_eq!(fill.brush(), Some(Brush::default()));
    }

    #[test]
    fn placing_uses_the_defaults_of_the_mode() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(
            r#"{"mode":"soldiers","tool":"place","team":"enemy",
                "defaults":{"orders":"onGuard","soldierClass":2,"direction":"east"}}"#,
        );
        let placed = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(50, 50), Level::Ground))
            .unwrap();
        let soldier = fixture.soldier(placed);
        assert_eq!(soldier.team.id, TeamId::ENEMY);
        assert_eq!(soldier.soldier_class.id, SoldierClassId::ELITE);
        assert_eq!(soldier.orders, Orders::OnGuard);
        assert_eq!(soldier.direction, Direction::East);
        assert_eq!(soldier.given_body, None);
        assert!(soldier.detailed.is_none());
        assert_eq!(soldier_slot(fixture.map(), placed), Some(20));
        let second = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(52, 50), Level::Ground))
            .unwrap();
        assert_eq!(soldier_slot(fixture.map(), second), Some(21));

        // Creatures get the body type of the mode
        let creatures = super::tests::mode(
            r#"{"mode":"soldiers","team":"creature","defaults":{"creatureBody":10}}"#,
        );
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &creatures, grid(60, 60), Level::Ground))
            .unwrap();
        let creature = fixture.soldier(id);
        assert_eq!(creature.body.id, SoldierBodyTypeId::QUEENMONSTER);
        assert_eq!(
            creature.given_body.as_ref().unwrap().id,
            SoldierBodyTypeId::QUEENMONSTER
        );
        assert_eq!(soldier_slot(fixture.map(), creature.id), Some(52));
    }

    /// Releases the drag of the selection as one undo command
    fn release(
        fixture: &mut SoldierFixture,
        selection: &mut SoldierSelection,
        visible: &VisibleTeams,
        to: GridNo,
    ) {
        fixture.edit(|context, sc| selection.release(context, sc, visible, Level::Ground, to));
    }

    #[test]
    fn pressing_selects_the_soldiers() {
        let mut fixture = SoldierFixture::new();
        let enemies = mode(r#"{"mode":"soldiers","team":"enemy"}"#);
        let civilians = mode(r#"{"mode":"soldiers","team":"civilian"}"#);
        let (a, b) = (grid(50, 50), grid(40, 60));
        let first = fixture
            .edit(|context, sc| context.place_soldier(sc, &enemies, a, Level::Ground))
            .unwrap();
        let second = fixture
            .edit(|context, sc| context.place_soldier(sc, &civilians, b, Level::Ground))
            .unwrap();
        let all = VisibleTeams::default();
        let mut selection = SoldierSelection::default();
        selection.press(fixture.map(), &all, Level::Ground, b, false);
        assert_eq!(selection.selected, Some(second));
        assert!(selection.dragging());
        release(&mut fixture, &mut selection, &all, b);
        assert_eq!(selection.selected, Some(second));
        assert!(!selection.dragging());
        selection.press(fixture.map(), &all, Level::Ground, a, false);
        assert_eq!(selection.selected, Some(first));
        // Tiles without shown soldiers deselect
        selection.press(fixture.map(), &all, Level::Ground, grid(10, 10), false);
        assert_eq!(selection.selected, None);
        assert!(!selection.dragging());
        let no_enemies = VisibleTeams {
            enemy: false,
            ..all
        };
        selection.press(fixture.map(), &no_enemies, Level::Ground, a, false);
        assert_eq!(selection.selected, None);
        // Nothing was changed
        assert_eq!(fixture.fixture.document.undo.depth(), 2);

        selection.press(fixture.map(), &all, Level::Ground, a, false);
        release(&mut fixture, &mut selection, &all, a);
        fixture.edit(|context, _| selection.delete(context));
        assert_eq!(selection.selected, None);
        assert_eq!(fixture.map().soldiers.len(), 1);
        fixture.undo();
        assert_eq!(fixture.map().soldiers.len(), 2);
        selection.selected = Some(second);
        fixture.undo();
        selection.validate(fixture.map());
        assert_eq!(selection.selected, None);
    }

    #[test]
    fn releasing_on_the_selected_soldier_selects_the_next_one_of_the_tile() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let a = grid(30, 30);
        let roof = fixture
            .fixture
            .tileset
            .tile(
                fixture
                    .fixture
                    .tileset
                    .tile_of("FIRSTROOF", 1)
                    .or_else(|| fixture.fixture.tileset.tile_of("ROOF1", 1))
                    .unwrap(),
            )
            .unwrap()
            .clone();
        fixture.edit(|context, _| context.add_roof_to_head(a, roof));
        let below = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, a, Level::Ground))
            .unwrap();
        let above = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, a, Level::Roof))
            .unwrap();
        let next = fixture
            .edit(|context, sc| {
                context.place_soldier(sc, &mode, a.offset(1).unwrap(), Level::Ground)
            })
            .unwrap();
        // A second soldier on the ground of the tile, which `place_soldier` doesn't allow
        fixture.edit(|context, _| {
            context.change_soldier(next, |p| p.grid_no = a);
        });
        let all = VisibleTeams::default();
        let mut selection = SoldierSelection::default();
        // The first press only selects, then the next soldier on the floor of the tile
        selection.press(fixture.map(), &all, Level::Ground, a, false);
        release(&mut fixture, &mut selection, &all, a);
        assert_eq!(selection.selected, Some(below));
        selection.press(fixture.map(), &all, Level::Ground, a, false);
        release(&mut fixture, &mut selection, &all, a);
        assert_eq!(selection.selected, Some(next));
        selection.press(fixture.map(), &all, Level::Ground, a, false);
        release(&mut fixture, &mut selection, &all, a);
        assert_eq!(selection.selected, Some(below));
        // The soldier on the roof is only selected on the roof floor
        selection.press(fixture.map(), &all, Level::Roof, a, false);
        fixture.edit(|context, sc| selection.release(context, sc, &all, Level::Roof, a));
        assert_eq!(selection.selected, Some(above));
        selection.press(fixture.map(), &all, Level::Roof, a, false);
        fixture.edit(|context, sc| selection.release(context, sc, &all, Level::Roof, a));
        assert_eq!(selection.selected, Some(above));
        assert_eq!(fixture.map().soldiers.len(), 3);
    }

    #[test]
    fn dragging_moves_the_soldier() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let (a, b) = (grid(30, 30), grid(35, 35));
        let all = VisibleTeams::default();
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, a, Level::Ground))
            .unwrap();
        fixture.edit(|context, sc| context.set_detailed(sc, id, true));
        let other = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(40, 40), Level::Ground))
            .unwrap();
        let mut selection = SoldierSelection::default();
        // Without a press, nothing moves
        release(&mut fixture, &mut selection, &all, b);
        assert_eq!(fixture.soldier(id).grid_no, a);

        selection.press(fixture.map(), &all, Level::Ground, a, false);
        release(&mut fixture, &mut selection, &all, b);
        assert_eq!(selection.selected, Some(id));
        let moved = fixture.soldier(id);
        assert_eq!(moved.grid_no, b);
        let detailed = moved.detailed.as_ref().unwrap();
        assert_eq!(detailed.insertion_grid_no, Some(b));
        // Not onto another soldier
        let depth = fixture.fixture.document.undo.depth();
        selection.press(fixture.map(), &all, Level::Ground, b, false);
        release(&mut fixture, &mut selection, &all, grid(40, 40));
        assert!(selection.notice.take().is_some());
        assert_eq!(fixture.soldier(id).grid_no, b);
        assert_eq!(fixture.soldier(other).grid_no, grid(40, 40));
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        fixture.undo();
        assert_eq!(fixture.soldier(id).grid_no, a);
    }

    #[test]
    fn dragging_with_ctrl_copies_the_soldier() {
        let mut fixture = SoldierFixture::new();
        let civilians =
            mode(r#"{"mode":"soldiers","team":"civilian","defaults":{"civilianGroup":4}}"#);
        let all = VisibleTeams::default();
        let (a, b) = (grid(30, 30), grid(31, 31));
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &civilians, a, Level::Ground))
            .unwrap();
        let schedule = ScheduleId::new(4).unwrap();
        fixture.fixture.map.schedules = vec![Schedule {
            id: schedule,
            soldier_id: 0,
            flags: ScheduleFlags::empty(),
            actions: [None; 4],
        }];
        fixture.edit(|context, sc| {
            context.set_detailed(sc, id, true);
            context.set_patrol_point(id, 2, grid(33, 33));
            context.change_soldier(id, |p| {
                p.orders = Orders::PointPatrol;
                let detailed = p.detailed.as_mut().unwrap();
                detailed.stats.agility = Some(77);
                detailed.schedule = Some(schedule);
            });
        });
        let mut selection = SoldierSelection::default();
        selection.press(fixture.map(), &all, Level::Ground, a, true);
        release(&mut fixture, &mut selection, &all, b);
        let copy = selection.selected.unwrap();
        assert_ne!(copy, id);
        assert_eq!(fixture.map().soldiers.len(), 2);
        assert_eq!(fixture.soldier(id).grid_no, a);
        let copied = fixture.soldier(copy);
        assert_eq!(copied.grid_no, b);
        assert_eq!(copied.team.id, TeamId::CIV);
        assert_eq!(copied.civilian_group, 4);
        assert_eq!(copied.orders, Orders::PointPatrol);
        assert_eq!(copied.patrol, [grid(33, 33), grid(33, 33)]);
        assert_eq!(soldier_slot(fixture.map(), copy), Some(117));
        let detailed = copied.detailed.as_ref().unwrap();
        assert_eq!(detailed.stats.agility, Some(77));
        assert_eq!(detailed.insertion_grid_no, Some(b));
        // Two soldiers can't follow the same schedule
        assert_eq!(detailed.schedule, None);
        assert_eq!(fixture.map().schedules.len(), 1);

        // The copy is undone as a whole
        fixture.undo();
        selection.validate(fixture.map());
        assert_eq!(selection.selected, None);
        assert_eq!(fixture.map().soldiers.len(), 1);
    }

    #[test]
    fn erasing_removes_the_shown_soldiers_of_the_brush() {
        let mut fixture = SoldierFixture::new();
        let enemies = mode(r#"{"mode":"soldiers","team":"enemy"}"#);
        let civilians = mode(r#"{"mode":"soldiers","team":"civilian"}"#);
        let mut place = |mode: &SoldierMode, x| {
            fixture
                .edit(|context, sc| context.place_soldier(sc, mode, grid(x, 50), Level::Ground))
                .unwrap()
        };
        let inside = place(&enemies, 50);
        let hidden = place(&civilians, 51);
        let outside = place(&enemies, 60);
        let schedule = ScheduleId::new(2).unwrap();
        fixture.fixture.map.schedules = vec![Schedule {
            id: schedule,
            soldier_id: 0,
            flags: ScheduleFlags::empty(),
            actions: [None; 4],
        }];
        fixture.edit(|context, sc| {
            context.set_detailed(sc, inside, true);
            context.change_soldier(inside, |p| {
                p.detailed.as_mut().unwrap().schedule = Some(schedule)
            });
        });
        let erase = mode(
            r#"{"mode":"soldiers","tool":"place","erase":true,
                "brush":{"kind":"square","size":3}}"#,
        );
        let no_civilians = VisibleTeams {
            civilian: false,
            ..VisibleTeams::default()
        };
        let area = erase.brush().unwrap().region(None, grid(50, 50).position());
        let depth = fixture.fixture.document.undo.depth();
        fixture.edit(|context, _| erase.apply(context, &area, &no_civilians, Level::Ground));
        let ids: Vec<_> = fixture.map().soldiers.iter().map(|p| p.id).collect();
        assert_eq!(ids, [hidden, outside]);
        assert!(fixture.map().schedules.is_empty());
        assert_eq!(fixture.fixture.document.undo.depth(), depth + 1);
        // Nothing to erase is not recorded
        fixture.edit(|context, _| erase.apply(context, &area, &no_civilians, Level::Ground));
        assert_eq!(fixture.fixture.document.undo.depth(), depth + 1);

        fixture.undo();
        assert_eq!(fixture.map().soldiers.len(), 3);
        assert_eq!(fixture.map().schedules.len(), 1);
    }

    #[test]
    fn soldiers_only_stand_where_they_fit() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let tree = fixture
            .fixture
            .tileset
            .tile(fixture.fixture.tileset.index(FIRSTFULLSTRUCT, 1).unwrap())
            .unwrap()
            .clone();
        fixture.edit(|context, _| context.add_struct_to_head(grid(40, 40), tree));
        assert!(!is_sittable(
            fixture.map(),
            &fixture.fixture.world,
            grid(40, 40),
            Level::Ground,
            true
        ));
        let result = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(40, 40), Level::Ground));
        assert!(result.is_err());

        let placed = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        // Not on another soldier
        let result = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground));
        assert!(result.is_err());
        // Not on the roof without a flat roof
        let roof =
            fixture.edit(|context, sc| context.place_soldier(sc, &mode, grid(31, 30), Level::Roof));
        assert_eq!(roof, Err(NO_FLAT_ROOF.to_owned()));
        assert_eq!(fixture.soldier(placed).level, Level::Ground);
    }

    #[test]
    fn teams_and_maps_have_a_limit() {
        let mut fixture = SoldierFixture::new();
        let enemies = mode(r#"{"mode":"soldiers","team":"enemy"}"#);
        let civilians = mode(r#"{"mode":"soldiers","team":"civilian"}"#);
        for x in 0..MAX_TEAM_SOLDIERS as u8 {
            fixture
                .edit(|context, sc| {
                    context.place_soldier(sc, &enemies, grid(10 + x, 20), Level::Ground)
                })
                .unwrap();
        }
        let result = fixture
            .edit(|context, sc| context.place_soldier(sc, &enemies, grid(10, 30), Level::Ground));
        assert!(result.unwrap_err().contains("team"));
        fixture
            .edit(|context, sc| context.place_soldier(sc, &civilians, grid(10, 30), Level::Ground))
            .unwrap();

        let skipped = crate::file_formats::ja2_map::Ja2SoldierPlacement::default();
        fixture.fixture.map.skipped_placements = vec![skipped; MAX_SOLDIERS - 33];
        let result = fixture
            .edit(|context, sc| context.place_soldier(sc, &civilians, grid(11, 30), Level::Ground));
        assert!(result.unwrap_err().contains("148"));
    }

    #[test]
    fn deleting_removes_the_soldier_and_its_schedule() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers","team":"civilian"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.edit(|context, sc| context.set_detailed(sc, id, true));
        let schedule = ScheduleId::new(4).unwrap();
        fixture.fixture.map.schedules = vec![Schedule {
            id: schedule,
            soldier_id: 0,
            flags: ScheduleFlags::empty(),
            actions: [None; 4],
        }];
        fixture
            .fixture
            .map
            .soldiers
            .iter_mut()
            .for_each(|p| p.detailed.as_mut().unwrap().schedule = Some(schedule));
        let mut selection = SoldierSelection {
            selected: Some(id),
            ..Default::default()
        };
        fixture.edit(|context, _| selection.delete(context));
        assert!(fixture.map().soldiers.is_empty());
        assert!(fixture.map().schedules.is_empty());
        assert_eq!(selection.selected, None);

        // Undo brings both back
        fixture.undo();
        assert_eq!(fixture.map().soldiers.len(), 1);
        assert_eq!(fixture.map().schedules.len(), 1);
        assert!(fixture.fixture.document.soldiers_dirty);
        selection.selected = Some(id);
        selection.validate(fixture.map());
        assert_eq!(selection.selected, Some(id));
        // Undoing the placement drops the selection
        fixture.undo();
        fixture.undo();
        selection.validate(fixture.map());
        assert_eq!(selection.selected, None);
    }

    #[test]
    fn detailed_placements_start_with_nothing_fixed() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.fixture.map.soldiers[0].priority_existence = true;
        fixture.edit(|context, sc| context.set_detailed(sc, id, true));
        let soldier = fixture.soldier(id);
        let detailed = soldier.detailed.as_ref().unwrap();
        assert!(detailed.is_static);
        assert_eq!(detailed.profile, None);
        assert_eq!(detailed.stats, SoldierStats::default());
        assert!(detailed.inventory.iter().all(Option::is_none));
        assert_eq!(detailed.insertion_grid_no, Some(grid(30, 30)));
        assert!(!soldier.priority_existence);
        fixture.edit(|context, sc| context.set_detailed(sc, id, false));
        assert!(fixture.soldier(id).detailed.is_none());
        // Setting it again changes nothing
        let depth = fixture.fixture.document.undo.depth();
        fixture.edit(|context, sc| context.set_detailed(sc, id, false));
        assert_eq!(fixture.fixture.document.undo.depth(), depth);
    }

    #[test]
    fn profiles_make_named_characters() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.edit(|context, sc| {
            context.set_detailed(sc, id, true);
            context.set_profile(sc, id, ProfileId::new(57));
        });
        let soldier = fixture.soldier(id);
        let detailed = soldier.detailed.as_ref().unwrap();
        assert_eq!(detailed.profile.map(ProfileId::number), Some(57));
        assert!(detailed.copy_profile_items_over);
        assert!(soldier.priority_existence);

        fixture.edit(|context, sc| context.set_profile(sc, id, None));
        let soldier = fixture.soldier(id);
        let detailed = soldier.detailed.as_ref().unwrap();
        assert_eq!(detailed.profile, None);
        assert!(!detailed.copy_profile_items_over);

        // Basic placements have no profile
        fixture.edit(|context, sc| {
            context.set_detailed(sc, id, false);
            context.set_profile(sc, id, ProfileId::new(57));
        });
        assert!(fixture.soldier(id).detailed.is_none());
    }

    #[test]
    fn patrol_points_fill_the_missing_ones() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.edit(|context, _| {
            context.set_patrol_point(id, 3, grid(40, 40));
            context.set_patrol_point(id, 1, grid(41, 41));
            // Only 9 points
            context.set_patrol_point(id, 10, grid(42, 42));
        });
        assert_eq!(
            fixture.soldier(id).patrol,
            [grid(41, 41), grid(40, 40), grid(40, 40)]
        );
        fixture.edit(|context, _| {
            context.remove_patrol_point(id, 1);
            context.remove_patrol_point(id, 5);
        });
        assert_eq!(fixture.soldier(id).patrol, [grid(40, 40), grid(40, 40)]);
    }

    #[test]
    fn soldiers_on_flat_roofs() {
        let mut fixture = SoldierFixture::new();
        let mode = mode(r#"{"mode":"soldiers"}"#);
        let position = GridPosition::new(30, 30);
        let roof = fixture
            .fixture
            .tileset
            .tile(
                fixture
                    .fixture
                    .tileset
                    .tile_of("FIRSTROOF", 1)
                    .or_else(|| fixture.fixture.tileset.tile_of("ROOF1", 1))
                    .unwrap(),
            )
            .unwrap()
            .clone();
        assert!(roof.is_flat_roof());
        fixture.edit(|context, _| {
            context.add_roof_to_head(GridNo::from(position), roof.clone());
            context.add_roof_to_head(grid(31, 30), roof);
        });
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Roof))
            .unwrap();
        assert_eq!(fixture.soldier(id).level, Level::Roof);
        // A soldier can stand below it
        let below = fixture
            .edit(|context, sc| context.place_soldier(sc, &mode, grid(30, 30), Level::Ground))
            .unwrap();
        assert_eq!(fixture.soldier(below).level, Level::Ground);
        // Only the soldier on the floor is found
        let all = VisibleTeams::default();
        assert_eq!(
            soldier_at(fixture.map(), grid(30, 30), &all, Level::Roof),
            Some(id)
        );
        assert_eq!(
            soldier_at(fixture.map(), grid(30, 30), &all, Level::Ground),
            Some(below)
        );

        // Moved and copied soldiers keep their level where there is a flat roof
        let ground_copy = fixture
            .edit(|context, sc| context.copy_soldier(sc, below, grid(31, 30)))
            .unwrap();
        assert_eq!(fixture.soldier(ground_copy).level, Level::Ground);
        fixture
            .edit(|context, _| context.move_soldier(id, grid(31, 30)))
            .unwrap();
        assert_eq!(fixture.soldier(id).level, Level::Roof);
        // Not where there is no flat roof
        let copy = fixture.edit(|context, sc| context.copy_soldier(sc, id, grid(40, 40)));
        assert_eq!(copy, Err(NO_FLAT_ROOF.to_owned()));
        let moved = fixture.edit(|context, _| context.move_soldier(id, grid(41, 40)));
        assert_eq!(moved, Err(NO_FLAT_ROOF.to_owned()));
        assert_eq!(fixture.soldier(id).grid_no, grid(31, 30));

        // Erasing only removes the soldiers on the floor
        let erase = self::mode(r#"{"mode":"soldiers","tool":"place","erase":true}"#);
        let area = erase.brush().unwrap().region(None, grid(31, 30).position());
        let all = VisibleTeams::default();
        fixture.edit(|context, _| erase.apply(context, &area, &all, Level::Roof));
        assert!(fixture.map().soldiers.iter().all(|p| p.id != id));
        assert!(fixture.map().soldiers.iter().any(|p| p.id == ground_copy));
    }
}
