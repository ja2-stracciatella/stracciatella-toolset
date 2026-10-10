import {
  Alert,
  Button,
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  Input,
  Select,
  Typography,
} from 'antd';
import { ChangeEvent, MouseEvent, useCallback, useMemo, useState } from 'react';
import {
  ScheduleAction,
  SCHEDULE_ACTIONS,
  SchedulePick,
  ScheduleRow,
} from '../../../lib/mapViewer';
import { sendSoldierCommand } from './soldierEdit';

/** The names of the actions in the original editor (`gszScheduleActions`) */
const ACTION_LABELS: Record<ScheduleAction, string> = {
  none: 'No action',
  lockDoor: 'Lock door',
  unlockDoor: 'Unlock door',
  openDoor: 'Open door',
  closeDoor: 'Close door',
  gridNo: 'Move to tile',
  leaveSector: 'Leave sector',
  enterSector: 'Enter sector',
  stayInSector: 'Stay in sector',
  sleep: 'Sleep',
  wake: 'Wake up',
};

/** Which tiles an action needs: the tile next to the door and where it goes, or one tile */
export function actionTiles(action: ScheduleAction): 'none' | 'one' | 'door' {
  switch (action) {
    case 'lockDoor':
    case 'unlockDoor':
    case 'openDoor':
    case 'closeDoor':
      return 'door';
    case 'gridNo':
    case 'enterSector':
    case 'sleep':
      return 'one';
    default:
      return 'none';
  }
}

/** A time of day as HH:MM, empty without a time */
export function formatTime(time: number | null): string {
  if (time === null) {
    return '';
  }
  const hours = Math.floor(time / 60);
  const minutes = time % 60;
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
}

/** Minutes since midnight of HH:MM or HHMM, `null` if it is empty, `undefined` if it is invalid */
export function parseTime(text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === '') {
    return null;
  }
  const match = /^(\d{1,2}):?(\d{2})$/.exec(trimmed);
  if (!match) {
    return undefined;
  }
  const hours = Number(match[1]);
  const minutes = Number(match[2]);
  if (hours > 23 || minutes > 59) {
    return undefined;
  }
  return hours * 60 + minutes;
}

/** What to click while a tile is picked (`gubScheduleInstructions`) */
export function pickInstructions(action: ScheduleAction, pick: SchedulePick) {
  const keyword: Partial<Record<ScheduleAction, string>> = {
    lockDoor: 'lock',
    unlockDoor: 'unlock',
    openDoor: 'open',
    closeDoor: 'close',
  };
  const verb = keyword[action];
  if (verb) {
    return pick.point === 'a'
      ? `Click on the tile next to the door that you wish to ${verb}.`
      : `Click on the tile where you wish to move after you ${verb} the door.`;
  }
  if (action === 'sleep') {
    return 'Click on the tile where you wish to sleep. The soldier returns to where it started after waking up.';
  }
  return 'Click on the tile where you wish to move to.';
}

interface TimeInputProps {
  slot: number;
  time: number | null;
  disabled: boolean;
  onCommit: (slot: number, time: number | null) => unknown;
}

/** The time of an action, sent when the input loses the focus or on Enter */
function TimeInput({ slot, time, disabled, onCommit }: TimeInputProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const onChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => setDraft(e.target.value),
    [],
  );
  const commit = useCallback(() => {
    if (draft === null) {
      return;
    }
    setDraft(null);
    const parsed = parseTime(draft);
    // Invalid times are dropped, the input shows the time of the viewer again
    if (parsed !== undefined && parsed !== time) {
      onCommit(slot, parsed);
    }
  }, [draft, onCommit, slot, time]);
  const style = useMemo(() => ({ width: 64 }) as const, []);
  return (
    <Input
      size="small"
      style={style}
      value={draft ?? formatTime(time)}
      placeholder="HH:MM"
      onChange={onChange}
      onBlur={commit}
      onPressEnter={commit}
      disabled={disabled}
      aria-label={`time ${slot + 1}`}
    />
  );
}

interface ScheduleRowEditorProps {
  slot: number;
  row: ScheduleRow;
  /** Whether a tile is picked, the rows can't be changed then */
  picking: boolean;
  onChange: (slot: number, row: ScheduleRow, pick?: SchedulePick) => unknown;
}

function ScheduleRowEditor({
  slot,
  row,
  picking,
  onChange,
}: ScheduleRowEditorProps) {
  const options = useMemo(
    () =>
      SCHEDULE_ACTIONS.filter(
        // Waking up is set by the game
        (action) => action !== 'wake' || row.action === 'wake',
      ).map((value) => ({ value, label: ACTION_LABELS[value] })),
    [row.action],
  );
  const tiles = actionTiles(row.action);
  // The time of staying in the sector doesn't matter (`DetermineScheduleEditability`)
  const timed = row.action !== 'stayInSector';
  const onActionChange = useCallback(
    (action: ScheduleAction) => {
      // A new action picks its tiles right away (`StartScheduleAction`)
      const changed = { ...row, action, data1: null, data2: null };
      const needsTiles = actionTiles(action) !== 'none';
      onChange(
        slot,
        changed,
        needsTiles ? { slot, point: 'a', single: false } : undefined,
      );
    },
    [onChange, row, slot],
  );
  const onTimeCommit = useCallback(
    (_: number, time: number | null) => onChange(slot, { ...row, time }),
    [onChange, row, slot],
  );
  const onVarianceChange = useCallback(
    (e: CheckboxChangeEvent) =>
      onChange(slot, { ...row, variance: e.target.checked }),
    [onChange, row, slot],
  );
  // A click picks the tile again, Ctrl, Cmd or Shift+click shows it (`FindScheduleGridNo`)
  const onPoint = useCallback(
    (point: 'a' | 'b', e: MouseEvent) => {
      const gridNo = point === 'a' ? row.data1 : row.data2;
      if ((e.ctrlKey || e.metaKey || e.shiftKey) && gridNo !== null) {
        sendSoldierCommand({ kind: 'center', gridNo });
      } else {
        sendSoldierCommand({ kind: 'pick', slot, point, single: true });
      }
    },
    [row.data1, row.data2, slot],
  );
  const onA = useCallback((e: MouseEvent) => onPoint('a', e), [onPoint]);
  const onB = useCallback((e: MouseEvent) => onPoint('b', e), [onPoint]);
  const selectStyle = useMemo(() => ({ width: 130 }) as const, []);
  return (
    <Flex gap={4} align="center" wrap>
      <Typography.Text>{`${slot + 1})`}</Typography.Text>
      <Select<ScheduleAction>
        size="small"
        style={selectStyle}
        options={options}
        value={row.action}
        onChange={onActionChange}
        disabled={picking}
        aria-label={`action ${slot + 1}`}
      />
      <TimeInput
        slot={slot}
        time={row.time}
        disabled={picking || !timed}
        onCommit={onTimeCommit}
      />
      <Checkbox
        checked={row.variance}
        onChange={onVarianceChange}
        disabled={picking || !timed}
        aria-label={`variance ${slot + 1}`}
        title="The time varies by up to 15 minutes"
      >
        ±15
      </Checkbox>
      {tiles !== 'none' ? (
        <Button
          size="small"
          onClick={onA}
          disabled={picking}
          aria-label={`tile ${slot + 1}A`}
          title="Click to pick the tile, Ctrl+click to show it"
        >
          {row.data1 ?? 'A'}
        </Button>
      ) : null}
      {tiles === 'door' ? (
        <Button
          size="small"
          onClick={onB}
          disabled={picking}
          aria-label={`tile ${slot + 1}B`}
          title="Click to pick the tile, Ctrl+click to show it"
        >
          {row.data2 ?? 'B'}
        </Button>
      ) : null}
    </Flex>
  );
}

export interface SoldierScheduleTabProps {
  rows: ScheduleRow[];
  pick: SchedulePick | null;
}

/**
 * The 4 actions of the schedule of a civilian, sorted by time when the map is saved. Actions
 * whose tiles are missing are not saved.
 */
export function SoldierScheduleTab({ rows, pick }: SoldierScheduleTabProps) {
  const onChange = useCallback(
    (slot: number, row: ScheduleRow, next?: SchedulePick) => {
      const changed = rows.map((current, index) =>
        index === slot ? row : current,
      );
      sendSoldierCommand({ kind: 'schedule', rows: changed });
      if (next) {
        sendSoldierCommand({ kind: 'pick', ...next });
      }
    },
    [rows],
  );
  const onClear = useCallback(
    () => sendSoldierCommand({ kind: 'clearSchedule' }),
    [],
  );
  const onCancel = useCallback(
    () => sendSoldierCommand({ kind: 'cancelPick' }),
    [],
  );
  const picked = pick ? rows[pick.slot] : undefined;
  return (
    <Flex vertical gap="small">
      {rows.map((row, slot) => (
        <ScheduleRowEditor
          key={slot}
          slot={slot}
          row={row}
          picking={pick !== null}
          onChange={onChange}
        />
      ))}
      {pick && picked ? (
        <Alert
          type="info"
          message={`${pickInstructions(picked.action, pick)} Press Esc to cancel.`}
          action={
            <Button size="small" onClick={onCancel}>
              Cancel
            </Button>
          }
        />
      ) : null}
      <Flex>
        <Button size="small" onClick={onClear} disabled={pick !== null}>
          Clear schedule
        </Button>
      </Flex>
    </Flex>
  );
}
