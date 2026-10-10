import { fireEvent, within } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import {
  ToolOption,
  ToolSelect,
} from '../../../../renderer/components/content/mapEditor/ToolSelect';
import { renderWithTestProviders } from '../../test-utils/render';

type Tool = 'select' | 'place';

const TOOLS: ToolOption<Tool>[] = [
  {
    label: 'Select',
    value: 'select',
    description: ['Selects things.', 'Ctrl+drag to copy.'],
  },
  {
    label: 'Place',
    value: 'place',
    description: ['Places things.'],
  },
];

function Harness() {
  const [tool, setTool] = useState<Tool>('select');
  return (
    <ToolSelect<Tool>
      tools={TOOLS}
      value={tool}
      onChange={setTool}
      label="test tool"
    />
  );
}

describe('ToolSelect', () => {
  it('should change the tool', () => {
    const onChange = vi.fn();
    const { getByLabelText } = renderWithTestProviders(
      <ToolSelect<Tool>
        tools={TOOLS}
        value="select"
        onChange={onChange}
        label="test tool"
      />,
    );
    fireEvent.click(within(getByLabelText('test tool')).getByText('Place'));
    expect(onChange).toHaveBeenCalledWith('place');
  });

  it('should describe the chosen tool in a popover', async () => {
    const { getByLabelText, findByText, queryByText } = renderWithTestProviders(
      <Harness />,
    );
    expect(queryByText('Selects things.')).toBeNull();
    fireEvent.mouseEnter(getByLabelText('test tool description'));
    expect(await findByText('Selects things.')).toBeInTheDocument();
    expect(await findByText('Ctrl+drag to copy.')).toBeInTheDocument();

    fireEvent.click(within(getByLabelText('test tool')).getByText('Place'));
    expect(await findByText('Places things.')).toBeInTheDocument();
    expect(queryByText('Selects things.')).toBeNull();
    expect(queryByText('Ctrl+drag to copy.')).toBeNull();
  });
});
