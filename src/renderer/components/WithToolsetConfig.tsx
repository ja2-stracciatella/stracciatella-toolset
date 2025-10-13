import { PropsWithChildren, useCallback, useEffect, useState } from 'react';
import { Alert, Button, Typography } from 'antd';
import { IChangeEvent } from '@rjsf/core';
import {
  PartialToolsetConfig,
  FullToolsetConfig,
  getToolsetConfig,
  setToolsetConfig,
} from '../state/toolset';
import { FullSizeLoader } from './FullSizeLoader';
import { JsonSchemaForm } from './JsonSchemaForm';
import { useAppDispatch, useAppSelector } from '../hooks/state';
import { FullSizeDialogLayout } from './FullSizeDialogLayout';
import { ErrorAlert } from './ErrorAlert';
import { HostPathWidget } from './form/HostPathWidget';

const CONFIG_JSON_SCHEMA = {
  type: 'object',
  properties: {
    stracciatellaHome: {
      title: 'JA2 Stracciatella Home Directory',
      description:
        'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your JA2 Stracciatella configuration and mods.',
      type: 'string',
      minLength: 1,
    },
    vanillaGameDir: {
      title: 'Vanilla Game Directory',
      description:
        'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your vanilla (original) Jagged Alliance 2 installation.',
      type: 'string',
      minLength: 1,
    },
    stracciatellaInstallDir: {
      title: 'JA2 Stracciatella Directory',
      description:
        'This should point at your JA2 Straccciatella install directory. It is used to read the JSON files included with the game.',
      type: 'string',
      minLength: 1,
    },
  },
  required: ['stracciatellaHome', 'vanillaGameDir', 'stracciatellaInstallDir'],
};
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
};

function Configure() {
  const dispatch = useAppDispatch();
  const config = useAppSelector((s) => s.toolset.config.value);
  const error = useAppSelector((s) => s.toolset.error);

  const [state, setState] = useState(config);
  const [valid, setValid] = useState(false);
  const change = useCallback((ev: IChangeEvent<PartialToolsetConfig>) => {
    if (ev.formData) {
      setState(ev.formData);
      setValid(ev.errors.length === 0);
    }
  }, []);
  const submit = useCallback(() => {
    if (valid) {
      dispatch(setToolsetConfig(state as FullToolsetConfig));
    }
  }, [dispatch, state, valid]);

  return (
    <FullSizeDialogLayout>
      <Typography.Title level={2}>
        Configure the Stracciatella Toolset
      </Typography.Title>
      <Typography.Paragraph>
        You need to configure the Stracciatella Toolset first, before you can
        use it.
      </Typography.Paragraph>
      <ErrorAlert error={error} />
      <JsonSchemaForm
        schema={CONFIG_JSON_SCHEMA}
        content={state}
        onChange={change}
        onSubmit={submit}
        uiSchema={CONFIG_UI_SCHEMA}
      />
      <Button type="primary" onClick={submit} disabled={!valid}>
        Submit
      </Button>
    </FullSizeDialogLayout>
  );
}

export function WithToolsetConfig({ children }: PropsWithChildren<{}>) {
  const dispatch = useAppDispatch();
  const loading = useAppSelector((s) => s.toolset.loading);
  const partial = useAppSelector((s) => s.toolset.config.partial);

  useEffect(() => {
    dispatch(getToolsetConfig());
  }, [dispatch]);

  if (loading) {
    return (
      <div>
        <FullSizeLoader />
      </div>
    );
  }
  if (partial) {
    return <Configure />;
  }
  return <div>{children}</div>;
}
