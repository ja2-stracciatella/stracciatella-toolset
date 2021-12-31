import { withTheme } from "@rjsf/core";
import { Theme as Bootstrap4Theme } from "@rjsf/bootstrap-4";
import ReactMarkdown from "react-markdown";

const Form = withTheme(Bootstrap4Theme);

export interface JsonSchemaFormProps {
  idPrefix?: string;
  schema: any;
  content: any;
}

function DescriptionField({
  id,
  description,
}: {
  id: string;
  description: string | undefined;
}) {
  if (!description) {
    return null;
  }
  return (
    <div id={id}>
      <ReactMarkdown>{description}</ReactMarkdown>
    </div>
  );
}

const fields = {
  DescriptionField: DescriptionField,
};

export function JsonSchemaForm({ idPrefix, schema, content }: JsonSchemaFormProps) {
  return (
    <Form
      idPrefix={idPrefix}
      schema={schema}
      formData={content}
      children={true}
      fields={fields as any}
    />
  );
}
