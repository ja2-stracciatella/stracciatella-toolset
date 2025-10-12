import { WidgetProps } from '@rjsf/utils';
import { Input, AutoComplete } from 'antd';
import { useCallback, useMemo } from 'react';
import { useJsons } from '../../hooks/files';
import { BaseOptionType } from 'antd/lib/select';

interface StringReferenceWidgetProps<T extends { [key: string]: string }>
  extends WidgetProps {
  references: { [key in keyof T]: { file: string; property: string } };
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
    const options: string[] = [];
    for (const key in results) {
      const fileResults = results[key]?.content ?? null;
      if (!fileResults) continue;
      for (const item of fileResults.value) {
        const value: string = item[references[key].property];
        if (!options.includes(value)) {
          options.push(value);
        }
      }
    }
    options.sort();
    return options.map((option) => ({ value: option }));
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
    />
  );
}

export function stringReferenceTo(file: string, property: string) {
  return function StringReference(props: WidgetProps) {
    return (
      <StringReferenceWidget
        references={{
          f: {
            property,
            file,
          },
        }}
        {...props}
      />
    );
  };
}

export function stringReferenceToMultiple(references: {
  [k: string]: { file: string; property: string };
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
  },
  explosives: {
    file: 'explosives.json',
    property: 'internalName',
  },
  items: {
    file: 'items.json',
    property: 'internalName',
  },
  magazines: {
    file: 'magazines.json',
    property: 'internalName',
  },
  weapons: {
    file: 'weapons.json',
    property: 'internalName',
  },
});

export const stringReferenceToWeapons = stringReferenceTo(
  'weapons.json',
  'internalName',
);

export const stringReferenceToArmours = stringReferenceTo(
  'armours.json',
  'internalName',
);

export const stringReferenceToMagazines = stringReferenceTo(
  'magazines.json',
  'internalName',
);

export const stringReferenceToMercProfiles = stringReferenceTo(
  'mercs-profile-info.json',
  'internalName',
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
