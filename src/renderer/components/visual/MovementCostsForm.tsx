import { useCallback, useMemo, useState } from 'react';
import {
  useFileJson,
  useFileLoading,
  useFileLoadingError,
} from '../../hooks/files';
import {
  coordsFromSectorIdString,
  NormalizedSectorId,
  StrategicMap,
} from '../content/StrategicMap';
import { EditorContent } from '../layout/EditorContent';
import { ErrorAlert } from '../common/ErrorAlert';
import { FullSizeLoader } from '../common/FullSizeLoader';
import { TextEditorOr } from '../TextEditor';
import { Flex, Typography } from 'antd';
import { JsonFormHeader } from './form/JsonFormHeader';
import { JsonSchema } from 'src/common/invokables/jsons';
import { JsonSchemaForm } from './form/JsonSchemaForm';
import { UiSchema } from '@rjsf/utils';
import { IChangeEvent } from '@rjsf/core';
import { clone } from 'remeda';

const TRAVERSABILITY_ENUM_JSON_SCHEMA: JsonSchema = {
  type: 'string',
  enum: [
    'A',
    'AR',
    'C',
    'CR',
    'D',
    'DR',
    'E',
    'F',
    'FR',
    'G',
    'H',
    'HR',
    'N',
    'P',
    'PR',
    'R',
    'S',
    'SR',
    'T',
    'TR',
    'W',
    'WR',
    'WT',
    'X',
  ],
};

const TRAVERSABILITY_ENUM_TITLES = [
  'SAND',
  'SAND_ROAD',
  'COASTAL',
  'COASTAL_ROAD',
  'DENSE',
  'DENSE_ROAD',
  'EDGEOFWORLD',
  'FARMLAND',
  'FARMLAND_ROAD',
  'GROUNDBARRIER',
  'HILLS',
  'HILLS_ROAD',
  'NS_RIVER',
  'PLAINS',
  'PLAINS_ROAD',
  'ROAD',
  'SPARSE',
  'SPARSE_ROAD',
  'TROPICS',
  'TROPICS_ROAD',
  'SWAMP',
  'SWAMP_ROAD',
  'WATER',
  'TOWN',
];

const MOVEMENT_COSTS_JSON_SCHEMA = {
  type: 'object',
  properties: {
    traverseWest: {
      title: 'Traverse West',
      description: 'The cost to traverse to the sector in the west.',
      ...TRAVERSABILITY_ENUM_JSON_SCHEMA,
    },
    traverseEast: {
      title: 'Traverse East',
      description: 'The cost to traverse to the sector in the east.',
      ...TRAVERSABILITY_ENUM_JSON_SCHEMA,
    },
    traverseSouth: {
      title: 'Traverse South',
      description: 'The cost to traverse to the sector in the south.',
      ...TRAVERSABILITY_ENUM_JSON_SCHEMA,
    },
    traverseNorth: {
      title: 'Traverse North',
      description: 'The cost to traverse to the sector in the north.',
      ...TRAVERSABILITY_ENUM_JSON_SCHEMA,
    },
    traverseThrough: {
      title: 'Traverse Through',
      description: 'The cost to traverse through this sector.',
      ...TRAVERSABILITY_ENUM_JSON_SCHEMA,
    },
    travelRating: {
      type: 'number',
      minimum: 0,
      maximum: 255,
    },
  },
  required: [
    'traverseWest',
    'traverseEast',
    'traverseSouth',
    'traverseNorth',
    'traverseThrough',
    'travelRating',
  ],
};

const MOVEMENT_COSTS_UI_SCHEMA: UiSchema = {
  'ui:order': [
    'traverseThrough',
    'traverseNorth',
    'traverseSouth',
    'traverseWest',
    'traverseEast',
    'travelRating',
  ],
  traverseWest: { 'ui:enumNames': TRAVERSABILITY_ENUM_TITLES },
  traverseEast: { 'ui:enumNames': TRAVERSABILITY_ENUM_TITLES },
  traverseSouth: { 'ui:enumNames': TRAVERSABILITY_ENUM_TITLES },
  traverseNorth: { 'ui:enumNames': TRAVERSABILITY_ENUM_TITLES },
  traverseThrough: { 'ui:enumNames': TRAVERSABILITY_ENUM_TITLES },
};

interface MovementCostsFormProps {
  file: string;
}

function MovementCosts({
  file,
  value,
  onChange,
}: MovementCostsFormProps & {
  value: any;
  onChange: (value: any) => unknown;
}) {
  const [selectedSector, setSelectedSector] =
    useState<NormalizedSectorId | null>(null);
  const coords = useMemo(
    () => (selectedSector ? coordsFromSectorIdString(selectedSector[0]) : null),
    [selectedSector],
  );
  const content = useMemo(() => {
    if (!coords) {
      return {};
    }
    return {
      traverseThrough: value.traverseThrough[coords[1]][coords[0]],
      traverseNorth: value.traverseNS[coords[1]][coords[0]],
      traverseSouth: value.traverseNS[coords[1] + 1][coords[0]],
      traverseWest: value.traverseWE[coords[1]][coords[0]],
      traverseEast: value.traverseWE[coords[1]][coords[0] + 1],
      travelRating: value.travelRatings[coords[1]][coords[0]],
    };
  }, [value, coords]);
  const handleChange = useCallback(
    (newContent: IChangeEvent<any>) => {
      if (!coords) {
        return;
      }
      const n = clone(value);
      n.traverseThrough[coords[1]][coords[0]] =
        newContent.formData.traverseThrough;
      n.traverseNS[coords[1]][coords[0]] = newContent.formData.traverseNorth;
      n.traverseNS[coords[1] + 1][coords[0]] =
        newContent.formData.traverseSouth;
      n.traverseWE[coords[1]][coords[0]] = newContent.formData.traverseWest;
      n.traverseWE[coords[1]][coords[0] + 1] = newContent.formData.traverseEast;
      n.travelRatings[coords[1]][coords[0]] = newContent.formData.travelRating;
      onChange(n);
    },
    [coords, onChange, value],
  );
  const contentElement = useMemo(() => {
    if (!coords) {
      return (
        <Typography.Paragraph>
          Select sector to view and edit movement costs.
        </Typography.Paragraph>
      );
    }
    return (
      <JsonSchemaForm
        schema={MOVEMENT_COSTS_JSON_SCHEMA}
        content={content}
        onChange={handleChange}
        uiSchema={MOVEMENT_COSTS_UI_SCHEMA}
      />
    );
  }, [content, coords, handleChange]);

  return (
    <Flex gap="middle">
      <StrategicMap
        selectedSectorId={selectedSector}
        onSectorClick={setSelectedSector}
      />
      <div>
        <JsonFormHeader file={file} />
        {contentElement}
      </div>
    </Flex>
  );
}

export function MovementCostsForm({ file }: MovementCostsFormProps) {
  const loading = useFileLoading(file);
  const error = useFileLoadingError(file);
  const [value, update] = useFileJson(file);

  if (loading == null || loading) {
    return <FullSizeLoader />;
  }
  if (error) {
    return <ErrorAlert error={error} />;
  }

  return (
    <EditorContent path={file}>
      <TextEditorOr file={file}>
        <MovementCosts file={file} value={value as any[][]} onChange={update} />
      </TextEditorOr>
    </EditorContent>
  );
}
