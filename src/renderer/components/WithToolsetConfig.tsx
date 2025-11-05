import { PropsWithChildren, useCallback, useEffect, useState } from 'react';
import { Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import {
  PartialToolsetConfig,
  FULL_TOOLSET_CONFIG_JSON_SCHEMA,
} from '../state/toolset';
import { FullSizeLoader } from './FullSizeLoader';
import { JsonSchemaForm } from './JsonSchemaForm';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';
import { HostPathWidget } from './form/HostPathWidget';
import { useToolsetConfig } from '../hooks/useToolsetConfig';

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
      setImmediate(() => setState(data.config));
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
        schema={FULL_TOOLSET_CONFIG_JSON_SCHEMA}
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
