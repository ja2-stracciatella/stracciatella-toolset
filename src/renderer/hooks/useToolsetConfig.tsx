import { useCallback } from 'react';
import { useAppDispatch } from './state';
import { useLoadable } from './useLoadable';
import { usePersistable } from './usePersistable';
import {
  readToolsetConfig,
  PartialToolsetConfig,
  updateToolsetConfig,
} from '../state/toolset';

export function useToolsetConfig() {
  const dispatch = useAppDispatch();
  const loadable = useLoadable((state) => state.toolset.config);
  const persistable = usePersistable((state) => state.toolset.config);
  const refresh = useCallback(() => {
    dispatch(readToolsetConfig());
  }, [dispatch]);
  const update = useCallback(
    (m: PartialToolsetConfig) => {
      dispatch(updateToolsetConfig(m));
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
    update,
  };
}
