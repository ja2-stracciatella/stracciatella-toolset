import {
  PropsWithChildren,
  ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { Alert } from "antd";
import {
  PartialToolsetConfig,
  ToolsetConfig,
  useFetchToolsetConfig,
  useToolsetConfig,
} from "../state/toolset";
import { FullSizeLoader } from "./FullSizeLoader";
import { JsonSchemaForm } from "./JsonSchemaForm";
import { IChangeEvent } from "@rjsf/core";

const CONFIG_JSON_SCHEMA = {
  type: "object",
  title: "Configure the Stracciatella Toolset",
  description:
    "You need to configure the Stracciatella Toolset first, before you can use it.",
  properties: {
    stracciatellaHome: {
      title: "JA2 Stracciatella Home Directory",
      description:
        "This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your JA2 Stracciatella configuration and mods.",
      type: "string",
      minLength: 1,
    },
    vanillaGameDir: {
      title: "Vanilla Game Directory",
      description:
        "This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your vanilla (original) Jagged Alliance 2 installation.",
      type: "string",
      minLength: 1,
    },
    stracciatellaInstallDir: {
      title: "JA2 Stracciatella Directory",
      description:
        "This should point at your JA2 Straccciatella install directory. It is used to read the JSON files included with the game.",
      type: "string",
      minLength: 1,
    },
  },
  required: ["stracciatellaHome", "vanillaGameDir", "stracciatellaInstallDir"],
};

function Configure({
  config,
  onSubmit,
}: {
  config: PartialToolsetConfig | null;
  onSubmit: (t: ToolsetConfig) => any;
}) {
  const { error } = useToolsetConfig();
  const [state, setState] = useState(config);
  const [valid, setValid] = useState(false);
  const change = useCallback((ev: IChangeEvent<PartialToolsetConfig>) => {
    setState(ev.formData);
    setValid(ev.errors.length === 0);
  }, []);
  const submit = useCallback(() => {
    console.log(valid);
    if (valid) {
      onSubmit(state as ToolsetConfig);
    }
  }, [onSubmit, state, valid]);
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
        renderButton={true}
        onChange={change}
        onSubmit={submit}
      />
    </div>
  );
}

export function WithToolsetConfig({ children }: PropsWithChildren<{}>) {
  const { loading, error, config, partial, setToolsetConfig } =
    useToolsetConfig();
  const fetchToolsetConfig = useFetchToolsetConfig();

  useEffect(() => {
    if (!config && !error) {
      fetchToolsetConfig();
    }
  }, [loading, config, error, fetchToolsetConfig]);

  let content = useMemo(() => {
    let message: ReactNode = children;
    if (loading) {
      message = (
        <div>
          <FullSizeLoader />
        </div>
      );
    }
    if ((config && partial) || error) {
      message = <Configure config={config} onSubmit={setToolsetConfig} />;
    }

    return message;
  }, [children, config, error, loading, partial, setToolsetConfig]);

  return <>{content}</>;
}
