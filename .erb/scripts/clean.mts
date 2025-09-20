import { rimraf } from 'rimraf';
import webpackPaths from '../configs/webpack.paths.mts';

const foldersToRemove = [
  webpackPaths.distPath,
  webpackPaths.buildPath,
  webpackPaths.dllPath,
];

await Promise.all(
  foldersToRemove.map(async (folder) => {
    await rimraf(folder);
  }),
);
