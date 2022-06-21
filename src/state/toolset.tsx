import { useCallback, useState } from "react";
import { createContainer } from "unstated-next";
import { z } from "zod";
import { invokeWithSchema } from "../lib/invoke";

const PartialToolsetConfig = z.object({
  stracciatellaHome: z.union([z.string(), z.null()]),
  vanillaGameDir: z.union([z.string(), z.null()]),
  stracciatellaInstallDir: z.union([z.string(), z.null()]),
  lastSelectedMod: z.union([z.string(), z.null()]),
});

export type PartialToolsetConfig = z.infer<typeof PartialToolsetConfig>;

const ToolsetConfig = z.object({
  stracciatellaHome: z.string(),
  vanillaGameDir: z.string(),
  stracciatellaInstallDir: z.string(),
  lastSelectedMod: z.union([z.string(), z.null()]),
});

export type ToolsetConfig = z.infer<typeof ToolsetConfig>;

const SerializedSchema = z.object({
  partial: z.boolean(),
  config: PartialToolsetConfig,
});

interface LoadingState {
  loading: true;
  error: null;
  partial: null;
  config: null;
}

interface ErrorState {
  loading: false;
  error: Error;
  partial: boolean | null;
  config: PartialToolsetConfig | null;
}

interface NotConfiguredState {
  loading: false;
  error: null;
  partial: true;
  config: PartialToolsetConfig;
}

interface ConfiguredState {
  loading: false;
  error: null;
  partial: false;
  config: ToolsetConfig;
}

type ToolsetState =
  | LoadingState
  | ErrorState
  | NotConfiguredState
  | ConfiguredState;

function useToolsetState(
  initialState: ToolsetState = {
    loading: true,
    error: null,
    config: null,
    partial: null,
  }
) {
  const [state, setState] = useState<ToolsetState>(initialState);
  const toolsetLoading = useCallback(
    () =>
      setState({
        loading: true,
        error: null,
        partial: null,
        config: null,
      }),
    []
  );
  const toolsetError = useCallback(
    (error: Error) =>
      setState({
        loading: false,
        error,
        partial: state.partial,
        config: state.config,
      }),
    [state.config, state.partial]
  );
  const toolsetPartialSuccess = useCallback(
    (config: PartialToolsetConfig) =>
      setState({
        loading: false,
        error: null,
        partial: true,
        config,
      }),
    []
  );
  const toolsetSuccess = useCallback(
    (config: ToolsetConfig) =>
      setState({
        loading: false,
        error: null,
        partial: false,
        config,
      }),
    []
  );
  const setToolsetConfig = useCallback(
    async (config: PartialToolsetConfig) => {
      const partial = [
        config.stracciatellaHome,
        config.stracciatellaInstallDir,
        config.vanillaGameDir,
      ].some((v) => v === null);
      try {
        const res = await invokeWithSchema(
          SerializedSchema,
          "set_toolset_config",
          {
            config: {
              partial,
              config,
            },
          }
        );

        if (res.partial) {
          toolsetPartialSuccess(res.config);
        } else {
          toolsetSuccess(res.config as ToolsetConfig);
        }
      } catch (e: any) {
        toolsetError(new Error(`${e}`));
      }
    },
    [toolsetError, toolsetPartialSuccess, toolsetSuccess]
  );
  return {
    ...state,
    toolsetLoading,
    toolsetError,
    toolsetPartialSuccess,
    toolsetSuccess,
    setToolsetConfig,
  };
}

const toolsetState = createContainer(useToolsetState);

export const ToolsetConfigProvider = toolsetState.Provider;
export const useToolsetConfig = toolsetState.useContainer;

export function useFetchToolsetConfig() {
  const {
    toolsetLoading,
    toolsetError,
    toolsetPartialSuccess,
    toolsetSuccess,
  } = useToolsetConfig();
  const fetchToolsetConfig = useCallback(async () => {
    toolsetLoading();
    try {
      const { partial, config } = await invokeWithSchema(
        SerializedSchema,
        "get_toolset_config"
      );
      if (partial) {
        toolsetPartialSuccess(config);
      } else {
        const fullConfig = ToolsetConfig.parse(config);
        toolsetSuccess(fullConfig);
      }
    } catch (e) {
      toolsetError(new Error(`error loading toolset config: ${e}`));
    }
  }, [toolsetLoading, toolsetError, toolsetPartialSuccess, toolsetSuccess]);

  return fetchToolsetConfig;
}
