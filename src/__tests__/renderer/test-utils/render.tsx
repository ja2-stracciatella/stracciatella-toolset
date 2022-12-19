import { render } from '@testing-library/react';
import { ReactElement } from 'react';
import { FilesProvider } from '../../../renderer/state/files';
import { ModsProvider } from '../../../renderer/state/mods';
import { ToolsetConfigProvider } from '../../../renderer/state/toolset';

export function renderWithTestProviders(element: ReactElement) {
  return {
    ...render(
      <ModsProvider>
        <ToolsetConfigProvider>
          <FilesProvider>{element}</FilesProvider>
        </ToolsetConfigProvider>
      </ModsProvider>
    ),
  };
}
