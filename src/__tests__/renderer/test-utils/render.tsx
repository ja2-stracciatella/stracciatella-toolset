import { render } from '@testing-library/react';
import { ReactElement } from 'react';
import { appStore } from '../../../renderer/state/store';
import { Provider } from 'react-redux';

export function renderWithTestProviders(element: ReactElement) {
  return {
    ...render(<Provider store={appStore}>{element}</Provider>),
  };
}
