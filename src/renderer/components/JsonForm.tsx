import { useCallback, useMemo } from 'react';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import { useFileLoading, useFileSchema } from '../hooks/files';
import { useFileError } from '../hooks/files';
import { useFileJson } from '../hooks/files';
import { ErrorAlert } from './ErrorAlert';
import { miniSerializeError } from '@reduxjs/toolkit';

export interface JsonFormProps {
  file: string;
  uiSchema?: UiSchema;
}

export function JsonForm({ file, uiSchema }: JsonFormProps) {
  const loading = useFileLoading(file);
  const error = useFileError(file);
  const [value, update] = useFileJson(file);
  const baseSchema = useFileSchema(file);
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

  if (error) {
    return <ErrorAlert error={error} />;
  }
  if (loading == null || loading) {
    return <FullSizeLoader />;
  }
  if (!schema || !value) {
    return (
      <ErrorAlert error={miniSerializeError(new Error('No schema or value'))} />
    );
  }

  return (
    <EditorContent path={file}>
      <JsonFormHeader file={file} />
      <JsonSchemaForm
        schema={schema}
        content={value}
        uiSchema={uiSchema}
        onChange={onFormChange}
      />
    </EditorContent>
  );
}
