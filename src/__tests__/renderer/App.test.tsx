import '@testing-library/jest-dom';
import { waitFor } from '@testing-library/react';
import { AppWithoutProviders } from '../../renderer/App';
import { renderWithTestProviders } from './test-utils/render';

describe('App', () => {
  it('should render the configuration dialog when toolset is not configured', async () => {
    const { getByText } = renderWithTestProviders(<AppWithoutProviders />);

    await waitFor(() => {
      expect(
        getByText('Configure the Stracciatella Toolset')
      ).toBeInTheDocument();
    });
  });
});
