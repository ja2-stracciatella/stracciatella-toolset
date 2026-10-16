import { Flex, Segmented, Typography } from 'antd';
import { useMemo } from 'react';
import { HelpIcon } from '../../common/HelpIcon';

/**
 * What a tool does, shown in the popover next to the tools, as paragraphs: what the tool is used
 * for, then only if it has any, the keys and mouse buttons it uses besides a left click
 */
export type ToolDescription = string[];

export interface ToolOption<T extends string> {
  label: string;
  value: T;
  description: ToolDescription;
}

export interface ToolSelectProps<T extends string> {
  tools: ToolOption<T>[];
  value: T;
  onChange: (value: T) => unknown;
  /** Names the tools, the description is named after it */
  label: string;
}

const SEGMENTED_STYLE = { minWidth: 0 } as const;

/** The tools of a mode, wrapping onto more lines, with the description of the chosen tool */
export function ToolSelect<T extends string>({
  tools,
  value,
  onChange,
  label,
}: ToolSelectProps<T>) {
  const options = useMemo(
    () => tools.map((tool) => ({ label: tool.label, value: tool.value })),
    [tools],
  );
  const selected = tools.find((tool) => tool.value === value);

  return (
    <Flex vertical gap="small">
      <Typography.Text>Tool</Typography.Text>
      <Flex gap="small" align="center">
        <Segmented<T>
          rootClassName="wrapping-segmented"
          style={SEGMENTED_STYLE}
          options={options}
          value={value}
          onChange={onChange}
          aria-label={label}
        />
        {selected ? (
          <HelpIcon
            title={selected.label}
            paragraphs={selected.description}
            label={`${label} description`}
          />
        ) : null}
      </Flex>
    </Flex>
  );
}
