import { AppState } from '../state/store';
import { Persistable } from '../state/types';
import { useAppSelector } from './state';

export function usePersistable<T>(
  selector: (state: AppState) => Persistable<T>,
) {
  const { persisting, persistingError, data } = useAppSelector((state) =>
    selector(state),
  );
  return { loading: persisting, error: persistingError, data };
}
