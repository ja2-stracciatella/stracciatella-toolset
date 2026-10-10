import {
  Button,
  Checkbox,
  type CheckboxChangeEvent,
  Flex,
  InputNumber,
  Modal,
  Select,
  Typography,
} from 'antd';
import { useCallback, useMemo, useState } from 'react';
import {
  DOOR_TRAP_KINDS,
  DoorLock,
  DoorTrapKind,
  EditorDoorEvent,
} from '../../../lib/mapViewer';
import { LockInfo } from './lockTable';

/** The highest lock id of the lock table (`NUM_LOCKS - 1`) */
export const MAX_LOCK_ID = 63;
/** Traps are 1 to 20 hard to find */
export const MAX_TRAP_LEVEL = 20;

const TRAP_LABELS: Record<DoorTrapKind, string> = {
  explosion: 'Explosion',
  electric: 'Electric',
  siren: 'Siren',
  silentAlarm: 'Silent alarm',
  brothelSiren: 'Brothel siren',
  superElectric: 'Super electric',
};

type TrapOption = DoorTrapKind | 'none';

const TRAP_OPTIONS: { label: string; value: TrapOption }[] = [
  { label: 'None', value: 'none' },
  ...DOOR_TRAP_KINDS.map((kind) => ({ label: TRAP_LABELS[kind], value: kind })),
];

const NO_LOCK: DoorLock = { lockId: 0, trap: null, locked: false };

/** The door as it is saved: a trap needs a level of at least 1 */
export function validDoor(door: DoorLock): DoorLock {
  const lockId = Math.max(0, Math.min(MAX_LOCK_ID, Math.round(door.lockId)));
  const trap = door.trap && {
    kind: door.trap.kind,
    level: Math.max(1, Math.min(MAX_TRAP_LEVEL, Math.round(door.trap.level))),
  };
  return { lockId, trap, locked: door.locked };
}

export interface DoorLockModalProps {
  /** The door to edit, the modal is closed without one */
  request: EditorDoorEvent | null;
  /** The locks of the lock table for their names, `null` while they load */
  locks?: LockInfo[] | null;
  /** Saves the door, `null` removes it from the door table */
  onSave: (gridNo: number, door: DoorLock | null) => unknown;
  onClose: () => unknown;
}

/** Edits the lock and the trap of a door (`InitDoorEditing`) */
export function DoorLockModal({
  request,
  locks,
  onSave,
  onClose,
}: DoorLockModalProps) {
  // The sidebar gives every request a new key, so the state starts with its door
  const [door, setDoor] = useState<DoorLock>(request?.door ?? NO_LOCK);

  const onLockChange = useCallback(
    (lockId: number | null) => setDoor((d) => ({ ...d, lockId: lockId ?? 0 })),
    [],
  );
  const onTrapChange = useCallback(
    (kind: TrapOption) =>
      setDoor((d) => ({
        ...d,
        trap: kind === 'none' ? null : { kind, level: d.trap?.level ?? 1 },
      })),
    [],
  );
  const onLevelChange = useCallback(
    (level: number | null) =>
      setDoor((d) =>
        d.trap ? { ...d, trap: { ...d.trap, level: level ?? 1 } } : d,
      ),
    [],
  );
  const onLockedChange = useCallback(
    (e: CheckboxChangeEvent) =>
      setDoor((d) => ({ ...d, locked: e.target.checked })),
    [],
  );
  const onOk = useCallback(() => {
    if (request) {
      onSave(request.gridNo, validDoor(door));
    }
    onClose();
  }, [door, onClose, onSave, request]);
  const onRemove = useCallback(() => {
    if (request) {
      onSave(request.gridNo, null);
    }
    onClose();
  }, [onClose, onSave, request]);

  const footer = useMemo(
    () => [
      <Button key="remove" danger onClick={onRemove} disabled={!request?.door}>
        Remove
      </Button>,
      <Button key="cancel" onClick={onClose}>
        Cancel
      </Button>,
      <Button key="ok" type="primary" onClick={onOk}>
        Okay
      </Button>,
    ],
    [onClose, onOk, onRemove, request],
  );

  return (
    <Modal
      title="Door lock"
      open={request !== null}
      onCancel={onClose}
      footer={footer}
      destroyOnHidden
    >
      <Flex vertical gap="small">
        <Typography.Text type="secondary">
          Lock attributes at map index {request?.gridNo}
        </Typography.Text>
        <Flex gap="small" align="center">
          <span style={LABEL_STYLE}>Lock ID</span>
          <InputNumber
            min={0}
            max={MAX_LOCK_ID}
            precision={0}
            value={door.lockId}
            onChange={onLockChange}
            aria-label="lock id"
          />
          <Typography.Text type="secondary">
            {locks?.[door.lockId]?.name ?? ''}
          </Typography.Text>
        </Flex>
        <Flex gap="small" align="center">
          <span style={LABEL_STYLE}>Trap type</span>
          <Select<TrapOption>
            style={SELECT_STYLE}
            options={TRAP_OPTIONS}
            value={door.trap?.kind ?? 'none'}
            onChange={onTrapChange}
            virtual={false}
            aria-label="trap type"
          />
        </Flex>
        <Flex gap="small" align="center">
          <span style={LABEL_STYLE}>Trap level</span>
          <InputNumber
            min={1}
            max={MAX_TRAP_LEVEL}
            precision={0}
            value={door.trap?.level ?? null}
            onChange={onLevelChange}
            disabled={!door.trap}
            aria-label="trap level"
          />
        </Flex>
        <Checkbox checked={door.locked} onChange={onLockedChange}>
          Locked
        </Checkbox>
      </Flex>
    </Modal>
  );
}

const LABEL_STYLE = { width: 80 } as const;
const SELECT_STYLE = { width: 160 } as const;
