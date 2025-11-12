import { useCallback, useMemo, useState } from 'react';
import { Space } from 'antd';

import { UiSchema } from '@rjsf/utils';
import { FullSizeLoader } from './FullSizeLoader';
import { NormalizedSectorId, StrategicMap } from './content/StrategicMap';
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
import { findIndex, isDeepEqual } from 'remeda';

interface ItemFormProps {
  file: string;
  index: number;
  transformSectorToItem: (sectorId: NormalizedSectorId) => any;
  uiSchema?: UiSchema;
  sectorId?: NormalizedSectorId;
  canAddNewItem?: boolean;
  getNewItem?: () => object;
}

function ItemForm({
  file,
  index,
  uiSchema,
  sectorId,
  transformSectorToItem,
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
  const addNewItem = useCallback(() => {
    if (!sectorId) return;
    dispatch(
      addJsonItem({
        filename: file,
        value: {
          ...(getNewItem ? getNewItem() : {}),
          ...transformSectorToItem(sectorId),
        },
      }),
    );
  }, [dispatch, file, getNewItem, sectorId, transformSectorToItem]);

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
  uiSchema?: UiSchema;
  canAddNewItem?: boolean;
  initialLevel?: number;
  canChangeLevel?: boolean;
  getNewItem?: () => object;
  extractSectorFromItem: (value: any) => NormalizedSectorId;
  transformSectorToItem: (sectorId: NormalizedSectorId) => any;
}

export function JsonStrategicMapForm({
  file,
  uiSchema,
  extractSectorFromItem,
  transformSectorToItem,
  canAddNewItem,
  initialLevel = 0,
  canChangeLevel,
  getNewItem,
}: StrategicMapFormProps) {
  const [level, setLevel] = useState(initialLevel);
  const loading = useFileLoading(file);
  const error = useFileLoadingError(file);
  const [value] = useFileJson(file);
  const [selectedSector, setSelectedSector] = useState<
    NormalizedSectorId | undefined
  >();
  const sectorsWithContent: NormalizedSectorId[] = useMemo(
    () => (value ?? []).map((d: any) => extractSectorFromItem(d)),
    [value, extractSectorFromItem],
  );
  const selectedItem = useMemo(() => {
    if (!selectedSector) return -1;
    return findIndex(sectorsWithContent, (sector) =>
      isDeepEqual(sector, selectedSector),
    );
  }, [sectorsWithContent, selectedSector]);
  const onSectorClick = useCallback(
    (sectorId: NormalizedSectorId) => {
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
          level={level}
          selectedSectorId={selectedSector}
          highlightedSectorIds={sectorsWithContent}
          onSectorClick={onSectorClick}
          onLevelChange={canChangeLevel ? setLevel : undefined}
        />
        <div>
          <JsonFormHeader file={file} />
          <ItemForm
            file={file}
            index={selectedItem}
            uiSchema={uiSchema}
            sectorId={selectedSector}
            transformSectorToItem={transformSectorToItem}
            canAddNewItem={canAddNewItem}
            getNewItem={getNewItem}
          />
        </div>
      </Space>
    );
  }, [
    canAddNewItem,
    canChangeLevel,
    file,
    getNewItem,
    level,
    onSectorClick,
    sectorsWithContent,
    selectedItem,
    selectedSector,
    transformSectorToItem,
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

export function makeStrategicMapFormPropsForProperty<S extends string>(
  prop: S,
) {
  return {
    extractSectorFromItem: (item: any): NormalizedSectorId => [item[prop], 0],
    transformSectorToItem: (sector: NormalizedSectorId) => ({
      [prop]: sector[0],
    }),
    uiSchema: {
      [prop]: { 'ui:disabled': true },
    },
  };
}

export function makeStrategicMapFormPropsForProperties<
  S extends string,
  L extends string,
>(sectorProp: S, levelProp: L) {
  return {
    extractSectorFromItem: (item: any): NormalizedSectorId => [
      item[sectorProp],
      item[levelProp] ?? 0,
    ],
    transformSectorToItem: (sector: NormalizedSectorId) => ({
      [sectorProp]: sector[0],
      [levelProp]: sector[1],
    }),
    canChangeLevel: true,
    uiSchema: {
      [sectorProp]: { 'ui:disabled': true },
      [levelProp]: { 'ui:disabled': true },
    },
  };
}
