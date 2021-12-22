import { useCallback, useState } from "react";
import { createContainer } from "unstated-next";
import { invoke } from "@tauri-apps/api/tauri";
import { z } from "zod";

const Mod = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  version: z.string(),
});

type Mod = z.infer<typeof Mod>;

interface ModsState {
  loading: boolean;
  error: Error | null;
  mods: Array<Mod> | null;
}

function useModsState(
  initialState: ModsState = { loading: true, error: null, mods: null }
) {
  const [modsState, setModsState] = useState(initialState);
  const modsLoading = useCallback(
    () =>
      setModsState({
        loading: true,
        error: null,
        mods: null,
      }),
    []
  );
  const modsLoadingSuccess = useCallback(
    (mods: Array<Mod>) =>
      setModsState({
        loading: false,
        error: null,
        mods,
      }),
    []
  );
  const modsLoadingError = useCallback(
    (error: Error) =>
      setModsState({
        loading: false,
        error,
        mods: null,
      }),
    []
  );
  return { ...modsState, modsLoading, modsLoadingSuccess, modsLoadingError };
}

const modsState = createContainer(useModsState);

export const ModsProvider = modsState.Provider;
export const useMods = modsState.useContainer;

export function useFetchMods() {
  const { modsLoading, modsLoadingError, modsLoadingSuccess } = useMods();
  const fetchMods = useCallback(async () => {
    modsLoading();
    try {
      console.log("invoking mods");
      const modsSchema = z.array(Mod);
      const modsResponse = await invoke("get_available_mods");
      const mods = modsSchema.parse(modsResponse);
      modsLoadingSuccess(mods);
    } catch (e) {
      console.log("error", e);
      modsLoadingError(new Error(`error loading mods: ${e}`));
    }
  }, [modsLoading, modsLoadingError, modsLoadingSuccess]);

  return fetchMods;
}
