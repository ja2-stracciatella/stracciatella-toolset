import { useCallback, useMemo } from 'react';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { JsonSchemaForm } from './form/JsonSchemaForm';
import { VisualFormProps, VisualFormWrapper } from './VisualFormWrapper';
import { VisualFormWithHeader } from './VisualFormWithHeader';
import { useFileJsonValue } from '../../hooks/useFileJsonValue';
import { useFileJsonUpdate } from '../../hooks/useFileJsonUpdate';
import { useFileJsonSchema } from '../../hooks/useFileJsonSchema';

export interface JsonFormProps extends VisualFormProps {
  uiSchema?: UiSchema;
}

function Form({ file, uiSchema }: Omit<JsonFormProps, 'extraFiles'>) {
  const value = useFileJsonValue(file);
  const update = useFileJsonUpdate(file);
  const baseSchema = useFileJsonSchema(file);
  const schema = useMemo(() => {
    if (!baseSchema) {
      return null;
    }
    return {
      ...baseSchema,
      title: undefined,
      description: undefined,
    };
  }, [baseSchema]);
  const onFormChange = useCallback(
    (value: IChangeEvent<any>) => update(value.formData),
    [update],
  );

  if (!schema || !value) {
    return null;
  }
  return (
    <VisualFormWithHeader file={file}>
      <JsonSchemaForm
        schema={schema}
        content={value}
        uiSchema={uiSchema}
        onChange={onFormChange}
      />
    </VisualFormWithHeader>
  );
}

export function JsonForm({ file, extraFiles, ...rest }: JsonFormProps) {
  return (
    <VisualFormWrapper file={file} extraFiles={extraFiles}>
      <Form file={file} {...rest} />
    </VisualFormWrapper>
  );
}
