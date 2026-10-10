import { fireEvent, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import {
  DoorLockModal,
  validDoor,
} from '../../../../renderer/components/content/mapEditor/DoorLockModal';
import { renderWithTestProviders } from '../../test-utils/render';

describe('validDoor', () => {
  it('should clamp the lock and give traps a level', () => {
    expect(
      validDoor({
        lockId: 99,
        trap: { kind: 'siren', level: 0 },
        locked: true,
      }),
    ).toEqual({ lockId: 63, trap: { kind: 'siren', level: 1 }, locked: true });
    expect(
      validDoor({
        lockId: -1,
        trap: { kind: 'electric', level: 40 },
        locked: false,
      }),
    ).toEqual({
      lockId: 0,
      trap: { kind: 'electric', level: 20 },
      locked: false,
    });
  });
});

describe('DoorLockModal', () => {
  it('should save and remove the door of the request', async () => {
    const onSave = vi.fn();
    const onClose = vi.fn();
    const request = {
      type: 'door' as const,
      map: 'maps/A9.dat',
      gridNo: 805,
      door: { lockId: 3, trap: null, locked: false },
    };
    const { getByLabelText, getByText } = renderWithTestProviders(
      <DoorLockModal request={request} onSave={onSave} onClose={onClose} />,
    );
    await waitFor(() => {
      expect(getByLabelText('lock id')).toHaveValue('3');
    });
    fireEvent.change(getByLabelText('lock id'), { target: { value: '7' } });
    fireEvent.click(getByText('Locked'));
    fireEvent.click(getByText('Okay'));
    expect(onSave).toHaveBeenCalledWith(805, {
      lockId: 7,
      trap: null,
      locked: true,
    });
    expect(onClose).toHaveBeenCalledTimes(1);

    fireEvent.click(getByText('Remove'));
    expect(onSave).toHaveBeenLastCalledWith(805, null);
  });
});
