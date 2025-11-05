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

function fulfilledLoadable<T>(state: WritableDraft<Loadable<T>>, payload: T) {
  state.loading = false;
  state.loadingError = null;
  state.data = payload as Draft<T>;
}

export function buildLoadableMapping<
  State,
  Input,
  Returned,
  Transformed = Returned,
>(
  builder: ActionReducerMapBuilder<State>,
  thunk: AsyncThunk<Returned, Input, AsyncThunkConfig>,
  selector: (
    state: Draft<State>,
    payload: PayloadAction<unknown, string, { arg: Input }>,
  ) => Draft<Loadable<Transformed>>,
  transform: (data: Returned) => Transformed,
  postProcess: (
    state: Draft<State>,
    payload: PayloadAction<Returned, string, { arg: Input }>,
  ) => void = () => {},
) {
  builder.addCase(thunk.pending, (state, payload) =>
    pendingLoadable(selector(state, payload)),
  );
  builder.addCase(thunk.rejected, (state, payload) =>
    rejectedLoadable(selector(state, payload), payload),
  );
  builder.addCase(thunk.fulfilled, (state, payload) => {
    fulfilledLoadable<Transformed>(
      selector(state, payload),
      transform(payload.payload),
    );
    postProcess(state, payload);
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
  payload: T,
) {
  state.persisting = false;
  state.persistingError = null;
  state.data = payload as Draft<T>;
}

export function buildPersistableMapping<
  State,
  Input,
  Returned,
  Transformed = Returned,
>(
  builder: ActionReducerMapBuilder<State>,
  thunk: AsyncThunk<Returned, Input, AsyncThunkConfig>,
  selector: (
    state: Draft<State>,
    payload: PayloadAction<unknown, string, { arg: Input }>,
  ) => Draft<Persistable<Transformed>>,
  transform: (data: Returned) => Transformed,
  postProcess: (
    state: Draft<State>,
    payload: PayloadAction<Returned, string, { arg: Input }>,
  ) => void = () => {},
) {
  builder.addCase(thunk.pending, (state, payload) =>
    pendingPersistable(selector(state, payload)),
  );
  builder.addCase(thunk.rejected, (state, payload) =>
    rejectedPersistable(selector(state, payload), payload),
  );
  builder.addCase(thunk.fulfilled, (state, payload) => {
    fulfilledPersistable(selector(state, payload), transform(payload.payload));
    postProcess(state, payload);
  });
}
