import { WidgetProps } from "@rjsf/core";
import { Input, AutoComplete } from "antd";
import { useMemo } from "react";
import { useJsonWithSchema } from "../../hooks/useJsonWithSchema";

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
  const { data, error } = useJsonWithSchema(referenceFile);
  const options = useMemo(() => {
    if (data?.content) {
      return data.content.map((d: any) => ({ value: d[referenceProperty] }));
    }
    return [];
  }, [data, referenceProperty]);

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
