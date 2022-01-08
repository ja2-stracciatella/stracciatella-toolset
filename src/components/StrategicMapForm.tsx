import ReactMarkdown from "react-markdown";
import { useCallback, useMemo, useState } from "react";
import { Alert, Typography, Space } from "antd";

import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { FullSizeLoader } from "./FullSizeLoader";
import { StrategicMap } from "./content/StrategicMap";
import { JsonSchemaForm } from "./JsonSchemaForm";
import { UiSchema } from "@rjsf/core";

export interface StrategicMapFormProps {
  file: string;
  property?: string;
  uiSchema?: UiSchema;
}

interface ReadySchema {
  title: string;
  description: string;
  schema: any;
  content: any;
  sectorsWithContent: string[];
}

export function JsonStrategicMapForm({
  file,
  property = "sector",
  uiSchema,
}: StrategicMapFormProps) {
  const { data, error } = useJsonWithSchema(file);
  const { schema, content, sectorsWithContent, title, description } =
    useMemo((): ReadySchema => {
      if (!data) {
        return {
          schema: null,
          content: null,
          title: "",
          description: "",
          sectorsWithContent: [],
        };
      }
      return {
        title: data.schema.items.title ?? file,
        description: data.schema.items.description,
        schema: data.schema.items,
        content: data.content,
        sectorsWithContent: data.content.map((d: any) =>
          d[property].toLowerCase()
        ),
      };
    }, [data, file, property]);
  const [selectedSector, setSelectedSector] = useState<string | null>(null);
  const onSectorClick = useCallback(
    (sectorId: string) => {
      const c = content.find(
        (i: any) => i[property].toLowerCase() === sectorId
      );
      setSelectedSector(c ? sectorId : null);
    },
    [content, property]
  );
  const c = useMemo(() => {
    if (!selectedSector) {
      return <div>Select a sector to the left to edit.</div>;
    }
    const c = content.find(
      (i: any) => i[property].toLowerCase() === selectedSector
    );
    return <JsonSchemaForm schema={schema} content={c} uiSchema={uiSchema} />;
  }, [content, property, schema, selectedSector, uiSchema]);

  if (error) {
    return <Alert type="error" message={error.toString()} />;
  }
  if (!schema || !content) {
    return <FullSizeLoader />;
  }

  return (
    <div>
      <Space direction="horizontal" align="start" size="large">
        <StrategicMap
          highlightedSectorIds={sectorsWithContent}
          onSectorClick={onSectorClick}
        />
        <div>
          <Typography.Title level={2}>{title}</Typography.Title>
          <div>
            <ReactMarkdown>{description}</ReactMarkdown>
          </div>
          {c}
        </div>
      </Space>
    </div>
  );
}
