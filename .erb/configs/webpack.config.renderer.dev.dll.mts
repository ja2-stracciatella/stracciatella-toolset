/**
 * Builds the DLL for development electron renderer process
 */

import webpack from 'webpack';
import path from 'node:path';
import { merge } from 'webpack-merge';
import baseConfig from './webpack.config.base.mts';
import webpackPaths from './webpack.paths.mts';
import packageJson from '../../package.json' with { type: 'json' };
import checkNodeEnv from '../scripts/check-node-env.mts';
import rendererModuleConfig from './webpack.module.renderer.mts';

checkNodeEnv('development');

const { dependencies } = packageJson;
const dist = webpackPaths.dllPath;

const configuration: webpack.Configuration = {
  context: webpackPaths.rootPath,

  devtool: 'eval',

  mode: 'development',

  target: 'electron-renderer',

  externals: ['fsevents', 'crypto-browserify'],

  /**
   * Use shared module configuration for renderer
   */
  module: rendererModuleConfig,

  entry: {
    renderer: Object.keys(dependencies || {}),
  },

  output: {
    path: dist,
    filename: '[name].dev.dll.js',
    library: {
      name: 'renderer',
      type: 'var',
    },
  },

  plugins: [
    new webpack.DllPlugin({
      path: path.join(dist, '[name].json'),
      name: '[name]',
    }),

    /**
     * Create global constants which can be configured at compile time.
     *
     * Useful for allowing different behaviour between development builds and
     * release builds
     *
     * NODE_ENV should be production so that modules do not perform certain
     * development checks
     */
    new webpack.EnvironmentPlugin({
      NODE_ENV: 'development',
    }),

    new webpack.LoaderOptionsPlugin({
      debug: true,
      options: {
        context: webpackPaths.srcPath,
        output: {
          path: webpackPaths.dllPath,
        },
      },
    }),
  ],
};

export default merge(baseConfig, configuration);
