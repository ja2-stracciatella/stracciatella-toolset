import ReactMarkdown from "react-markdown";
import { useMemo } from "react";
import { Alert, Collapse, Space, Typography } from "antd";

import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { JsonSchemaForm } from "./JsonSchemaForm";
import { FullSizeLoader } from "./FullSizeLoader";
import "./JsonItemsForm.css";
import { UiSchema } from "@rjsf/core";

const { Panel } = Collapse;

export interface JsonItemsFormProps {
  file: string;
  name: string | ((item: any) => string);
  preview?: (item: any) => JSX.Element;
  uiSchema?: UiSchema;
}

export function JsonItemsForm({
  file,
  name,
  preview,
  uiSchema,
}: JsonItemsFormProps) {
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
        const label = typeof name === "string" ? item[name] : name(item);
        const p = preview ? preview(item) : null;

        const header = (
          <Space direction="horizontal">
            {p}
            {label}
          </Space>
        );

        return (
          <Panel key={index} header={header}>
            <div className="json-items-form-form">
              <JsonSchemaForm
                idPrefix={index.toString()}
                schema={itemsSchema}
                content={item}
                uiSchema={uiSchema}
              />
            </div>
          </Panel>
        );
      });
    }
    return null;
  }, [data, itemsSchema, name, preview, uiSchema]);

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
      <Collapse bordered={false}>{items}</Collapse>
    </div>
  );
}
