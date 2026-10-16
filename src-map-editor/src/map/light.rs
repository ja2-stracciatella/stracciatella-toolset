//! The lighting of the map: the ambient light and the light sprites

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    components::map::GridPosition, file_formats::ja2_map::Ja2LightSprite, light::LightTemplate,
};

/// Light sprite flags of the map file (`LIGHT_SPR_ACTIVE`, `LIGHT_SPR_ON`, `LIGHT_PRIMETIME`,
/// `LIGHT_NIGHTTIME`)
pub const LIGHT_SPR_ACTIVE: u32 = 0x1;
pub const LIGHT_SPR_ON: u32 = 0x2;
pub const LIGHT_PRIMETIME: u32 = 0x80;
pub const LIGHT_NIGHTTIME: u32 = 0x100;

/// Ambient light of basements and caves (`LightSetBaseLevel` of the saved level)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmbientLight {
    pub basement: bool,
    pub caves: bool,
    /// Ambient light level, higher is darker. For caves of old maps this is already the level
    /// the game uses instead of the saved one.
    pub level: u8,
}

/// When a light sprite is on (`LIGHT_PRIMETIME`, `LIGHT_NIGHTTIME`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LightTime {
    /// Always on
    Always,
    /// On at night
    Night,
    /// On in the evening, the default of the original editor
    #[default]
    PrimeTime,
}

impl LightTime {
    /// The time of the flags of a light sprite of the map file
    pub fn from_flags(flags: u32) -> Self {
        if flags & LIGHT_PRIMETIME != 0 {
            LightTime::PrimeTime
        } else if flags & LIGHT_NIGHTTIME != 0 {
            LightTime::Night
        } else {
            LightTime::Always
        }
    }

    /// The time flag of a light sprite
    pub fn flags(self) -> u32 {
        match self {
            LightTime::Always => 0,
            LightTime::Night => LIGHT_NIGHTTIME,
            LightTime::PrimeTime => LIGHT_PRIMETIME,
        }
    }
}

/// Identifies a light while the map is edited, ids are never used twice
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LightId(pub u32);

/// The template file of the lights of a radius that the editor places (`PlaceLight`)
pub fn light_template_name(radius: u8) -> String {
    format!("L-R{:02}.LHT", radius)
}

/// A light sprite of the map
#[derive(Clone, Debug)]
pub struct MapLight {
    pub id: LightId,
    /// Tile of the light
    pub position: GridPosition,
    pub time: LightTime,
    /// File name of the template as the map file has it, e.g. `L-R05.LHT`
    pub template_name: String,
    pub template: Arc<LightTemplate>,
    /// The light sprite of the map file, saved as it was while the light is unchanged
    pub file_sprite: Option<Ja2LightSprite>,
}

impl MapLight {
    /// The radius of the editor's templates (`L-R0N.LHT`), `None` for other templates
    pub fn radius(&self) -> Option<u8> {
        let name = self.template_name.to_lowercase();
        let radius = name.strip_prefix("l-r")?.strip_suffix(".lht")?;
        radius
            .parse()
            .ok()
            .filter(|radius| (1..=99).contains(radius))
    }
}

impl PartialEq for MapLight {
    /// Templates are equal if they have the same name
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.position == other.position
            && self.time == other.time
            && self.template_name == other.template_name
            && self.file_sprite == other.file_sprite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_come_from_the_template_names() {
        let light = |name: &str| MapLight {
            id: LightId(0),
            position: GridPosition::new(1, 1),
            time: LightTime::Always,
            template_name: name.to_owned(),
            template: Arc::default(),
            file_sprite: None,
        };
        assert_eq!(light(&light_template_name(3)).radius(), Some(3));
        assert_eq!(light("l-r08.lht").radius(), Some(8));
        assert_eq!(light("L-R00.LHT").radius(), None);
        assert_eq!(light("LAMP.LHT").radius(), None);
        assert_eq!(light_template_name(5), "L-R05.LHT");
        for time in [LightTime::Always, LightTime::Night, LightTime::PrimeTime] {
            assert_eq!(LightTime::from_flags(time.flags() | LIGHT_SPR_ON), time);
        }
    }
}
