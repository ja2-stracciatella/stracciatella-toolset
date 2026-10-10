import { describe, expect, it } from 'vitest';
import {
  BIPOD,
  canAttach,
  CERAMIC_PLATES,
  DETONATOR,
  editorCategories,
  IC_AMMO,
  IC_BOMB,
  IC_GUN,
  IC_KEY,
  IC_MISC,
  itemCatalog,
  placedCount,
  REMDETONATOR,
  triggerFrequency,
  triggerName,
  UNDER_GLAUNCHER,
  validAttachment,
} from '../../../../renderer/components/content/mapEditor/itemData';

const small = (path: string, subImageIndex: number) => ({
  inventoryGraphics: { small: { path, subImageIndex } },
});

const catalog = itemCatalog([
  [
    {
      itemIndex: 2,
      internalName: 'GLOCK_18',
      internalType: 'M_PISTOL',
      ubMagSize: 15,
      attachment_Silencer: true,
      attachment_Bipod: false,
      ...small('interface/mdguns.sti', 2),
    },
    { itemIndex: 3, internalName: 'KNIFE', internalType: 'BLADE' },
    {
      itemIndex: 43,
      internalName: 'CLAWS',
      internalType: 'BLADE',
      bNotEditor: true,
    },
  ],
  [{ itemIndex: 71, internalName: 'CLIP9_15', capacity: 15, ubPerPocket: 8 }],
  [
    { itemIndex: 161, internalName: 'FLAK_JACKET', armourClass: 'VEST' },
    { itemIndex: 168, internalName: 'TSHIRT', armourClass: 'VEST' },
  ],
  [{ itemIndex: 137, internalName: 'TNT', itemClass: 'BOMB' }],
  [
    { itemIndex: 202, internalName: 'MEDICKIT', usItemClass: 4096 },
    { itemIndex: 233, internalName: 'SWITCH', usItemClass: IC_MISC },
    {
      itemIndex: 234,
      internalName: 'ACTION_ITEM',
      usItemClass: IC_MISC,
      ...small('interface/mdp1items.sti', 47),
    },
    {
      itemIndex: 271,
      internalName: 'KEY_1',
      usItemClass: IC_KEY,
      ubClassIndex: 0,
    },
    {
      itemIndex: 272,
      internalName: 'KEY_2',
      usItemClass: IC_KEY,
      ubClassIndex: 1,
    },
    { itemIndex: 300, internalName: 'NOTHING_MUCH', usItemClass: 1 },
  ],
]);
const byIndex = (index: number) => catalog.find((item) => item.index === index);

describe('itemCatalog', () => {
  it('should know the classes and the editor values of the items', () => {
    expect(byIndex(2)).toMatchObject({
      itemClass: IC_GUN,
      magSize: 15,
      graphic: { file: 'interface/mdguns.sti', subImage: 2 },
      attachments: ['Silencer'],
    });
    expect(byIndex(71)).toMatchObject({ itemClass: IC_AMMO, perPocket: 8 });
    expect(byIndex(137)?.itemClass).toBe(IC_BOMB);
    expect(byIndex(161)?.armourClass).toBe('VEST');
    expect(byIndex(272)).toMatchObject({ itemClass: IC_KEY, classIndex: 1 });
    expect(byIndex(43)?.notEditor).toBe(true);
  });
});

describe('editorCategories', () => {
  const locks = [
    { name: 'Front door', keyItem: 1 },
    { name: '', keyItem: 0 },
    // No key item has this key
    { name: 'Nowhere', keyItem: 9 },
  ];
  const categories = editorCategories(catalog, locks);

  it('should sort the items into the categories of the original', () => {
    const labels = (category: keyof typeof categories) =>
      categories[category].map((entry) => entry.label);
    // Not for the editor, no class, triggers and keys are left out
    expect(labels('weapons')).toEqual(['GLOCK_18', 'KNIFE']);
    expect(labels('ammo')).toEqual(['CLIP9_15']);
    expect(labels('armour')).toEqual(['FLAK_JACKET', 'TSHIRT']);
    expect(labels('explosives')).toEqual(['TNT']);
    expect(labels('equipment')).toEqual(['MEDICKIT']);
    expect(categories.weapons[0]?.choice).toEqual({ kind: 'item', item: 2 });
  });

  it('should have the trigger slots and a key for each lock', () => {
    expect(categories.triggers).toHaveLength(27);
    expect(categories.triggers.slice(0, 4).map((e) => e.label)).toEqual([
      'Panic Trigger1',
      'Panic Action1',
      'Panic Trigger2',
      'Panic Action2',
    ]);
    expect(categories.triggers[1]).toMatchObject({
      choice: { kind: 'trigger', slot: 1 },
      graphic: { file: 'interface/mdp1items.sti', subImage: 47 },
    });
    expect(categories.keys.map((e) => [e.label, e.choice])).toEqual([
      ['Front door', { kind: 'key', lockId: 0, item: 272 }],
      ['Lock 1', { kind: 'key', lockId: 1, item: 271 }],
    ]);
  });

  it('should count the placed items of each entry', () => {
    const counts = {
      items: [{ item: 71, placed: 2, quantity: 5 }],
      triggers: [0, 3, ...Array.from({ length: 25 }, () => 0)],
      keys: [{ lockId: 1, placed: 1 }],
    };
    expect(placedCount(categories.ammo[0]!, counts)).toEqual({
      placed: 2,
      quantity: 5,
    });
    expect(placedCount(categories.weapons[0]!, counts)).toBe(null);
    expect(placedCount(categories.triggers[1]!, counts)).toEqual({
      placed: 3,
      quantity: 3,
    });
    expect(placedCount(categories.keys[1]!, counts)?.placed).toBe(1);
    expect(placedCount(categories.keys[0]!, counts)).toBe(null);
  });
});

describe('triggers', () => {
  it('should have the frequencies and names of the original', () => {
    expect([0, 1, 2, 3, 4, 5, 6, 7, 25].map(triggerFrequency)).toEqual([
      127, 127, 126, 126, 125, 125, 51, 51, 60,
    ]);
    expect([4, 6, 25, 26].map(triggerName)).toEqual([
      'Panic Trigger3',
      'Trigger1',
      'Action10',
      'Pressure Action',
    ]);
  });
});

describe('attachments', () => {
  it('should only offer the attachments that fit', () => {
    expect(validAttachment(207, byIndex(2))).toBe(true);
    expect(validAttachment(BIPOD, byIndex(2))).toBe(false);
    expect(validAttachment(CERAMIC_PLATES, byIndex(161))).toBe(true);
    expect(validAttachment(CERAMIC_PLATES, byIndex(168))).toBe(false);
    expect(validAttachment(DETONATOR, byIndex(137))).toBe(true);
    expect(validAttachment(DETONATOR, byIndex(2))).toBe(false);
    expect(validAttachment(DETONATOR, undefined)).toBe(false);
  });

  it('should keep attachments apart that exclude each other', () => {
    expect(canAttach(BIPOD, [207])).toBe(true);
    expect(canAttach(BIPOD, [UNDER_GLAUNCHER])).toBe(false);
    expect(canAttach(UNDER_GLAUNCHER, [BIPOD])).toBe(false);
    expect(canAttach(DETONATOR, [REMDETONATOR])).toBe(false);
    expect(canAttach(207, [207])).toBe(false);
    expect(canAttach(207, [1, 2, 3, 4])).toBe(false);
  });
});
