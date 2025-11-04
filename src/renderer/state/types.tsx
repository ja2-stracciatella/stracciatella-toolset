import {
  ActionReducerMapBuilder,
  AsyncThunk,
  AsyncThunkConfig,
  Draft,
  PayloadAction,
  SerializedError,
  WritableDraft,
} from '@reduxjs/toolkit';

export type Loadable<T> = {
  loading: boolean;
  loadingError: SerializedError | null;
  data: T | null;
};

export function makeLoadable<T>(initialData: T | null): Loadable<T> {
  return {
    loading: false,
    loadingError: null,
    data: initialData,
  };
}

function pendingLoadable<T>(state: WritableDraft<Loadable<T>>) {
  state.loading = true;
  state.loadingError = null;
}

function rejectedLoadable<T>(
  state: WritableDraft<Loadable<T>>,
  payload: { error: SerializedError },
) {
  state.loading = false;
  state.loadingError = payload.error;
}

function fulfilledLoadable<T>(
  state: WritableDraft<Loadable<T>>,
  payload: PayloadAction<T>,
) {
  state.loading = false;
  state.loadingError = null;
  state.data = payload.payload as Draft<T>;
}

export function buildLoadableMapping<S, I, T>(
  builder: ActionReducerMapBuilder<S>,
  fn: (state: Draft<S>) => Draft<Loadable<T>>,
  thunk: AsyncThunk<T, I, AsyncThunkConfig>,
) {
  builder.addCase(thunk.pending, (state) => pendingLoadable(fn(state)));
  builder.addCase(thunk.rejected, (state, payload) =>
    rejectedLoadable(fn(state), payload),
  );
  builder.addCase(thunk.fulfilled, (state, action) => {
    fulfilledLoadable<T>(fn(state), action);
  });
}

export type Persistable<T> = Loadable<T> & {
  persisting: boolean;
  persistingError: SerializedError | null;
};

export function makePersistable<T>(initialData: T | null): Persistable<T> {
  return {
    ...makeLoadable(initialData),
    persisting: false,
    persistingError: null,
  };
}

function pendingPersistable<T>(state: WritableDraft<Persistable<T>>) {
  state.persisting = true;
  state.persistingError = null;
}

function rejectedPersistable<T>(
  state: WritableDraft<Persistable<T>>,
  payload: { error: SerializedError },
) {
  state.persisting = false;
  state.persistingError = payload.error;
}

function fulfilledPersistable<T>(
  state: WritableDraft<Persistable<T>>,
  payload: PayloadAction<T>,
) {
  state.persisting = false;
  state.persistingError = null;
  state.data = payload.payload as Draft<T>;
}

export function buildPersistableMapping<S, I, T>(
  builder: ActionReducerMapBuilder<S>,
  fn: (state: Draft<S>) => Draft<Persistable<T>>,
  thunk: AsyncThunk<T, I, AsyncThunkConfig>,
) {
  builder.addCase(thunk.pending, (state) => pendingPersistable(fn(state)));
  builder.addCase(thunk.rejected, (state, payload) =>
    rejectedPersistable(fn(state), payload),
  );
  builder.addCase(thunk.fulfilled, (state, payload) => {
    fulfilledPersistable(fn(state), payload);
  });
}
