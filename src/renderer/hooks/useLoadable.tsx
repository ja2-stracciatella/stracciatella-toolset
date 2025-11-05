import { AppState } from '../state/store';
import { Loadable } from '../state/types';
import { useAppSelector } from './state';

export function useLoadable<T>(selector: (state: AppState) => Loadable<T>) {
  const { loading, loadingError, data } = useAppSelector((state) =>
    selector(state),
  );
  return { loading, error: loadingError, data };
}
