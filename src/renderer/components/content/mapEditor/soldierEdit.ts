import { useCallback } from 'react';
import {
  setSoldier,
  Soldier,
  SoldierCommand,
  soldierCommand,
  SoldierFields,
} from '../../../lib/mapViewer';

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

/** Sends a soldier command to the viewer */
export function sendSoldierCommand(command: SoldierCommand) {
  soldierCommand(command).catch(reportError);
}

/** The values of a soldier that the viewer edits */
export function fieldsOf(soldier: Soldier): SoldierFields {
  return {
    orders: soldier.orders,
    attitude: soldier.attitude,
    direction: soldier.direction,
    relativeAttributeLevel: soldier.relativeAttributeLevel,
    relativeEquipmentLevel: soldier.relativeEquipmentLevel,
    soldierClass: soldier.soldierClass,
    civilianGroup: soldier.civilianGroup,
    body: soldier.body,
    priorityExistence: soldier.priorityExistence,
    hasKeys: soldier.hasKeys,
    detailed: soldier.detailed,
  };
}

/** Changes values of the selected soldier */
export type SoldierEditFn = (changes: Partial<SoldierFields>) => void;

/** Sends changed values of a soldier to the viewer, which sends the soldier back */
export function useSoldierEdit(soldier: Soldier | null): SoldierEditFn {
  return useCallback(
    (changes: Partial<SoldierFields>) => {
      if (!soldier) {
        return;
      }
      setSoldier({ id: soldier.id, ...fieldsOf(soldier), ...changes }).catch(
        reportError,
      );
    },
    [soldier],
  );
}
