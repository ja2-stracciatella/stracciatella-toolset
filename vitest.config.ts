import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react-swc';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/__tests__/vitest.setup.ts'],
    // The antd-heavy map editor tests take several seconds on slow CI runners
    testTimeout: 30_000,
    alias: [
      {
        find: /^monaco-editor$/,
        replacement:
          __dirname + '/node_modules/monaco-editor/esm/vs/editor/editor.api',
      },
      {
        find: /^ja2-map-editor$/,
        replacement: __dirname + '/src/__tests__/stubs/ja2-map-editor.ts',
      },
    ],
  },
  plugins: [react()],
});
