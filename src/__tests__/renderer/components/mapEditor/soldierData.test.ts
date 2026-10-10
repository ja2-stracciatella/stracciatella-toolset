import { describe, expect, it } from 'vitest';
import { clampStats } from '../../../../renderer/components/content/mapEditor/SoldierAttributesTab';
import {
  itemCatalog,
  ItemInfo,
  newItemStack,
  OBJECT_NO_OVERWRITE,
  OBJECT_UNDROPPABLE,
  profileList,
} from '../../../../renderer/components/content/mapEditor/soldierData';
import { detailedPlacement } from '../../test-utils/soldier';

describe('itemCatalog', () => {
  it('should know the guns and magazines of the item files', () => {
    const catalog = itemCatalog([
      [{ itemIndex: 2, internalName: 'GLOCK_18', ubMagSize: 15 }],
      [{ itemIndex: 72, internalName: 'CLIP9_30', capacity: 30 }],
      null,
      [],
      [
        { itemIndex: 0, internalName: 'NOTHING' },
        // Earlier files win
        { itemIndex: 2, internalName: 'DUPLICATE' },
        { itemIndex: 202, internalName: 'MEDICKIT' },
      ],
    ]);
    expect(catalog).toMatchObject([
      { index: 72, name: 'CLIP9_30', kind: 'ammo', magSize: 0, capacity: 30 },
      { index: 2, name: 'GLOCK_18', kind: 'gun', magSize: 15, capacity: 0 },
      { index: 202, name: 'MEDICKIT', kind: 'other', magSize: 0, capacity: 0 },
    ]);
  });

  it('should read the profiles', () => {
    expect(
      profileList([
        { profileID: 0, internalName: 'BARRY' },
        { profileID: 57, internalName: 'KINGPIN' },
        { internalName: 'NOBODY' },
      ]),
    ).toEqual([
      { id: 0, name: 'BARRY' },
      { id: 57, name: 'KINGPIN' },
    ]);
    expect(profileList(null)).toEqual([]);
  });
});

describe('newItemStack', () => {
  const gun: ItemInfo = {
    index: 2,
    name: 'GLOCK_18',
    kind: 'gun',
    itemClass: 2,
    magSize: 15,
    capacity: 0,
    perPocket: 1,
    classIndex: 0,
    notEditor: false,
    graphic: null,
    attachments: [],
    armourClass: null,
  };
  const clip: ItemInfo = { ...gun, index: 72, kind: 'ammo', capacity: 30 };

  it('should create items like the original editor', () => {
    const loaded = newItemStack(2, gun, true, () => 0.5);
    expect(loaded).toMatchObject({ item: 2, number: 1, imprintId: 200 });
    // Status 80 to 100, regular ammo, a full magazine
    expect(loaded.data.slice(0, 3)).toEqual([90, 0, 15]);
    expect(loaded.flags).toBe(OBJECT_NO_OVERWRITE | OBJECT_UNDROPPABLE);
    // Magazines are full, their status is not chosen
    expect(newItemStack(72, clip, false, () => 0.99).data[0]).toBe(30);
    expect(newItemStack(72, clip, false).flags).toBe(OBJECT_NO_OVERWRITE);
    expect(newItemStack(500, undefined, false, () => 0.999).data[0]).toBe(100);
  });
});

describe('clampStats', () => {
  it('should clamp like the original, with the morale as morale', () => {
    const stats = clampStats({
      ...detailedPlacement().stats,
      expLevel: 12,
      life: 90,
      lifeMax: 0,
      agility: 150,
      morale: 70,
    });
    expect(stats).toMatchObject({
      expLevel: 9,
      lifeMax: 1,
      life: 1,
      agility: 100,
      morale: 70,
      mechanical: null,
    });
  });
});
