import { WidgetProps } from '@rjsf/utils';
import { Input, AutoComplete } from 'antd';
import { useCallback, useMemo } from 'react';
import { useJson } from '../../hooks/files';
import { BaseOptionType } from 'antd/lib/select';

interface StringReferenceWidgetProps extends WidgetProps {
  referenceFile: string;
  referenceProperty: string;
}

export function StringReferenceWidget({
  value,
  onChange,
  required,
  referenceFile,
  referenceProperty,
}: StringReferenceWidgetProps) {
  const { content, error } = useJson(referenceFile);
  const options = useMemo(() => {
    if (!content) {
      return [];
    }
    return content.value.map((d: any) => ({ value: d[referenceProperty] }));
  }, [content, referenceProperty]);
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
        referenceFile={file}
        referenceProperty={property}
        {...props}
      />
    );
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
export const stringReferenceToItems = stringReferenceTo(
  'items.json',
  'internalName',
);

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
