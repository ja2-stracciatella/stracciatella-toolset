import { Segmented } from 'antd';
import { useCallback } from 'react';

type EraseOption = 'add' | 'remove';

const OPTIONS: { label: string; value: EraseOption }[] = [
  { label: 'Add', value: 'add' },
  { label: 'Remove', value: 'remove' },
];

export interface EraseControlProps {
  erase: boolean;
  onChange: (erase: boolean) => unknown;
}

/** Whether clicks add or remove, the erase flag of the modes */
export function EraseControl({ erase, onChange }: EraseControlProps) {
  const onOptionChange = useCallback(
    (value: EraseOption) => onChange(value === 'remove'),
    [onChange],
  );

  return (
    <Segmented<EraseOption>
      block
      options={OPTIONS}
      value={erase ? 'remove' : 'add'}
      onChange={onOptionChange}
      aria-label="erase"
    />
  );
}
