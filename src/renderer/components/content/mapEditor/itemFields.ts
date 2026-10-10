import { ItemStack } from '../../../lib/mapViewer';
import {
  ACTION_ITEM,
  IC_AMMO,
  IC_ARMOUR,
  IC_BLADE,
  IC_BOMB,
  IC_FACE,
  IC_GRENADE,
  IC_GUN,
  IC_KEY,
  IC_KIT,
  IC_LAUNCHER,
  IC_MEDKIT,
  IC_MISC,
  IC_MONEY,
  IC_PUNCH,
  IC_THROWING_KNIFE,
  IC_THROWN,
  ItemInfo,
  OWNERSHIP,
  SWITCH,
} from './itemData';

// The item class specific data of an item stack, the layout of `map/item_data.rs` in
// `src-map-editor` (`OBJECTTYPE`). Each view only changes its own bytes.

/** The 12 bytes of an item stack */
export type ItemData = number[];

const signed = (byte: number) => (byte > 127 ? byte - 256 : byte);
const unsigned = (value: number) => ((Math.round(value) % 256) + 256) % 256;
const readU16 = (data: ItemData, offset: number) =>
  (data[offset] ?? 0) | ((data[offset + 1] ?? 0) << 8);

function writeU16(data: ItemData, offset: number, value: number) {
  data[offset] = value & 0xff;
  data[offset + 1] = (value >> 8) & 0xff;
}

/** A gun (`bGunStatus`, `ubGunAmmoType`, `ubGunShotsLeft`, `usGunAmmoItem`, `bGunAmmoStatus`) */
export interface GunData {
  status: number;
  ammoType: number;
  shotsLeft: number;
  ammoItem: number;
  ammoStatus: number;
}

export function readGun(data: ItemData): GunData {
  return {
    status: signed(data[0] ?? 0),
    ammoType: data[1] ?? 0,
    shotsLeft: data[2] ?? 0,
    ammoItem: readU16(data, 4),
    ammoStatus: signed(data[6] ?? 0),
  };
}

export function writeGun(data: ItemData, gun: GunData): ItemData {
  const written = [...data];
  written[0] = unsigned(gun.status);
  written[1] = unsigned(gun.ammoType);
  written[2] = unsigned(gun.shotsLeft);
  writeU16(written, 4, gun.ammoItem);
  written[6] = unsigned(gun.ammoStatus);
  return written;
}

/** Money (`bMoneyStatus`, `uiMoneyAmount`) */
export interface MoneyData {
  status: number;
  amount: number;
}

export function readMoney(data: ItemData): MoneyData {
  const amount =
    ((data[4] ?? 0) |
      ((data[5] ?? 0) << 8) |
      ((data[6] ?? 0) << 16) |
      ((data[7] ?? 0) << 24)) >>>
    0;
  return { status: signed(data[0] ?? 0), amount };
}

export function writeMoney(data: ItemData, money: MoneyData): ItemData {
  const written = [...data];
  written[0] = unsigned(money.status);
  for (let i = 0; i < 4; i++) {
    written[4 + i] = (money.amount >>> (8 * i)) & 0xff;
  }
  return written;
}

/** How a placed bomb goes off (`DetonatorType`) */
export const BOMB_REMOTE = 2;
export const BOMB_PRESSURE = 3;

/** A placed bomb, a trigger or an action item */
export interface BombData {
  status: number;
  detonator: number;
  /** The item that explodes, 0 for actions without an explosion */
  bombItem: number;
  /** The frequency of remote bombs and triggers, the delay of timed bombs */
  frequency: number;
  owner: number;
  /** What the action item does (`ItemActionType`) */
  actionValue: number;
  /** How likely the AI is to use a panic trigger */
  tolerance: number;
}

export function readBomb(data: ItemData): BombData {
  return {
    status: signed(data[0] ?? 0),
    detonator: signed(data[1] ?? 0),
    bombItem: readU16(data, 2),
    frequency: signed(data[4] ?? 0),
    owner: data[5] ?? 0,
    actionValue: data[6] ?? 0,
    tolerance: data[7] ?? 0,
  };
}

export function writeBomb(data: ItemData, bomb: BombData): ItemData {
  const written = [...data];
  written[0] = unsigned(bomb.status);
  written[1] = unsigned(bomb.detonator);
  writeU16(written, 2, bomb.bombItem);
  written[4] = unsigned(bomb.frequency);
  written[5] = unsigned(bomb.owner);
  written[6] = unsigned(bomb.actionValue);
  written[7] = unsigned(bomb.tolerance);
  return written;
}

/** A key (`bKeyStatus[0]`, `ubKeyID`) */
export interface KeyData {
  status: number;
  keyId: number;
}

export function readKey(data: ItemData): KeyData {
  return { status: signed(data[0] ?? 0), keyId: data[6] ?? 0 };
}

export function writeKey(data: ItemData, key: KeyData): ItemData {
  const written = [...data];
  written[0] = unsigned(key.status);
  written[6] = unsigned(key.keyId);
  return written;
}

/** An ownership item (`ubOwnerProfile`, `ubOwnerCivGroup`) */
export interface OwnerData {
  profile: number;
  civGroup: number;
}

export function readOwner(data: ItemData): OwnerData {
  return { profile: data[0] ?? 0, civGroup: data[1] ?? 0 };
}

export function writeOwner(data: ItemData, owner: OwnerData): ItemData {
  const written = [...data];
  written[0] = unsigned(owner.profile);
  written[1] = unsigned(owner.civGroup);
  return written;
}

/** What an action item does with `ACTION_ITEM_BLOW_UP`: explode its bomb item */
const ACTION_ITEM_BLOW_UP = 3;

/**
 * The actions of the editor by their index (`ChangeActionItem`): the item that explodes, or
 * `null` and the action, like `ACTION_ITEMS` in `src-map-editor`
 */
export const ACTION_ITEMS: [number | null, number][] = [
  [144, ACTION_ITEM_BLOW_UP],
  [143, ACTION_ITEM_BLOW_UP],
  [132, ACTION_ITEM_BLOW_UP],
  [131, ACTION_ITEM_BLOW_UP],
  [151, ACTION_ITEM_BLOW_UP],
  [133, ACTION_ITEM_BLOW_UP],
  [141, ACTION_ITEM_BLOW_UP],
  [null, 1],
  [null, 2],
  [null, 4],
  [null, 5],
  [135, ACTION_ITEM_BLOW_UP],
  [137, ACTION_ITEM_BLOW_UP],
  [142, ACTION_ITEM_BLOW_UP],
  [null, 6],
  [null, 7],
  [null, 8],
  [null, 9],
  [null, 10],
  [null, 11],
  [null, 12],
  [null, 13],
  [null, 14],
  [null, 15],
  [null, 16],
  [null, 17],
  [null, 18],
  [null, 19],
  [null, 20],
  [null, 21],
  [null, 22],
  [null, 23],
  [null, 24],
  [157, ACTION_ITEM_BLOW_UP],
];

/** The index of the action of an action item, `null` if the editor has none (`GetActionItemName`) */
export function actionIndexOf(data: ItemData): number | null {
  const bomb = readBomb(data);
  const index = ACTION_ITEMS.findIndex(([item, value]) =>
    bomb.actionValue === ACTION_ITEM_BLOW_UP
      ? item === bomb.bombItem
      : item === null && value === bomb.actionValue,
  );
  return index < 0 ? null : index;
}

/** Sets what an action item does (`ChangeActionItem`) */
export function changeAction(data: ItemData, action: number): ItemData {
  const [bombItem, actionValue] = ACTION_ITEMS[action] ?? [137, 3];
  return writeBomb(data, {
    ...readBomb(data),
    bombItem: bombItem ?? 0,
    actionValue,
  });
}

/** The fields the stats editor shows for an item (`gbEditingMode`) */
export type StatsMode =
  | 'guns'
  | 'ammo'
  | 'armour'
  | 'equipment'
  | 'explosives'
  | 'money'
  | 'ownership'
  | 'keys'
  | 'actions'
  | 'triggers'
  | 'none';

/** What the stats editor edits of an item (`SpecifyItemToEdit`) */
export function statsMode(item: number, info: ItemInfo | undefined): StatsMode {
  const itemClass = info?.itemClass ?? 0;
  if (itemClass === IC_MONEY) {
    return 'money';
  }
  if (item === ACTION_ITEM) {
    return 'actions';
  }
  if (item === SWITCH) {
    return 'triggers';
  }
  if (item === OWNERSHIP) {
    return 'ownership';
  }
  switch (itemClass) {
    case IC_GUN:
      return 'guns';
    case IC_AMMO:
      return 'ammo';
    case IC_ARMOUR:
      return 'armour';
    case IC_MEDKIT:
    case IC_KIT:
    case IC_MISC:
    case IC_FACE:
    case IC_BLADE:
    case IC_LAUNCHER:
    case IC_THROWING_KNIFE:
    case IC_PUNCH:
      return 'equipment';
    case IC_THROWN:
    case IC_GRENADE:
    case IC_BOMB:
      return 'explosives';
    case IC_KEY:
      return 'keys';
    default:
      return 'none';
  }
}

/** Whether the stats editor offers to hide an item: not for actions, triggers and ownership */
export function canHide(mode: StatsMode): boolean {
  return mode !== 'actions' && mode !== 'triggers' && mode !== 'ownership';
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, Math.round(value)));
}

/** The status of an item, the first byte for all but guns, money and ammo */
export function statusOf(stack: ItemStack): number {
  return signed(stack.data[0] ?? 0);
}

/** Sets the status of the first item of a stack, 0 to 100 */
export function withStatus(stack: ItemStack, status: number): ItemStack {
  const data = [...stack.data];
  data[0] = clamp(status, 0, 100);
  return { ...stack, data };
}

/**
 * Sets the number of items of a stack, the new ones get the status or the rounds of the first one
 * (`CreateItems`): full magazines for ammo
 */
export function withQuantity(
  stack: ItemStack,
  number: number,
  info: ItemInfo | undefined,
): ItemStack {
  const perPocket = Math.max(1, info?.perPocket ?? 1);
  const quantity = clamp(number, 1, perPocket);
  const data = [...stack.data];
  const first = info?.itemClass === IC_AMMO ? info.capacity : (data[0] ?? 0);
  for (let i = 0; i < Math.min(quantity, 8); i++) {
    data[i] = first;
  }
  return { ...stack, number: quantity, data };
}

/** Sets the rounds of a gun, at most a full magazine */
export function withRounds(
  stack: ItemStack,
  rounds: number,
  info: ItemInfo | undefined,
): ItemStack {
  const gun = readGun(stack.data);
  const max = info?.magSize || 255;
  return {
    ...stack,
    data: writeGun(stack.data, { ...gun, shotsLeft: clamp(rounds, 0, max) }),
  };
}

/** Adds an attachment with the status of the item, or removes it (`ToggleAttachment`) */
export function withAttachment(
  stack: ItemStack,
  attachment: number,
  attached: boolean,
): ItemStack {
  const attachments = stack.attachments.filter((a) => a.item !== attachment);
  if (attached) {
    attachments.push({
      item: attachment,
      status: clamp(statusOf(stack), 1, 100),
    });
  }
  return { ...stack, attachments: attachments.slice(0, 4) };
}

/** World item flags of the game modes (`WORLD_ITEM_REALISTIC_ONLY`, `WORLD_ITEM_SCIFI_ONLY`) */
export const SCIFI_ONLY = 0x0080;
export const REALISTIC_ONLY = 0x0100;
/** Flag of panic triggers that the AI doesn't use once it saw the player (`OBJECT_ALARM_TRIGGER`) */
export const OBJECT_ALARM_TRIGGER = 0x40;

export type GameModes = 'both' | 'realistic' | 'scifi';

export function gameModesOf(flags: number): GameModes {
  if (flags & REALISTIC_ONLY) {
    return 'realistic';
  }
  return flags & SCIFI_ONLY ? 'scifi' : 'both';
}

export function withGameModes(flags: number, modes: GameModes): number {
  const others = flags & ~(REALISTIC_ONLY | SCIFI_ONLY);
  if (modes === 'realistic') {
    return others | REALISTIC_ONLY;
  }
  return modes === 'scifi' ? others | SCIFI_ONLY : others;
}

/** The most money of a stack (`MAX_MONEY_PER_SLOT`) */
export const MAX_MONEY = 20000;
/** The highest tolerance of a panic trigger */
export const MAX_TOLERANCE = 99;
