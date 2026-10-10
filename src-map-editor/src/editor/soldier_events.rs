//! What the host is told about the soldiers and how it edits them
//!
//! Once a map is loaded, the host gets the choices of the soldiers panel (`soldierPalette`).
//! Whenever the selected soldier, the roof placement or the number of soldiers change, it gets
//! the selected soldier (`soldier`). The host sends the edited values of the selected soldier
//! back with `set_soldier` and everything else with `soldier_command`. Failed edits are reported
//! with a `notice`.

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    components::map::{game_to_world, JA2_TILE_HEIGHT, JA2_TILE_WIDTH},
    host::report_editor_event,
    item::ItemId,
    map::{
        Attitude, DetailedPlacement, Direction, GridNo, ItemStack, Level, Orders, PlacementId,
        ProfileId, SoldierPlacement, SoldierStats, TacticalMap, NUM_INV_SLOTS,
    },
    render::{
        draw_list::VisibleTeams, world::MapWorld, MapSoldiers, PendingSceneUpdate, RenderOptions,
    },
    soldier::{
        body_type::SoldierBodyTypeId,
        palette::ResolvedSoldierPalette,
        palette_files::{ColPalette, PaletteReplacements},
    },
    states::loading::{soldier_files, MapAssets, MapToLoad},
    systems::pixel_view::PixelView,
    tileset::StciSheet,
};

use super::{
    document::{EditContext, EditorDocument},
    schedules::{SchedulePick, ScheduleRow},
    soldiers::{
        edit_once, is_shown, placement, soldier_slot, SoldierContext, SoldierSelection,
        SoldierSources, SoldierTeam, ENEMY_CLASSES, MAX_SOLDIERS, MAX_TEAM_SOLDIERS,
        NUM_CIV_GROUPS,
    },
    MapEdit,
};

/// The names of the civilian groups (`gszCivGroupNames`)
pub const CIV_GROUP_NAMES: [&str; NUM_CIV_GROUPS as usize] = [
    "NONE",
    "REBEL",
    "KINGPIN",
    "SANMONA ARMS",
    "ANGELS",
    "BEGGARS",
    "TOURISTS",
    "ALMA MIL",
    "DOCTORS",
    "COUPLE1",
    "HICKS",
    "WARDEN",
    "JUNKYARD",
    "FACTORY KIDS",
    "QUEENS",
    "UNUSED15",
    "UNUSED16",
    "UNUSED17",
    "UNUSED18",
    "UNUSED19",
];

/// The name the editor shows for a body type (`DisplayBodyTypeInfo`)
pub fn body_type_name(body: SoldierBodyTypeId) -> &'static str {
    use SoldierBodyTypeId as B;
    match body {
        B::REGMALE => "Reg Male",
        B::BIGMALE => "Big Male",
        B::STOCKYMALE => "Stocky Male",
        B::REGFEMALE => "Reg Female",
        B::TANK_NE => "NE Tank",
        B::TANK_NW => "NW Tank",
        B::FATCIV => "Fat Civilian",
        B::MANCIV => "M Civilian",
        B::MINICIV => "Miniskirt",
        B::DRESSCIV => "F Civilian",
        B::HATKIDCIV => "Kid w/ Hat",
        B::HUMVEE => "Humvee",
        B::ELDORADO => "Eldorado",
        B::ICECREAMTRUCK => "Icecream Truck",
        B::JEEP => "Jeep",
        B::KIDCIV => "Kid Civilian",
        B::COW => "Domestic Cow",
        B::CRIPPLECIV => "Cripple",
        B::ROBOTNOWEAPON => "Unarmed Robot",
        B::LARVAE_MONSTER => "Larvae",
        B::INFANT_MONSTER => "Infant",
        B::YAF_MONSTER => "Yng F Monster",
        B::YAM_MONSTER => "Yng M Monster",
        B::ADULTFEMALEMONSTER => "Adt F Monster",
        B::AM_MONSTER => "Adt M Monster",
        B::QUEENMONSTER => "Queen Monster",
        B::BLOODCAT => "Bloodcat",
        B::CROW => "Crow",
        _ => "Unknown",
    }
}

/// An item stack as the host edits it, `item` 0 is no item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStackValue {
    pub item: u16,
    pub number: u8,
    /// Item class specific data, e.g. the status of most items in the first byte
    pub data: [u8; 12],
    pub attachments: Vec<AttachmentValue>,
    /// `OBJECT_UNDROPPABLE` (0x01), `OBJECT_NO_OVERWRITE` (0x80) and the like
    pub flags: u8,
    pub mission: u8,
    pub trap: i8,
    pub imprint_id: u8,
    pub weight: u8,
    pub used: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentValue {
    pub item: u16,
    pub status: i8,
}

impl ItemStackValue {
    pub fn of(stack: &ItemStack) -> Self {
        Self {
            item: stack.item.index(),
            number: stack.number,
            data: stack.data,
            attachments: stack
                .attachments
                .iter()
                .map(|&(item, status)| AttachmentValue {
                    item: item.index(),
                    status,
                })
                .collect(),
            flags: stack.flags as u8,
            mission: stack.mission,
            trap: stack.trap,
            imprint_id: stack.imprint_id,
            weight: stack.weight,
            used: stack.used,
        }
    }

    /// The item stack, `None` without an item
    pub fn stack(&self) -> Option<ItemStack> {
        Some(ItemStack {
            item: ItemId::new(self.item)?,
            number: self.number,
            data: self.data,
            attachments: self
                .attachments
                .iter()
                .take(4)
                .filter_map(|a| Some((ItemId::new(a.item)?, a.status)))
                .collect(),
            flags: self.flags as i8,
            mission: self.mission,
            trap: self.trap,
            imprint_id: self.imprint_id,
            weight: self.weight,
            used: self.used,
        })
    }
}

/// The values of a detailed placement that the host edits
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailedValue {
    /// The profile of a named character, `None` for none
    pub profile: Option<u8>,
    pub copy_profile_items_over: bool,
    pub stats: SoldierStats,
    /// The 19 inventory slots of the game (`NUM_INV_SLOTS`)
    pub inventory: Vec<Option<ItemStackValue>>,
    /// Whether the soldier keeps the colours (`fVisible`)
    pub visible: bool,
    pub head_palette: String,
    pub pants_palette: String,
    pub vest_palette: String,
    pub skin_palette: String,
}

impl DetailedValue {
    pub fn of(detailed: &DetailedPlacement) -> Self {
        Self {
            profile: detailed.profile.map(ProfileId::number),
            copy_profile_items_over: detailed.copy_profile_items_over,
            stats: detailed.stats,
            inventory: detailed
                .inventory
                .iter()
                .map(|slot| slot.as_ref().map(ItemStackValue::of))
                .collect(),
            visible: detailed.visible,
            head_palette: detailed.head_palette.clone(),
            pants_palette: detailed.pants_palette.clone(),
            vest_palette: detailed.vest_palette.clone(),
            skin_palette: detailed.skin_palette.clone(),
        }
    }

    /// Puts the values into a detailed placement, clamped like the original
    /// (`ExtractAndUpdateMercAttributes`), but the morale is the morale
    fn apply(&self, detailed: &mut DetailedPlacement) {
        detailed.profile = self.profile.and_then(ProfileId::new);
        detailed.copy_profile_items_over = self.copy_profile_items_over;
        detailed.stats = clamp_stats(self.stats);
        if self.inventory.len() == NUM_INV_SLOTS {
            for (slot, value) in detailed.inventory.iter_mut().zip(&self.inventory) {
                *slot = value.as_ref().and_then(ItemStackValue::stack);
            }
        }
        detailed.visible = self.visible;
        detailed.head_palette = self.head_palette.clone();
        detailed.pants_palette = self.pants_palette.clone();
        detailed.vest_palette = self.vest_palette.clone();
        detailed.skin_palette = self.skin_palette.clone();
    }
}

/// The attributes clamped to 100, the experience level to 1 to 9 and the life to the life
/// maximum, which is at least 1 (`ExtractAndUpdateMercAttributes`)
pub fn clamp_stats(stats: SoldierStats) -> SoldierStats {
    let stat = |value: Option<i8>| value.map(|v| v.clamp(0, 100));
    let life_max = stat(stats.life_max).map(|v| v.max(1));
    let life = match (stat(stats.life), life_max) {
        (Some(life), Some(max)) => Some(life.min(max)),
        (life, _) => life,
    };
    SoldierStats {
        life_max,
        life,
        agility: stat(stats.agility),
        dexterity: stat(stats.dexterity),
        exp_level: stats.exp_level.map(|v| v.clamp(1, 9)),
        marksmanship: stat(stats.marksmanship),
        medical: stat(stats.medical),
        mechanical: stat(stats.mechanical),
        explosive: stat(stats.explosive),
        leadership: stat(stats.leadership),
        strength: stat(stats.strength),
        wisdom: stat(stats.wisdom),
        morale: stat(stats.morale),
        ai_morale: stat(stats.ai_morale),
    }
}

/// The values of a placement that the host edits
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoldierFields {
    pub orders: Orders,
    pub attitude: Attitude,
    pub direction: Direction,
    /// 0 to 4, from bad to great
    pub relative_attribute_level: i8,
    pub relative_equipment_level: i8,
    pub soldier_class: u8,
    pub civilian_group: u8,
    /// The body type, `None` if it is chosen at random
    pub body: Option<u8>,
    pub priority_existence: bool,
    pub has_keys: bool,
    /// Only used if the placement is detailed, see the `detailed` command
    pub detailed: Option<DetailedValue>,
}

impl SoldierFields {
    pub fn of(placement: &SoldierPlacement) -> Self {
        Self {
            orders: placement.orders,
            attitude: placement.attitude,
            direction: placement.direction,
            relative_attribute_level: placement.relative_attribute_level,
            relative_equipment_level: placement.relative_equipment_level,
            soldier_class: placement.soldier_class.id.index() as u8,
            civilian_group: placement.civilian_group,
            body: placement
                .given_body
                .as_ref()
                .map(|body| body.id.index() as u8),
            priority_existence: placement.priority_existence,
            has_keys: placement.has_keys,
            detailed: placement.detailed.as_ref().map(DetailedValue::of),
        }
    }

    /// Puts the values into a placement, unknown classes and body types are ignored
    pub fn apply(&self, placement: &mut SoldierPlacement, sc: &SoldierContext) {
        let definitions = sc.definitions;
        placement.orders = self.orders;
        placement.attitude = self.attitude;
        placement.direction = self.direction;
        placement.relative_attribute_level = self.relative_attribute_level.clamp(0, 4);
        placement.relative_equipment_level = self.relative_equipment_level.clamp(0, 4);
        if let Some(class) = definitions.classes.validate(self.soldier_class) {
            placement.soldier_class = class;
        }
        placement.civilian_group = self.civilian_group.min(NUM_CIV_GROUPS - 1);
        match self.body {
            None => placement.given_body = None,
            Some(body) => {
                if let Some(body) = definitions.bodies.validate(body) {
                    placement.given_body = Some(body);
                }
            }
        }
        placement.priority_existence = self.priority_existence;
        placement.has_keys = self.has_keys;
        if let (Some(value), Some(detailed)) = (&self.detailed, &mut placement.detailed) {
            value.apply(detailed);
        }
        placement.resolve_appearance(&sc.appearance);
    }
}

/// The values of the selected soldier sent with `set_soldier`
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoldierEdit {
    pub id: u32,
    #[serde(flatten)]
    pub fields: SoldierFields,
}

/// The palette names a soldier is drawn with
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaletteNames {
    pub head: String,
    pub skin: String,
    pub vest: String,
    pub pants: String,
}

/// A placement as the host shows it
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoldierValue {
    pub id: u32,
    /// The number of the soldier in the game (`Slot #`)
    pub slot: Option<usize>,
    /// `None` for the teams of the player
    pub team: Option<SoldierTeam>,
    pub grid_no: u16,
    pub on_roof: bool,
    /// The body type it is drawn with, the given one or the one chosen at random
    pub drawn_body: u8,
    /// The colours it is drawn with, `None` if the body type has no colours to choose
    pub drawn_palette: Option<PaletteNames>,
    pub patrol: Vec<u16>,
    #[serde(flatten)]
    pub fields: SoldierFields,
}

impl SoldierValue {
    pub fn of(
        map: &TacticalMap,
        placement: &SoldierPlacement,
        palettes: Option<&PaletteReplacements>,
    ) -> Self {
        let drawn_palette = match (&placement.palette, palettes) {
            (ResolvedSoldierPalette::Generated(ids), Some(palettes)) => {
                let names: Vec<&str> = ids.iter().filter_map(|&id| palettes.name(id)).collect();
                match names[..] {
                    [head, vest, pants, skin] => Some(PaletteNames {
                        head: head.to_owned(),
                        skin: skin.to_owned(),
                        vest: vest.to_owned(),
                        pants: pants.to_owned(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        };
        Self {
            id: placement.id.0,
            slot: soldier_slot(map, placement.id),
            team: SoldierTeam::of(placement.team.id),
            grid_no: placement.grid_no.index() as u16,
            on_roof: placement.level == Level::Roof,
            drawn_body: placement.body.id.index() as u8,
            drawn_palette,
            patrol: placement
                .patrol
                .iter()
                .map(|grid_no| grid_no.index() as u16)
                .collect(),
            fields: SoldierFields::of(placement),
        }
    }
}

/// How many soldiers each team has
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TeamCounts {
    pub enemy: usize,
    pub creature: usize,
    pub rebel: usize,
    pub civilian: usize,
    /// All soldiers of the map, also those of other teams and those that could not be loaded
    pub total: usize,
}

impl TeamCounts {
    pub fn of(map: &TacticalMap) -> Self {
        let count = |team: SoldierTeam| {
            map.soldiers
                .iter()
                .filter(|p| p.team.id == team.team_id())
                .count()
        };
        Self {
            enemy: count(SoldierTeam::Enemy),
            creature: count(SoldierTeam::Creature),
            rebel: count(SoldierTeam::Rebel),
            civilian: count(SoldierTeam::Civilian),
            total: map.soldiers.len() + map.skipped_placements.len(),
        }
    }
}

/// The selected soldier, `null` if none is
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "soldier", rename_all = "camelCase")]
pub struct SoldierEvent {
    pub map: String,
    pub soldier: Option<SoldierValue>,
    pub counts: TeamCounts,
    /// The actions of the schedule of the selected soldier, `None` without a detailed placement
    pub schedule: Option<[ScheduleRow; 4]>,
    /// The tile of the schedule that the next click picks
    pub pick: Option<SchedulePick>,
}

/// Why an edit did not happen, e.g. a soldier could not be placed
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "notice", rename_all = "camelCase")]
pub struct NoticeEvent {
    pub map: String,
    pub message: String,
}

/// A body type the host offers, `id` is `None` for a random one
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BodyOption {
    pub id: Option<u8>,
    pub name: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ClassOption {
    pub id: u8,
    pub name: &'static str,
}

/// Something of each team
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ByTeam<T> {
    pub enemy: T,
    pub creature: T,
    pub rebel: T,
    pub civilian: T,
}

impl<T> ByTeam<T> {
    fn from_fn(f: impl Fn(SoldierTeam) -> T) -> Self {
        Self {
            enemy: f(SoldierTeam::Enemy),
            creature: f(SoldierTeam::Creature),
            rebel: f(SoldierTeam::Rebel),
            civilian: f(SoldierTeam::Civilian),
        }
    }
}

/// The palette replacements of the colour sets of the editor
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ColourSets {
    pub head: Vec<String>,
    pub skin: Vec<String>,
    pub vest: Vec<String>,
    pub pants: Vec<String>,
}

impl ColourSets {
    /// The ranges of `ja2pal.dat` are the hair, the pants, the skin and the vest
    /// (`EDIT_COLOR_HEAD` and so on)
    pub fn of(palettes: &PaletteReplacements) -> Self {
        let names = |range| palettes.names_in_range(range).map(str::to_owned).collect();
        Self {
            head: names(0),
            pants: names(1),
            skin: names(2),
            vest: names(3),
        }
    }
}

/// The choices of the soldiers panel
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "soldierPalette", rename_all = "camelCase")]
pub struct SoldierPaletteEvent {
    pub map: String,
    pub max_soldiers: usize,
    pub max_team_soldiers: usize,
    pub enemy_classes: Vec<ClassOption>,
    /// The body types that soldiers of each team can have (`bEnemyArray` and so on)
    pub bodies: ByTeam<Vec<BodyOption>>,
    pub civilian_groups: Vec<&'static str>,
    pub colours: ColourSets,
}

impl SoldierPaletteEvent {
    pub fn new(map: String, palettes: Option<&PaletteReplacements>) -> Self {
        Self {
            map,
            max_soldiers: MAX_SOLDIERS,
            max_team_soldiers: MAX_TEAM_SOLDIERS,
            enemy_classes: ENEMY_CLASSES
                .iter()
                .map(|&(id, name)| ClassOption {
                    id: id.index() as u8,
                    name,
                })
                .collect(),
            bodies: ByTeam::from_fn(|team| {
                team.body_types()
                    .iter()
                    .map(|body| BodyOption {
                        id: body.map(|body| body.index() as u8),
                        name: body.map_or("Random", body_type_name),
                    })
                    .collect()
            }),
            civilian_groups: CIV_GROUP_NAMES.to_vec(),
            colours: palettes.map(ColourSets::of).unwrap_or_default(),
        }
    }
}

/// What the host asks of the soldiers besides editing the selected one
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SoldierCommand {
    /// Selects a soldier, or none
    Select { id: Option<u32> },
    /// Gives the selected soldier a detailed placement or removes it, a detailed placement with
    /// the profile or without one
    Detailed {
        detailed: bool,
        #[serde(default)]
        profile: Option<u8>,
    },
    /// Centers the view on a tile
    Center { grid_no: u16 },
    /// Sets the actions of the schedule of the selected soldier
    Schedule { rows: [ScheduleRow; 4] },
    /// Removes all actions of the schedule of the selected soldier
    ClearSchedule,
    /// The next left click picks a tile of an action of the schedule
    Pick(SchedulePick),
    /// Stops picking, the action is removed if its tiles are missing
    CancelPick,
}

/// What soldier commands need besides the map
#[derive(SystemParam)]
pub struct SoldierCommandParams<'w> {
    pub selection: ResMut<'w, SoldierSelection>,
    pub sources: SoldierSources<'w>,
    pub view: Option<ResMut<'w, PixelView>>,
    pub options: Res<'w, RenderOptions>,
}

/// The center of a tile in world coordinates, raised by its height
pub fn tile_center(map: &TacticalMap, grid_no: GridNo) -> Vec2 {
    let mut game = grid_no.position().tile_box() + IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT) / 2;
    game.y -= i32::from(map.tiles[grid_no].height);
    game_to_world(game)
}

/// Edits the selected soldier with the values of the host, ignored if another one is selected
pub fn update_soldier(context: &mut EditContext, sc: &SoldierContext, edit: &SoldierEdit) {
    let id = PlacementId(edit.id);
    context.change_soldier(id, |placement| edit.fields.apply(placement, sc));
}

/// Runs a soldier command of the host
pub fn run_soldier_command(
    command: SoldierCommand,
    edit: &mut MapEdit,
    params: &mut SoldierCommandParams,
) {
    let visible = params.options.layers.soldier_teams;
    let selection = &mut *params.selection;
    match command {
        SoldierCommand::Select { id } => {
            selection.selected = id
                .map(PlacementId)
                .filter(|&id| placement(&edit.map, id).is_some_and(|p| is_shown(p, &visible)));
        }
        SoldierCommand::Detailed { detailed, profile } => {
            let (Some(sc), Some(selected)) = (params.sources.context(), selection.selected) else {
                return;
            };
            edit_once(edit, |context| {
                context.set_detailed(&sc, selected, detailed);
                context.set_profile(&sc, selected, profile.and_then(ProfileId::new));
            });
        }
        SoldierCommand::Schedule { rows } => {
            selection.schedule.sync(&edit.map, selection.selected);
            edit_once(edit, |context| selection.schedule.set_rows(context, rows));
        }
        SoldierCommand::ClearSchedule => {
            selection.schedule.sync(&edit.map, selection.selected);
            selection.schedule.pick = None;
            edit_once(edit, |context| {
                selection
                    .schedule
                    .set_rows(context, [ScheduleRow::default(); 4])
            });
        }
        SoldierCommand::Pick(pick) => {
            selection.schedule.sync(&edit.map, selection.selected);
            selection.schedule.start_pick(pick);
        }
        SoldierCommand::CancelPick => {
            edit_once(edit, |context| selection.schedule.cancel_pick(context));
        }
        SoldierCommand::Center { grid_no } => {
            if let (Some(grid_no), Some(view)) =
                (GridNo::new(usize::from(grid_no)), params.view.as_mut())
            {
                view.position = tile_center(&edit.map, grid_no);
            }
        }
    }
}

/// The soldier event that was reported last, `None` for a new map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReportedSoldier(Option<SoldierEvent>);

/// Sends the choices of the soldiers panel once a map is loaded
pub fn report_soldier_palette(
    map_to_load: Res<MapToLoad>,
    map_assets: Res<MapAssets>,
    replacements: Res<Assets<PaletteReplacements>>,
    mut reported: ResMut<ReportedSoldier>,
) {
    reported.0 = None;
    let palettes = replacements.get(&map_assets.soldier_palettes);
    report_editor_event(&SoldierPaletteEvent::new(map_to_load.0.clone(), palettes));
}

/// Tells the host about the selected soldier when it changes, and about failed edits
pub fn report_soldier(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    map_assets: Res<MapAssets>,
    replacements: Res<Assets<PaletteReplacements>>,
    mut selection: ResMut<SoldierSelection>,
    mut reported: ResMut<ReportedSoldier>,
) {
    if let Some(message) = selection.notice.take() {
        report_editor_event(&NoticeEvent {
            map: map_to_load.0.clone(),
            message,
        });
    }
    let palettes = replacements.get(&map_assets.soldier_palettes);
    let event = SoldierEvent {
        map: map_to_load.0.clone(),
        soldier: selection
            .selected
            .and_then(|id| placement(&map, id))
            .map(|selected| SoldierValue::of(&map, selected, palettes)),
        counts: TeamCounts::of(&map),
        schedule: selection.schedule.draft.as_ref().map(|draft| draft.rows),
        pick: selection.schedule.pick,
    };
    if reported.0.as_ref() != Some(&event) {
        report_editor_event(&event);
        reported.0 = Some(event);
    }
}

/// The teams whose soldiers the world has, all of them for a new map
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ShownTeams(pub VisibleTeams);

pub fn reset_shown_teams(mut shown: ResMut<ShownTeams>) {
    *shown = ShownTeams::default();
}

/// Brings the soldiers of the world up to date after the placements or the shown teams changed,
/// the scene catches up with the tiles they were and are on
pub fn sync_soldier_scene(
    options: Res<RenderOptions>,
    mut document: ResMut<EditorDocument>,
    mut shown: ResMut<ShownTeams>,
    map: Res<TacticalMap>,
    mut world: ResMut<MapWorld>,
    mut soldiers: ResMut<MapSoldiers>,
    mut pending: ResMut<PendingSceneUpdate>,
    map_assets: Res<MapAssets>,
    sheets: Res<Assets<StciSheet>>,
    replacements: Res<Assets<PaletteReplacements>>,
    col_palettes: Res<Assets<ColPalette>>,
) {
    let visible = options.layers.soldier_teams;
    if !document.soldiers_dirty && shown.0 == visible {
        return;
    }
    document.soldiers_dirty = false;
    shown.0 = visible;
    let files = soldier_files(&map_assets, &sheets, &replacements, &col_palettes);
    let touched = crate::render::soldiers::sync_soldiers(
        &mut world,
        &map.soldiers,
        |placement| visible.shows(placement.team.id),
        &files,
        &mut soldiers,
    );
    pending.tiles.extend(touched);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{
            document::tests::grid,
            soldiers::tests::{mode, SoldierFixture},
        },
        map::HANDPOS,
        soldier::standing_surface::StandingSurfaceId,
    };

    #[test]
    fn the_host_edits_the_selected_soldier() {
        let mut fixture = SoldierFixture::new();
        let enemies = mode(r#"{"mode":"soldiers"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &enemies, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.edit(|context, sc| context.set_detailed(sc, id, true));
        let map = fixture.map();
        let value = SoldierValue::of(map, fixture.soldier(id), Some(&fixture.palettes));
        let mut json = serde_json::to_value(&value).unwrap();
        assert_eq!(json["slot"], 20);
        assert_eq!(json["team"], "enemy");
        assert_eq!(json["orders"], "stationary");
        assert_eq!(json["direction"], "northWest");
        assert_eq!(json["detailed"]["stats"]["life"], serde_json::Value::Null);
        assert_eq!(json["detailed"]["inventory"].as_array().unwrap().len(), 19);
        assert!(json["drawnPalette"]["vest"].is_string());

        // The host sends the soldier back with its changes
        json["orders"] = "pointPatrol".into();
        json["soldierClass"] = 2.into();
        json["body"] = 3.into();
        json["relativeEquipmentLevel"] = 9.into();
        json["detailed"]["stats"]["life"] = 120.into();
        json["detailed"]["stats"]["lifeMax"] = 60.into();
        json["detailed"]["inventory"][HANDPOS] = serde_json::json!({
            "item": 1, "number": 1, "data": [90, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "attachments": [], "flags": 128, "mission": 0, "trap": 0, "imprintId": 200,
            "weight": 0, "used": 0
        });
        let edit: SoldierEdit = serde_json::from_value(json).unwrap();
        fixture.edit(|context, sc| update_soldier(context, sc, &edit));
        let edited = fixture.soldier(id);
        assert_eq!(edited.orders, Orders::PointPatrol);
        assert_eq!(edited.soldier_class.id.index(), 2);
        assert_eq!(
            edited.given_body.as_ref().unwrap().id,
            SoldierBodyTypeId::REGFEMALE
        );
        assert_eq!(edited.body.id, SoldierBodyTypeId::REGFEMALE);
        assert_eq!(edited.relative_equipment_level, 4);
        let detailed = edited.detailed.as_ref().unwrap();
        assert_eq!(detailed.stats.life, Some(60));
        assert_eq!(detailed.inventory[HANDPOS].as_ref().unwrap().flags, -128);
        // The pistol in the hand changes the standing animation
        assert_eq!(edited.surface.id, StandingSurfaceId::REGFEMALE_PISTOL);
        assert_eq!(fixture.fixture.document.undo.depth(), 3);

        // Sending the same values changes nothing, edits of other soldiers are ignored
        let same = SoldierEdit {
            id: id.0,
            fields: SoldierFields::of(fixture.soldier(id)),
        };
        fixture.edit(|context, sc| update_soldier(context, sc, &same));
        let other = SoldierEdit { id: 99, ..same };
        fixture.edit(|context, sc| update_soldier(context, sc, &other));
        assert_eq!(fixture.fixture.document.undo.depth(), 3);
    }

    #[test]
    fn commands_are_read_from_json() {
        let parse = |json: &str| serde_json::from_str::<SoldierCommand>(json).unwrap();
        assert_eq!(
            parse(r#"{"kind":"select","id":null}"#),
            SoldierCommand::Select { id: None }
        );
        assert_eq!(
            parse(r#"{"kind":"center","gridNo":1234}"#),
            SoldierCommand::Center { grid_no: 1234 }
        );
        assert_eq!(
            parse(r#"{"kind":"detailed","detailed":true}"#),
            SoldierCommand::Detailed {
                detailed: true,
                profile: None
            }
        );
        assert_eq!(
            parse(r#"{"kind":"detailed","detailed":true,"profile":57}"#),
            SoldierCommand::Detailed {
                detailed: true,
                profile: Some(57)
            }
        );
        assert_eq!(
            parse(r#"{"kind":"pick","slot":1,"point":"a","single":true}"#),
            SoldierCommand::Pick(SchedulePick {
                slot: 1,
                point: crate::editor::schedules::SchedulePoint::A,
                single: true
            })
        );
        let rows = r#"{"action":"none","time":null,"variance":false,"data1":null,"data2":null}"#;
        let schedule = format!(r#"{{"kind":"schedule","rows":[{rows},{rows},{rows},{rows}]}}"#);
        assert_eq!(
            parse(&schedule),
            SoldierCommand::Schedule {
                rows: [ScheduleRow::default(); 4]
            }
        );
    }

    #[test]
    fn stats_are_clamped_like_the_original() {
        let stats = clamp_stats(SoldierStats {
            life_max: Some(0),
            life: Some(50),
            exp_level: Some(12),
            agility: Some(120),
            morale: Some(70),
            ..Default::default()
        });
        assert_eq!(stats.life_max, Some(1));
        assert_eq!(stats.life, Some(1));
        assert_eq!(stats.exp_level, Some(9));
        assert_eq!(stats.agility, Some(100));
        // Unlike the original, the morale is not the mechanical skill
        assert_eq!(stats.morale, Some(70));
        assert_eq!(stats.mechanical, None);
        let stats = clamp_stats(SoldierStats {
            life: Some(50),
            exp_level: Some(0),
            ..Default::default()
        });
        assert_eq!(stats.life, Some(50));
        assert_eq!(stats.exp_level, Some(1));
    }

    #[test]
    fn item_stacks_are_json_for_the_host() {
        let stack = ItemStack {
            item: ItemId::new(5).unwrap(),
            number: 1,
            data: [90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            attachments: vec![(ItemId::new(7).unwrap(), 60)],
            flags: -127,
            mission: 0,
            trap: 0,
            imprint_id: 200,
            weight: 0,
            used: 0,
        };
        let value = ItemStackValue::of(&stack);
        assert_eq!(value.flags, 0x81);
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json["imprintId"], 200);
        assert_eq!(json["attachments"][0]["status"], 60);
        assert_eq!(value.stack(), Some(stack));
        let nothing = ItemStackValue { item: 0, ..value };
        assert_eq!(nothing.stack(), None);
    }

    #[test]
    fn the_palette_has_the_choices_of_the_original() {
        let palettes = PaletteReplacements::with_ranges(&[
            (0, "BROWNHEAD"),
            (1, "BLUEPANTS"),
            (2, "PINKSKIN"),
            (3, "REDVEST"),
            (0, "BLONDHEAD"),
        ]);
        let event = SoldierPaletteEvent::new("maps/a9.dat".to_owned(), Some(&palettes));
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "soldierPalette");
        assert_eq!(json["maxSoldiers"], 148);
        assert_eq!(json["enemyClasses"][2]["name"], "Elite");
        assert_eq!(json["bodies"]["enemy"][0]["id"], serde_json::Value::Null);
        assert_eq!(json["bodies"]["creature"][0]["name"], "Bloodcat");
        assert_eq!(json["bodies"]["civilian"].as_array().unwrap().len(), 18);
        assert_eq!(json["civilianGroups"][3], "SANMONA ARMS");
        assert_eq!(json["colours"]["head"][1], "BLONDHEAD");
        assert_eq!(json["colours"]["vest"][0], "REDVEST");
    }
}
