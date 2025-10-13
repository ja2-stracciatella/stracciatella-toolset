import { useCallback, useMemo, useState, JSX, memo } from 'react';
import { Alert, Collapse, Space } from 'antd';

import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import './JsonItemsForm.css';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import { useJson, useJsonItem } from '../hooks/files';

type PreviewFn = (item: any) => JSX.Element | string | null;

type NameOrPreviewFn = string | PreviewFn;

interface ItemFormHeaderProps {
  file: string;
  index: number;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
}

const ItemFormHeader = memo(
  ({ file, index, name, preview }: ItemFormHeaderProps) => {
    const { value } = useJsonItem(file, index);
    const label = useMemo(
      () => (typeof name === 'string' ? value[name] : name(value)),
      [name, value],
    );
    const p = useMemo(
      () => (preview ? preview(value) : null),
      [preview, value],
    );

    return (
      <Space direction="horizontal">
        {p}
        {label}
      </Space>
    );
  },
);

interface ItemFormProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
  index: number;
}

function ItemForm({ file, name, preview, uiSchema, index }: ItemFormProps) {
  const { schema, value, update } = useJsonItem(file, index);
  const [isCollapsed, setCollapsed] = useState(true);
  const onPanelChange = useCallback((v: any) => {
    setCollapsed(v.length === 0);
  }, []);
  const onItemChange = useCallback(
    (ev: IChangeEvent<any>) => update(ev.formData),
    [update],
  );

  return (
    <Collapse
      onChange={onPanelChange}
      items={[
        {
          label: (
            <ItemFormHeader
              file={file}
              index={index}
              name={name}
              preview={preview}
            />
          ),
          children: isCollapsed ? null : (
            <JsonSchemaForm
              idPrefix={index.toString()}
              schema={schema}
              content={value}
              uiSchema={uiSchema}
              onChange={onItemChange}
            />
          ),
        },
      ]}
    />
  );
}

export interface FormItemsProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  numItems: number | null;
  uiSchema?: UiSchema;
}

const FormItems = memo(
  ({ file, name, preview, numItems, uiSchema }: FormItemsProps) => {
    const items = useMemo(() => {
      if (numItems == null) {
        return null;
      }
      const i = [];
      for (let it = 0; it < numItems; it++) {
        i.push(
          <ItemForm
            file={file}
            key={it}
            name={name}
            index={it}
            preview={preview}
            uiSchema={uiSchema}
          />,
        );
      }
      return i;
    }, [file, name, numItems, preview, uiSchema]);

    return (
      <Space direction="vertical" style={{ width: '100%' }}>
        {items}
      </Space>
    );
  },
);

export interface JsonItemsFormProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
}

export function JsonItemsForm({
  file,
  name,
  preview,
  uiSchema,
}: JsonItemsFormProps) {
  const { content, error } = useJson(file);
  const numItems = (content?.value?.length as number) ?? null;

  if (error) {
    return <Alert type="error" message={error.message} />;
  }

  if (numItems == null) {
    return <FullSizeLoader />;
  }

  return (
    <EditorContent path={file}>
      <JsonFormHeader file={file} />
      <FormItems
        file={file}
        name={name}
        preview={preview}
        numItems={numItems}
        uiSchema={uiSchema}
      />
    </EditorContent>
  );
}
