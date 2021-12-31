import { Alert, Collapse, Typography } from "antd";
import ReactMarkdown from "react-markdown";
import { useMemo } from "react";

import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { JsonSchemaForm } from "./JsonSchemaForm";

export interface JsonItemsFormProps {
  file: string;
  name: string | ((item: any) => string);
}

export function JsonItemsForm({ file, name }: JsonItemsFormProps) {
  const { data, error } = useJsonWithSchema(file);
  const itemsSchema = useMemo(() => {
    if (data) {
      return {
        ...data.schema.items,
        // Title and description are not necessary to render within an item
        title: undefined,
        description: undefined,
      };
    }
    return null;
  }, [data]);
  const title = useMemo(() => {
    if (data) {
      return data.schema.title ?? file;
    }
    return "";
  }, [data, file]);
  const description = useMemo(() => {
    if (data) {
      const description = [data.schema.description, data.schema.items.description].filter(v => !!v).join("\n\n");
      return description;
    }
    return "";
  }, [data]);
  const items = useMemo(() => {
    if (data) {
      return data.content.map((item: any, index: number) => {
        const header = typeof name === "string" ? item[name] : name(item);
        return (
          <Collapse.Panel key={index} header={header}>
            <JsonSchemaForm schema={itemsSchema} content={item} />
          </Collapse.Panel>
        );
      });
    }
    return null;
  }, [data, itemsSchema, name]);

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }

  if (!itemsSchema) {
    return null;
  }

  return (
    <div>
      <Typography.Title>{title}</Typography.Title>
      <div><ReactMarkdown>{description}</ReactMarkdown></div>
      <Collapse accordion>{items}</Collapse>
    </div>
  );
}
