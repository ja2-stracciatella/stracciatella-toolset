import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Calculate paths similar to webpack.paths.ts
const rootPath = join(__dirname, '../..');
const srcPath = join(rootPath, 'src');
const appNodeModulesPath = join(rootPath, 'release/app/node_modules');

// Webpack configuration for ESLint import resolution
const webpackConfig = {
  resolve: {
    extensions: ['.js', '.jsx', '.json', '.ts', '.tsx'],
    modules: [srcPath, 'node_modules', appNodeModulesPath],
  },
  mode: 'development',
  target: ['web', 'electron-renderer'],
};

export default webpackConfig;
