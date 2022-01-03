import ReactMarkdown from "react-markdown";
import { useMemo } from "react";
import { Alert, Collapse, Typography } from "antd";

import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { JsonSchemaForm } from "./JsonSchemaForm";
import { FullSizeLoader } from "./FullSizeLoader";
import "./JsonItemsForm.css";

const { Panel } = Collapse;

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
      const description = [
        data.schema.items.description,
        data.schema.description,
      ]
        .filter((v) => !!v)
        .join("\n\n");
      return description;
    }
    return "";
  }, [data]);
  const items = useMemo(() => {
    if (data) {
      return data.content.map((item: any, index: number) => {
        const header = typeof name === "string" ? item[name] : name(item);
        return (
          <Panel key={index} header={header}>
            <div className="json-items-form-form">
              <JsonSchemaForm
                idPrefix={index.toString()}
                schema={itemsSchema}
                content={item}
              />
            </div>
          </Panel>
        );
      });
    }
    return null;
  }, [data, itemsSchema, name]);

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }

  if (!itemsSchema) {
    return <FullSizeLoader />;
  }

  return (
    <div>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <Collapse bordered={false}>
        {items}
      </Collapse>
    </div>
  );
}
