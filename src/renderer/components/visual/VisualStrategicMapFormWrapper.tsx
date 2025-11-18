import { Space } from 'antd';
import { VisualFormProps, VisualFormWrapper } from './VisualFormWrapper';
import { StrategicMap, StrategicMapProps } from '../content/StrategicMap';
import { JsonFormHeader } from './form/JsonFormHeader';
import { PropsWithChildren } from 'react';

export interface VisualSrategicMapFormWrapperProps
  extends PropsWithChildren<VisualFormProps> {
  strategicMap: StrategicMapProps;
}

export function VisualStrategicMapFormWrapper({
  file,
  extraFiles,
  strategicMap,
  children,
}: VisualSrategicMapFormWrapperProps) {
  return (
    <VisualFormWrapper file={file} extraFiles={extraFiles}>
      <Space direction="horizontal" align="start" size="large">
        <StrategicMap {...strategicMap} />
        <div>
          <JsonFormHeader file={file} />
          {children}
        </div>
      </Space>
    </VisualFormWrapper>
  );
}
