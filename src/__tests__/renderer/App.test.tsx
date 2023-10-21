import { waitFor } from '@testing-library/react';
import { AppWithoutProviders } from '../../renderer/App';
import { renderWithTestProviders } from './test-utils/render';
import { it, describe, expect } from 'vitest';

describe('App', () => {
  it('should render the configuration dialog when toolset is not configured', async () => {
    const { getByLabelText } = renderWithTestProviders(<AppWithoutProviders />);

    await waitFor(() => {
      expect(getByLabelText('loading')).toBeInTheDocument();
    });
  });
});
