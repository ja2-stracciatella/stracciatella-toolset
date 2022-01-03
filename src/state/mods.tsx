import { invoke } from "@tauri-apps/api";
import { useCallback, useState } from "react";
import { createContainer } from "unstated-next";
import { z } from "zod";
import { invokeWithSchema } from "../lib/invoke";

const Mod = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string(),
  version: z.string(),
});

export type Mod = z.infer<typeof Mod>;

const EditableMod = Mod.extend({
  path: z.string(),
});

export type EditableMod = z.infer<typeof Mod>;

interface LoadingModsState {
  loading: true;
  error: null;
  selectedMod: null;
  editableMods: null;
  availableMods: null;
}

interface ErroredModsState {
  loading: false;
  error: Error;
  selectedMod: null;
  editableMods: null;
  availableMods: null;
}

interface LoadedModsState {
  loading: false,
  error: null,
  selectedMod: null,
  editableMods: Array<EditableMod>,
  availableMods: Array<Mod>,
}

interface SelectedModState {
  loading: false,
  error: null,
  selectedMod: EditableMod,
  editableMods: Array<EditableMod>,
  availableMods: Array<Mod>,
}

type ModsState = LoadingModsState | ErroredModsState | LoadedModsState | SelectedModState;

function useModsState(
  initialState: ModsState = { loading: true, error: null, selectedMod: null, editableMods: null, availableMods: null }
) {
  const [modsState, setModsState] = useState<ModsState>(initialState);
  const modsLoading = useCallback(
    () =>
      setModsState({
        loading: true,
        error: null,
        selectedMod: null,
        editableMods: null,
        availableMods: null,
      }),
    []
  );
  const modsLoadingSuccess = useCallback(
    (editableMods: Array<EditableMod>, availableMods: Array<Mod>) =>
      setModsState({
        loading: false,
        error: null,
        selectedMod: null,
        editableMods,
        availableMods
      }),
    []
  );
  const modsError = useCallback(
    (error: Error) =>
      setModsState({
        loading: false,
        error,
        selectedMod: null,
        editableMods: null,
        availableMods: null,
      }),
    []
  );
  const selectMod = useCallback(
    async (mod: EditableMod) => {
      if (modsState.editableMods === null) {
        modsError(new Error("cannot select mod while mods are loading"));
        return;
      }
      if (!modsState.editableMods.find(m => m.id === mod.id)) {
        modsError(new Error(`cannot select unknown mod "${mod.id}"`));
        return;
      }
      try {
        await invoke("set_selected_mod", {
          modId: mod.id,
        });

        setModsState({
          ...modsState,
          selectedMod: mod,
        });
      } catch (e: any) {
        modsError(new Error(`cannot select mod "${mod.id}": ${e}`));
      }
    },
    [modsState, modsError]
  );
  return { ...modsState, modsLoading, modsLoadingSuccess, modsError, selectMod };
}

const modsState = createContainer(useModsState);

export const ModsProvider = modsState.Provider;
export const useMods = modsState.useContainer;

const MODS_SCHEMA = z.array(Mod);
const EDITABLE_MODS_SCHEMA = z.array(EditableMod);


export function useFetchMods() {
  const { modsLoading, modsError, modsLoadingSuccess } = useMods();
  const fetchMods = useCallback(async () => {
    modsLoading();
    try {
      const [availableMods, editableMods] = await Promise.all([
        invokeWithSchema(MODS_SCHEMA, "get_available_mods"),
        invokeWithSchema(EDITABLE_MODS_SCHEMA, "get_editable_mods"),
      ]);
      modsLoadingSuccess(editableMods, availableMods);
    } catch (e) {
      modsError(new Error(`error loading mods: ${e}`));
    }
  }, [modsLoading, modsError, modsLoadingSuccess]);

  return fetchMods;
}
