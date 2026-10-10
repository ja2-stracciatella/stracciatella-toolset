import { isArray } from 'remeda';
import { JsonRoot } from '../../../../common/invokables/jsons';
import { ItemChoice, ItemCounts } from '../../../lib/mapViewer';
import { LockInfo } from './lockTable';

/** The files with the items, guns first (`ITEM_FILES` in `src-map-editor`) */
export const ITEM_FILES = [
  'weapons.json',
  'magazines.json',
  'armours.json',
  'explosives.json',
  'items.json',
];

/** Item classes (`IC_*`) */
export const IC_NONE = 0x0001;
export const IC_GUN = 0x0002;
export const IC_BLADE = 0x0004;
export const IC_THROWING_KNIFE = 0x0008;
export const IC_LAUNCHER = 0x0010;
export const IC_THROWN = 0x0040;
export const IC_PUNCH = 0x0080;
export const IC_GRENADE = 0x0100;
export const IC_BOMB = 0x0200;
export const IC_AMMO = 0x0400;
export const IC_ARMOUR = 0x0800;
export const IC_MEDKIT = 0x1000;
export const IC_KIT = 0x2000;
export const IC_FACE = 0x8000;
export const IC_KEY = 0x10000;
export const IC_MISC = 0x10000000;
export const IC_MONEY = 0x20000000;

/** Items the editor treats specially (`Item_Types.h`) */
export const SWITCH = 233;
export const ACTION_ITEM = 234;
export const OWNERSHIP = 240;
export const ROCKET_LAUNCHER = 51;
export const UNDER_GLAUNCHER = 50;
export const BIPOD = 209;
export const DETONATOR = 223;
export const REMDETONATOR = 224;
export const CERAMIC_PLATES = 183;

/** An image of an item */
export interface ItemGraphic {
  file: string;
  subImage: number;
}

/** What the editor needs to know about an item */
export interface ItemInfo {
  index: number;
  name: string;
  kind: 'gun' | 'ammo' | 'other';
  /** Item class (`IC_*`) */
  itemClass: number;
  /** Rounds of a full magazine of a gun */
  magSize: number;
  /** Rounds of a magazine */
  capacity: number;
  /** How many fit in a pocket, the most items of a stack */
  perPocket: number;
  /** `ubClassIndex`, the key id of keys */
  classIndex: number;
  /** Items the editor doesn't offer */
  notEditor: boolean;
  /** The small inventory graphic, `null` if it has none */
  graphic: ItemGraphic | null;
  /** The attachments of `weapons.json` that a gun takes, e.g. `Silencer` */
  attachments: string[];
  /** `HELMET`, `VEST`, `LEGGINGS`, `PLATE` or `CREATURE` for armour */
  armourClass: string | null;
}

function numberOf(value: unknown): number {
  return typeof value === 'number' ? value : 0;
}

/** The item class of a weapon type of `weapons.json` (`WeaponModel::deserialize`) */
function weaponClass(weaponType: unknown): number {
  switch (weaponType) {
    case 'BLADE':
      return IC_BLADE;
    case 'THROWINGBLADE':
      return IC_THROWING_KNIFE;
    case 'PUNCH':
    case 'PUNCHWEAPON':
      return IC_PUNCH;
    case 'THROWN':
      return IC_THROWN;
    case 'LAUNCHER':
      return IC_LAUNCHER;
    case 'NOWEAPON':
      return IC_NONE;
    default:
      return IC_GUN;
  }
}

/** The class of an item, which the files other than `items.json` decide */
function itemClassOf(file: string, entry: Record<string, unknown>): number {
  switch (file) {
    case 'weapons.json':
      return weaponClass(entry['internalType']);
    case 'magazines.json':
      return IC_AMMO;
    case 'armours.json':
      return IC_ARMOUR;
    case 'explosives.json':
      return entry['itemClass'] === 'BOMB' ? IC_BOMB : IC_GRENADE;
    default:
      return typeof entry['usItemClass'] === 'number'
        ? entry['usItemClass']
        : IC_NONE;
  }
}

function graphicOf(entry: Record<string, unknown>): ItemGraphic | null {
  const graphics = entry['inventoryGraphics'];
  if (typeof graphics !== 'object' || graphics === null || isArray(graphics)) {
    return null;
  }
  const small = (graphics as Record<string, unknown>)['small'];
  if (typeof small !== 'object' || small === null || isArray(small)) {
    return null;
  }
  const { path, subImageIndex } = small as Record<string, unknown>;
  if (typeof path !== 'string') {
    return null;
  }
  return { file: path, subImage: numberOf(subImageIndex) };
}

/** The items of the item files, by their index in the order of the names */
export function itemCatalog(values: Array<JsonRoot | null>): ItemInfo[] {
  const items = new Map<number, ItemInfo>();
  values.forEach((value, fileIndex) => {
    if (!isArray(value)) {
      return;
    }
    const file = ITEM_FILES[fileIndex] ?? '';
    for (const entry of value) {
      if (isArray(entry) || typeof entry !== 'object' || entry === null) {
        continue;
      }
      const index = entry['itemIndex'];
      const name = entry['internalName'];
      // Item 0 is no item, earlier files win like in the game
      if (typeof index !== 'number' || index === 0 || items.has(index)) {
        continue;
      }
      const armourClass = entry['armourClass'];
      items.set(index, {
        index,
        name: typeof name === 'string' ? name : `Item ${index}`,
        kind:
          file === 'weapons.json'
            ? 'gun'
            : file === 'magazines.json'
              ? 'ammo'
              : 'other',
        itemClass: itemClassOf(file, entry),
        magSize: numberOf(entry['ubMagSize']),
        capacity: numberOf(entry['capacity']),
        perPocket: numberOf(entry['ubPerPocket']),
        classIndex: numberOf(entry['ubClassIndex']),
        notEditor: entry['bNotEditor'] === true,
        graphic: graphicOf(entry),
        attachments: Object.keys(entry)
          .filter((key) => key.startsWith('attachment_') && entry[key] === true)
          .map((key) => key.slice('attachment_'.length)),
        armourClass: typeof armourClass === 'string' ? armourClass : null,
      });
    }
  });
  return [...items.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** The categories of the items palette (`TBAR_MODE_ITEM_*`) */
export const ITEM_CATEGORIES = [
  'weapons',
  'ammo',
  'armour',
  'explosives',
  'equipment',
  'triggers',
  'keys',
] as const;
export type ItemCategory = (typeof ITEM_CATEGORIES)[number];

export const CATEGORY_LABELS: Record<ItemCategory, string> = {
  weapons: 'Weapons',
  ammo: 'Ammo',
  armour: 'Armour',
  explosives: 'Explosives',
  equipment: 'Equipment',
  triggers: 'Triggers',
  keys: 'Keys',
};

/** The category of the items of a class, `null` if the editor doesn't offer them */
function categoryOf(itemClass: number): ItemCategory | null {
  switch (itemClass) {
    case IC_GUN:
    case IC_BLADE:
    case IC_THROWN:
    case IC_LAUNCHER:
    case IC_THROWING_KNIFE:
    case IC_PUNCH:
      return 'weapons';
    case IC_AMMO:
      return 'ammo';
    case IC_ARMOUR:
      return 'armour';
    case IC_GRENADE:
    case IC_BOMB:
      return 'explosives';
    case IC_MEDKIT:
    case IC_KIT:
    case IC_FACE:
    case IC_MISC:
    case IC_MONEY:
      return 'equipment';
    default:
      return null;
  }
}

/** The trigger slots: pairs of a trigger and its action, the last one is the pressure action */
export const NUMBER_TRIGGERS = 27;
export const PRESSURE_ACTION_SLOT = NUMBER_TRIGGERS - 1;
/** Triggers from this frequency are panic triggers, which can be alarm triggers */
export const PANIC_FREQUENCY_3 = 125;

/** The frequency of the triggers and actions of a slot (`CalcItemFrequency`) */
export function triggerFrequency(slot: number): number {
  if (slot < 2) {
    return 127;
  }
  if (slot < 4) {
    return 126;
  }
  if (slot < 6) {
    return PANIC_FREQUENCY_3;
  }
  return 50 + Math.floor((slot - 4) / 2);
}

/** Whether a slot places a trigger, the others place actions */
export function isTriggerSlot(slot: number): boolean {
  return slot < PRESSURE_ACTION_SLOT && slot % 2 === 0;
}

/** The name of a trigger slot (`InitEditorItemsInfo`) */
export function triggerName(slot: number): string {
  if (slot === PRESSURE_ACTION_SLOT) {
    return 'Pressure Action';
  }
  const kind = isTriggerSlot(slot) ? 'Trigger' : 'Action';
  if (slot < 6) {
    return `Panic ${kind}${Math.floor(slot / 2) + 1}`;
  }
  return `${kind}${Math.floor((slot - 4) / 2)}`;
}

/** An entry of the items palette */
export interface ItemPaletteEntry {
  /** Unique in its category */
  key: string;
  label: string;
  choice: ItemChoice;
  graphic: ItemGraphic | null;
}

/** The key item of a lock: the key whose class index is the key of the lock (`getKeyItemForKeyId`) */
export function keyItemOf(
  catalog: ItemInfo[],
  lock: LockInfo,
): ItemInfo | undefined {
  return catalog.find(
    (item) => item.itemClass === IC_KEY && item.classIndex === lock.keyItem,
  );
}

/**
 * The entries of each category of the items palette, in the order of the original
 * (`InitEditorItemsInfo`): the items in the order of their index, the 27 trigger slots and a key
 * for each lock
 */
export function editorCategories(
  catalog: ItemInfo[],
  locks: LockInfo[],
): Record<ItemCategory, ItemPaletteEntry[]> {
  const categories: Record<ItemCategory, ItemPaletteEntry[]> = {
    weapons: [],
    ammo: [],
    armour: [],
    explosives: [],
    equipment: [],
    triggers: [],
    keys: [],
  };
  const byIndex = [...catalog].sort((a, b) => a.index - b.index);
  for (const item of byIndex) {
    if (item.notEditor || item.index === SWITCH || item.index === ACTION_ITEM) {
      continue;
    }
    const category = categoryOf(item.itemClass);
    if (category) {
      categories[category].push({
        key: `${item.index}`,
        label: item.name,
        choice: { kind: 'item', item: item.index },
        graphic: item.graphic,
      });
    }
  }
  const items = new Map(catalog.map((item) => [item.index, item]));
  for (let slot = 0; slot < NUMBER_TRIGGERS; slot++) {
    const item = items.get(isTriggerSlot(slot) ? SWITCH : ACTION_ITEM);
    categories.triggers.push({
      key: `${slot}`,
      label: triggerName(slot),
      choice: { kind: 'trigger', slot },
      graphic: item?.graphic ?? null,
    });
  }
  locks.forEach((lock, lockId) => {
    const key = keyItemOf(catalog, lock);
    if (!key) {
      return;
    }
    categories.keys.push({
      key: `${lockId}`,
      label: lock.name || `Lock ${lockId}`,
      choice: { kind: 'key', lockId, item: key.index },
      graphic: key.graphic,
    });
  });
  return categories;
}

/** The attachments the original editor offers for guns, with the flag of `weapons.json` */
export const WEAPON_ATTACHMENTS: {
  item: number;
  flag: string;
  label: string;
}[] = [
  { item: 207, flag: 'Silencer', label: 'Silencer' },
  { item: 208, flag: 'SniperScope', label: 'Sniper scope' },
  { item: 241, flag: 'LaserScope', label: 'Laser scope' },
  { item: BIPOD, flag: 'Bipod', label: 'Bipod' },
  { item: 244, flag: 'Duckbill', label: 'Duckbill' },
  { item: UNDER_GLAUNCHER, flag: 'UnderGLauncher', label: 'G-launcher' },
];

/** The vests that take ceramic plates (`platesCanBeAttached`) */
const PLATE_VESTS = new Set([
  'FLAK_JACKET',
  'FLAK_JACKET_18',
  'FLAK_JACKET_Y',
  'KEVLAR_VEST',
  'KEVLAR_VEST_18',
  'KEVLAR_VEST_Y',
  'KEVLAR2_VEST',
  'KEVLAR2_VEST_18',
  'KEVLAR2_VEST_Y',
  'SPECTRA_VEST',
  'SPECTRA_VEST_18',
  'SPECTRA_VEST_Y',
]);

/** The explosives that take detonators */
const DETONATED_EXPLOSIVES = new Set([137, 138, 139, 142]);

/** Whether an attachment fits an item (`ValidAttachment`) */
export function validAttachment(
  attachment: number,
  item: ItemInfo | undefined,
): boolean {
  if (!item) {
    return false;
  }
  if (attachment === CERAMIC_PLATES) {
    return PLATE_VESTS.has(item.name);
  }
  if (attachment === DETONATOR || attachment === REMDETONATOR) {
    return DETONATED_EXPLOSIVES.has(item.index);
  }
  const weapon = WEAPON_ATTACHMENTS.find((a) => a.item === attachment);
  return weapon !== undefined && item.attachments.includes(weapon.flag);
}

/**
 * The attachments that exclude each other (`ValidItemAttachment`): a bipod and a grenade
 * launcher, a detonator and a remote detonator
 */
const EXCLUSIVE_ATTACHMENTS: [number, number][] = [
  [BIPOD, UNDER_GLAUNCHER],
  [DETONATOR, REMDETONATOR],
];

/** Whether an attachment can be added to the attachments an item has */
export function canAttach(attachment: number, attached: number[]): boolean {
  if (attached.includes(attachment) || attached.length >= 4) {
    return false;
  }
  return !EXCLUSIVE_ATTACHMENTS.some(
    ([a, b]) =>
      (attachment === a && attached.includes(b)) ||
      (attachment === b && attached.includes(a)),
  );
}

/** The names of the actions of action items, by their index (`gszActionItemDesc`) */
export const ACTION_ITEM_NAMES = [
  'Klaxon Mine',
  'Flare Mine',
  'Teargas Explosion',
  'Stun Explosion',
  'Smoke Explosion',
  'Mustard Gas',
  'Land Mine',
  'Open Door',
  'Close Door',
  '3x3 Hidden Pit',
  '5x5 Hidden Pit',
  'Small Explosion',
  'Medium Explosion',
  'Large Explosion',
  'Toggle Door',
  'Toggle Action1s',
  'Toggle Action2s',
  'Toggle Action3s',
  'Toggle Action4s',
  'Enter Brothel',
  'Exit Brothel',
  'Kingpin Alarm',
  'Sex with Prostitute',
  'Reveal Room',
  'Local Alarm',
  'Global Alarm',
  'Klaxon Sound',
  'Unlock door',
  'Toggle lock',
  'Untrap door',
  'Tog pressure items',
  'Museum alarm',
  'Bloodcat alarm',
  'Big teargas',
];

/** How many items of an entry of the palette the map has, `null` if none */
export function placedCount(
  entry: ItemPaletteEntry,
  counts: ItemCounts | null,
): { placed: number; quantity: number } | null {
  if (!counts) {
    return null;
  }
  const { choice } = entry;
  if (choice.kind === 'item') {
    const count = counts.items.find((c) => c.item === choice.item);
    return count && count.placed > 0
      ? { placed: count.placed, quantity: count.quantity }
      : null;
  }
  const placed =
    choice.kind === 'trigger'
      ? (counts.triggers[choice.slot] ?? 0)
      : (counts.keys.find((c) => c.lockId === choice.lockId)?.placed ?? 0);
  return placed > 0 ? { placed, quantity: placed } : null;
}
