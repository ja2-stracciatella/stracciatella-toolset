import ReactMarkdown from "react-markdown";
import { useMemo, useState } from "react";
import { Alert, Typography, Row, Col, Divider } from "antd";

import { useJsonWithSchema } from "../hooks/useJsonWithSchema";
import { FullSizeLoader } from "./FullSizeLoader";
import { StrategicMap } from "./StrategicMap";
import { JsonSchemaForm } from "./JsonSchemaForm";

export interface StrategicMapFormProps {
  file: string;
  property?: string;
}

interface ReadySchema {
  title: string;
  description: string;
  schema: any;
  content: any;
  sectorsWithContent: string[];
}

export function JsonStrategicMapForm({ file, property = "sector" }: StrategicMapFormProps) {
  const { data, error } = useJsonWithSchema(file);
  const { schema, content, sectorsWithContent, title, description } =
    useMemo((): ReadySchema => {
      if (!data) {
        return { schema: null, content: null, title: "", description: "", sectorsWithContent: [] };
      }
      return {
        title: data.schema.title ?? file,
        description: data.schema.description,
        schema: data.schema.items,
        content: data.content,
        sectorsWithContent: data.content.map((d: any) => d[property].toLowerCase())
      };
    }, [data, file, property]);
  const [selectedSector, setSelectedSector] = useState<string | null>(null);
  const c = useMemo(() => {
      if (!selectedSector) {
          return <div>Select a sector to the left to edit.</div>
      }
      const c = content.find((i: any) => i[property].toLowerCase() === selectedSector);
      return <JsonSchemaForm schema={schema} content={c} />;
  }, [content, property, schema, selectedSector])

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
      <Row>
          <Col span={12}>
            <StrategicMap highlightedSectorIds={sectorsWithContent} onSectorClick={setSelectedSector} />
          </Col>
          <Col span={12}>
            {c}
          </Col>
        </Row>
    </div>
  );
}
