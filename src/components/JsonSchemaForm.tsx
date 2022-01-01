import {
  FieldProps,
  FieldTemplateProps,
  WidgetProps,
  withTheme,
} from "@rjsf/core";
import { Theme as Bootstrap4Theme, FieldTemplate } from "@rjsf/bootstrap-4";
import ReactMarkdown from "react-markdown";
import { useMemo } from "react";
import { Form } from "react-bootstrap";

export interface DescriptionFieldProps extends Partial<FieldProps> {
  description?: string;
}

function MarkdownDescriptionField({ id, description }: DescriptionFieldProps) {
  if (!description) {
    return null;
  }
  return (
    <div id={id}>
      <ReactMarkdown>{description}</ReactMarkdown>
    </div>
  );
}

function MarkdownFieldTemplate(props: FieldTemplateProps) {
  // Any is a workaround because FieldTemplate expects a string not a react element
  const rawDescription: any = useMemo(() => {
    if (!props.rawDescription) {
      return props.rawDescription;
    }
    return <ReactMarkdown>{props.rawDescription}</ReactMarkdown>;
  }, [props.rawDescription]);
  return <FieldTemplate {...props} rawDescription={rawDescription} />;
}

const CheckboxWidgetWithDescription = (props: WidgetProps) => {
  const {
    id,
    value,
    required,
    disabled,
    readonly,
    label,
    schema,
    autofocus,
    onChange,
    onBlur,
    onFocus
  } = props;

  const _onChange = ({
    target: { checked },
  }: React.FocusEvent<HTMLInputElement>) => onChange(checked);
  const _onBlur = ({
    target: { checked },
  }: React.FocusEvent<HTMLInputElement>) => onBlur(id, checked);
  const _onFocus = ({
    target: { checked },
  }: React.FocusEvent<HTMLInputElement>) => onFocus(id, checked);

  const desc = label || schema.description;
  console.log("checkbox widget", schema);
  return (
    <Form.Group
      className={`checkbox ${disabled || readonly ? "disabled" : ""}`}
    >
      <Form.Check
        id={id}
        label={desc}
        checked={typeof value === "undefined" ? false : value}
        required={required}
        disabled={disabled || readonly}
        autoFocus={autofocus}
        onChange={_onChange}
        type="checkbox"
        onBlur={_onBlur}
        onFocus={_onFocus}
      />
      {schema.description ? (
        <Form.Text
          className="text-muted"
        >
          <ReactMarkdown>{schema.description}</ReactMarkdown>
        </Form.Text>
      ) : null}
    </Form.Group>
  );
};

const RjsfForm = withTheme({
  ...Bootstrap4Theme,
  FieldTemplate: MarkdownFieldTemplate,
  widgets: {
    ...Bootstrap4Theme.widgets,
    CheckboxWidget: CheckboxWidgetWithDescription,
  },
  fields: {
    ...Bootstrap4Theme.fields,
    DescriptionField: MarkdownDescriptionField,
  }
});

export interface JsonSchemaFormProps {
  idPrefix?: string;
  schema: any;
  content: any;
}

export function JsonSchemaForm({
  idPrefix,
  schema,
  content,
}: JsonSchemaFormProps) {
  return (
    <RjsfForm
      idPrefix={idPrefix}
      schema={schema}
      formData={content}
      children={true}
      liveValidate={true}
      noHtml5Validate={true}
      showErrorList={false}
    />
  );
}
