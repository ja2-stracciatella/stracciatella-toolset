import { useCallback, useMemo, JSX, memo } from 'react';
import { Collapse, Space } from 'antd';

import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import './JsonItemsForm.css';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import {
  useFileLoading,
  useFileError,
  useFileJsonItem,
  useFileJsonItemSchema,
  useFileJsonNumberOfItems,
} from '../hooks/files';
import { ErrorAlert } from './ErrorAlert';
import { TextEditorOr } from './TextEditor';

type PreviewFn = (item: any) => JSX.Element | string | null;

type NameOrPreviewFn = string | PreviewFn;

interface ItemFormHeaderProps {
  file: string;
  index: number;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
}

const ItemFormHeader = memo(function ItemFormHeader({
  file,
  index,
  name,
  preview,
}: ItemFormHeaderProps) {
  const [value] = useFileJsonItem(file, index);
  const label = useMemo(() => {
    if (typeof name === 'string') {
      const label = value ? value[name] : null;
      if (typeof label == 'string') {
        return label;
      }
      return '';
    }
    return name(value);
  }, [name, value]);
  const p = useMemo(() => (preview ? preview(value) : null), [preview, value]);

  return (
    <Space direction="horizontal">
      {p}
      {label}
    </Space>
  );
});

interface ItemFormProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
  index: number;
}

function ItemForm({ file, name, preview, uiSchema, index }: ItemFormProps) {
  const schema = useFileJsonItemSchema(file);
  const [value, update] = useFileJsonItem(file, index);
  const onItemChange = useCallback(
    (ev: IChangeEvent<any>) => update(ev.formData),
    [update],
  );

  return (
    <Collapse
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
          children: (
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

const FormItems = memo(function FormItems({
  file,
  name,
  preview,
  numItems,
  uiSchema,
}: FormItemsProps) {
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
});

export interface JsonItemsFormProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
}

export const JsonItemsForm = memo(function JsonItemsForm({
  file,
  name,
  preview,
  uiSchema,
}: JsonItemsFormProps) {
  const loading = useFileLoading(file);
  const error = useFileError(file);
  const numItems = useFileJsonNumberOfItems(file);
  const content = useMemo(() => {
    if (numItems == null) {
      return <ErrorAlert error={{ message: 'No items after loading' }} />;
    }
    return (
      <>
        <JsonFormHeader file={file} />
        <FormItems
          file={file}
          name={name}
          preview={preview}
          numItems={numItems}
          uiSchema={uiSchema}
        />
      </>
    );
  }, [file, name, numItems, preview, uiSchema]);

  if (error) {
    return <ErrorAlert error={error} />;
  }
  if (loading) {
    return <FullSizeLoader />;
  }
  return (
    <EditorContent path={file}>
      <TextEditorOr file={file}>{content}</TextEditorOr>
    </EditorContent>
  );
});
