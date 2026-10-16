//! The items of the game, as far as the editor needs them: their graphics on the ground, how
//! soldiers hold them and what new world items get (`ItemModel` of the game)
//!
//! The items are read from the externalized item files (`ITEM_FILES`).

use std::{collections::HashMap, io};

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use serde::Deserialize;

use crate::file_formats::ja2_tile_types::{TileType, GUNS, P1ITEMS, P2ITEMS, P3ITEMS};

/// Registers the item file asset and its loader
pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ItemFile>()
            .init_asset_loader::<ItemFileLoader>();
    }
}

/// An item (`usItem`), the item index of the item files
///
/// Item 0 (`NOTHING`) is not an item, empty inventory slots and attachments are `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u16);

impl ItemId {
    /// Items that are not moved onto structures (`OWNERSHIP`, `ACTION_ITEM`)
    pub const OWNERSHIP: Self = Self(240);
    pub const ACTION_ITEM: Self = Self(234);
    /// Panic triggers and triggers of action items
    pub const SWITCH: Self = Self(233);
    pub const ROCKET_LAUNCHER: Self = Self(51);
    pub const TANK_CANNON: Self = Self(60);

    /// The item with the given index, `None` for `NOTHING`
    pub const fn new(index: u16) -> Option<Self> {
        if index == 0 {
            None
        } else {
            Some(Self(index))
        }
    }

    pub const fn index(self) -> u16 {
        self.0
    }
}

/// Files with the tile graphics of the items, earlier files win (`DefaultContentManager`)
pub const ITEM_FILES: [&str; 5] = [
    "weapons.json",
    "magazines.json",
    "armours.json",
    "explosives.json",
    "items.json",
];

/// Item classes (`IC_*`)
pub const IC_NONE: u32 = 0x0001;
pub const IC_GUN: u32 = 0x0002;
pub const IC_BLADE: u32 = 0x0004;
pub const IC_THROWING_KNIFE: u32 = 0x0008;
pub const IC_LAUNCHER: u32 = 0x0010;
pub const IC_THROWN: u32 = 0x0040;
pub const IC_PUNCH: u32 = 0x0080;
pub const IC_GRENADE: u32 = 0x0100;
pub const IC_BOMB: u32 = 0x0200;
pub const IC_AMMO: u32 = 0x0400;
pub const IC_ARMOUR: u32 = 0x0800;
pub const IC_MEDKIT: u32 = 0x1000;
pub const IC_KIT: u32 = 0x2000;
pub const IC_FACE: u32 = 0x8000;
pub const IC_KEY: u32 = 0x10000;
pub const IC_MISC: u32 = 0x1000_0000;
pub const IC_MONEY: u32 = 0x2000_0000;

/// The ammo types of the game by their index (`ammo-types.json`), magazines of other types are
/// treated as regular ammo
const AMMO_TYPES: [&str; 13] = [
    "AMMO_REGULAR",
    "AMMO_HP",
    "AMMO_AP",
    "AMMO_SUPER_AP",
    "AMMO_BUCKSHOT",
    "AMMO_FLECHETTE",
    "AMMO_GRENADE",
    "AMMO_MONSTER",
    "AMMO_KNIFE",
    "AMMO_HE",
    "AMMO_HEAT",
    "AMMO_SLEEP_DART",
    "AMMO_FLAME",
];

#[derive(Clone, Debug, Deserialize)]
struct TileGraphic {
    #[serde(rename = "type")]
    tile_type: String,
    #[serde(rename = "subIndex")]
    sub_index: u16,
}

#[derive(Clone, Debug, Deserialize)]
struct ItemEntry {
    #[serde(rename = "itemIndex")]
    item_index: u16,
    #[serde(rename = "internalName")]
    internal_name: Option<String>,
    #[serde(rename = "usItemClass")]
    item_class: Option<u32>,
    /// Weapon type, only in `weapons.json`
    #[serde(rename = "internalType")]
    weapon_type: Option<String>,
    #[serde(rename = "bTwoHanded", default)]
    two_handed: bool,
    #[serde(rename = "tileGraphic")]
    tile_graphic: Option<TileGraphic>,
    /// Item class of the explosives, `GRENADE` or `BOMB`
    #[serde(rename = "itemClass")]
    explosive_class: Option<String>,
    #[serde(rename = "ubClassIndex", default)]
    class_index: u16,
    #[serde(rename = "ubPerPocket", default)]
    per_pocket: u8,
    #[serde(rename = "ubMagSize", default)]
    mag_size: u8,
    #[serde(default)]
    capacity: u8,
    calibre: Option<String>,
    #[serde(rename = "ammoType")]
    ammo_type: Option<String>,
    #[serde(rename = "dontUseAsDefaultMagazine", default)]
    not_default_magazine: bool,
    #[serde(rename = "bNotEditor", default)]
    not_editor: bool,
    #[serde(rename = "bSinks", default)]
    sinks: bool,
    #[serde(rename = "bDefaultUndroppable", default)]
    default_undroppable: bool,
}

/// What new world items of an item get and whether the editor offers it
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemKind {
    /// Item class (`IC_*`)
    pub class: u32,
    /// `ubClassIndex`, the key id of keys
    pub class_index: u16,
    pub per_pocket: u8,
    /// Rounds of a full gun
    pub mag_size: u8,
    /// Rounds of a magazine
    pub capacity: u8,
    /// Calibre of guns and magazines
    pub calibre: Option<String>,
    /// Index of the ammo type of magazines
    pub ammo_type: u8,
    /// Magazines that `DefaultMagazine` skips
    pub not_default_magazine: bool,
    /// Items the editor doesn't offer (`ITEM_NOT_EDITOR`)
    pub not_editor: bool,
    /// Items that are lost in water (`ITEM_SINKS`)
    pub sinks: bool,
    /// Items that stay with dead soldiers (`ITEM_DEFAULT_UNDROPPABLE`)
    pub default_undroppable: bool,
}

/// The item class of a weapon type of `weapons.json` (`WeaponModel::deserialize`)
fn weapon_class(weapon_type: &str) -> u32 {
    match weapon_type {
        "BLADE" => IC_BLADE,
        "THROWINGBLADE" => IC_THROWING_KNIFE,
        "PUNCH" | "PUNCHWEAPON" => IC_PUNCH,
        "THROWN" => IC_THROWN,
        "LAUNCHER" => IC_LAUNCHER,
        "NOWEAPON" => IC_NONE,
        _ => IC_GUN,
    }
}

/// How a soldier holds an item in the hand, for the standing animation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandItem {
    /// A pistol or another one-handed gun
    OneHandedGun,
    /// A rifle or another two-handed gun or launcher
    TwoHandedGun,
}

/// An item of an items file
#[derive(Clone, Debug, PartialEq)]
pub struct ItemInfo {
    pub index: ItemId,
    /// Tile type and sub index
    pub graphic: Option<(TileType, u16)>,
    /// Set for guns and launchers (`IC_GUN`, `IC_LAUNCHER`) except the rocket launcher
    pub hand: Option<HandItem>,
    pub kind: ItemKind,
}

/// The items of one items file
#[derive(Asset, TypePath, Clone, Debug, Default, PartialEq)]
pub struct ItemFile {
    pub items: Vec<ItemInfo>,
}

impl ItemFile {
    /// Parses one of the [`ITEM_FILES`], by its lower case name, whose items get their class
    /// from the file
    pub fn parse(bytes: &[u8], file: &str) -> io::Result<Self> {
        let items_json = file == "items.json";
        let entries: Vec<ItemEntry> = serde_json::from_slice(bytes)?;
        let skipped = if items_json {
            IC_NONE | IC_GRENADE | IC_BOMB | IC_ARMOUR
        } else {
            IC_NONE
        };
        let items = entries
            .into_iter()
            .filter(|entry| entry.item_class.is_none_or(|class| class & skipped == 0))
            .filter_map(|entry| {
                // `NOTHING` is not an item
                let index = ItemId::new(entry.item_index)?;
                let graphic = entry.tile_graphic.and_then(|graphic| {
                    let tile_type = match graphic.tile_type.as_str() {
                        "GUNS" => GUNS,
                        "P1ITEMS" => P1ITEMS,
                        "P2ITEMS" => P2ITEMS,
                        "P3ITEMS" => P3ITEMS,
                        other => {
                            warn!(
                                "Unsupported tile type {} of item {}",
                                other, entry.item_index
                            );
                            return None;
                        }
                    };
                    Some((tile_type, graphic.sub_index))
                });
                let is_gun = matches!(
                    entry.weapon_type.as_deref(),
                    Some(
                        "PISTOL"
                            | "M_PISTOL"
                            | "SMG"
                            | "RIFLE"
                            | "SN_RIFLE"
                            | "ASRIFLE"
                            | "LMG"
                            | "SHOTGUN"
                            | "LAUNCHER"
                            | "LAW"
                            | "CANNON"
                    )
                ) && entry.internal_name.as_deref() != Some("ROCKET_LAUNCHER");
                let hand = is_gun.then_some(if entry.two_handed {
                    HandItem::TwoHandedGun
                } else {
                    HandItem::OneHandedGun
                });
                let class = match file {
                    "weapons.json" => weapon_class(entry.weapon_type.as_deref().unwrap_or("")),
                    "magazines.json" => IC_AMMO,
                    "armours.json" => IC_ARMOUR,
                    "explosives.json" => match entry.explosive_class.as_deref() {
                        Some("BOMB") => IC_BOMB,
                        _ => IC_GRENADE,
                    },
                    _ => entry.item_class.unwrap_or(IC_NONE),
                };
                let ammo_type = entry.ammo_type.as_deref().map_or(0, |name| {
                    AMMO_TYPES.iter().position(|&t| t == name).unwrap_or(0) as u8
                });
                let kind = ItemKind {
                    class,
                    class_index: entry.class_index,
                    per_pocket: entry.per_pocket,
                    mag_size: entry.mag_size,
                    capacity: entry.capacity,
                    calibre: entry.calibre,
                    ammo_type,
                    not_default_magazine: entry.not_default_magazine,
                    not_editor: entry.not_editor,
                    sinks: entry.sinks,
                    default_undroppable: entry.default_undroppable,
                };
                Some(ItemInfo {
                    index,
                    graphic,
                    hand,
                    kind,
                })
            })
            .collect();
        Ok(Self { items })
    }
}

#[derive(Default, TypePath)]
pub struct ItemFileLoader;

impl AssetLoader for ItemFileLoader {
    type Asset = ItemFile;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<ItemFile, io::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let file = load_context
            .path()
            .path()
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        ItemFile::parse(&bytes, &file)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Tile graphics, hand items and kinds of all items
#[derive(Resource, Clone, Debug, Default)]
pub struct ItemTable {
    graphics: HashMap<ItemId, (TileType, u16)>,
    hands: HashMap<ItemId, HandItem>,
    kinds: HashMap<ItemId, ItemKind>,
    /// The magazines in the order of the files, which `DefaultMagazine` searches
    magazines: Vec<ItemId>,
}

impl ItemTable {
    /// Combines the item files, in the order of [`ITEM_FILES`]
    pub fn new<'a>(files: impl IntoIterator<Item = &'a ItemFile>) -> Self {
        let mut table = Self::default();
        for file in files {
            for item in &file.items {
                if table.graphics.contains_key(&item.index) {
                    continue;
                }
                // Items without a graphic show the first gun (`TilesetTileIndexModel`)
                table
                    .graphics
                    .insert(item.index, item.graphic.unwrap_or((GUNS, 1)));
                if let Some(hand) = item.hand {
                    table.hands.insert(item.index, hand);
                }
                if item.kind.class == IC_AMMO {
                    table.magazines.push(item.index);
                }
                table.kinds.insert(item.index, item.kind.clone());
            }
        }
        table
    }

    /// Tile type and sub index of the graphic of an item on the ground, `None` if the item is
    /// not in any items file
    pub fn graphic(&self, item: ItemId) -> Option<(TileType, u16)> {
        self.graphics.get(&item).copied()
    }

    /// How an item is held, `None` if it is not a gun
    pub fn hand(&self, item: ItemId) -> Option<HandItem> {
        self.hands.get(&item).copied()
    }

    /// The class and the values of an item, `None` if it is not in any items file
    pub fn info(&self, item: ItemId) -> Option<&ItemKind> {
        self.kinds.get(&item)
    }

    /// The magazine a new gun is loaded with: the first one of its calibre that holds a full gun
    /// (`DefaultMagazine`), `None` if there is none
    pub fn default_magazine(&self, gun: ItemId) -> Option<ItemId> {
        let gun = self.info(gun)?;
        let calibre = gun.calibre.as_ref()?;
        self.magazines.iter().copied().find(|&magazine| {
            self.info(magazine).is_some_and(|mag| {
                !mag.not_default_magazine
                    && mag.calibre.as_ref() == Some(calibre)
                    && mag.capacity == gun.mag_size
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(index: u16) -> ItemId {
        ItemId::new(index).unwrap()
    }

    #[test]
    fn parses_item_files() {
        let json = br#"[
            { "itemIndex": 0, "internalName": "NOTHING" },
            { "itemIndex": 2, "internalType": "PISTOL", "calibre": "AMMO9", "ubMagSize": 15,
              "tileGraphic": { "type": "GUNS", "subIndex": 3 } },
            { "itemIndex": 5, "usItemClass": 256, "tileGraphic": { "type": "P1ITEMS", "subIndex": 1 } },
            { "itemIndex": 6, "usItemClass": 4096, "tileGraphic": { "type": "P3ITEMS", "subIndex": 4 } },
            { "itemIndex": 7, "internalType": "RIFLE", "bTwoHanded": true }
        ]"#;
        let file = ItemFile::parse(json, "items.json").unwrap();
        let summary: Vec<_> = file
            .items
            .iter()
            .map(|item| (item.index, item.graphic, item.hand))
            .collect();
        assert_eq!(
            summary,
            vec![
                (id(2), Some((GUNS, 3)), Some(HandItem::OneHandedGun)),
                (id(6), Some((P3ITEMS, 4)), None),
                (id(7), None, Some(HandItem::TwoHandedGun)),
            ]
        );
        // Grenades are only skipped in items.json
        assert_eq!(
            ItemFile::parse(json, "weapons.json").unwrap().items.len(),
            4
        );

        let other = ItemFile {
            items: vec![
                ItemInfo {
                    index: id(2),
                    graphic: Some((P1ITEMS, 1)),
                    hand: None,
                    kind: ItemKind::default(),
                },
                ItemInfo {
                    index: id(8),
                    graphic: Some((P2ITEMS, 2)),
                    hand: None,
                    kind: ItemKind::default(),
                },
            ],
        };
        let table = ItemTable::new([&file, &other]);
        assert_eq!(table.graphic(id(2)), Some((GUNS, 3)));
        assert_eq!(table.graphic(id(7)), Some((GUNS, 1)));
        assert_eq!(table.graphic(id(8)), Some((P2ITEMS, 2)));
        assert_eq!(table.graphic(id(9)), None);
        assert_eq!(table.hand(id(2)), Some(HandItem::OneHandedGun));
        assert_eq!(table.hand(id(8)), None);
        // Items without a class in the file are no items
        assert_eq!(table.info(id(7)).map(|kind| kind.class), Some(IC_NONE));
        assert_eq!(table.info(id(6)).map(|kind| kind.class), Some(IC_MEDKIT));
    }

    #[test]
    fn items_get_their_class_from_the_file() {
        let weapons = br#"[
            { "itemIndex": 1, "internalType": "PISTOL", "calibre": "AMMO9", "ubMagSize": 15,
              "bSinks": true },
            { "itemIndex": 2, "internalType": "BLADE", "bNotEditor": true },
            { "itemIndex": 3, "internalType": "LAUNCHER" }
        ]"#;
        let magazines = br#"[
            { "itemIndex": 10, "calibre": "AMMO9", "capacity": 15, "ammoType": "AMMO_AP",
              "dontUseAsDefaultMagazine": true },
            { "itemIndex": 11, "calibre": "AMMO9", "capacity": 30, "ammoType": "AMMO_REGULAR" },
            { "itemIndex": 12, "calibre": "AMMO9", "capacity": 15, "ammoType": "AMMO_HP",
              "ubPerPocket": 8 }
        ]"#;
        let explosives = br#"[
            { "itemIndex": 20, "itemClass": "BOMB" },
            { "itemIndex": 21, "itemClass": "GRENADE" }
        ]"#;
        let items = br#"[
            { "itemIndex": 30, "usItemClass": 65536, "ubClassIndex": 4,
              "bDefaultUndroppable": true }
        ]"#;
        let table = ItemTable::new(&[
            ItemFile::parse(weapons, "weapons.json").unwrap(),
            ItemFile::parse(magazines, "magazines.json").unwrap(),
            ItemFile::parse(explosives, "explosives.json").unwrap(),
            ItemFile::parse(items, "items.json").unwrap(),
        ]);
        let class = |index| table.info(id(index)).unwrap().class;
        assert_eq!(
            [1, 2, 3, 10, 20, 21, 30].map(class),
            [
                IC_GUN,
                IC_BLADE,
                IC_LAUNCHER,
                IC_AMMO,
                IC_BOMB,
                IC_GRENADE,
                IC_KEY
            ]
        );
        let pistol = table.info(id(1)).unwrap();
        assert!(pistol.sinks && !pistol.not_editor);
        assert!(table.info(id(2)).unwrap().not_editor);
        assert_eq!(table.info(id(12)).unwrap().ammo_type, 1);
        assert_eq!(table.info(id(12)).unwrap().per_pocket, 8);
        let key = table.info(id(30)).unwrap();
        assert_eq!(key.class_index, 4);
        assert!(key.default_undroppable);
        // The first magazine of the calibre that fills the gun and may be used
        assert_eq!(table.default_magazine(id(1)), Some(id(12)));
        assert_eq!(table.default_magazine(id(3)), None);
    }
}
