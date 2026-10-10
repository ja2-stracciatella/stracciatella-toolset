import { describe, expect, it } from 'vitest';
import {
  NUM_LOCKS,
  parseLockTable,
} from '../../../../renderer/components/content/mapEditor/lockTable';

/** A lock of `Locks.bin`: 40 bytes of name, the key item and 4 more bytes */
function lockBytes(name: string, keyItem: number): number[] {
  const bytes = Array.from({ length: 46 }, () => 0);
  [...name].forEach((char, i) => {
    bytes[i] = char.charCodeAt(0);
  });
  bytes[40] = keyItem & 0xff;
  bytes[41] = keyItem >> 8;
  bytes[42] = 3;
  return bytes;
}

describe('parseLockTable', () => {
  it('should read the names and key items of the locks', () => {
    const bytes = new Uint8Array([
      ...lockBytes('Front door ', 4),
      ...lockBytes('', 0x0102),
      // An incomplete lock is left out
      ...lockBytes('Cut off', 1).slice(0, 20),
    ]);
    expect(parseLockTable(bytes)).toEqual([
      { name: 'Front door', keyItem: 4 },
      { name: '', keyItem: 0x0102 },
    ]);
  });

  it('should read at most the locks of the game', () => {
    const bytes = new Uint8Array(
      Array.from({ length: NUM_LOCKS + 2 }, (_, i) =>
        lockBytes(`Lock ${i}`, i),
      ).flat(),
    );
    const locks = parseLockTable(bytes);
    expect(locks).toHaveLength(NUM_LOCKS);
    expect(locks[63]).toEqual({ name: 'Lock 63', keyItem: 63 });
  });
});
