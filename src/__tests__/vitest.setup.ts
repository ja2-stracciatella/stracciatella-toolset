import '@testing-library/jest-dom/vitest';
import { beforeEach, vi } from 'vitest';

beforeEach(() => {
  window.matchMedia = vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(), // Deprecated
    removeListener: vi.fn(), // Deprecated
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }));
  window.electronAPI = {
    invoke: vi.fn(),
    onMainEvent: vi.fn(),
  };
});
