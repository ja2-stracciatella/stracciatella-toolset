import { Alert, Typography } from "antd";
import ReactMarkdown from "react-markdown";
import { useMemo } from "react";

import { JsonSchemaForm } from "./JsonSchemaForm";
import { useJsonWithSchema } from "../hooks/useJsonWithSchema";

export interface JsonFormProps {
  file: string;
}

interface SchemaWithDescription {
  title: string,
  description: string,
  schema: any,
  content: any,
}

export function JsonForm({ file }: JsonFormProps) {
  const { data, error } = useJsonWithSchema(file);
  const { schema, content, title, description } = useMemo((): SchemaWithDescription => {
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
      content: data.content
    }
  }, [data, file])
  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!schema || !content) {
    return null;
  }

  return (
    <div>
      <Typography.Title>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <JsonSchemaForm schema={schema} content={content} />
    </div>
  );
}
