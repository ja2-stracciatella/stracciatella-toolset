import { describe, expect, it } from 'vitest';
import {
  actionTiles,
  formatTime,
  parseTime,
  pickInstructions,
} from '../../../../renderer/components/content/mapEditor/SoldierScheduleTab';

describe('schedule times', () => {
  it('should read and show times of day', () => {
    expect(parseTime('8:30')).toBe(510);
    expect(parseTime('0830')).toBe(510);
    expect(parseTime(' 23:59 ')).toBe(1439);
    expect(parseTime('')).toBe(null);
    expect(parseTime('24:00')).toBe(undefined);
    expect(parseTime('8:61')).toBe(undefined);
    expect(parseTime('noon')).toBe(undefined);
    expect(formatTime(510)).toBe('08:30');
    expect(formatTime(null)).toBe('');
  });

  it('should know the tiles of the actions', () => {
    expect(actionTiles('lockDoor')).toBe('door');
    expect(actionTiles('sleep')).toBe('one');
    expect(actionTiles('leaveSector')).toBe('none');
    expect(
      pickInstructions('closeDoor', { slot: 0, point: 'b', single: false }),
    ).toBe(
      'Click on the tile where you wish to move after you close the door.',
    );
    expect(
      pickInstructions('gridNo', { slot: 0, point: 'a', single: false }),
    ).toBe('Click on the tile where you wish to move to.');
  });
});
