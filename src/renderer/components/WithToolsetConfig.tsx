import { PropsWithChildren, useCallback, useEffect, useState } from 'react';
import { Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import { FullSizeLoader } from './FullSizeLoader';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';
import { HostPathWidget } from './form/HostPathWidget';
import { useToolsetConfig } from '../hooks/useToolsetConfig';
import { toJSONSchema } from 'zod';
import {
  FULL_TOOLSET_CONFIG_SCHEMA,
  PartialToolsetConfig,
} from '../../common/invokables/toolset';
import jsonSchemaValidator from '@rjsf/validator-ajv8';

const CONFIG_JSON_SCHEMA = toJSONSchema(FULL_TOOLSET_CONFIG_SCHEMA);
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
        setState(data.config);

        setValid(
          !jsonSchemaValidator.rawValidation(
            CONFIG_JSON_SCHEMA as any,
            data.config,
          ).validationError,
        );
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
