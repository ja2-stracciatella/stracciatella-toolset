import { FieldProps } from '@rjsf/utils';
import { ResourceReferenceWidget } from './ResourceReferenceWidget';
import { ResourceType } from '../../../lib/resourceType';
import { useCallback, useEffect, useMemo } from 'react';
import { Select, Space } from 'antd';
import { useImageMetadata } from '../../../hooks/useImageMetadata';
import { StiPreview } from '../../content/StiPreview';

function SubImageSelector({
  file,
  subimage,
  onChange,
}: {
  file: string;
  subimage?: number;
  onChange: (value: number) => unknown;
}) {
  const { data, refresh } = useImageMetadata(file);
  const options = useMemo(() => {
    if (!data) return [];
    return data.images.map((_, index) => ({
      label: (
        <Space>
          <StiPreview file={file} subimage={index} />
          {index}
        </Space>
      ),
      value: index,
    }));
  }, [data, file]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (!data) return null;

  return (
    <Select
      options={options}
      value={subimage}
      onChange={onChange}
      style={{ width: '150px' }}
    />
  );
}

export function InventoryGraphicsField({
  name,
  fieldPathId,
  formData,
  onChange,
  schema,
  registry,
  onBlur,
  onFocus,
}: FieldProps) {
  const path: string = formData?.path ?? '';
  const subImageIndex = formData?.subImageIndex ?? 0;
  const handleResourceChange = useCallback(
    (resource: string) => {
      onChange({ path: resource, subImageIndex: 0 }, fieldPathId.path);
    },
    [fieldPathId, onChange],
  );
  const handleSubitemChange = useCallback(
    (index: number) => {
      const subImageIndex = index !== 0 ? index : undefined;
      onChange({ path, subImageIndex }, fieldPathId.path);
    },
    [fieldPathId.path, onChange, path],
  );
  const selector = useMemo(() => {
    return (
      <SubImageSelector
        file={path}
        subimage={subImageIndex}
        onChange={handleSubitemChange}
      />
    );
  }, [handleSubitemChange, path, subImageIndex]);

  return (
    <ResourceReferenceWidget
      value={path}
      onChange={handleResourceChange}
      preview={selector}
      resourceType={ResourceType.Graphics}
      pathPrefix={[]}
      postProcess={(path) => path}
      id={fieldPathId.$id}
      name={name}
      label="Graphics"
      onBlur={onBlur}
      onFocus={onFocus}
      options={{}}
      schema={schema}
      registry={registry}
    />
  );
}
