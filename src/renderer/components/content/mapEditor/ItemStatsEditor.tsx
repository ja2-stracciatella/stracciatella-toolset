import {
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  InputNumber,
  Radio,
  type RadioChangeEvent,
  Select,
  Typography,
} from 'antd';
import { Dispatch, ReactNode, useCallback, useMemo, useState } from 'react';
import { ItemEdit, setItem, WorldItem } from '../../../lib/mapViewer';
import {
  ACTION_ITEM_NAMES,
  canAttach,
  CERAMIC_PLATES,
  DETONATOR,
  ItemInfo,
  PANIC_FREQUENCY_3,
  validAttachment,
  WEAPON_ATTACHMENTS,
} from './itemData';
import {
  actionIndexOf,
  canHide,
  changeAction,
  GameModes,
  gameModesOf,
  MAX_MONEY,
  MAX_TOLERANCE,
  OBJECT_ALARM_TRIGGER,
  readBomb,
  readGun,
  readKey,
  readMoney,
  readOwner,
  statsMode,
  statusOf,
  withAttachment,
  withGameModes,
  withQuantity,
  withRounds,
  withStatus,
  writeBomb,
  writeMoney,
  writeOwner,
} from './itemFields';
import { ItemModeAction, MAX_TRAP_LEVEL } from './itemMode';
import { LockInfo } from './lockTable';
import { useProfiles } from './soldierData';

/** Profile of items that nobody owns (`NO_PROFILE`) */
const NO_PROFILE = 200;

function reportError(e: unknown) {
  console.error('Map editor request failed', e);
}

interface NumberFieldProps {
  label: string;
  min: number;
  max: number;
  value: number;
  disabled?: boolean;
  onCommit: (value: number) => unknown;
}

/** A value of the item, sent to the viewer when the input loses the focus or on Enter */
function NumberField({
  label,
  min,
  max,
  value,
  disabled,
  onCommit,
}: NumberFieldProps) {
  // What is typed, until it is committed. The value of the viewer otherwise.
  const [draft, setDraft] = useState<{ typed: number | null } | null>(null);
  const shown = draft ? draft.typed : value;
  const onChange = useCallback(
    (typed: number | null) => setDraft({ typed }),
    [],
  );
  const commit = useCallback(() => {
    if (draft) {
      setDraft(null);
      // Empty fields keep the value
      if (draft.typed !== null && draft.typed !== value) {
        onCommit(Math.max(min, Math.min(max, Math.round(draft.typed))));
      }
    }
  }, [draft, max, min, onCommit, value]);
  const style = useMemo(() => ({ width: 88 }) as const, []);
  return (
    <Flex justify="space-between" align="center" gap="small">
      <Typography.Text>{label}</Typography.Text>
      <InputNumber<number>
        size="small"
        style={style}
        min={min}
        max={max}
        precision={0}
        value={shown}
        onChange={onChange}
        onBlur={commit}
        onPressEnter={commit}
        disabled={disabled}
        aria-label={label}
      />
    </Flex>
  );
}

interface AttachmentToggleProps {
  item: number;
  label: string;
  attached: boolean;
  disabled: boolean;
  onToggle: (item: number, attached: boolean) => unknown;
}

function AttachmentToggle({
  item,
  label,
  attached,
  disabled,
  onToggle,
}: AttachmentToggleProps) {
  const onChange = useCallback(
    (e: CheckboxChangeEvent) => onToggle(item, e.target.checked),
    [item, onToggle],
  );
  return (
    <Checkbox checked={attached} disabled={disabled} onChange={onChange}>
      {label}
    </Checkbox>
  );
}

const GAME_MODE_OPTIONS: { label: string; value: GameModes }[] = [
  { label: 'Both', value: 'both' },
  { label: 'Realistic', value: 'realistic' },
  { label: 'Sci-Fi', value: 'scifi' },
];

const ACTION_OPTIONS = ACTION_ITEM_NAMES.map((label, value) => ({
  value,
  label,
}));

const SELECT_STYLE = { width: 200 } as const;

export interface ItemStatsEditorProps {
  item: WorldItem;
  /** The items of the selected mod, `null` while they load */
  catalog: ItemInfo[] | null;
  locks: LockInfo[] | null;
  /** The names of the civilian groups */
  civilianGroups: string[];
  dispatch: Dispatch<ItemModeAction>;
}

/**
 * The values of the selected item like the stats panel of the original (`Item_Statistics.cc`):
 * the fields depend on the kind of item, every change is sent to the viewer, which sends the
 * item back
 */
export function ItemStatsEditor({
  item,
  catalog,
  locks,
  civilianGroups,
  dispatch,
}: ItemStatsEditorProps) {
  const profiles = useProfiles();
  const { stack } = item;
  const info = useMemo(
    () => catalog?.find((entry) => entry.index === stack.item),
    [catalog, stack.item],
  );
  const mode = statsMode(stack.item, info);
  const edit = useCallback(
    (changes: Partial<ItemEdit>) => {
      setItem({
        id: item.id,
        stack: item.stack,
        visibility: item.visibility,
        flags: item.flags,
        nonExistChance: item.nonExistChance,
        ...changes,
      }).catch(reportError);
    },
    [item],
  );

  const onStatus = useCallback(
    (status: number) => edit({ stack: withStatus(stack, status) }),
    [edit, stack],
  );
  const onTrap = useCallback(
    (trap: number) => {
      // The trap level of an action becomes the one of new actions, like in the original
      if (mode === 'actions') {
        dispatch({ type: 'setTrapLevel', trapLevel: trap });
      }
      edit({ stack: { ...stack, trap } });
    },
    [dispatch, edit, mode, stack],
  );
  const onQuantity = useCallback(
    (number: number) => edit({ stack: withQuantity(stack, number, info) }),
    [edit, info, stack],
  );
  const onRounds = useCallback(
    (rounds: number) => edit({ stack: withRounds(stack, rounds, info) }),
    [edit, info, stack],
  );
  const onAmount = useCallback(
    (amount: number) =>
      edit({
        stack: {
          ...stack,
          data: writeMoney(stack.data, { status: 100, amount }),
        },
      }),
    [edit, stack],
  );
  const onTolerance = useCallback(
    (tolerance: number) =>
      edit({
        stack: {
          ...stack,
          data: writeBomb(stack.data, { ...readBomb(stack.data), tolerance }),
        },
      }),
    [edit, stack],
  );
  const onAlarmChange = useCallback(
    (e: CheckboxChangeEvent) => {
      const flags = e.target.checked
        ? stack.flags | OBJECT_ALARM_TRIGGER
        : stack.flags & ~OBJECT_ALARM_TRIGGER;
      edit({ stack: { ...stack, flags } });
    },
    [edit, stack],
  );
  const onActionChange = useCallback(
    (action: number) => {
      // The action becomes the one of new actions, like in the original
      dispatch({ type: 'setAction', action });
      edit({ stack: { ...stack, data: changeAction(stack.data, action) } });
    },
    [dispatch, edit, stack],
  );
  const owner = useMemo(() => readOwner(stack.data), [stack.data]);
  const onProfileChange = useCallback(
    (profile: number) =>
      edit({
        stack: {
          ...stack,
          data: writeOwner(stack.data, { ...owner, profile }),
        },
      }),
    [edit, owner, stack],
  );
  const onCivGroupChange = useCallback(
    (civGroup: number) =>
      edit({
        stack: {
          ...stack,
          data: writeOwner(stack.data, { ...owner, civGroup }),
        },
      }),
    [edit, owner, stack],
  );
  const onAttachment = useCallback(
    (attachment: number, attached: boolean) =>
      edit({ stack: withAttachment(stack, attachment, attached) }),
    [edit, stack],
  );
  const onExistChance = useCallback(
    (existChance: number) => edit({ nonExistChance: 100 - existChance }),
    [edit],
  );
  const onGameModes = useCallback(
    (e: RadioChangeEvent) =>
      edit({ flags: withGameModes(item.flags, e.target.value as GameModes) }),
    [edit, item.flags],
  );
  const onHiddenChange = useCallback(
    (e: CheckboxChangeEvent) =>
      edit({ visibility: e.target.checked ? 'hiddenItem' : 'invisible' }),
    [edit],
  );

  const profileOptions = useMemo(() => {
    const options = (profiles ?? []).map((profile) => ({
      value: profile.id,
      label: `${profile.id} ${profile.name}`,
    }));
    if (
      owner.profile !== NO_PROFILE &&
      !options.some((o) => o.value === owner.profile)
    ) {
      options.push({ value: owner.profile, label: `Profile ${owner.profile}` });
    }
    return [{ value: NO_PROFILE, label: 'None' }, ...options];
  }, [owner.profile, profiles]);
  const civGroupOptions = useMemo(
    () => civilianGroups.map((label, value) => ({ value, label })),
    [civilianGroups],
  );

  const attachedItems = stack.attachments.map((a) => a.item);
  const attachmentToggles = (attachments: { item: number; label: string }[]) =>
    attachments
      .filter((a) => validAttachment(a.item, info))
      .map((a) => {
        const attached = attachedItems.includes(a.item);
        return (
          <AttachmentToggle
            key={a.item}
            item={a.item}
            label={a.label}
            attached={attached}
            disabled={!attached && !canAttach(a.item, attachedItems)}
            onToggle={onAttachment}
          />
        );
      });

  const status = (
    <NumberField
      label="Status"
      min={0}
      max={100}
      value={statusOf(stack)}
      onCommit={onStatus}
    />
  );
  const trap = (
    <NumberField
      label="Trap level"
      min={0}
      max={MAX_TRAP_LEVEL}
      value={stack.trap}
      onCommit={onTrap}
    />
  );
  const perPocket = Math.max(1, info?.perPocket ?? 1);
  const bomb = readBomb(stack.data);

  let fields: ReactNode = null;
  switch (mode) {
    case 'guns':
      fields = (
        <>
          {status}
          <NumberField
            label="Rounds"
            min={0}
            max={info?.magSize || 255}
            value={readGun(stack.data).shotsLeft}
            onCommit={onRounds}
          />
          {trap}
          <Flex wrap gap="small" role="group" aria-label="attachments">
            {attachmentToggles(WEAPON_ATTACHMENTS)}
          </Flex>
        </>
      );
      break;
    case 'ammo':
      fields = (
        <>
          <NumberField
            label="Quantity"
            min={1}
            max={perPocket}
            value={stack.number}
            onCommit={onQuantity}
          />
          {trap}
        </>
      );
      break;
    case 'armour':
      fields = (
        <>
          {status}
          {trap}
          {attachmentToggles([
            { item: CERAMIC_PLATES, label: 'Ceramic plates' },
          ])}
        </>
      );
      break;
    case 'equipment':
      fields = (
        <>
          {status}
          {trap}
        </>
      );
      break;
    case 'explosives':
      fields = (
        <>
          {status}
          <NumberField
            label="Quantity"
            min={1}
            max={perPocket}
            value={stack.number}
            disabled={perPocket <= 1}
            onCommit={onQuantity}
          />
          {trap}
          {attachmentToggles([{ item: DETONATOR, label: 'Detonator' }])}
        </>
      );
      break;
    case 'money':
      fields = (
        <NumberField
          label="Amount"
          min={1}
          max={MAX_MONEY}
          value={readMoney(stack.data).amount}
          onCommit={onAmount}
        />
      );
      break;
    case 'ownership':
      fields = (
        <>
          <Flex justify="space-between" align="center" gap="small">
            <Typography.Text>Profile</Typography.Text>
            <Select<number>
              size="small"
              style={SELECT_STYLE}
              showSearch
              optionFilterProp="label"
              options={profileOptions}
              value={owner.profile}
              onChange={onProfileChange}
              virtual={false}
              aria-label="owner profile"
            />
          </Flex>
          <Flex justify="space-between" align="center" gap="small">
            <Typography.Text>Civilian group</Typography.Text>
            <Select<number>
              size="small"
              style={SELECT_STYLE}
              options={civGroupOptions}
              value={owner.civGroup}
              onChange={onCivGroupChange}
              virtual={false}
              aria-label="owner civilian group"
            />
          </Flex>
        </>
      );
      break;
    case 'keys': {
      const lock = locks?.[readKey(stack.data).keyId];
      fields = (
        <Typography.Text>
          {`Opens lock ${readKey(stack.data).keyId}${lock?.name ? `: ${lock.name}` : ''}`}
        </Typography.Text>
      );
      break;
    }
    case 'actions':
      fields = (
        <>
          {status}
          {trap}
          <Flex justify="space-between" align="center" gap="small">
            <Typography.Text>Action</Typography.Text>
            <Select<number>
              size="small"
              style={SELECT_STYLE}
              options={ACTION_OPTIONS}
              value={actionIndexOf(stack.data) ?? undefined}
              placeholder="Unknown action"
              onChange={onActionChange}
              virtual={false}
              aria-label="action"
            />
          </Flex>
        </>
      );
      break;
    case 'triggers':
      fields = (
        <>
          {trap}
          <NumberField
            label="Tolerance"
            min={0}
            max={MAX_TOLERANCE}
            value={bomb.tolerance}
            onCommit={onTolerance}
          />
          {bomb.frequency >= PANIC_FREQUENCY_3 ? (
            <Checkbox
              checked={(stack.flags & OBJECT_ALARM_TRIGGER) !== 0}
              onChange={onAlarmChange}
              title="Enemies don't use alarm triggers once they know of the player"
            >
              Alarm trigger
            </Checkbox>
          ) : null}
        </>
      );
      break;
    case 'none':
      fields = (
        <Typography.Text type="secondary">
          The editor has no values for this item.
        </Typography.Text>
      );
      break;
  }

  return (
    <Flex vertical gap={4} role="group" aria-label="item values">
      {fields}
      {mode !== 'ownership' ? (
        <NumberField
          label="Exist chance"
          min={1}
          max={100}
          value={100 - item.nonExistChance}
          onCommit={onExistChance}
        />
      ) : null}
      <Radio.Group
        size="small"
        optionType="button"
        options={GAME_MODE_OPTIONS}
        value={gameModesOf(item.flags)}
        onChange={onGameModes}
        aria-label="game modes"
      />
      {canHide(mode) ? (
        <Checkbox
          checked={item.visibility === 'hiddenItem'}
          onChange={onHiddenChange}
        >
          Hidden
        </Checkbox>
      ) : null}
    </Flex>
  );
}
