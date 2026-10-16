import { QuestionCircleOutlined } from '@ant-design/icons';
import { Popover, Typography } from 'antd';
import { useMemo } from 'react';

export interface HelpIconProps {
  /** Shown above the paragraphs */
  title?: string;
  paragraphs: string[];
  /** aria-label of the icon */
  label: string;
}

const CONTENT_STYLE = { maxWidth: 320 } as const;
const LAST_PARAGRAPH_STYLE = { marginBottom: 0 } as const;

/** A question mark that shows help in a popover on hover */
export function HelpIcon({ title, paragraphs, label }: HelpIconProps) {
  const content = useMemo(
    () => (
      <div style={CONTENT_STYLE}>
        {paragraphs.map((paragraph, index) => (
          <Typography.Paragraph
            key={index}
            style={
              index === paragraphs.length - 1 ? LAST_PARAGRAPH_STYLE : undefined
            }
          >
            {paragraph}
          </Typography.Paragraph>
        ))}
      </div>
    ),
    [paragraphs],
  );

  return (
    <Popover title={title} content={content}>
      <QuestionCircleOutlined aria-label={label} />
    </Popover>
  );
}
