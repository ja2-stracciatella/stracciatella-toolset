import { configureStore } from '@reduxjs/toolkit';
import { combineReducers } from 'redux';
import { toolset } from './toolset';
import { mods } from './mods';
import { files } from './files';

const reducer = combineReducers({
  toolset,
  mods,
  files,
});

export const appStore = configureStore({
  reducer,
});

export type AppState = ReturnType<typeof appStore.getState>;
export type AppDispatch = typeof appStore.dispatch;
