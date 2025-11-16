import { FormProps, IChangeEvent, withTheme } from '@rjsf/core';
import { UiSchema } from '@rjsf/utils';
import { Theme as AntdTheme } from '@rjsf/antd';
import validator from '@rjsf/validator-ajv8';
import { useMemo } from 'react';

const RjsfForm = withTheme(AntdTheme);

const DEFAULT_UI_SCHEMA: UiSchema = {
  'ui:globalOptions': {
    enableMarkdownInDescription: true,
    enableMarkdownInHelp: true,
  },
};

const DEFAULT_PROPS: Pick<
  FormProps,
  | 'experimental_defaultFormStateBehavior'
  | 'showErrorList'
  | 'validator'
  | 'liveValidate'
  | 'liveOmit'
  | 'noHtml5Validate'
> = {
  showErrorList: false,
  noHtml5Validate: true,
  validator: validator,
  liveValidate: 'onChange',
  liveOmit: 'onChange',
  experimental_defaultFormStateBehavior: {
    arrayMinItems: {
      populate: 'never',
      mergeExtraDefaults: false,
    },
    emptyObjectFields: 'skipDefaults',
    allOf: 'skipDefaults',
  },
};

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
  const appliedUiSchema: UiSchema = useMemo(
    () => ({
      ...DEFAULT_UI_SCHEMA,
      ...uiSchema,
    }),
    [uiSchema],
  );

  return (
    <RjsfForm
      idPrefix={idPrefix}
      schema={schema}
      formData={content}
      // eslint-disable-next-line react/no-children-prop
      children={renderButton ? undefined : true}
      uiSchema={appliedUiSchema}
      onChange={onChange}
      onSubmit={onSubmit}
      {...DEFAULT_PROPS}
    />
  );
}
