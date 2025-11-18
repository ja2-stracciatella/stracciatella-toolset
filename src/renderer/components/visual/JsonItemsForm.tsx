import { useCallback, useMemo, JSX } from 'react';
import { Collapse, Flex } from 'antd';
import { JsonSchemaForm } from './form/JsonSchemaForm';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { useAppDispatch } from '../../hooks/state';
import { addJsonItem } from '../../state/files';
import { AddNewButton } from './form/AddNewButton';
import { RemoveButton } from './form/RemoveButton';
import { VisualFormProps, VisualFormWrapper } from './VisualFormWrapper';
import { VisualFormWithHeader } from './VisualFormWithHeader';
import { useFileJsonItemCount } from '../../hooks/useFileJsonItemCount';
import { useFileJsonItemSchema } from '../../hooks/useFileJsonItemSchema';
import { useFileJsonItem } from '../../hooks/useFileJsonItem';
import { useFileJsonItemUpdate } from '../../hooks/useFileJsonItemUpdate';
import { AnyJsonObject } from '../../../common/invokables/jsons';

type PreviewFn = (item: AnyJsonObject) => JSX.Element | string | null;

type NameOrPreviewFn = string | PreviewFn;

export interface JsonItemsFormProps extends VisualFormProps {
  name: NameOrPreviewFn;
  preview?: PreviewFn;
  uiSchema?: UiSchema;
  canAddNewItem?: boolean;
  getNewItem?: () => object;
}

interface ItemFormHeaderProps
  extends Pick<JsonItemsFormProps, 'file' | 'name' | 'preview'> {
  index: number;
}

const ItemFormHeader = function ItemFormHeader({
  file,
  index,
  name,
  preview,
}: ItemFormHeaderProps) {
  const value = useFileJsonItem(file, index);
  const label = useMemo(() => {
    if (!value) return '';
    if (typeof name === 'string') {
      return name in value ? value[name] : '';
    }
    return name(value);
  }, [name, value]);
  const p = useMemo(
    () => (preview && value ? preview(value) : null),
    [preview, value],
  );

  return (
    <Flex justify="space-between" align="center">
      <Flex gap="small" align="center">
        {p}
        {label}
      </Flex>
      <RemoveButton file={file} index={index} />
    </Flex>
  );
};

interface ItemFormProps
  extends Pick<JsonItemsFormProps, 'file' | 'name' | 'preview' | 'uiSchema'> {
  index: number;
}

function ItemForm({ file, name, preview, uiSchema, index }: ItemFormProps) {
  const schema = useFileJsonItemSchema(file);
  const value = useFileJsonItem(file, index);
  const update = useFileJsonItemUpdate(file, index);
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

const FormItems = function FormItems({
  file,
  name,
  preview,
  uiSchema,
}: JsonItemsFormProps) {
  const numItems = useFileJsonItemCount(file);
  const items = useMemo(() => {
    return [...Array(numItems).keys()].map((idx) => (
      <ItemForm
        file={file}
        key={idx}
        name={name}
        index={idx}
        preview={preview}
        uiSchema={uiSchema}
      />
    ));
  }, [file, name, numItems, preview, uiSchema]);

  return (
    <Flex vertical gap="small">
      {items}
    </Flex>
  );
};

export const JsonItemsForm = function JsonItemsForm({
  file,
  extraFiles,
  name,
  preview,
  uiSchema,
  canAddNewItem,
  getNewItem,
}: JsonItemsFormProps) {
  const dispatch = useAppDispatch();
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

  return (
    <VisualFormWrapper file={file} extraFiles={extraFiles}>
      <VisualFormWithHeader file={file}>
        <Flex vertical gap="middle">
          <FormItems
            file={file}
            name={name}
            preview={preview}
            uiSchema={uiSchema}
          />
          {addButton}
        </Flex>
      </VisualFormWithHeader>
    </VisualFormWrapper>
  );
};
