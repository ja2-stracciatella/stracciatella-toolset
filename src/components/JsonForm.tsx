import ReactMarkdown from "react-markdown";
import { useMemo } from "react";
import { Alert } from "react-bootstrap";

import { JsonSchemaForm } from "./JsonSchemaForm";
import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { FullSizeLoader } from "./FullSizeLoader";

export interface JsonFormProps {
  file: string;
}

interface SchemaWithDescription {
  title: string;
  description: string;
  schema: any;
  content: any;
}

export function JsonForm({ file }: JsonFormProps) {
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
    return <Alert variant="danger">{error.toString()}</Alert>;
  }
  if (!schema || !content) {
    return <FullSizeLoader />;
  }

  return (
    <div>
      <h1>{title}</h1>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <JsonSchemaForm schema={schema} content={content} />
    </div>
  );
}
