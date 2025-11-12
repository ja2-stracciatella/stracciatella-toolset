import { useCallback, useMemo, JSX, memo } from 'react';
import { Collapse, Flex, Space } from 'antd';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import './JsonItemsForm.css';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import {
  useFileLoading,
  useFileLoadingError,
  useFileJsonItem,
  useFileJsonItemSchema,
  useFileJsonNumberOfItems,
} from '../hooks/files';
import { ErrorAlert } from './ErrorAlert';
import { TextEditorOr } from './TextEditor';
import { useAppDispatch } from '../hooks/state';
import { addJsonItem } from '../state/files';
import { AddNewButton } from './form/AddNewButton';

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
    <Flex vertical gap="small">
      {items}
    </Flex>
  );
});

export interface JsonItemsFormProps {
  file: string;
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
  canAddNewItem?: boolean;
  getNewItem?: () => object;
}

export const JsonItemsForm = memo(function JsonItemsForm({
  file,
  name,
  preview,
  uiSchema,
  canAddNewItem,
  getNewItem,
}: JsonItemsFormProps) {
  const dispatch = useAppDispatch();
  const loading = useFileLoading(file);
  const error = useFileLoadingError(file);
  const numItems = useFileJsonNumberOfItems(file);
  const addNewItem = useCallback(() => {
    dispatch(
      addJsonItem({ filename: file, value: getNewItem ? getNewItem() : {} }),
    );
  }, [dispatch, file, getNewItem]);
  const addButton = useMemo(() => {
    const render = typeof canAddNewItem === 'undefined' ? true : canAddNewItem;
    if (!render) return null;
    return <AddNewButton onClick={addNewItem} />;
  }, [addNewItem, canAddNewItem]);
  const content = useMemo(() => {
    if (numItems == null) {
      return <ErrorAlert error={{ message: 'No items after loading' }} />;
    }
    return (
      <>
        <JsonFormHeader file={file} />
        <Flex vertical gap="middle">
          <FormItems
            file={file}
            name={name}
            preview={preview}
            numItems={numItems}
            uiSchema={uiSchema}
          />
          {addButton}
        </Flex>
      </>
    );
  }, [addButton, file, name, numItems, preview, uiSchema]);

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
