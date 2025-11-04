import { useCallback, useEffect } from 'react';
import { useAppDispatch } from './state';
import { loadMods } from '../state/mods';
import { useLoadable } from './useLoadable';

export function useMods() {
  const dispatch = useAppDispatch();
  const loadable = useLoadable((state) => state.mods.mods);
  const refresh = useCallback(() => {
    dispatch(loadMods());
  }, [dispatch]);

  return {
    ...loadable,
    refresh,
  };
}
