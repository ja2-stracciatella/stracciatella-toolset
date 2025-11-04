import { memo, useMemo } from 'react';
import { Typography } from 'antd';
import ReactMarkdown from 'react-markdown';
import { useFileSchema } from '../../hooks/files';

interface JsonFormHeaderProps {
  file: string;
}

export const JsonFormHeader = memo(function Header({
  file,
}: JsonFormHeaderProps) {
  const schema = useFileSchema(file);
  const title = useMemo(() => schema?.title ?? file, [schema, file]);
  const description = useMemo(() => schema?.description ?? null, [schema]);
  const itemsDescription = useMemo(
    () => schema?.items?.description ?? null,
    [schema?.items?.description],
  );
  const combinedDescription = useMemo(
    () => [itemsDescription, description].filter((v) => !!v).join('\n\n'),
    [itemsDescription, description],
  );

  return (
    <div>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{combinedDescription}</ReactMarkdown>
      </div>
    </div>
  );
});
