//! Typed views of the item class specific data of an item stack (the union in `OBJECTTYPE`)
//!
//! The 12 bytes mean different things for guns, money, bombs, keys and ownership items, the item
//! class decides which view applies. Each view only reads and writes its own bytes, the others
//! are kept. The offsets are the ones of `Item_Types.h`, the host decodes the same layout
//! (`itemFields.ts`).

/// Raw data of an item stack
pub type ItemData = [u8; 12];

fn read_u16(data: &ItemData, offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn write_u16(data: &mut ItemData, offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

/// A gun (`bGunStatus`, `ubGunAmmoType`, `ubGunShotsLeft`, `usGunAmmoItem`, `bGunAmmoStatus`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GunData {
    pub status: i8,
    pub ammo_type: u8,
    pub shots_left: u8,
    /// The magazine in the gun, 0 for none
    pub ammo_item: u16,
    pub ammo_status: i8,
}

impl GunData {
    pub fn read(data: &ItemData) -> Self {
        Self {
            status: data[0] as i8,
            ammo_type: data[1],
            shots_left: data[2],
            ammo_item: read_u16(data, 4),
            ammo_status: data[6] as i8,
        }
    }

    pub fn write(&self, data: &mut ItemData) {
        data[0] = self.status as u8;
        data[1] = self.ammo_type;
        data[2] = self.shots_left;
        write_u16(data, 4, self.ammo_item);
        data[6] = self.ammo_status as u8;
    }
}

/// Money (`bMoneyStatus`, `uiMoneyAmount`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoneyData {
    pub status: i8,
    pub amount: u32,
}

impl MoneyData {
    pub fn read(data: &ItemData) -> Self {
        Self {
            status: data[0] as i8,
            amount: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
        }
    }

    pub fn write(&self, data: &mut ItemData) {
        data[0] = self.status as u8;
        data[4..8].copy_from_slice(&self.amount.to_le_bytes());
    }
}

/// How a placed bomb goes off (`DetonatorType`)
pub const BOMB_TIMED: i8 = 1;
pub const BOMB_REMOTE: i8 = 2;
pub const BOMB_PRESSURE: i8 = 3;

/// A placed bomb, a switch or an action item (`bBombStatus`, `bDetonatorType`, `usBombItem`,
/// `bFrequency` or `bDelay`, `ubBombOwner`, `bActionValue`, `ubTolerance`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BombData {
    pub status: i8,
    pub detonator: i8,
    /// The item that explodes, 0 for actions without an explosion
    pub bomb_item: u16,
    /// The frequency of remote bombs and triggers, the delay of timed bombs
    pub frequency: i8,
    pub owner: u8,
    /// What the action item does (`ItemActionType`)
    pub action_value: u8,
    /// How likely the AI is to use a panic trigger
    pub tolerance: u8,
}

impl BombData {
    pub fn read(data: &ItemData) -> Self {
        Self {
            status: data[0] as i8,
            detonator: data[1] as i8,
            bomb_item: read_u16(data, 2),
            frequency: data[4] as i8,
            owner: data[5],
            action_value: data[6],
            tolerance: data[7],
        }
    }

    pub fn write(&self, data: &mut ItemData) {
        data[0] = self.status as u8;
        data[1] = self.detonator as u8;
        write_u16(data, 2, self.bomb_item);
        data[4] = self.frequency as u8;
        data[5] = self.owner;
        data[6] = self.action_value;
        data[7] = self.tolerance;
    }
}

/// A key (`bKeyStatus[0]`, `ubKeyID`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyData {
    pub status: i8,
    /// The lock the key opens, the index in `Locks.bin`
    pub key_id: u8,
}

impl KeyData {
    pub fn read(data: &ItemData) -> Self {
        Self {
            status: data[0] as i8,
            key_id: data[6],
        }
    }

    pub fn write(&self, data: &mut ItemData) {
        data[0] = self.status as u8;
        data[6] = self.key_id;
    }
}

/// An ownership item (`ubOwnerProfile`, `ubOwnerCivGroup`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnerData {
    pub profile: u8,
    pub civ_group: u8,
}

impl OwnerData {
    pub fn read(data: &ItemData) -> Self {
        Self {
            profile: data[0],
            civ_group: data[1],
        }
    }

    pub fn write(&self, data: &mut ItemData) {
        data[0] = self.profile;
        data[1] = self.civ_group;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_use_the_offsets_of_the_game() {
        let mut data = [0xee; 12];
        GunData {
            status: 90,
            ammo_type: 1,
            shots_left: 15,
            ammo_item: 0x0147,
            ammo_status: 100,
        }
        .write(&mut data);
        assert_eq!(
            data,
            [90, 1, 15, 0xee, 0x47, 0x01, 100, 0xee, 0xee, 0xee, 0xee, 0xee]
        );
        assert_eq!(GunData::read(&data).ammo_item, 0x0147);

        let mut data = [0; 12];
        MoneyData {
            status: 100,
            amount: 20000,
        }
        .write(&mut data);
        assert_eq!(data[..8], [100, 0, 0, 0, 0x20, 0x4e, 0, 0]);
        assert_eq!(MoneyData::read(&data).amount, 20000);

        let mut data = [0; 12];
        let bomb = BombData {
            status: 100,
            detonator: BOMB_PRESSURE,
            bomb_item: 137,
            frequency: -1,
            owner: 1,
            action_value: 3,
            tolerance: 50,
        };
        bomb.write(&mut data);
        assert_eq!(data[..8], [100, 3, 137, 0, 0xff, 1, 3, 50]);
        assert_eq!(BombData::read(&data), bomb);

        // Views only change their own bytes
        let mut data = [7; 12];
        KeyData {
            status: 80,
            key_id: 12,
        }
        .write(&mut data);
        assert_eq!(data, [80, 7, 7, 7, 7, 7, 12, 7, 7, 7, 7, 7]);
        OwnerData {
            profile: 200,
            civ_group: 3,
        }
        .write(&mut data);
        assert_eq!(data[..3], [200, 3, 7]);
        assert_eq!(
            OwnerData::read(&data),
            OwnerData {
                profile: 200,
                civ_group: 3
            }
        );
    }
}
