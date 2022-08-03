import ReactMarkdown from "react-markdown";
import { useCallback, useMemo, useState } from "react";
import { Alert, Collapse, Space, Typography } from "antd";

import { JsonSchemaForm } from "./JsonSchemaForm";
import { FullSizeLoader } from "./FullSizeLoader";
import "./JsonItemsForm.css";
import { IChangeEvent, UiSchema } from "@rjsf/core";
import { useModifyableJsonWithSchema } from "../state/files";
import { EditorContent } from "./EditorContent";

interface JsonItemFormProps {
  name: string | ((item: any) => string);
  preview?: (item: any) => JSX.Element;
  schema: any;
  uiSchema?: UiSchema;
  index: number;
  item: any;
  onItemChange: (index: number, ev: IChangeEvent<any>) => unknown;
}

function JsonItemForm({
  name,
  preview,
  schema,
  uiSchema,
  index,
  item,
  onItemChange,
}: JsonItemFormProps) {
  const [isCollapsed, setCollapsed] = useState(true);
  const label = useMemo(
    () => (typeof name === "string" ? item[name] : name(item)),
    [item, name]
  );
  const p = useMemo(() => (preview ? preview(item) : null), [item, preview]);
  const header = useMemo(
    () => (
      <Space direction="horizontal">
        {p}
        {label}
      </Space>
    ),
    [label, p]
  );
  const onPanelChange = useCallback((v) => {
    setCollapsed(v.length === 0);
  }, []);

  return (
    <Collapse onChange={onPanelChange}>
      <Collapse.Panel key={index} header={header}>
        <div className="json-items-form-form">
          {isCollapsed ? null : (
            <JsonSchemaForm
              idPrefix={index.toString()}
              schema={schema}
              content={item}
              uiSchema={uiSchema}
              onChange={(v) => onItemChange(index, v)}
            />
          )}
        </div>
      </Collapse.Panel>
    </Collapse>
  );
}

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
  const { content, schema, error, fileChanged } =
    useModifyableJsonWithSchema(file);
  const itemsSchema = useMemo(() => {
    if (schema) {
      return {
        ...schema.items,
        // Title and description are not necessary to render within an item
        title: undefined,
        description: undefined,
      };
    }
    return null;
  }, [schema]);
  const title = useMemo(() => {
    if (!schema) {
      return "";
    }
    return schema.title ?? file;
  }, [schema, file]);
  const description = useMemo(() => {
    if (!schema) {
      return "";
    }
    const description = [schema.items.description, schema.description]
      .filter((v) => !!v)
      .join("\n\n");
    return description;
  }, [schema]);
  const onItemChange = useCallback(
    (index: number, ev: IChangeEvent<any>) => {
      if (!content) {
        return;
      }
      const newData = content.map((v: any, idx: number) => {
        if (idx === index) {
          return ev.formData;
        }
        return v;
      });
      fileChanged(newData);
    },
    [content, fileChanged]
  );

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }

  if (!itemsSchema) {
    return <FullSizeLoader />;
  }

  return (
    <EditorContent path={file}>
      <Typography.Title level={2}>{title}</Typography.Title>
      <div>
        <ReactMarkdown>{description}</ReactMarkdown>
      </div>
      <Space direction="vertical" style={{ width: "100%" }}>
        {content.map((item: any, index: number) => {
          return (
            <JsonItemForm
              key={index}
              name={name}
              schema={itemsSchema}
              index={index}
              item={item}
              onItemChange={onItemChange}
              preview={preview}
              uiSchema={uiSchema}
            />
          );
        }) ?? null}
      </Space>
    </EditorContent>
  );
}
