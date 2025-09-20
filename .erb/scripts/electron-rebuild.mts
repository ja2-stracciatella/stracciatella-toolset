import { execSync } from 'node:child_process';
import fs from 'node:fs';
import packageJson from '../../release/app/package.json' with { type: 'json' };
import webpackPaths from '../configs/webpack.paths.mts';

const { dependencies } = packageJson;

if (
  Object.keys(dependencies || {}).length > 0 &&
  fs.existsSync(webpackPaths.appNodeModulesPath)
) {
  const electronRebuildCmd =
    '../../node_modules/.bin/electron-rebuild --force --types prod,dev,optional --module-dir .';
  const cmd =
    process.platform === 'win32'
      ? electronRebuildCmd.replace(/\//g, '\\')
      : electronRebuildCmd;
  execSync(cmd, {
    cwd: webpackPaths.appPath,
    stdio: 'inherit',
  });
}
