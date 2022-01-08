import ReactMarkdown from "react-markdown";
import { useMemo } from "react";
import { Alert, Typography } from "antd";

import { JsonSchemaForm } from "./JsonSchemaForm";
import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { FullSizeLoader } from "./FullSizeLoader";
import { UiSchema } from "@rjsf/core";

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

export function JsonForm({ file, uiSchema }: JsonFormProps) {
  const { data, error } = useJsonWithSchema(file);
  const { schema, content, title, description } =
    useMemo((): SchemaWithDescription => {
      if (!data) {
        return { schema: null, content: null, title: "", description: "" };
      }
      return {
        title: data.schema.title ?? file,
        description: data.schema.description,
        schema: {
          ...data.schema,
          title: undefined,
          description: undefined,
        },
        content: data.content,
      };
    }, [data, file]);
  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!schema || !content) {
    return <FullSizeLoader />;
  }

  return (
    <div>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <JsonSchemaForm schema={schema} content={content} uiSchema={uiSchema} />
    </div>
  );
}
