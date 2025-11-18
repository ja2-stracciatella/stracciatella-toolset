import { useCallback, useMemo, useState } from 'react';
import {
  coordsFromSectorIdString,
  NormalizedSectorId,
} from '../content/StrategicMap';
import { Typography } from 'antd';
import { JsonSchema } from 'src/common/invokables/jsons';
import { JsonSchemaForm } from './form/JsonSchemaForm';
import { UiSchema } from '@rjsf/utils';
import { IChangeEvent } from '@rjsf/core';
import { clone } from 'remeda';
import { VisualFormProps } from './VisualFormWrapper';
import { VisualStrategicMapFormWrapper } from './VisualStrategicMapFormWrapper';
import { useFileJsonValue } from '../../hooks/useFileJsonValue';
import { useFileJsonUpdate } from '../../hooks/useFileJsonUpdate';

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

function Form({
  file,
  selectedSectorId,
}: VisualFormProps & {
  selectedSectorId: NormalizedSectorId | null;
}) {
  const values = useFileJsonValue(file);
  const update = useFileJsonUpdate(file);
  const coords = useMemo(
    () =>
      selectedSectorId ? coordsFromSectorIdString(selectedSectorId[0]) : null,
    [selectedSectorId],
  );
  const content = useMemo(() => {
    if (!coords || !values) {
      return {};
    }
    const v = values as any;
    return {
      traverseThrough: v.traverseThrough[coords[1]][coords[0]],
      traverseNorth: v.traverseNS[coords[1]][coords[0]],
      traverseSouth: v.traverseNS[coords[1] + 1][coords[0]],
      traverseWest: v.traverseWE[coords[1]][coords[0]],
      traverseEast: v.traverseWE[coords[1]][coords[0] + 1],
      travelRating: v.travelRatings[coords[1]][coords[0]],
    };
  }, [values, coords]);
  const handleChange = useCallback(
    (newContent: IChangeEvent<any>) => {
      if (!coords || !values) {
        return;
      }
      const n = clone(values) as any;
      n.traverseThrough[coords[1]][coords[0]] =
        newContent.formData.traverseThrough;
      n.traverseNS[coords[1]][coords[0]] = newContent.formData.traverseNorth;
      n.traverseNS[coords[1] + 1][coords[0]] =
        newContent.formData.traverseSouth;
      n.traverseWE[coords[1]][coords[0]] = newContent.formData.traverseWest;
      n.traverseWE[coords[1]][coords[0] + 1] = newContent.formData.traverseEast;
      n.travelRatings[coords[1]][coords[0]] = newContent.formData.travelRating;
      update(n);
    },
    [coords, update, values],
  );

  if (!selectedSectorId) {
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
}

export function MovementCostsForm({ file, extraFiles }: VisualFormProps) {
  const [selectedSectorId, setSelectedSectorId] =
    useState<NormalizedSectorId | null>(null);

  return (
    <VisualStrategicMapFormWrapper
      strategicMap={{ selectedSectorId, onSectorClick: setSelectedSectorId }}
      file={file}
      extraFiles={extraFiles}
    >
      <Form file={file} selectedSectorId={selectedSectorId} />
    </VisualStrategicMapFormWrapper>
  );
}
