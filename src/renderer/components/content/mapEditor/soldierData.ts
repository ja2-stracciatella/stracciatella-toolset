import { useEffect, useMemo } from 'react';
import { isArray } from 'remeda';
import { JsonRoot } from '../../../../common/invokables/jsons';
import { useFileLoad } from '../../../hooks/useFileLoad';
import { useFilesJsonDiskValue } from '../../../hooks/useFilesJsonDiskValue';
import { useAppSelector } from '../../../hooks/state';
import { ItemStack } from '../../../lib/mapViewer';
import { ITEM_FILES, itemCatalog, ItemInfo } from './itemData';

export { ITEM_FILES, itemCatalog, type ItemInfo };

/** The named characters */
export const PROFILE_FILE = 'mercs-profile-info.json';

/** Flag of items that stay with a dead soldier (`OBJECT_UNDROPPABLE`) */
export const OBJECT_UNDROPPABLE = 0x01;
/** Flag of items that the game doesn't replace by generated equipment (`OBJECT_NO_OVERWRITE`) */
export const OBJECT_NO_OVERWRITE = 0x80;
/** Imprint of guns that nobody owns (`NO_PROFILE`) */
const NO_PROFILE = 200;

/** A named character */
export interface ProfileInfo {
  id: number;
  name: string;
}

export function profileList(value: JsonRoot | null): ProfileInfo[] {
  if (!isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    if (isArray(entry) || typeof entry !== 'object' || entry === null) {
      return [];
    }
    const id = entry['profileID'];
    const name = entry['internalName'];
    if (typeof id !== 'number' || typeof name !== 'string') {
      return [];
    }
    return [{ id, name }];
  });
}

/** Loads the files that are not loaded yet, returns whether all of them are loaded */
function useLoadedFiles(files: string[]): Array<JsonRoot | null> | null {
  const loadFile = useFileLoad();
  const missing = useAppSelector((state) =>
    files.filter((file) => !state.files.disk[file]).join(','),
  );
  useEffect(() => {
    if (missing) {
      missing.split(',').forEach((file) => loadFile(file));
    }
  }, [loadFile, missing]);
  const values = useFilesJsonDiskValue(files);
  const loaded = useAppSelector((state) =>
    files.every((file) => {
      const disk = state.files.disk[file];
      return disk !== undefined && !disk.loading;
    }),
  );
  return loaded ? values : null;
}

/** The items of the selected mod, `null` while they load */
export function useItemCatalog(): ItemInfo[] | null {
  const values = useLoadedFiles(ITEM_FILES);
  return useMemo(() => (values ? itemCatalog(values) : null), [values]);
}

const PROFILE_FILES = [PROFILE_FILE];

/** The named characters of the selected mod, `null` while they load */
export function useProfiles(): ProfileInfo[] | null {
  const values = useLoadedFiles(PROFILE_FILES);
  return useMemo(
    () => (values ? profileList(values[0] ?? null) : null),
    [values],
  );
}

/**
 * A new item for an inventory slot, like the original creates it
 * (`AddNewItemToSelectedMercsInventory` and `CreateItem`): guns are loaded, magazines full and the
 * status of other items is 80 to 100. The game keeps it instead of generating equipment.
 */
export function newItemStack(
  item: number,
  info: ItemInfo | undefined,
  undroppable: boolean,
  random: () => number = Math.random,
): ItemStack {
  const data = Array.from({ length: 12 }, () => 0);
  const status = 80 + Math.floor(random() * 21);
  let imprintId = 0;
  switch (info?.kind) {
    case 'gun':
      // Status, regular ammo, rounds
      data[0] = status;
      data[2] = info.magSize;
      imprintId = NO_PROFILE;
      break;
    case 'ammo':
      // Rounds of the magazine
      data[0] = info.capacity;
      break;
    default:
      data[0] = status;
  }
  return {
    item,
    number: 1,
    data,
    attachments: [],
    flags: OBJECT_NO_OVERWRITE | (undroppable ? OBJECT_UNDROPPABLE : 0),
    mission: 0,
    trap: 0,
    imprintId,
    weight: 0,
    used: 0,
  };
}
