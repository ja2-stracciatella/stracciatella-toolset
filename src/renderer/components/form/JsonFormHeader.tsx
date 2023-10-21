import { memo, useMemo } from 'react';
import { useAppSelector } from '../../hooks/state';
import { Typography } from 'antd';
import ReactMarkdown from 'react-markdown';

interface JsonFormHeaderProps {
  file: string;
}

function Header({ file }: JsonFormHeaderProps) {
  const schema = useAppSelector((s) => s.files.json[file]?.content?.schema);
  const title = useMemo(() => schema?.title, [schema]) ?? file;
  const description = useMemo(() => schema?.description, [schema]) ?? null;
  const itemsDescription =
    useMemo(() => schema?.items?.description, [schema?.items?.description]) ??
    null;
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
}

export const JsonFormHeader = memo(Header);
