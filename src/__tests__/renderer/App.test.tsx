import { waitFor } from '@testing-library/dom';
import { AppWithoutProviders } from '../../renderer/App';
import { renderWithTestProviders } from './test-utils/render';
import { it, describe, expect } from 'vitest';

describe('App', () => {
  it('should render loading as the default', async () => {
    const { getByLabelText } = renderWithTestProviders(<AppWithoutProviders />);

    await waitFor(() => {
      expect(getByLabelText('loading')).toBeInTheDocument();
    });
  });
});
