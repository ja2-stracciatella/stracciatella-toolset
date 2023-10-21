import { useCallback, useMemo } from 'react';
import { Alert } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import { EditorContent } from './EditorContent';
import { JsonFormHeader } from './form/JsonFormHeader';
import { useJson } from '../hooks/files';

export interface JsonFormProps {
  file: string;
  uiSchema?: UiSchema;
}

export function JsonForm({ file, uiSchema }: JsonFormProps) {
  const { content, update, error } = useJson(file);
  const schema = useMemo(() => {
    if (!content?.schema) {
      return null;
    }
    return {
      ...content.schema,
      title: undefined,
      description: undefined,
    };
  }, [content?.schema]);
  const onFormChange = useCallback(
    (value: IChangeEvent<any>) => update(value.formData),
    [update],
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!schema || !content) {
    return <FullSizeLoader />;
  }

  return (
    <EditorContent path={file}>
      <JsonFormHeader file={file} />
      <JsonSchemaForm
        schema={schema}
        content={content.value}
        uiSchema={uiSchema}
        onChange={onFormChange}
      />
    </EditorContent>
  );
}
