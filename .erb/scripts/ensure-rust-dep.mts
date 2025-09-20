import { execSync } from 'node:child_process';
import webpackPaths from '../configs/webpack.paths.mts';

const cargoRebuildCmd =
  'cargo build --release --manifest-path=./src-rust/Cargo.toml --message-format=json-render-diagnostics';
const copyArtifactCommandRaw =
  'node_modules/.bin/cargo-cp-artifact -a cdylib stracciatella-toolset ./src-rust/target/stracciatella-toolset.node';
const copyArtifactCommand =
  process.platform === 'win32'
    ? copyArtifactCommandRaw.replace(/\//g, '\\')
    : copyArtifactCommandRaw;

execSync(`${copyArtifactCommand} -- ${cargoRebuildCmd}`, {
  cwd: webpackPaths.rootPath,
  stdio: 'inherit',
});
