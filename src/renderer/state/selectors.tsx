import { AppState } from './store';

export function selectStracciatellaHome(state: AppState) {
  return state.toolset.config.data?.config.stracciatellaHome ?? null;
}

export function selectCloseModRequested(state: AppState): boolean {
  return state.toolset.closeRequested;
}
