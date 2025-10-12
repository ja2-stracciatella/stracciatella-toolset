import { WidgetProps } from '@rjsf/utils';
import { Input, AutoComplete, AutoCompleteProps } from 'antd';
import { JSX, useCallback, useMemo } from 'react';
import { useJsons } from '../../hooks/files';
import { BaseOptionType } from 'antd/lib/select';
import { Space } from 'antd/lib';
import { MercPreview } from '../content/MercPreview';
import { ItemPreview } from '../content/ItemPreview';

type PreviewFn = (item: any) => JSX.Element | string | null;

interface StringReferenceWidgetProps<T extends { [key: string]: string }>
  extends WidgetProps {
  references: {
    [key in keyof T]: { file: string; property: string; preview?: PreviewFn };
  };
}

export function StringReferenceWidget<T extends { [key: string]: string }>({
  value,
  onChange,
  required,
  references,
}: StringReferenceWidgetProps<T>) {
  const files = useMemo(() => {
    const f: { [key in keyof T]: string } = {} as any;
    for (const key in references) {
      f[key] = references[key].file;
    }
    return f;
  }, [references]);
  const { loading, error, results } = useJsons(files);
  const options = useMemo(() => {
    if (loading || error) {
      return [];
    }
    const options: Array<
      NonNullable<AutoCompleteProps['options']>[0] & { value: string }
    > = [];
    for (const key in results) {
      const fileResults = results[key]?.content ?? null;
      if (!fileResults) continue;
      for (const item of fileResults.value) {
        const value: string = item[references[key].property];
        let label: JSX.Element | string | null = value;
        if (references[key].preview) {
          label = (
            <Space>
              {references[key].preview(item)}
              <span>{value}</span>
            </Space>
          );
        }
        if (!options.some((option) => option.value === value)) {
          options.push({
            value,
            label,
          });
        }
      }
    }
    options.sort((a, b) => a.value.localeCompare(b.value));
    return options;
  }, [references, loading, error, results]);
  const onChangeMemo = useCallback(
    (value: BaseOptionType) => {
      onChange(value);
    },
    [onChange],
  );

  if (error) {
    return <Input value={value} onChange={onChange} required={required} />;
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
        references={{
          f: {
            property,
            file,
            preview,
          },
        }}
        {...props}
      />
    );
  };
}

export function stringReferenceToMultiple(references: {
  [k: string]: { file: string; property: string; preview?: PreviewFn };
}) {
  return function StringReference(props: WidgetProps) {
    return <StringReferenceWidget references={references} {...props} />;
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
