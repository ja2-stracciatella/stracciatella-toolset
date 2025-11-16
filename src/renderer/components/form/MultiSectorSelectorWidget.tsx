import { WidgetProps } from '@rjsf/utils';
import {
  DEFAULT_HIGHLIGHT_COLOR,
  NormalizedSectorId,
  StrategicMap,
} from '../content/StrategicMap';
import { useCallback, useMemo, useState } from 'react';
import { Flex } from 'antd';
import { find, isDeepEqual } from 'remeda';

interface ExtraProps<T> {
  initialLevel?: number;
  canChangeLevel?: boolean;
  extractSectorFromItem?: (value: T) => NormalizedSectorId;
  transformSectorToItem?: (sectorId: NormalizedSectorId) => T;
}

type MultiSectorSelectorWidgetProps<T> = WidgetProps<T[]> & ExtraProps<T>;

function MultiSectorSelectorWidget<T>({
  initialLevel = 0,
  canChangeLevel = false,
  extractSectorFromItem,
  transformSectorToItem,
  value,
  onChange,
}: MultiSectorSelectorWidgetProps<T>) {
  const [level, setLevel] = useState(initialLevel);
  const highlightedSectorIds = useMemo(
    () => ({
      [DEFAULT_HIGHLIGHT_COLOR]: value.map(
        extractSectorFromItem ?? ((s: T) => s),
      ),
    }),
    [value, extractSectorFromItem],
  );
  const handleSectorClick = useCallback(
    (sectorId: NormalizedSectorId) => {
      const transformed = transformSectorToItem
        ? transformSectorToItem(sectorId)
        : sectorId;
      const newValue = find(value, (s) => isDeepEqual(s, transformed))
        ? value.filter((s: T) => !isDeepEqual(s, transformed))
        : [...value, transformed];
      onChange(newValue);
    },
    [transformSectorToItem, value, onChange],
  );

  return (
    <Flex>
      <StrategicMap
        level={level}
        highlightedSectorIds={highlightedSectorIds}
        onSectorClick={handleSectorClick}
        onLevelChange={canChangeLevel ? setLevel : undefined}
      />
    </Flex>
  );
}

export const makeMultiSectorSelectorWidget = <T,>(
  extraProps: ExtraProps<T>,
) => {
  return function MultiSectorSelectorWidgetWrapper(
    props: MultiSectorSelectorWidgetProps<T>,
  ) {
    return <MultiSectorSelectorWidget {...extraProps} {...props} />;
  };
};
