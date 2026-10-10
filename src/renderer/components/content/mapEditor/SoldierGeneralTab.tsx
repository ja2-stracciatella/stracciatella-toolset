import {
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  Segmented,
  Select,
  Typography,
} from 'antd';
import { Dispatch, ReactNode, useCallback, useMemo } from 'react';
import {
  Direction,
  DIRECTIONS,
  Soldier,
  SoldierAttitude,
  SOLDIER_ATTITUDES,
  SOLDIER_ORDERS,
  SoldierOrders,
  SoldierPaletteEvent,
} from '../../../lib/mapViewer';
import { SoldierEditFn } from './soldierEdit';
import { RELATIVE_LEVELS, SoldierModeAction } from './soldierMode';

const ORDER_LABELS: Record<SoldierOrders, string> = {
  stationary: 'Stationary',
  onGuard: 'On guard',
  closePatrol: 'Close patrol',
  farPatrol: 'Far patrol',
  pointPatrol: 'Point patrol',
  onCall: 'On call',
  seekEnemy: 'Seek enemy',
  randomPointPatrol: 'Random point patrol',
};

const ATTITUDE_LABELS: Record<SoldierAttitude, string> = {
  defensive: 'Defensive',
  braveSolo: 'Brave solo',
  braveAid: 'Brave aid',
  cunningSolo: 'Cunning solo',
  cunningAid: 'Cunning aid',
  aggressive: 'Aggressive',
  attackSlayOnly: 'Attack Slay only',
};

const DIRECTION_LABELS: Record<Direction, string> = {
  north: 'North',
  northEast: 'North east',
  east: 'East',
  southEast: 'South east',
  south: 'South',
  southWest: 'South west',
  west: 'West',
  northWest: 'North west',
};

const ORDER_OPTIONS = SOLDIER_ORDERS.map((value) => ({
  value,
  label: ORDER_LABELS[value],
}));
const DIRECTION_OPTIONS = DIRECTIONS.map((value) => ({
  value,
  label: DIRECTION_LABELS[value],
}));
const LEVEL_OPTIONS = RELATIVE_LEVELS.map((label, value) => ({
  label,
  value,
}));

/** The value of the random body type in the body type select */
const RANDOM_BODY = -1;

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <Flex vertical gap={2}>
      <Typography.Text type="secondary">{label}</Typography.Text>
      {children}
    </Flex>
  );
}

export interface SoldierGeneralTabProps {
  soldier: Soldier;
  palette: SoldierPaletteEvent | null;
  edit: SoldierEditFn;
  dispatch: Dispatch<SoldierModeAction>;
}

/**
 * Orders, attitude, direction, relative levels, class, civilian group and body type of the
 * selected soldier (the basic and general modes of the original). A profile decides the
 * existence, the levels, the class and the group, like `SetMercEditability`, they are hidden
 * then.
 */
export function SoldierGeneralTab({
  soldier,
  palette,
  edit,
  dispatch,
}: SoldierGeneralTabProps) {
  const team = soldier.team;
  const locked = soldier.detailed?.profile != null;
  const attitudeOptions = useMemo(
    () =>
      SOLDIER_ATTITUDES.filter(
        // Only maps of the game have soldiers that only attack Slay
        (value) =>
          value !== 'attackSlayOnly' || soldier.attitude === 'attackSlayOnly',
      ).map((value) => ({ value, label: ATTITUDE_LABELS[value] })),
    [soldier.attitude],
  );
  const bodyOptions = useMemo(() => {
    const options = (team && palette ? palette.bodies[team] : []).map(
      (body) => ({ value: body.id ?? RANDOM_BODY, label: body.name }),
    );
    // Maps can have body types that the editor doesn't offer
    const body = soldier.body ?? RANDOM_BODY;
    if (options.length > 0 && !options.some((o) => o.value === body)) {
      options.push({ value: body, label: `Body type ${body}` });
    }
    return options;
  }, [palette, soldier.body, team]);
  const classOptions = useMemo(
    () =>
      (palette?.enemyClasses ?? []).map((option) => ({
        value: option.id,
        label: option.name,
      })),
    [palette],
  );
  const groupOptions = useMemo(
    () =>
      (palette?.civilianGroups ?? []).map((label, value) => ({
        label,
        value,
      })),
    [palette],
  );

  const onPriorityChange = useCallback(
    (e: CheckboxChangeEvent) => edit({ priorityExistence: e.target.checked }),
    [edit],
  );
  const onKeysChange = useCallback(
    (e: CheckboxChangeEvent) => edit({ hasKeys: e.target.checked }),
    [edit],
  );
  const onOrdersChange = useCallback(
    (orders: SoldierOrders) => {
      edit({ orders });
      dispatch({ type: 'setDefaults', defaults: { orders } });
    },
    [dispatch, edit],
  );
  const onAttitudeChange = useCallback(
    (attitude: SoldierAttitude) => {
      edit({ attitude });
      dispatch({ type: 'setDefaults', defaults: { attitude } });
    },
    [dispatch, edit],
  );
  const onDirectionChange = useCallback(
    (direction: Direction) => {
      edit({ direction });
      dispatch({ type: 'setDefaults', defaults: { direction } });
    },
    [dispatch, edit],
  );
  const onEquipmentChange = useCallback(
    (relativeEquipmentLevel: number) => {
      edit({ relativeEquipmentLevel });
      dispatch({ type: 'setDefaults', defaults: { relativeEquipmentLevel } });
    },
    [dispatch, edit],
  );
  const onAttributesChange = useCallback(
    (relativeAttributeLevel: number) => {
      edit({ relativeAttributeLevel });
      dispatch({ type: 'setDefaults', defaults: { relativeAttributeLevel } });
    },
    [dispatch, edit],
  );
  const onClassChange = useCallback(
    (soldierClass: number) => {
      edit({ soldierClass });
      dispatch({ type: 'setDefaults', defaults: { soldierClass } });
    },
    [dispatch, edit],
  );
  const onGroupChange = useCallback(
    (civilianGroup: number) => {
      edit({ civilianGroup });
      dispatch({ type: 'setDefaults', defaults: { civilianGroup } });
    },
    [dispatch, edit],
  );
  const onBodyChange = useCallback(
    (value: number) => {
      const body = value === RANDOM_BODY ? null : value;
      edit({ body });
      if (team === 'creature' && body !== null) {
        dispatch({ type: 'setDefaults', defaults: { creatureBody: body } });
      }
    },
    [dispatch, edit, team],
  );

  const selectStyle = useMemo(() => ({ width: '100%' }) as const, []);

  return (
    <Flex vertical gap="small">
      <Flex wrap gap="small">
        {!locked ? (
          <Checkbox
            checked={soldier.priorityExistence}
            onChange={onPriorityChange}
          >
            Priority existence
          </Checkbox>
        ) : null}
        <Checkbox checked={soldier.hasKeys} onChange={onKeysChange}>
          Has keys
        </Checkbox>
      </Flex>
      <Field label="Orders">
        <Select<SoldierOrders>
          size="small"
          style={selectStyle}
          options={ORDER_OPTIONS}
          value={soldier.orders}
          onChange={onOrdersChange}
          aria-label="orders"
        />
      </Field>
      <Field label="Attitude">
        <Select<SoldierAttitude>
          size="small"
          style={selectStyle}
          options={attitudeOptions}
          value={soldier.attitude}
          onChange={onAttitudeChange}
          aria-label="attitude"
        />
      </Field>
      <Field label="Direction">
        <Select<Direction>
          size="small"
          style={selectStyle}
          options={DIRECTION_OPTIONS}
          value={soldier.direction}
          onChange={onDirectionChange}
          aria-label="direction"
        />
      </Field>
      {team !== 'creature' && !locked ? (
        <Field label="Relative equipment">
          <Segmented<number>
            block
            size="small"
            options={LEVEL_OPTIONS}
            value={soldier.relativeEquipmentLevel}
            onChange={onEquipmentChange}
            aria-label="relative equipment"
          />
        </Field>
      ) : null}
      {!locked ? (
        <Field label="Relative attributes">
          <Segmented<number>
            block
            size="small"
            options={LEVEL_OPTIONS}
            value={soldier.relativeAttributeLevel}
            onChange={onAttributesChange}
            aria-label="relative attributes"
          />
        </Field>
      ) : null}
      {team === 'enemy' && !locked ? (
        <Field label="Class">
          <Segmented<number>
            block
            size="small"
            options={classOptions}
            value={soldier.soldierClass}
            onChange={onClassChange}
            aria-label="class"
          />
        </Field>
      ) : null}
      {team === 'civilian' && !locked ? (
        <Field label="Civilian group">
          <Select<number>
            size="small"
            style={selectStyle}
            options={groupOptions}
            value={soldier.civilianGroup}
            onChange={onGroupChange}
            aria-label="civilian group"
          />
        </Field>
      ) : null}
      {bodyOptions.length > 0 ? (
        <Field label="Body type">
          <Select<number>
            size="small"
            style={selectStyle}
            options={bodyOptions}
            value={soldier.body ?? RANDOM_BODY}
            onChange={onBodyChange}
            aria-label="body type"
          />
        </Field>
      ) : null}
    </Flex>
  );
}
