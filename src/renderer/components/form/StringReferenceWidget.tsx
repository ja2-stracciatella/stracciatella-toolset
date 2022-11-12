import { WidgetProps } from '@rjsf/core';
import { Input, AutoComplete } from 'antd';
import { useMemo } from 'react';
import { z } from 'zod';
import { useJsonWithSchema } from '../../hooks/useJsonWithSchema';

interface StringReferenceWidgetProps extends WidgetProps {
  referenceFile: string;
  referenceProperty: string;
}

const schemaSchema = z.any();

const contentSchema = z.array(z.record(z.any()));

export function StringReferenceWidget({
  value,
  onChange,
  required,
  referenceFile,
  referenceProperty,
}: StringReferenceWidgetProps) {
  const { content, error } = useJsonWithSchema(
    schemaSchema,
    contentSchema,
    referenceFile
  );
  const options = useMemo(() => {
    if (content) {
      return content.map((d) => ({ value: d[referenceProperty] }));
    }
    return [];
  }, [content, referenceProperty]);

  if (error) {
    return <Input value={value} onChange={onChange} required={required} />;
  }

  return (
    <AutoComplete
      options={options}
      value={value}
      onChange={onChange}
      placeholder="Please select..."
    />
  );
}

export function stringReferenceTo(file: string, property: string) {
  return (props: WidgetProps) => (
    <StringReferenceWidget
      referenceFile={file}
      referenceProperty={property}
      {...props}
    />
  );
}
