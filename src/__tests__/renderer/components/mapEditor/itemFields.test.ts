import { describe, expect, it } from 'vitest';
import { itemCatalog } from '../../../../renderer/components/content/mapEditor/itemData';
import {
  ACTION_ITEMS,
  actionIndexOf,
  BOMB_PRESSURE,
  canHide,
  changeAction,
  gameModesOf,
  readBomb,
  readGun,
  readKey,
  readMoney,
  readOwner,
  REALISTIC_ONLY,
  SCIFI_ONLY,
  statsMode,
  withAttachment,
  withGameModes,
  withQuantity,
  withRounds,
  withStatus,
  writeBomb,
  writeGun,
  writeKey,
  writeMoney,
  writeOwner,
} from '../../../../renderer/components/content/mapEditor/itemFields';

const empty = () => Array.from({ length: 12 }, () => 0);

const catalog = itemCatalog([
  [{ itemIndex: 2, internalName: 'GLOCK_18', ubMagSize: 15 }],
  [{ itemIndex: 71, internalName: 'CLIP9_15', capacity: 15, ubPerPocket: 8 }],
  [{ itemIndex: 161, internalName: 'FLAK_JACKET' }],
  [{ itemIndex: 131, internalName: 'STUN_GRENADE', ubPerPocket: 4 }],
  [
    { itemIndex: 202, internalName: 'MEDICKIT', usItemClass: 0x1000 },
    { itemIndex: 219, internalName: 'MONEY', usItemClass: 0x20000000 },
    { itemIndex: 233, internalName: 'SWITCH', usItemClass: 0x10000000 },
    { itemIndex: 234, internalName: 'ACTION_ITEM', usItemClass: 0x10000000 },
    { itemIndex: 240, internalName: 'OWNERSHIP', usItemClass: 0x10000000 },
    { itemIndex: 271, internalName: 'KEY_1', usItemClass: 0x10000 },
    { itemIndex: 300, internalName: 'NOTHING_MUCH', usItemClass: 1 },
  ],
]);
const info = (index: number) => catalog.find((item) => item.index === index);

function stack(item: number, data = empty()) {
  return {
    item,
    number: 1,
    data,
    attachments: [],
    flags: 0,
    mission: 0,
    trap: 0,
    imprintId: 200,
    weight: 0,
    used: 0,
  };
}

describe('item data', () => {
  it('should use the offsets of the game, like the viewer', () => {
    // The same bytes as `views_use_the_offsets_of_the_game` in `map/item_data.rs`
    const gun = writeGun(
      Array.from({ length: 12 }, () => 0xee),
      {
        status: 90,
        ammoType: 1,
        shotsLeft: 15,
        ammoItem: 0x0147,
        ammoStatus: 100,
      },
    );
    expect(gun).toEqual([
      90, 1, 15, 0xee, 0x47, 0x01, 100, 0xee, 0xee, 0xee, 0xee, 0xee,
    ]);
    expect(readGun(gun).ammoItem).toBe(0x0147);

    const money = writeMoney(empty(), { status: 100, amount: 20000 });
    expect(money.slice(0, 8)).toEqual([100, 0, 0, 0, 0x20, 0x4e, 0, 0]);
    expect(readMoney(money)).toEqual({ status: 100, amount: 20000 });

    const bomb = {
      status: 100,
      detonator: BOMB_PRESSURE,
      bombItem: 137,
      frequency: -1,
      owner: 1,
      actionValue: 3,
      tolerance: 50,
    };
    const bombData = writeBomb(empty(), bomb);
    expect(bombData.slice(0, 8)).toEqual([100, 3, 137, 0, 0xff, 1, 3, 50]);
    expect(readBomb(bombData)).toEqual(bomb);

    const key = writeKey(
      Array.from({ length: 12 }, () => 7),
      { status: 80, keyId: 12 },
    );
    expect(key).toEqual([80, 7, 7, 7, 7, 7, 12, 7, 7, 7, 7, 7]);
    expect(readKey(key)).toEqual({ status: 80, keyId: 12 });
    const owner = writeOwner(key, { profile: 200, civGroup: 3 });
    expect(owner.slice(0, 3)).toEqual([200, 3, 7]);
    expect(readOwner(owner)).toEqual({ profile: 200, civGroup: 3 });
  });

  it('should know the actions of the action items', () => {
    expect(ACTION_ITEMS).toHaveLength(34);
    const pit = changeAction(writeBomb(empty(), readBomb(empty())), 9);
    expect(readBomb(pit)).toMatchObject({ bombItem: 0, actionValue: 4 });
    expect(actionIndexOf(pit)).toBe(9);
    const tnt = changeAction(pit, 12);
    expect(readBomb(tnt)).toMatchObject({ bombItem: 137, actionValue: 3 });
    expect(actionIndexOf(tnt)).toBe(12);
    expect(actionIndexOf(changeAction(tnt, 33))).toBe(33);
    // Explosions of items the editor doesn't offer
    expect(
      actionIndexOf(writeBomb(empty(), { ...readBomb(tnt), bombItem: 999 })),
    ).toBe(null);
  });
});

describe('statsMode', () => {
  it('should edit the items like the original', () => {
    const mode = (item: number) => statsMode(item, info(item));
    expect(
      [2, 71, 161, 131, 202, 219, 233, 234, 240, 271, 300].map(mode),
    ).toEqual([
      'guns',
      'ammo',
      'armour',
      'explosives',
      'equipment',
      'money',
      'triggers',
      'actions',
      'ownership',
      'keys',
      'none',
    ]);
    expect(statsMode(5, undefined)).toBe('none');
    expect(
      (['guns', 'actions', 'triggers', 'ownership'] as const).map(canHide),
    ).toEqual([true, false, false, false]);
  });
});

describe('item edits', () => {
  it('should change only what is edited', () => {
    const gun = {
      ...stack(
        2,
        writeGun(empty(), {
          ...readGun(empty()),
          status: 80,
          shotsLeft: 3,
          ammoItem: 71,
        }),
      ),
      trap: 4,
    };
    const rounds = withRounds(gun, 40, info(2));
    expect(readGun(rounds.data)).toMatchObject({
      status: 80,
      shotsLeft: 15,
      ammoItem: 71,
    });
    expect(rounds.trap).toBe(4);
    expect(withStatus(gun, 120).data[0]).toBe(100);

    // New magazines are full, new explosives get the status of the first one
    const clips = withQuantity(
      stack(71, [5, ...empty().slice(1)]),
      3,
      info(71),
    );
    expect(clips.number).toBe(3);
    expect(clips.data.slice(0, 4)).toEqual([15, 15, 15, 0]);
    expect(withQuantity(stack(71), 20, info(71)).number).toBe(8);
    const grenades = withQuantity(
      stack(131, [70, ...empty().slice(1)]),
      2,
      info(131),
    );
    expect(grenades.data.slice(0, 3)).toEqual([70, 70, 0]);
    expect(withQuantity(stack(202), 5, info(202)).number).toBe(1);
  });

  it('should attach items with the status of the item', () => {
    const vest = stack(161, [77, ...empty().slice(1)]);
    const plated = withAttachment(vest, 183, true);
    expect(plated.attachments).toEqual([{ item: 183, status: 77 }]);
    expect(withAttachment(plated, 183, false).attachments).toEqual([]);
    // Attaching twice keeps one
    expect(withAttachment(plated, 183, true).attachments).toHaveLength(1);
  });

  it('should set the game modes', () => {
    expect(gameModesOf(0)).toBe('both');
    expect(gameModesOf(REALISTIC_ONLY | 0x40)).toBe('realistic');
    expect(gameModesOf(SCIFI_ONLY)).toBe('scifi');
    expect(withGameModes(REALISTIC_ONLY | 0x40, 'scifi')).toBe(
      SCIFI_ONLY | 0x40,
    );
    expect(withGameModes(SCIFI_ONLY, 'both')).toBe(0);
  });
});
