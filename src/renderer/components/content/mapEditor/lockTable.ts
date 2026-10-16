import { useEffect, useState } from 'react';
import { invoke } from '../../../lib/invoke';

/** The lock table of the game (`LoadLockTable`) */
export const LOCK_TABLE_FILE = 'binarydata/locks.bin';
/** Number of locks of the lock table (`NUM_LOCKS`) */
export const NUM_LOCKS = 64;
/** Size of a lock in the file (`LOCK`): the name, the key item and 4 bytes */
const LOCK_SIZE = 46;
/** Length of the name of a lock (`MAXLOCKDESCLENGTH`) */
const NAME_LENGTH = 40;

/** A lock of the lock table */
export interface LockInfo {
  /** The name the editor shows (`ubEditorName`) */
  name: string;
  /** The key of the lock, the class index of its key item (`usKeyItem`) */
  keyItem: number;
}

/** The locks of `Locks.bin`, as many as the file has, at most `NUM_LOCKS` */
export function parseLockTable(bytes: Uint8Array): LockInfo[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const count = Math.min(NUM_LOCKS, Math.floor(bytes.length / LOCK_SIZE));
  const locks: LockInfo[] = [];
  for (let lock = 0; lock < count; lock++) {
    const offset = lock * LOCK_SIZE;
    let name = '';
    for (let i = 0; i < NAME_LENGTH; i++) {
      const char = bytes[offset + i] ?? 0;
      if (char === 0) {
        break;
      }
      name += String.fromCharCode(char);
    }
    locks.push({
      name: name.trim(),
      keyItem: view.getUint16(offset + NAME_LENGTH, true),
    });
  }
  return locks;
}

function decodeBase64(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/** Reads the lock table of the selected mod, empty if it has none */
export async function loadLockTable(): Promise<LockInfo[]> {
  const base64 = await invoke('resources/read', { file: LOCK_TABLE_FILE });
  return base64 === null ? [] : parseLockTable(decodeBase64(base64));
}

/** The locks of the selected mod, `null` while they load or if `enabled` is false */
export function useLockTable(enabled = true): LockInfo[] | null {
  const [locks, setLocks] = useState<LockInfo[] | null>(null);
  useEffect(() => {
    if (!enabled || locks !== null) {
      return;
    }
    let cancelled = false;
    loadLockTable()
      .then((loaded) => {
        if (!cancelled) {
          setLocks(loaded);
        }
        return loaded;
      })
      .catch((e: unknown) => {
        console.error('Could not read the lock table', e);
        if (!cancelled) {
          setLocks([]);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, locks]);
  return enabled ? locks : null;
}
