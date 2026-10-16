import { Button, Flex, Segmented, Typography } from 'antd';
import { Dispatch, useCallback, useMemo, useState } from 'react';
import {
  SchedulePick,
  ScheduleRow,
  Soldier,
  SoldierPaletteEvent,
} from '../../../lib/mapViewer';
import { SoldierAppearanceTab } from './SoldierAppearanceTab';
import { SoldierAttributesTab } from './SoldierAttributesTab';
import { SoldierGeneralTab } from './SoldierGeneralTab';
import { SoldierInventoryTab } from './SoldierInventoryTab';
import { SoldierProfileSelect } from './SoldierProfileSelect';
import { SoldierScheduleTab } from './SoldierScheduleTab';
import { useProfiles } from './soldierData';
import { sendSoldierCommand, useSoldierEdit } from './soldierEdit';
import { SoldierModeAction } from './soldierMode';

/**
 * How a soldier is placed: with the general values only, with fixed attributes, items and
 * colours, or as a named character whose profile decides them
 */
export type SoldierPlacementKind = 'normal' | 'detailed' | 'profile';

const PLACEMENT_LABELS: Record<SoldierPlacementKind, string> = {
  normal: 'Normal',
  detailed: 'Detailed',
  profile: 'Profile',
};

export function placementOf(soldier: Soldier): SoldierPlacementKind {
  if (soldier.detailed === null) {
    return 'normal';
  }
  return soldier.detailed.profile !== null ? 'profile' : 'detailed';
}

/** The parts of a detailed placement, like the modes of the original's Mercs tab */
export type SoldierTab =
  | 'general'
  | 'attributes'
  | 'inventory'
  | 'appearance'
  | 'schedule';

const TAB_LABELS: Record<SoldierTab, string> = {
  general: 'General',
  attributes: 'Attributes',
  inventory: 'Inventory',
  appearance: 'Appearance',
  schedule: 'Schedule',
};

/**
 * The tabs of a detailed placement (`SetMercEditingMode`): creatures have no colours to choose,
 * only civilians have schedules
 */
export function soldierTabs(soldier: Soldier): SoldierTab[] {
  const tabs: SoldierTab[] = ['general', 'attributes', 'inventory'];
  if (soldier.team !== 'creature') {
    tabs.push('appearance');
  }
  if (soldier.team === 'civilian') {
    tabs.push('schedule');
  }
  return tabs;
}

export interface SoldierEditorProps {
  soldier: Soldier;
  palette: SoldierPaletteEvent | null;
  /** The actions of its schedule, `null` without a detailed placement */
  schedule: ScheduleRow[] | null;
  /** The tile of the schedule that the next click picks */
  pick: SchedulePick | null;
  dispatch: Dispatch<SoldierModeAction>;
}

/**
 * The selected soldier by its placement, a detailed one in tabs like the modes of the original's
 * Mercs tab
 */
export function SoldierEditor({
  soldier,
  palette,
  schedule,
  pick,
  dispatch,
}: SoldierEditorProps) {
  const edit = useSoldierEdit(soldier);
  const profiles = useProfiles();
  const placement = placementOf(soldier);
  const [tab, setTab] = useState<SoldierTab>('general');
  const tabs = useMemo(() => soldierTabs(soldier), [soldier]);
  // A soldier without the tab shows the general one, and the tab again once it has it, like
  // the last detailed mode of the original
  const shown = tabs.includes(tab) ? tab : 'general';
  const tabOptions = useMemo(
    () => tabs.map((value) => ({ value, label: TAB_LABELS[value] })),
    [tabs],
  );
  // Profiles can only be chosen once they are loaded
  const firstProfile = profiles?.[0]?.id ?? null;
  const placementOptions = useMemo(
    () =>
      (['normal', 'detailed', 'profile'] as const)
        .filter((value) => value !== 'profile' || firstProfile !== null)
        .map((value) => ({ value, label: PLACEMENT_LABELS[value] })),
    [firstProfile],
  );
  const onPlacementChange = useCallback(
    (value: SoldierPlacementKind) => {
      if (value === 'normal') {
        sendSoldierCommand({ kind: 'detailed', detailed: false });
      } else if (value === 'detailed') {
        sendSoldierCommand({ kind: 'detailed', detailed: true, profile: null });
      } else if (firstProfile !== null) {
        sendSoldierCommand({
          kind: 'detailed',
          detailed: true,
          profile: firstProfile,
        });
      }
    },
    [firstProfile],
  );
  const onFind = useCallback(
    () => sendSoldierCommand({ kind: 'center', gridNo: soldier.gridNo }),
    [soldier.gridNo],
  );
  return (
    <Flex vertical gap="small">
      <Flex gap="small" align="center">
        <Typography.Text strong>
          {soldier.slot !== null ? `Slot #${soldier.slot}` : 'Soldier'}
        </Typography.Text>
        <Typography.Text type="secondary">
          {`Tile ${soldier.gridNo}${soldier.onRoof ? ', on the roof' : ''}`}
        </Typography.Text>
        <Button size="small" type="link" onClick={onFind}>
          Find
        </Button>
      </Flex>
      <Segmented<SoldierPlacementKind>
        block
        size="small"
        options={placementOptions}
        value={placement}
        onChange={onPlacementChange}
        aria-label="placement"
      />
      {placement === 'detailed' ? (
        <Segmented<SoldierTab>
          block
          size="small"
          options={tabOptions}
          value={shown}
          onChange={setTab}
          aria-label="soldier tab"
        />
      ) : null}
      {placement === 'profile' && soldier.detailed && profiles ? (
        <SoldierProfileSelect detailed={soldier.detailed} profiles={profiles} />
      ) : null}
      {placement !== 'detailed' || shown === 'general' ? (
        <SoldierGeneralTab
          soldier={soldier}
          palette={palette}
          edit={edit}
          dispatch={dispatch}
        />
      ) : null}
      {placement === 'detailed' &&
      shown === 'attributes' &&
      soldier.detailed ? (
        <SoldierAttributesTab detailed={soldier.detailed} edit={edit} />
      ) : null}
      {placement === 'detailed' && shown === 'inventory' && soldier.detailed ? (
        <SoldierInventoryTab detailed={soldier.detailed} edit={edit} />
      ) : null}
      {placement === 'detailed' && shown === 'appearance' ? (
        <SoldierAppearanceTab soldier={soldier} palette={palette} edit={edit} />
      ) : null}
      {placement === 'detailed' && shown === 'schedule' && schedule ? (
        <SoldierScheduleTab rows={schedule} pick={pick} />
      ) : null}
      {placement === 'profile' && soldier.team === 'civilian' && schedule ? (
        <Flex vertical gap="small">
          <Typography.Text strong>Schedule</Typography.Text>
          <SoldierScheduleTab rows={schedule} pick={pick} />
        </Flex>
      ) : null}
    </Flex>
  );
}
