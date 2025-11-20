import { PropsWithChildren, useCallback, useEffect, useState } from 'react';
import { Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import { FullSizeLoader } from './common/FullSizeLoader';
import { JsonSchemaForm } from './visual/form/JsonSchemaForm';
import { FullSizeDialogLayout } from './layout/FullSizeDialogLayout';
import { ErrorAlert } from './common/ErrorAlert';
import { HostPathWidget } from './visual/form/HostPathWidget';
import { useToolsetConfig } from '../hooks/useToolsetConfig';
import { toJSONSchema } from 'zod';
import {
  FULL_TOOLSET_CONFIG_SCHEMA,
  PartialToolsetConfig,
} from '../../common/invokables/toolset';
import jsonSchemaValidator from '@rjsf/validator-ajv8';
import { omit } from 'remeda';

const CONFIG_JSON_SCHEMA = omit(toJSONSchema(FULL_TOOLSET_CONFIG_SCHEMA), [
  '$schema',
]);
const CONFIG_UI_SCHEMA = {
  stracciatellaHome: {
    'ui:widget': HostPathWidget,
  },
  vanillaGameDir: {
    'ui:widget': HostPathWidget,
  },
  stracciatellaInstallDir: {
    'ui:widget': HostPathWidget,
  },
  lastSelectedMod: { 'ui:widget': 'hidden' },
};

function Configure() {
  const { persisting, persistingError, data, update } = useToolsetConfig();
  const [state, setState] = useState<PartialToolsetConfig>({
    stracciatellaHome: '',
    vanillaGameDir: '',
    stracciatellaInstallDir: '',
    lastSelectedMod: null,
  });
  const [valid, setValid] = useState(false);
  const change = useCallback((ev: IChangeEvent<PartialToolsetConfig>) => {
    if (ev.formData) {
      setState(ev.formData);
      setValid(ev.errors.length === 0);
    }
  }, []);
  const submit = useCallback(() => {
    if (valid) {
      update(state);
    }
  }, [state, update, valid]);

  useEffect(() => {
    if (data) {
      setTimeout(() => {
        const { validationError } = jsonSchemaValidator.rawValidation(
          CONFIG_JSON_SCHEMA as any,
          data.config,
        );
        setState(data.config);
        setValid(!validationError);
      }, 0);
    }
  }, [data]);

  return (
    <FullSizeDialogLayout>
      <Typography.Title level={2}>
        Configure the Stracciatella Toolset
      </Typography.Title>
      <Typography.Paragraph>
        You need to configure the Stracciatella Toolset first, before you can
        use it.
      </Typography.Paragraph>
      <ErrorAlert error={persistingError} />
      <JsonSchemaForm
        schema={CONFIG_JSON_SCHEMA}
        content={state}
        onChange={change}
        onSubmit={submit}
        uiSchema={CONFIG_UI_SCHEMA}
      />
      <Button
        type="primary"
        onClick={submit}
        disabled={!valid}
        loading={persisting}
      >
        Submit
      </Button>
    </FullSizeDialogLayout>
  );
}

export function WithToolsetConfig({ children }: PropsWithChildren<unknown>) {
  const { loading, loadingError, data, refresh } = useToolsetConfig();

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (loading) {
    return <FullSizeLoader />;
  }
  if (loadingError) {
    return (
      <FullSizeDialogLayout>
        <ErrorAlert error={loadingError} />
      </FullSizeDialogLayout>
    );
  }
  if (!data || data.partial) {
    return <Configure />;
  }
  return <>{children}</>;
}
