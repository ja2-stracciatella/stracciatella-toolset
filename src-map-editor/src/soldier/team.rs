//! Teams (`Team`)

use super::{definition_id, Definition};

definition_id!(
    /// A team of [`TEAMS`]
    TeamId(u8)
);

impl TeamId {
    pub const OUR: Self = Self(0);
    pub const ENEMY: Self = Self(1);
    pub const CREATURE: Self = Self(2);
    pub const MILITIA: Self = Self(3);
    pub const CIV: Self = Self(4);
    pub const PLAYER_PLAN: Self = Self(5);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    pub id: TeamId,
    pub name: &'static str,
    /// The editor adds the soldiers of these teams, in the order of their ids (`LoadScreen.cc`)
    pub placed_by_editor: bool,
    /// Glows instead of being lit (`bNeutral`, `bSide`)
    pub hostile: bool,
}

const fn team(id: TeamId, name: &'static str, placed_by_editor: bool, hostile: bool) -> Team {
    Team {
        id,
        name,
        placed_by_editor,
        hostile,
    }
}

pub const TEAMS: [Team; 6] = [
    team(TeamId::OUR, "OUR_TEAM", false, false),
    team(TeamId::ENEMY, "ENEMY_TEAM", true, true),
    team(TeamId::CREATURE, "CREATURE_TEAM", true, true),
    team(TeamId::MILITIA, "MILITIA_TEAM", true, false),
    team(TeamId::CIV, "CIV_TEAM", true, false),
    team(TeamId::PLAYER_PLAN, "PLAYER_PLAN", false, false),
];

impl Definition for Team {
    type Id = TeamId;

    fn id(&self) -> TeamId {
        self.id
    }
}
