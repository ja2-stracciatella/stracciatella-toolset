import { useCallback, useMemo, useState } from 'react';
import { Space } from 'antd';

import { UiSchema } from '@rjsf/utils';
import { FullSizeLoader } from './FullSizeLoader';
import { StrategicMap } from './content/StrategicMap';
import { JsonSchemaForm } from './JsonSchemaForm';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import {
  useFileLoadingError,
  useFileJson,
  useFileJsonItem,
  useFileJsonItemSchema,
  useFileLoading,
} from '../hooks/files';
import { IChangeEvent } from '@rjsf/core';
import { ErrorAlert } from './ErrorAlert';
import { TextEditorOr } from './TextEditor';
import { AddNewButton } from './form/AddNewButton';
import { useAppDispatch } from '../hooks/state';
import { addJsonItem } from '../state/files';

interface ItemFormProps {
  file: string;
  index: number;
  property: string;
  uiSchema?: UiSchema;
  sectorId?: string;
  canAddNewItem?: boolean;
  getNewItem?: () => object;
}

function ItemForm({
  file,
  index,
  property,
  uiSchema,
  sectorId,
  canAddNewItem,
  getNewItem,
}: ItemFormProps) {
  const dispatch = useAppDispatch();
  const schema = useFileJsonItemSchema(file);
  const [value, update] = useFileJsonItem(file, index);
  const onItemChange = useCallback(
    (ev: IChangeEvent<any>) => update(ev.formData),
    [update],
  );
  const addNewItem = useCallback(
    () =>
      dispatch(
        addJsonItem({
          filename: file,
          value: {
            ...(getNewItem ? getNewItem() : {}),
            [property]: sectorId,
          },
        }),
      ),
    [dispatch, file, getNewItem, property, sectorId],
  );

  if (index === -1 || !value) {
    if (sectorId && (typeof canAddNewItem === 'undefined' || canAddNewItem)) {
      return <AddNewButton onClick={addNewItem} />;
    } else {
      return <div>Select a sector to the left to edit.</div>;
    }
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
  canAddNewItem?: boolean;
  getNewItem?: () => object;
}

export function JsonStrategicMapForm({
  file,
  property = 'sector',
  uiSchema,
  canAddNewItem,
  getNewItem,
}: StrategicMapFormProps) {
  const loading = useFileLoading(file);
  const error = useFileLoadingError(file);
  const [value] = useFileJson(file);
  const [selectedSector, setSelectedSector] = useState<string | undefined>();
  const sectorsWithContent = useMemo(
    () => (value ? value.map((d: any) => d[property]) : []),
    [value, property],
  );
  const selectedItem = useMemo(() => {
    if (!selectedSector) return null;
    return sectorsWithContent.indexOf(selectedSector);
  }, [sectorsWithContent, selectedSector]);
  const onSectorClick = useCallback(
    (sectorId: string) => {
      if (!value) {
        return;
      }
      setSelectedSector(sectorId);
    },
    [value],
  );
  const contents = useMemo(() => {
    if (!value) {
      return <ErrorAlert error={{ message: 'No items after loading' }} />;
    }
    return (
      <Space direction="horizontal" align="start" size="large">
        <StrategicMap
          selectedSectorId={selectedSector}
          highlightedSectorIds={sectorsWithContent}
          onSectorClick={onSectorClick}
        />
        <div>
          <JsonFormHeader file={file} />
          <ItemForm
            file={file}
            index={selectedItem}
            property={property}
            uiSchema={uiSchema}
            sectorId={selectedSector}
            canAddNewItem={canAddNewItem}
            getNewItem={getNewItem}
          />
        </div>
      </Space>
    );
  }, [
    canAddNewItem,
    file,
    getNewItem,
    onSectorClick,
    property,
    sectorsWithContent,
    selectedItem,
    selectedSector,
    uiSchema,
    value,
  ]);

  if (loading) {
    return <FullSizeLoader />;
  }
  if (error) {
    return <ErrorAlert error={error} />;
  }

  return (
    <EditorContent path={file}>
      <TextEditorOr file={file}>{contents}</TextEditorOr>
    </EditorContent>
  );
}
