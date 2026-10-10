//! The palette of a soldier (`CreateSoldierPalettes`, `GeneratePaletteForSoldier`)

use super::{
    body_type::{BodyPalette, SoldierBodyType},
    class::SoldierClass,
    palette_files::{PaletteReplacementId, PaletteReplacements},
    team::{Team, TeamId},
};
use crate::random::Random;

/// The palette a soldier is drawn with, with the names of the replacements
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SoldierPalette {
    /// The palette of the animation without replacements
    Own,
    /// A palette file, relative to the data root
    Substitution(&'static str),
    /// The palette of the animation with named replacements from `ja2pal.dat`
    Generated(GeneratedPalette),
}

/// The palette a soldier is drawn with, with the replacements looked up in `ja2pal.dat`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedSoldierPalette {
    /// The palette of the animation without replacements
    Own,
    /// A palette file, relative to the data root
    Substitution(&'static str),
    /// The palette of the animation with these replacements applied in order. Names that are
    /// not in `ja2pal.dat` are left out, like the game ignores them.
    Generated(Vec<PaletteReplacementId>),
}

/// The names of the palette replacements of a soldier
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedPalette {
    pub head: String,
    pub vest: String,
    pub pants: String,
    pub skin: String,
}

impl GeneratedPalette {
    /// The replacements in the order they are applied
    pub fn replacements(&self) -> [&str; 4] {
        [&self.head, &self.vest, &self.pants, &self.skin]
    }
}

impl SoldierPalette {
    /// Looks up the replacements, without the replacements file there are none
    pub fn resolve(&self, replacements: Option<&PaletteReplacements>) -> ResolvedSoldierPalette {
        match self {
            Self::Own => ResolvedSoldierPalette::Own,
            Self::Substitution(file) => ResolvedSoldierPalette::Substitution(file),
            Self::Generated(generated) => ResolvedSoldierPalette::Generated(
                replacements
                    .map(|replacements| {
                        generated
                            .replacements()
                            .into_iter()
                            .filter_map(|name| replacements.find(name))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
        }
    }
}

/// The palette of a soldier with the body type `body` (`CreateSoldierPalettes`)
///
/// `given_clothes` are the vest and pants of a visible detailed placement, if it has both.
/// Continues with the `random` generator that chose the body type.
pub fn soldier_palette(
    body: &SoldierBodyType,
    team: &Team,
    class: &SoldierClass,
    given_clothes: Option<(&str, &str)>,
    random: &mut Random,
) -> SoldierPalette {
    match body.palette {
        BodyPalette::Own => SoldierPalette::Own,
        BodyPalette::Substitution(file) => SoldierPalette::Substitution(file),
        BodyPalette::Generated => {
            SoldierPalette::Generated(generated_palette(team, class, given_clothes, random))
        }
    }
}

/// Head, vest, pants and skin palette names (`GeneratePaletteForSoldier`)
fn generated_palette(
    team: &Team,
    class: &SoldierClass,
    given_clothes: Option<(&str, &str)>,
    random: &mut Random,
) -> GeneratedPalette {
    // Skin and hair are always chosen at random
    let skin = random.pick(&["PINKSKIN", "TANSKIN", "DARKSKIN", "BLACKSKIN"]);
    let head = match skin {
        "DARKSKIN" | "BLACKSKIN" => random.pick(&["BLACKHEAD", "BROWNHEAD"]),
        _ => random.pick(&[
            "BROWNHEAD",
            "BLACKHEAD",
            "WHITEHEAD",
            "BLONDHEAD",
            "REDHEAD",
        ]),
    };
    let (vest, pants) = match (class.uniform, given_clothes) {
        (Some(uniform), _) => (uniform.vest, uniform.pants),
        // Visible placements keep their clothes
        (None, Some(given)) => given,
        (None, None) if team.id == TeamId::CIV && random.next(100) < 40 => {
            if random.next(100) < 30 {
                let pants = random.pick(&["TANPANTS", "BEIGEPANTS", "GREENPANTS"]);
                ("GYELLOWSHIRT", pants)
            } else {
                let vest = random.pick(&[
                    "WHITEVEST",
                    "BLACKSHIRT",
                    "PURPLESHIRT",
                    "BLUEVEST",
                    "BROWNVEST",
                    "JEANVEST",
                    "REDVEST",
                ]);
                (vest, "JEANPANTS")
            }
        }
        (None, None) => {
            let (pants, vests): (&str, &[&str]) = match random.next(3) {
                0 => (
                    "GREENPANTS",
                    &["YELLOWVEST", "WHITEVEST", "BROWNVEST", "GREENVEST"],
                ),
                1 => (
                    "TANPANTS",
                    &[
                        "YELLOWVEST",
                        "WHITEVEST",
                        "BLACKSHIRT",
                        "BLUEVEST",
                        "BROWNVEST",
                        "GREENVEST",
                        "JEANVEST",
                        "REDVEST",
                    ],
                ),
                _ => (
                    "BLUEPANTS",
                    &["YELLOWVEST", "WHITEVEST", "REDVEST", "BLACKSHIRT"],
                ),
            };
            (random.pick(vests), pants)
        }
    };
    GeneratedPalette {
        head: head.to_owned(),
        vest: vest.to_owned(),
        pants: pants.to_owned(),
        skin: skin.to_owned(),
    }
}
