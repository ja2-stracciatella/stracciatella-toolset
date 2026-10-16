//! Soldier placements of the map (`BASIC_SOLDIERCREATE_STRUCT`, `SOLDIERCREATE_STRUCT`)
//!
//! The team, class and body type of a placement refer to the soldier definitions, and the random
//! choices that the game makes when it creates the soldier (body type and palette) are already
//! made.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::{exit_grid::SectorId, schedule::ScheduleId, Direction, GridNo, ItemStack, Level};
use crate::{
    item::ItemTable,
    soldier::{
        body_type::{choose_body_type, SoldierBodyType},
        class::SoldierClass,
        palette::{soldier_palette, ResolvedSoldierPalette},
        palette_files::PaletteReplacements,
        random::placement_random,
        standing_surface::StandingSurface,
        team::Team,
        SoldierBodyTypes,
    },
};

/// Number of inventory slots of a detailed placement (`NUM_INV_SLOTS`)
pub const NUM_INV_SLOTS: usize = 19;

/// Hand item slot of the inventory (`HANDPOS`)
pub const HANDPOS: usize = 5;

/// A profile of a named character (`ubProfile`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProfileId(u8);

impl ProfileId {
    /// Profile of soldiers without a profile (`NO_PROFILE`)
    const NO_PROFILE: u8 = 200;

    /// The profile with the given number, `None` for `NO_PROFILE`
    pub fn new(number: u8) -> Option<Self> {
        (number != Self::NO_PROFILE).then_some(Self(number))
    }

    pub fn number(self) -> u8 {
        self.0
    }
}

/// What a soldier does until it is alerted (`bOrders`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Orders {
    /// Moves at most 1 tile
    Stationary,
    /// Moves at most 2 tiles
    OnGuard,
    /// Patrols within 5 tiles
    ClosePatrol,
    /// Patrols within 15 tiles
    FarPatrol,
    /// Patrols along the patrol points
    PointPatrol,
    /// Helps others anywhere in the sector
    OnCall,
    SeekEnemy,
    /// Patrols between random patrol points
    RandomPointPatrol,
}

impl Orders {
    pub fn from_number(number: i8) -> Option<Self> {
        Some(match number {
            0 => Self::Stationary,
            1 => Self::OnGuard,
            2 => Self::ClosePatrol,
            3 => Self::FarPatrol,
            4 => Self::PointPatrol,
            5 => Self::OnCall,
            6 => Self::SeekEnemy,
            7 => Self::RandomPointPatrol,
            _ => return None,
        })
    }

    /// The raw orders, the inverse of [`Orders::from_number`]
    pub fn number(self) -> i8 {
        match self {
            Self::Stationary => 0,
            Self::OnGuard => 1,
            Self::ClosePatrol => 2,
            Self::FarPatrol => 3,
            Self::PointPatrol => 4,
            Self::OnCall => 5,
            Self::SeekEnemy => 6,
            Self::RandomPointPatrol => 7,
        }
    }

    /// Whether the soldier walks along the patrol points, which it then needs
    pub fn uses_patrol_points(self) -> bool {
        matches!(self, Self::PointPatrol | Self::RandomPointPatrol)
    }
}

/// How a soldier fights (`bAttitude`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Attitude {
    Defensive,
    BraveSolo,
    BraveAid,
    CunningSolo,
    CunningAid,
    Aggressive,
    /// Only aggressive against Slay (`ATTACKSLAYONLY`)
    AttackSlayOnly,
}

impl Attitude {
    pub fn from_number(number: i8) -> Option<Self> {
        Some(match number {
            0 => Self::Defensive,
            1 => Self::BraveSolo,
            2 => Self::BraveAid,
            3 => Self::CunningSolo,
            4 => Self::CunningAid,
            5 => Self::Aggressive,
            7 => Self::AttackSlayOnly,
            _ => return None,
        })
    }

    /// The raw attitude, the inverse of [`Attitude::from_number`]
    pub fn number(self) -> i8 {
        match self {
            Self::Defensive => 0,
            Self::BraveSolo => 1,
            Self::BraveAid => 2,
            Self::CunningSolo => 3,
            Self::CunningAid => 4,
            Self::Aggressive => 5,
            Self::AttackSlayOnly => 7,
        }
    }
}

/// Attributes and skills of a detailed placement, `None` if the game chooses them
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SoldierStats {
    pub life_max: Option<i8>,
    pub life: Option<i8>,
    pub agility: Option<i8>,
    pub dexterity: Option<i8>,
    pub exp_level: Option<i8>,
    pub marksmanship: Option<i8>,
    pub medical: Option<i8>,
    pub mechanical: Option<i8>,
    pub explosive: Option<i8>,
    pub leadership: Option<i8>,
    pub strength: Option<i8>,
    pub wisdom: Option<i8>,
    pub morale: Option<i8>,
    pub ai_morale: Option<i8>,
}

/// The part of a detailed placement that the basic placement doesn't have
///
/// The detailed placement repeats team, class, body type, orders, attitude, direction, patrol
/// points, level, civilian group and keys of the basic placement. The game uses the ones of the
/// basic placement, so those are only in [`SoldierPlacement`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailedPlacement {
    /// Whether the stats and the inventory are fixed (`fStatic`)
    pub is_static: bool,
    pub profile: Option<ProfileId>,
    pub copy_profile_items_over: bool,
    /// `None` if the saved sector is not a valid sector
    pub sector: Option<SectorId>,
    pub insertion_grid_no: Option<GridNo>,
    pub stats: SoldierStats,
    pub inventory: [Option<ItemStack>; NUM_INV_SLOTS],
    /// Palette names of the clothes, skin and hair, used if the soldier is `visible`
    pub head_palette: String,
    pub pants_palette: String,
    pub vest_palette: String,
    pub skin_palette: String,
    /// Whether the soldier keeps the given palettes (`fVisible`)
    pub visible: bool,
    pub name: String,
    pub schedule: Option<ScheduleId>,
    /// Vehicle the soldier uses (`fUseGivenVehicle`, `bUseGivenVehicleID`)
    pub vehicle_id: Option<i8>,
}

/// Identifies a placement while the map is edited, it stays the same when the placement is
/// changed, moved or restored by undo
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlacementId(pub u32);

/// Most patrol points of a placement (`MAXPATROLGRIDS`, the first entry is not a patrol point)
pub const MAX_PATROL_POINTS: usize = 9;

/// A soldier placement whose team, class and body type exist
#[derive(Clone, Debug, PartialEq)]
pub struct SoldierPlacement {
    pub id: PlacementId,
    /// Index in the map file, seeds the random choices
    pub index: usize,
    pub grid_no: GridNo,
    pub direction: Direction,
    pub level: Level,
    pub team: Arc<Team>,
    pub soldier_class: Arc<SoldierClass>,
    /// The body type of the placement, `None` if it is chosen at random (`BODY_RANDOM`)
    pub given_body: Option<Arc<SoldierBodyType>>,
    /// The body type, `given_body` or the one chosen at random
    pub body: Arc<SoldierBodyType>,
    /// The standing animation for the body and the item in the hand
    pub surface: Arc<StandingSurface>,
    pub palette: ResolvedSoldierPalette,
    pub relative_attribute_level: i8,
    pub relative_equipment_level: i8,
    pub orders: Orders,
    pub attitude: Attitude,
    /// The patrol points in their order, at most [`MAX_PATROL_POINTS`]
    pub patrol: Vec<GridNo>,
    pub civilian_group: u8,
    pub priority_existence: bool,
    pub has_keys: bool,
    pub detailed: Option<DetailedPlacement>,
}

/// What the appearance of a soldier depends on besides its placement
#[derive(Clone, Copy)]
pub struct AppearanceContext<'a> {
    pub items: &'a ItemTable,
    pub bodies: &'a SoldierBodyTypes,
    /// `None` if `ja2pal.dat` could not be loaded, soldiers then keep their animation palette
    pub palettes: Option<&'a PaletteReplacements>,
}

/// The body type, the standing animation and the palette of a soldier
pub(crate) struct Appearance {
    pub body: Arc<SoldierBodyType>,
    pub surface: Arc<StandingSurface>,
    pub palette: ResolvedSoldierPalette,
}

/// Makes the random choices of the game for a placement and picks the standing animation for
/// the item in its hand (`CreateDetailedPlacementGivenBasicPlacementInfo`,
/// `TacticalCreateSoldier`)
///
/// The choices are seeded with the index and the grid number, so they are the same every time.
pub(crate) fn appearance(
    index: usize,
    grid_no: GridNo,
    team: &Team,
    class: &SoldierClass,
    given_body: Option<&Arc<SoldierBodyType>>,
    detailed: Option<&DetailedPlacement>,
    ctx: &AppearanceContext,
) -> Appearance {
    let mut random = placement_random(index, grid_no);
    let has_profile = detailed.is_some_and(|detailed| detailed.profile.is_some());
    let body = choose_body_type(
        team,
        class,
        given_body,
        has_profile,
        ctx.bodies,
        &mut random,
    );
    let hand = detailed
        .and_then(|detailed| detailed.inventory[HANDPOS].as_ref())
        .and_then(|stack| ctx.items.hand(stack.item));
    let surface = body.standing.surface(hand).clone();
    let given_clothes = detailed
        .filter(|detailed| detailed.visible)
        .filter(|detailed| !detailed.vest_palette.is_empty() && !detailed.pants_palette.is_empty())
        .map(|detailed| {
            (
                detailed.vest_palette.as_str(),
                detailed.pants_palette.as_str(),
            )
        });
    let palette =
        soldier_palette(&body, team, class, given_clothes, &mut random).resolve(ctx.palettes);
    Appearance {
        body,
        surface,
        palette,
    }
}

impl SoldierPlacement {
    /// Chooses the body type, the standing animation and the palette again, after the team, the
    /// class, the body type, the item in the hand or the clothes changed
    pub fn resolve_appearance(&mut self, ctx: &AppearanceContext) {
        let Appearance {
            body,
            surface,
            palette,
        } = appearance(
            self.index,
            self.grid_no,
            &self.team,
            &self.soldier_class,
            self.given_body.as_ref(),
            self.detailed.as_ref(),
            ctx,
        );
        self.body = body;
        self.surface = surface;
        self.palette = palette;
    }
}
