import { WidgetProps } from '@rjsf/utils';
import { Input, AutoComplete, Flex } from 'antd';
import { JSX, useCallback, useEffect, useMemo } from 'react';
import { Space } from 'antd/lib';
import { MercPreview } from '../../content/MercPreview';
import { ItemPreview } from '../../content/ItemPreview';
import { useAnyFileLoading } from '../../../hooks/useAnyFileLoading';
import { useAnyFileLoadingError } from '../../../hooks/useAnyFileLoadingError';
import { useFilesJsonDiskValue } from '../../../hooks/useFilesJsonDiskValue';
import { isArray, uniqueBy } from 'remeda';
import { AnyJsonObject } from '../../../../common/invokables/jsons';
import { Loader } from '../../common/Loader';
import { ExclamationCircleOutlined } from '@ant-design/icons';
import { useFileLoad } from '../../../hooks/useFileLoad';

type PreviewFn = (item: AnyJsonObject) => JSX.Element | string | null;

interface StringReferenceWidgetProps extends WidgetProps {
  references: Array<{ file: string; property: string; preview?: PreviewFn }>;
}

export function StringReferenceWidget({
  value,
  onChange,
  required,
  references,
}: StringReferenceWidgetProps) {
  const files = useMemo(() => references.map((r) => r.file), [references]);
  const loading = useAnyFileLoading(files);
  const error = useAnyFileLoadingError(files);
  const values = useFilesJsonDiskValue(files);
  const loadFile = useFileLoad();
  const options = useMemo(() => {
    const options = values.flatMap((value, index) => {
      if (!isArray(value)) return [];
      return value.flatMap((o) => {
        const reference = references[index];
        if (isArray(o) || !reference) return [];
        const value = (o[reference.property] as any) ?? '';
        if (typeof value !== 'string') return [];
        let label: JSX.Element | string | null = value;
        if (reference.preview) {
          label = (
            <Space>
              {reference.preview(o)}
              <span>{value}</span>
            </Space>
          );
        }

        return {
          value,
          label,
        };
      });
    });
    options.sort((a, b) => a.value.localeCompare(b.value));
    return uniqueBy(options, (option) => option.value);
  }, [values, references]);
  const onChangeMemo = useCallback(
    (value: string) => {
      onChange(value);
    },
    [onChange],
  );

  useEffect(() => {
    files.forEach((file, idx) => {
      if (!values[idx] && !error) {
        loadFile(file);
      }
    });
  }, [error, files, loadFile, values]);

  if (loading) {
    return (
      <Flex>
        <Loader />
      </Flex>
    );
  }
  if (error) {
    return (
      <Flex gap="small">
        <Input value={value} onChange={onChange} required={required} />;
        <ExclamationCircleOutlined title={error.message} />
      </Flex>
    );
  }

  return (
    <AutoComplete
      options={options}
      value={value}
      onChange={onChangeMemo}
      placeholder="Please select..."
      filterOption={(inputValue, option) =>
        option!.value.toUpperCase().indexOf(inputValue.toUpperCase()) !== -1
      }
    />
  );
}

export function stringReferenceTo(
  file: string,
  property: string,
  preview?: PreviewFn,
) {
  return function StringReference(props: WidgetProps) {
    return (
      <StringReferenceWidget
        references={[
          {
            property,
            file,
            preview,
          },
        ]}
        {...props}
      />
    );
  };
}

export function stringReferenceToMultiple(references: {
  [k: string]: { file: string; property: string; preview?: PreviewFn };
}) {
  const referencesArr = Object.values(references);
  return function StringReference(props: WidgetProps) {
    return <StringReferenceWidget references={referencesArr} {...props} />;
  };
}

export const stringReferenceToArmyCompositions = stringReferenceTo(
  'army-compositions.json',
  'name',
);

export const stringReferenceToAmmoTypes = stringReferenceTo(
  'ammo-types.json',
  'internalName',
);

export const stringReferenceToCalibres = stringReferenceTo(
  'calibres.json',
  'internalName',
);

// FIXME also needs to include weapons etc.
export const stringReferenceToItems = stringReferenceToMultiple({
  armours: {
    file: 'armours.json',
    property: 'internalName',
    preview: (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
  },
  explosives: {
    file: 'explosives.json',
    property: 'internalName',
    preview: (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
  },
  items: {
    file: 'items.json',
    property: 'internalName',
    preview: (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
  },
  magazines: {
    file: 'magazines.json',
    property: 'internalName',
    preview: (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
  },
  weapons: {
    file: 'weapons.json',
    property: 'internalName',
    preview: (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
  },
});

export const stringReferenceToWeapons = stringReferenceTo(
  'weapons.json',
  'internalName',
  (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
);

export const stringReferenceToArmours = stringReferenceTo(
  'armours.json',
  'internalName',
  (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
);

export const stringReferenceToMagazines = stringReferenceTo(
  'magazines.json',
  'internalName',
  (i) => <ItemPreview inventoryGraphics={i.inventoryGraphics} />,
);

export const stringReferenceToMercProfiles = stringReferenceTo(
  'mercs-profile-info.json',
  'internalName',
  (i) => <MercPreview profile={i.internalName} />,
);

export const stringReferenceToExplosiveCalibres = stringReferenceTo(
  'explosive-calibres.json',
  'internalName',
);

export const stringReferenceToExplosionAnimations = stringReferenceTo(
  'explosion-animations.json',
  'name',
);

export const stringReferenceToLoadingScreens = stringReferenceTo(
  'loading-screens.json',
  'internalName',
);

export const stringReferenceToTowns = stringReferenceTo(
  'strategic-map-towns.json',
  'internalName',
);
