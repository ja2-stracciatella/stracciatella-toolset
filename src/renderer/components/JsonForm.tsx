import ReactMarkdown from 'react-markdown';
import { useCallback, useMemo } from 'react';
import { Alert, Typography } from 'antd';

import { IChangeEvent, UiSchema } from '@rjsf/core';
import { z } from 'zod';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeLoader } from './FullSizeLoader';
import { EditorContent } from './EditorContent';
import { useModifyableJsonWithSchema } from '../state/files';

export interface JsonFormProps {
  file: string;
  uiSchema?: UiSchema;
}

interface SchemaWithDescription {
  title: string;
  description: string;
  schema: any;
  content: any;
}

const jsonFormSchemaSchema = z.any();

const jsonFormSchemaContent = z.any();

export function JsonForm({ file, uiSchema }: JsonFormProps) {
  const {
    content: origContent,
    schema: origSchema,
    error,
    fileChanged,
  } = useModifyableJsonWithSchema(
    jsonFormSchemaSchema,
    jsonFormSchemaContent,
    file
  );
  const { schema, content, title, description } =
    useMemo((): SchemaWithDescription => {
      if (!origContent) {
        return { schema: null, content: null, title: '', description: '' };
      }
      return {
        title: origSchema?.title ?? file,
        description: origSchema?.description,
        schema: {
          ...origSchema,
          title: undefined,
          description: undefined,
        },
        content: origContent,
      };
    }, [file, origContent, origSchema]);
  const onFormChange = useCallback(
    (data: IChangeEvent<any>) => fileChanged(data.formData),
    [fileChanged]
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!schema || !content) {
    return <FullSizeLoader />;
  }

  return (
    <EditorContent path={file}>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <JsonSchemaForm
        schema={schema}
        content={content}
        uiSchema={uiSchema}
        onChange={onFormChange}
      />
    </EditorContent>
  );
}
