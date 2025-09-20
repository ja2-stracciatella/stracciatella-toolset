import path from 'node:path';
import { rimraf } from 'rimraf';
import webpackPaths from '../configs/webpack.paths.mts';

export default async function deleteSourceMaps() {
  await rimraf(path.join(webpackPaths.distMainPath, '*.js.map'), {
    glob: true,
  });
  await rimraf(path.join(webpackPaths.distRendererPath, '*.js.map'), {
    glob: true,
  });
}
