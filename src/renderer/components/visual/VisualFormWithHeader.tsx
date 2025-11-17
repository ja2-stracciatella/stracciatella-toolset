import { PropsWithChildren } from 'react';
import { VisualFormProps } from './VisualFormWrapper';
import { JsonFormHeader } from './form/JsonFormHeader';

export function VisualFormWithHeader({
  file,
  children,
}: PropsWithChildren<VisualFormProps>) {
  return (
    <>
      <JsonFormHeader file={file} />
      {children}
    </>
  );
}
