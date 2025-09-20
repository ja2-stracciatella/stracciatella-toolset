import path from 'node:path';
import rimraf from 'rimraf';
import webpackPaths from "../configs/webpack.paths.mjs";
export default function deleteSourceMaps() {
    rimraf.sync(path.join(webpackPaths.distMainPath, '*.js.map'));
    rimraf.sync(path.join(webpackPaths.distRendererPath, '*.js.map'));
}
