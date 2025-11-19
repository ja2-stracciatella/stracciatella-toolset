import { render } from '@testing-library/react';
import { ReactElement } from 'react';
import { createAppStore } from '../../../renderer/state/store';
import { Provider } from 'react-redux';

export function renderWithTestProviders(element: ReactElement) {
  const appStore = createAppStore();
  return {
    ...render(<Provider store={appStore}>{element}</Provider>),
  };
}
