import { useCallback, useMemo, useState } from 'react';
import { Typography } from 'antd';
import { UiSchema } from '@rjsf/utils';
import {
  DEFAULT_HIGHLIGHT_COLOR,
  NormalizedSectorId,
} from '../content/StrategicMap';
import { JsonSchemaForm } from './form/JsonSchemaForm';
import { IChangeEvent } from '@rjsf/core';
import { AddNewButton } from './form/AddNewButton';
import { useAppDispatch } from '../../hooks/state';
import { addJsonItem } from '../../state/files';
import { findIndex, isDeepEqual } from 'remeda';
import { RemoveButton } from './form/RemoveButton';
import { VisualFormProps } from './VisualFormWrapper';
import { VisualStrategicMapFormWrapper } from './VisualStrategicMapFormWrapper';
import { useFileJsonValue } from '../../hooks/useFileJsonValue';
import { useFileJsonItemSchema } from '../../hooks/useFileJsonItemSchema';
import { useFileJsonItem } from '../../hooks/useFileJsonItem';
import { useFileJsonItemUpdate } from '../../hooks/useFileJsonItemUpdate';
import { AnyJsonObject } from '../../../common/invokables/jsons';

interface ItemFormProps {
  file: string;
  index: number;
  transformSectorToItem: (sectorId: NormalizedSectorId) => AnyJsonObject;
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
  const value = useFileJsonItem(file, index);
  const update = useFileJsonItemUpdate(file, index);
  const onItemChange = useCallback(
    (ev: IChangeEvent<AnyJsonObject>) => {
      if (ev.formData) {
        update(ev.formData);
      }
    },
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

export interface StrategicMapFormProps extends VisualFormProps {
  uiSchema?: UiSchema;
  canAddNewItem?: boolean;
  initialLevel?: number;
  canChangeLevel?: boolean;
  getNewItem?: () => AnyJsonObject;
  extractSectorFromItem: (value: AnyJsonObject) => NormalizedSectorId;
  transformSectorToItem: (sectorId: NormalizedSectorId) => AnyJsonObject;
}

export function JsonStrategicMapForm({
  file,
  uiSchema,
  extractSectorFromItem,
  transformSectorToItem,
  canAddNewItem,
  initialLevel,
  canChangeLevel,
  getNewItem,
}: StrategicMapFormProps) {
  const [level, setLevel] = useState(initialLevel ?? 0);
  const value = useFileJsonValue(file);
  const [selectedSectorId, setSelectedSector] = useState<
    NormalizedSectorId | undefined
  >();
  const sectorsWithContent: NormalizedSectorId[] = useMemo(
    () => (value ?? []).map((d: any) => extractSectorFromItem(d)),
    [value, extractSectorFromItem],
  );
  const highlightedSectorIds = useMemo(() => {
    return { [DEFAULT_HIGHLIGHT_COLOR]: sectorsWithContent };
  }, [sectorsWithContent]);
  const selectedItem = useMemo(() => {
    if (!selectedSectorId) return -1;
    return findIndex(sectorsWithContent, (sector) =>
      isDeepEqual(sector, selectedSectorId),
    );
  }, [sectorsWithContent, selectedSectorId]);
  const onSectorClick = useCallback(
    (sectorId: NormalizedSectorId) => {
      if (isDeepEqual(sectorId, selectedSectorId)) return;
      setSelectedSector(sectorId);
    },
    [selectedSectorId],
  );
  const removeButton = useMemo(() => {
    if (selectedItem === -1) return null;
    return (
      <Typography.Paragraph>
        <RemoveButton
          file={file}
          index={selectedItem}
          label="Remove from this sector"
        />
      </Typography.Paragraph>
    );
  }, [file, selectedItem]);
  const onLevelChange = canChangeLevel ? setLevel : undefined;

  return (
    <VisualStrategicMapFormWrapper
      file={file}
      strategicMap={{
        level,
        selectedSectorId,
        highlightedSectorIds,
        onSectorClick,
        onLevelChange,
      }}
    >
      {removeButton}
      <ItemForm
        file={file}
        index={selectedItem}
        uiSchema={uiSchema}
        sectorId={selectedSectorId}
        transformSectorToItem={transformSectorToItem}
        canAddNewItem={canAddNewItem}
        getNewItem={getNewItem}
      />
    </VisualStrategicMapFormWrapper>
  );
}

export function makeStrategicMapFormPropsForProperty<S extends string>(
  prop: S,
) {
  return {
    extractSectorFromItem: (item: AnyJsonObject): NormalizedSectorId => [
      item[prop],
      0,
    ],
    transformSectorToItem: (sector: NormalizedSectorId): AnyJsonObject => ({
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
    extractSectorFromItem: (item: AnyJsonObject): NormalizedSectorId => [
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
