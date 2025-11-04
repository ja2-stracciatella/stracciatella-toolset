import { useCallback, useMemo, useState } from 'react';
import { Space } from 'antd';

import { UiSchema } from '@rjsf/utils';
import { FullSizeLoader } from './FullSizeLoader';
import { StrategicMap } from './content/StrategicMap';
import { JsonSchemaForm } from './JsonSchemaForm';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import {
  useFileError,
  useFileJson,
  useFileJsonItem,
  useFileJsonItemSchema,
  useFileLoading,
} from '../hooks/files';
import { IChangeEvent } from '@rjsf/core';
import { ErrorAlert } from './ErrorAlert';

interface ItemFormProps {
  file: string;
  index: number;
  uiSchema?: UiSchema;
}

function ItemForm({ file, index, uiSchema }: ItemFormProps) {
  const schema = useFileJsonItemSchema(file);
  const [value, update] = useFileJsonItem(file, index);
  const onItemChange = useCallback(
    (ev: IChangeEvent<any>) => update(ev.formData),
    [update],
  );

  if (index === -1 || !value) {
    return <div>Select a sector to the left to edit.</div>;
  }

  return (
    <JsonSchemaForm
      schema={schema}
      content={value}
      uiSchema={uiSchema}
      onChange={onItemChange}
    />
  );
}

export interface StrategicMapFormProps {
  file: string;
  property?: string;
  uiSchema?: UiSchema;
}

export function JsonStrategicMapForm({
  file,
  property = 'sector',
  uiSchema,
}: StrategicMapFormProps) {
  const loading = useFileLoading(file);
  const error = useFileError(file);
  const [selectedItem, setSelectedItem] = useState(-1);
  const [value] = useFileJson(file);
  const sectorsWithContent = useMemo(
    () => (value ? value.map((d: any) => d[property].toLowerCase()) : []),
    [value, property],
  );
  const onSectorClick = useCallback(
    (sectorId: string) => {
      if (!value) {
        return;
      }
      const idx = sectorsWithContent.indexOf(sectorId.toLowerCase());
      setSelectedItem(idx);
    },
    [sectorsWithContent, value],
  );

  if (loading) {
    return <FullSizeLoader />;
  }
  if (error) {
    return <ErrorAlert error={error} />;
  }
  if (!value) {
    return <ErrorAlert error={{ message: 'No items after loading' }} />;
  }

  return (
    <EditorContent path={file}>
      <Space direction="horizontal" align="start" size="large">
        <StrategicMap
          highlightedSectorIds={sectorsWithContent}
          onSectorClick={onSectorClick}
        />
        <div>
          <JsonFormHeader file={file} />
          <ItemForm file={file} index={selectedItem} uiSchema={uiSchema} />
        </div>
      </Space>
    </EditorContent>
  );
}
