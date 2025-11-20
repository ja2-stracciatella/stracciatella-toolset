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

export const createAppStore = () =>
  configureStore({
    reducer,
  });

export type AppState = ReturnType<
  ReturnType<typeof createAppStore>['getState']
>;
export type AppDispatch = ReturnType<typeof createAppStore>['dispatch'];
