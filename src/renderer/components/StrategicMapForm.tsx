import { useCallback, useMemo, useState } from 'react';
import { Alert, Space } from 'antd';

import { UiSchema } from '@rjsf/utils';
import { FullSizeLoader } from './FullSizeLoader';
import { StrategicMap } from './content/StrategicMap';
import { JsonSchemaForm } from './JsonSchemaForm';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import { useJson, useJsonItem } from '../hooks/files';
import { IChangeEvent } from '@rjsf/core';

interface ItemFormProps {
  file: string;
  index: number;
  uiSchema?: UiSchema;
}

function ItemForm({ file, index, uiSchema }: ItemFormProps) {
  const { schema, value, error, update } = useJsonItem(file, index);
  const onItemChange = useCallback(
    (ev: IChangeEvent<any>) => update(ev.formData),
    [update],
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
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
  const [selectedItem, setSelectedItem] = useState(-1);
  const { content } = useJson(file);
  const sectorsWithContent = useMemo(
    () =>
      content ? content.value.map((d: any) => d[property].toLowerCase()) : [],
    [content, property],
  );
  const onSectorClick = useCallback(
    (sectorId: string) => {
      if (!content) {
        return;
      }
      const idx = sectorsWithContent.indexOf(sectorId.toLowerCase());
      setSelectedItem(idx);
    },
    [content, sectorsWithContent],
  );

  if (!content) {
    return <FullSizeLoader />;
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
