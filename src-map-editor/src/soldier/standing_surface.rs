//! Animation surfaces that show a standing soldier (`gAnimSurfaceDatabase`)

use super::{definition_id, Definition};

definition_id!(
    /// A surface of [`STANDING_SURFACES`]
    StandingSurfaceId(u8)
);

impl StandingSurfaceId {
    pub const REGMALE_RIFLE: Self = Self(0);
    pub const REGMALE_NOTHING: Self = Self(1);
    pub const REGMALE_PISTOL: Self = Self(2);
    pub const BIGMALE_RIFLE: Self = Self(3);
    pub const BIGMALE_NOTHING: Self = Self(4);
    pub const BIGMALE_PISTOL: Self = Self(5);
    pub const REGFEMALE_RIFLE: Self = Self(6);
    pub const REGFEMALE_NOTHING: Self = Self(7);
    pub const REGFEMALE_PISTOL: Self = Self(8);
    pub const ADULT_MONSTER: Self = Self(9);
    pub const LARVAE_MONSTER: Self = Self(10);
    pub const INFANT_MONSTER: Self = Self(11);
    pub const QUEEN_MONSTER: Self = Self(12);
    pub const FATCIV: Self = Self(13);
    pub const MANCIV: Self = Self(14);
    pub const MINICIV: Self = Self(15);
    pub const DRESSCIV: Self = Self(16);
    pub const HATKIDCIV: Self = Self(17);
    pub const KIDCIV: Self = Self(18);
    pub const CRIPPLECIV: Self = Self(19);
    pub const COW: Self = Self(20);
    pub const CROW: Self = Self(21);
    pub const BLOODCAT: Self = Self(22);
    pub const ROBOT: Self = Self(23);
    pub const HUMVEE: Self = Self(24);
    pub const TANK_NW: Self = Self(25);
    pub const TANK_NE: Self = Self(26);
    pub const ICECREAMTRUCK: Self = Self(27);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandingSurface {
    pub id: StandingSurfaceId,
    /// Path relative to the data root
    pub file: &'static str,
    /// Number of directions of the animation
    pub directions: u8,
}

const fn surface(id: StandingSurfaceId, file: &'static str, directions: u8) -> StandingSurface {
    StandingSurface {
        id,
        file,
        directions,
    }
}

/// All standing surfaces, the viewer loads all of them
pub const STANDING_SURFACES: [StandingSurface; 28] = {
    use StandingSurfaceId as S;
    [
        surface(S::REGMALE_RIFLE, "anims/s_merc/s_r_std.sti", 8),
        surface(S::REGMALE_NOTHING, "anims/s_merc/s_n_std.sti", 8),
        surface(S::REGMALE_PISTOL, "anims/s_merc/s_p_brth.sti", 8),
        surface(S::BIGMALE_RIFLE, "anims/m_merc/m_breth2.sti", 8),
        surface(S::BIGMALE_NOTHING, "anims/m_merc/m_n_std.sti", 8),
        surface(S::BIGMALE_PISTOL, "anims/m_merc/m_p_brth.sti", 8),
        surface(S::REGFEMALE_RIFLE, "anims/f_merc/f_breth2.sti", 8),
        surface(S::REGFEMALE_NOTHING, "anims/f_merc/n_breth2.sti", 8),
        surface(S::REGFEMALE_PISTOL, "anims/f_merc/f_p_brth.sti", 8),
        surface(S::ADULT_MONSTER, "anims/monsters/mn_breat.sti", 8),
        surface(S::LARVAE_MONSTER, "anims/monsters/l_breath.sti", 8),
        surface(S::INFANT_MONSTER, "anims/monsters/i_breath.sti", 8),
        surface(S::QUEEN_MONSTER, "anims/monsters/qmn_breat.sti", 1),
        surface(S::FATCIV, "anims/civs/ft_brth.sti", 8),
        surface(S::MANCIV, "anims/civs/m_breath.sti", 8),
        surface(S::MINICIV, "anims/civs/mi_breath.sti", 8),
        surface(S::DRESSCIV, "anims/civs/ds_breath.sti", 8),
        surface(S::HATKIDCIV, "anims/civs/h_breath.sti", 8),
        surface(S::KIDCIV, "anims/civs/k_breath.sti", 8),
        surface(S::CRIPPLECIV, "anims/civs/cp_breth.sti", 8),
        surface(S::COW, "anims/animals/c_breath.sti", 8),
        surface(S::CROW, "anims/animals/cr_eat.sti", 8),
        surface(S::BLOODCAT, "anims/animals/ct_breath.sti", 8),
        surface(S::ROBOT, "anims/civs/j_r_bret.sti", 8),
        surface(S::HUMVEE, "anims/vehicles/hummer2.sti", 32),
        surface(S::TANK_NW, "anims/vehicles/tank_rot.sti", 32),
        surface(S::TANK_NE, "anims/vehicles/tnk2_rot.sti", 32),
        surface(S::ICECREAMTRUCK, "anims/vehicles/icecrm.sti", 32),
    ]
};

impl Definition for StandingSurface {
    type Id = StandingSurfaceId;

    fn id(&self) -> StandingSurfaceId {
        self.id
    }
}
