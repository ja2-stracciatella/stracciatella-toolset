import { useCallback } from 'react';
import {
  createNewMod,
  EditableMod,
  readSelectedMod,
  Mod,
  updateSelectedMod,
} from '../state/mods';
import { useAppDispatch } from './state';
import { usePersistable } from './usePersistable';
import { useLoadable } from './useLoadable';

export function useSelectedMod() {
  const dispatch = useAppDispatch();
  const loadable = useLoadable((state) => state.mods.selected);
  const persistable = usePersistable((state) => state.mods.selected);
  const refresh = useCallback(() => {
    dispatch(readSelectedMod());
  }, [dispatch]);
  const create = useCallback(
    (m: Mod) => {
      dispatch(createNewMod(m));
    },
    [dispatch],
  );
  const update = useCallback(
    (m: EditableMod) => {
      dispatch(updateSelectedMod(m));
    },
    [dispatch],
  );

  return {
    loading: loadable.loading,
    persisting: persistable.loading,
    loadingError: loadable.error,
    persistingError: persistable.error,
    data: loadable.data,
    refresh,
    create,
    update,
  };
}
