import { PropsWithChildren, useCallback, useEffect, useState } from 'react';
import { Alert } from 'antd';
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

const CONFIG_JSON_SCHEMA = {
  type: 'object',
  title: 'Configure the Stracciatella Toolset',
  description:
    'You need to configure the Stracciatella Toolset first, before you can use it.',
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

function Configure() {
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
      setToolsetConfig(state as FullToolsetConfig);
    }
  }, [state, valid]);
  const errbox = error ? (
    <div className="with-toolset-config">
      <Alert type="error" message={error.toString()} />
    </div>
  ) : null;

  return (
    <div>
      {errbox}
      <JsonSchemaForm
        schema={CONFIG_JSON_SCHEMA}
        content={state}
        renderButton
        onChange={change}
        onSubmit={submit}
      />
    </div>
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
