import { IChangeEvent, withTheme } from '@rjsf/core';
import { UiSchema, FieldTemplateProps, FieldProps } from '@rjsf/utils';
// eslint-disable-next-line
// @ts-ignore
import { Theme as AntdTheme } from '@rjsf/antd';
import { Form } from 'antd';
import ReactMarkdown from 'react-markdown';
import { useMemo } from 'react';
import validator from '@rjsf/validator-ajv8';

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

const HORIZONTAL_LABEL_COL = { span: 6 };
const HORIZONTAL_WRAPPER_COL = { span: 18 };

// Cloned from Antd theme with some changes
function MarkdownFieldTemplate({
  children,
  classNames,
  description,
  disabled,
  displayLabel,
  // errors,
  // fields,
  formContext,
  help,
  hidden,
  id,
  label,
  onDropPropertyClick,
  onKeyChange,
  rawDescription,
  rawErrors,
  rawHelp,
  readonly,
  required,
  schema,
}: // uiSchema,
FieldTemplateProps) {
  const { colon, wrapperStyle } = formContext;
  const fieldErrors = useMemo(() => {
    if (!rawErrors) {
      return null;
    }
    return [...Array.from(new Set(rawErrors))].map((error: any) => (
      <div key={`field-${id}-error-${error}`}>{error}</div>
    ));
  }, [id, rawErrors]);
  const renderedDescription = useMemo(() => {
    if (!rawDescription) {
      return null;
    }
    return <ReactMarkdown>{rawDescription}</ReactMarkdown>;
  }, [rawDescription]);

  if (hidden) {
    return <div className="field-hidden">{children}</div>;
  }

  return id === 'root' ? (
    children
  ) : (
    <Form.Item
      colon={colon}
      extra={
        schema.type !== 'array' &&
        schema.type !== 'object' &&
        renderedDescription
      }
      hasFeedback={schema.type !== 'array' && schema.type !== 'object'}
      help={schema.type !== 'array' && schema.type !== 'object' && fieldErrors}
      htmlFor={id}
      label={displayLabel && label}
      labelCol={HORIZONTAL_LABEL_COL}
      labelAlign="left"
      required={required}
      style={wrapperStyle}
      validateStatus={rawErrors ? 'error' : undefined}
      wrapperCol={HORIZONTAL_WRAPPER_COL}
    >
      {children}
    </Form.Item>
  );
}

const RjsfForm = withTheme({
  ...AntdTheme,
  widgets: {
    ...AntdTheme.widgets,
    // CheckboxWidget: CheckboxWidgetWithDescription,
  },
  fields: {
    ...AntdTheme.fields,
    DescriptionField: MarkdownDescriptionField,
  },
  templates: {
    ...AntdTheme.templates,
    FieldTemplate: MarkdownFieldTemplate,
  },
});

export interface JsonSchemaFormProps {
  idPrefix?: string;
  schema: any;
  content: any;
  uiSchema?: UiSchema;
  renderButton?: boolean;
  onChange?: (e: IChangeEvent<any>, id?: string) => any;
  onSubmit?: (e: IChangeEvent<any>) => any;
}

export function JsonSchemaForm({
  idPrefix,
  schema,
  content,
  uiSchema,
  renderButton,
  onChange,
  onSubmit,
}: JsonSchemaFormProps) {
  return (
    <RjsfForm
      idPrefix={idPrefix}
      schema={schema}
      formData={content}
      children={renderButton ? undefined : true}
      validator={validator}
      liveValidate
      noHtml5Validate
      showErrorList={false}
      uiSchema={uiSchema}
      onChange={onChange}
      onSubmit={onSubmit}
    />
  );
}
