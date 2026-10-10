//! Soldier classes (`SoldierClass`)

use super::{definition_id, Definition};

definition_id!(
    /// A soldier class of [`SOLDIER_CLASSES`]
    SoldierClassId(u8)
);

impl SoldierClassId {
    pub const NONE: Self = Self(0);
    pub const ADMINISTRATOR: Self = Self(1);
    pub const ELITE: Self = Self(2);
    pub const ARMY: Self = Self(3);
    pub const GREEN_MILITIA: Self = Self(4);
    pub const REG_MILITIA: Self = Self(5);
    pub const ELITE_MILITIA: Self = Self(6);
    pub const CREATURE: Self = Self(7);
    pub const MINER: Self = Self(8);
}

/// Palette names of the clothes that a soldier class always wears
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uniform {
    pub vest: &'static str,
    pub pants: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoldierClass {
    pub id: SoldierClassId,
    pub name: &'static str,
    /// Clothes of the class (`GeneratePaletteForSoldier`), `None` if they are chosen per soldier
    pub uniform: Option<Uniform>,
}

const fn class(id: SoldierClassId, name: &'static str, uniform: Option<Uniform>) -> SoldierClass {
    SoldierClass { id, name, uniform }
}

const fn uniform(vest: &'static str, pants: &'static str) -> Option<Uniform> {
    Some(Uniform { vest, pants })
}

pub const SOLDIER_CLASSES: [SoldierClass; 9] = [
    class(SoldierClassId::NONE, "SOLDIER_CLASS_NONE", None),
    class(
        SoldierClassId::ADMINISTRATOR,
        "SOLDIER_CLASS_ADMINISTRATOR",
        uniform("YELLOWVEST", "GREENPANTS"),
    ),
    class(
        SoldierClassId::ELITE,
        "SOLDIER_CLASS_ELITE",
        uniform("BLACKSHIRT", "BLACKPANTS"),
    ),
    class(
        SoldierClassId::ARMY,
        "SOLDIER_CLASS_ARMY",
        uniform("REDVEST", "GREENPANTS"),
    ),
    class(
        SoldierClassId::GREEN_MILITIA,
        "SOLDIER_CLASS_GREEN_MILITIA",
        uniform("GREENVEST", "BEIGEPANTS"),
    ),
    class(
        SoldierClassId::REG_MILITIA,
        "SOLDIER_CLASS_REG_MILITIA",
        uniform("JEANVEST", "BEIGEPANTS"),
    ),
    class(
        SoldierClassId::ELITE_MILITIA,
        "SOLDIER_CLASS_ELITE_MILITIA",
        uniform("BLUEVEST", "BEIGEPANTS"),
    ),
    class(SoldierClassId::CREATURE, "SOLDIER_CLASS_CREATURE", None),
    class(
        SoldierClassId::MINER,
        "SOLDIER_CLASS_MINER",
        uniform("greyVEST", "BEIGEPANTS"),
    ),
];

impl Definition for SoldierClass {
    type Id = SoldierClassId;

    fn id(&self) -> SoldierClassId {
        self.id
    }
}
