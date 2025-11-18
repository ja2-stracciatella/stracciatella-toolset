import { Typography } from 'antd';
import ReactMarkdown from 'react-markdown';
import { useFileTitle } from '../../../hooks/useFileTitle';
import { useFileDescription } from '../../../hooks/useFileDescription';

interface JsonFormHeaderProps {
  file: string;
}

export const JsonFormHeader = function Header({ file }: JsonFormHeaderProps) {
  const title = useFileTitle(file);
  const description = useFileDescription(file);

  return (
    <div>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
    </div>
  );
};
