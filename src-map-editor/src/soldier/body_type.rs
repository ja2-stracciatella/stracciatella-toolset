//! Body types (`SoldierBodyType`)

use std::sync::Arc;

use super::{
    class::{SoldierClass, SoldierClassId},
    definition_id,
    standing_surface::{StandingSurface, StandingSurfaceId},
    team::{Team, TeamId},
    Definition, SoldierBodyTypes, StandingSurfaces,
};
use crate::item::HandItem;
use crate::random::Random;

definition_id!(
    /// A body type of [`soldier_body_types`]
    SoldierBodyTypeId(u8)
);

impl SoldierBodyTypeId {
    pub const REGMALE: Self = Self(0);
    pub const BIGMALE: Self = Self(1);
    pub const STOCKYMALE: Self = Self(2);
    pub const REGFEMALE: Self = Self(3);
    pub const ADULTFEMALEMONSTER: Self = Self(4);
    pub const AM_MONSTER: Self = Self(5);
    pub const YAF_MONSTER: Self = Self(6);
    pub const YAM_MONSTER: Self = Self(7);
    pub const LARVAE_MONSTER: Self = Self(8);
    pub const INFANT_MONSTER: Self = Self(9);
    pub const QUEENMONSTER: Self = Self(10);
    pub const FATCIV: Self = Self(11);
    pub const MANCIV: Self = Self(12);
    pub const MINICIV: Self = Self(13);
    pub const DRESSCIV: Self = Self(14);
    pub const HATKIDCIV: Self = Self(15);
    pub const KIDCIV: Self = Self(16);
    pub const CRIPPLECIV: Self = Self(17);
    pub const COW: Self = Self(18);
    pub const CROW: Self = Self(19);
    pub const BLOODCAT: Self = Self(20);
    pub const ROBOTNOWEAPON: Self = Self(21);
    pub const HUMVEE: Self = Self(22);
    pub const TANK_NW: Self = Self(23);
    pub const TANK_NE: Self = Self(24);
    pub const ELDORADO: Self = Self(25);
    pub const ICECREAMTRUCK: Self = Self(26);
    pub const JEEP: Self = Self(27);
}

/// The standing animation of a body type (`gubAnimSurfaceIndex`, `SubstituteBodyTypeAnimation`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StandingAnimation {
    /// Depends on the item in the hand (`SetSoldierAnimationSurface`)
    Merc {
        rifle: Arc<StandingSurface>,
        nothing: Arc<StandingSurface>,
        pistol: Arc<StandingSurface>,
    },
    Fixed(Arc<StandingSurface>),
}

impl StandingAnimation {
    pub fn surface(&self, hand: Option<HandItem>) -> &Arc<StandingSurface> {
        match self {
            Self::Merc {
                rifle,
                nothing,
                pistol,
            } => match hand {
                None => nothing,
                Some(HandItem::TwoHandedGun) => rifle,
                Some(HandItem::OneHandedGun) => pistol,
            },
            Self::Fixed(surface) => surface,
        }
    }
}

/// Where the palette of a body type comes from (`CreateSoldierPalettes`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyPalette {
    /// The palette of the animation with the soldier's clothes, skin and hair
    Generated,
    /// The palette of the animation without replacements
    Own,
    /// A palette file, relative to the data root (`GetBodyTypePaletteSubstitution`)
    Substitution(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoldierBodyType {
    pub id: SoldierBodyTypeId,
    pub name: &'static str,
    pub standing: StandingAnimation,
    pub palette: BodyPalette,
    /// Vehicles are drawn at the struct z level
    pub vehicle: bool,
}

fn body(
    id: SoldierBodyTypeId,
    name: &'static str,
    standing: StandingAnimation,
    palette: BodyPalette,
    vehicle: bool,
) -> SoldierBodyType {
    SoldierBodyType {
        id,
        name,
        standing,
        palette,
        vehicle,
    }
}

/// All body types, their standing animations refer to `surfaces`
pub fn soldier_body_types(surfaces: &StandingSurfaces) -> [SoldierBodyType; 28] {
    use BodyPalette::{Generated, Own, Substitution};
    use SoldierBodyTypeId as B;
    use StandingSurfaceId as S;
    let fixed = |surface| StandingAnimation::Fixed(surfaces.get(surface).clone());
    let merc = |rifle, nothing, pistol| StandingAnimation::Merc {
        rifle: surfaces.get(rifle).clone(),
        nothing: surfaces.get(nothing).clone(),
        pistol: surfaces.get(pistol).clone(),
    };
    let regmale = merc(S::REGMALE_RIFLE, S::REGMALE_NOTHING, S::REGMALE_PISTOL);
    [
        body(B::REGMALE, "REGMALE", regmale.clone(), Generated, false),
        // Big guys always use the threatening stance
        body(
            B::BIGMALE,
            "BIGMALE",
            merc(S::BIGMALE_RIFLE, S::BIGMALE_NOTHING, S::BIGMALE_PISTOL),
            Generated,
            false,
        ),
        body(B::STOCKYMALE, "STOCKYMALE", regmale, Generated, false),
        body(
            B::REGFEMALE,
            "REGFEMALE",
            merc(
                S::REGFEMALE_RIFLE,
                S::REGFEMALE_NOTHING,
                S::REGFEMALE_PISTOL,
            ),
            Generated,
            false,
        ),
        body(
            B::ADULTFEMALEMONSTER,
            "ADULTFEMALEMONSTER",
            fixed(S::ADULT_MONSTER),
            Substitution("anims/monsters/femmon.col"),
            false,
        ),
        body(
            B::AM_MONSTER,
            "AM_MONSTER",
            fixed(S::ADULT_MONSTER),
            Substitution("anims/monsters/monster.col"),
            false,
        ),
        body(
            B::YAF_MONSTER,
            "YAF_MONSTER",
            fixed(S::ADULT_MONSTER),
            Substitution("anims/monsters/fm_brite.col"),
            false,
        ),
        body(
            B::YAM_MONSTER,
            "YAM_MONSTER",
            fixed(S::ADULT_MONSTER),
            Substitution("anims/monsters/mn_brite.col"),
            false,
        ),
        body(
            B::LARVAE_MONSTER,
            "LARVAE_MONSTER",
            fixed(S::LARVAE_MONSTER),
            Generated,
            false,
        ),
        body(
            B::INFANT_MONSTER,
            "INFANT_MONSTER",
            fixed(S::INFANT_MONSTER),
            Generated,
            false,
        ),
        body(
            B::QUEENMONSTER,
            "QUEENMONSTER",
            fixed(S::QUEEN_MONSTER),
            Own,
            false,
        ),
        body(B::FATCIV, "FATCIV", fixed(S::FATCIV), Generated, false),
        body(B::MANCIV, "MANCIV", fixed(S::MANCIV), Generated, false),
        body(B::MINICIV, "MINICIV", fixed(S::MINICIV), Generated, false),
        body(
            B::DRESSCIV,
            "DRESSCIV",
            fixed(S::DRESSCIV),
            Generated,
            false,
        ),
        body(
            B::HATKIDCIV,
            "HATKIDCIV",
            fixed(S::HATKIDCIV),
            Generated,
            false,
        ),
        body(B::KIDCIV, "KIDCIV", fixed(S::KIDCIV), Generated, false),
        body(
            B::CRIPPLECIV,
            "CRIPPLECIV",
            fixed(S::CRIPPLECIV),
            Generated,
            false,
        ),
        body(B::COW, "COW", fixed(S::COW), Own, false),
        body(B::CROW, "CROW", fixed(S::CROW), Own, false),
        body(B::BLOODCAT, "BLOODCAT", fixed(S::BLOODCAT), Own, false),
        body(
            B::ROBOTNOWEAPON,
            "ROBOTNOWEAPON",
            fixed(S::ROBOT),
            Own,
            false,
        ),
        body(B::HUMVEE, "HUMVEE", fixed(S::HUMVEE), Own, true),
        body(B::TANK_NW, "TANK_NW", fixed(S::TANK_NW), Own, true),
        body(B::TANK_NE, "TANK_NE", fixed(S::TANK_NE), Own, true),
        body(B::ELDORADO, "ELDORADO", fixed(S::HUMVEE), Own, true),
        body(
            B::ICECREAMTRUCK,
            "ICECREAMTRUCK",
            fixed(S::ICECREAMTRUCK),
            Own,
            true,
        ),
        body(B::JEEP, "JEEP", fixed(S::HUMVEE), Own, true),
    ]
}

impl Definition for SoldierBodyType {
    type Id = SoldierBodyTypeId;

    fn id(&self) -> SoldierBodyTypeId {
        self.id
    }
}

impl SoldierBodyTypes {
    /// The distinct palette files of [`BodyPalette::Substitution`], in the order of the body types
    pub fn palette_files(&self) -> Vec<&'static str> {
        let mut files = vec![];
        for body in self.iter() {
            if let BodyPalette::Substitution(file) = body.palette {
                if !files.contains(&file) {
                    files.push(file);
                }
            }
        }
        files
    }
}

/// The body type of a soldier, chosen at random if the placement has none (`given` is `None`)
/// (`CreateDetailedPlacementGivenBasicPlacementInfo`). Female enemies that are not elite and have
/// no profile get a random male body (`TacticalCreateSoldier`).
///
/// This is the first choice made with `random`.
pub fn choose_body_type(
    team: &Team,
    class: &SoldierClass,
    given: Option<&Arc<SoldierBodyType>>,
    has_profile: bool,
    bodies: &SoldierBodyTypes,
    random: &mut Random,
) -> Arc<SoldierBodyType> {
    use SoldierBodyTypeId as B;
    let mut body_type = match given {
        Some(body) => body.id,
        None => match team.id {
            TeamId::CIV if class.id == SoldierClassId::MINER => {
                random.pick(&[B::REGMALE, B::BIGMALE, B::MANCIV])
            }
            TeamId::CIV => match random.next(100) {
                0..8 => B::FATCIV,
                8..38 => B::MANCIV,
                38..57 => B::MINICIV,
                57..76 => B::DRESSCIV,
                76..88 => B::HATKIDCIV,
                _ => B::KIDCIV,
            },
            TeamId::CREATURE => B::REGMALE,
            _ => random.pick(&[B::REGMALE, B::BIGMALE, B::STOCKYMALE, B::REGFEMALE]),
        },
    };
    // `TacticalCreateSoldier` only has elite female enemies
    if team.id == TeamId::ENEMY
        && body_type == B::REGFEMALE
        && class.id != SoldierClassId::ELITE
        && !has_profile
    {
        body_type = random.pick(&[B::REGMALE, B::BIGMALE, B::STOCKYMALE]);
    }
    bodies.get(body_type).clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soldier::Registries;

    fn standing(body: SoldierBodyTypeId, hand: Option<HandItem>) -> StandingSurfaceId {
        let registries = Registries::build();
        registries.bodies.get(body).standing.surface(hand).id
    }

    #[test]
    fn standing_surfaces() {
        use SoldierBodyTypeId as B;
        assert_eq!(
            standing(B::REGMALE, None),
            StandingSurfaceId::REGMALE_NOTHING
        );
        assert_eq!(standing(B::REGMALE, None).index(), 1);
        assert_eq!(
            standing(B::STOCKYMALE, Some(HandItem::TwoHandedGun)).index(),
            0
        );
        assert_eq!(
            standing(B::REGFEMALE, Some(HandItem::OneHandedGun)).index(),
            8
        );
        assert_eq!(standing(B::JEEP, None).index(), 24);
        let registries = Registries::build();
        let queen = registries
            .bodies
            .get(B::QUEENMONSTER)
            .standing
            .surface(None);
        assert_eq!(queen.directions, 1);
        // The body types share the registered surfaces
        assert!(Arc::ptr_eq(queen, registries.surfaces.get(queen.id)));
    }

    #[test]
    fn palette_files() {
        let registries = Registries::build();
        assert_eq!(
            registries.bodies.palette_files(),
            [
                "anims/monsters/femmon.col",
                "anims/monsters/monster.col",
                "anims/monsters/fm_brite.col",
                "anims/monsters/mn_brite.col",
            ]
        );
    }
}
