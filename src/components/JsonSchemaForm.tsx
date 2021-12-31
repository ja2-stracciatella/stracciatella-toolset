import { withTheme } from "@rjsf/core";
// @ts-ignore
import { Theme as AntDTheme } from "@rjsf/antd";
import ReactMarkdown from "react-markdown";

const Form = withTheme(AntDTheme);

export interface JsonSchemaFormProps {
  schema: any;
  content: any;
}

function DescriptionField({ id, description }: { id: string, description: string | undefined }) {
  if (!description) {
    return null;
  }
  return <div id={id}><ReactMarkdown>{description}</ReactMarkdown></div>
}

const fields = {
  DescriptionField: DescriptionField
};

export function JsonSchemaForm({ schema, content }: JsonSchemaFormProps) {
  return <Form schema={schema} formData={content} children={true} fields={fields as any} />;
}
