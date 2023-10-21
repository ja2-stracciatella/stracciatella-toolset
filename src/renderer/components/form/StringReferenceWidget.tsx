import { WidgetProps } from '@rjsf/utils';
import { Input, AutoComplete } from 'antd';
import { useMemo } from 'react';
import { useJson } from '../../hooks/files';

interface StringReferenceWidgetProps extends WidgetProps {
  referenceFile: string;
  referenceProperty: string;
}

export function StringReferenceWidget({
  value,
  onChange,
  required,
  referenceFile,
  referenceProperty,
}: StringReferenceWidgetProps) {
  const { content, error } = useJson(referenceFile);
  const options = useMemo(() => {
    if (!content) {
      return [];
    }
    return content.value.map((d: any) => ({ value: d[referenceProperty] }));
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
  return function StringReference(props: WidgetProps) {
    return (
      <StringReferenceWidget
        referenceFile={file}
        referenceProperty={property}
        {...props}
      />
    );
  };
}
