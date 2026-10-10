import { execFileSync, execSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import chalk from 'chalk';
import webpackPaths from '../configs/webpack.paths.mts';

// Builds the tactical map viewer (src-map-editor) to wasm and generates the JS glue in src-map-editor/pkg
// Pass `--dist` to build a smaller wasm with LTO for packaging, which takes much longer

const profile = process.argv.includes('--dist') ? 'wasm-dist' : 'release';
const mapEditorPath = path.join(webpackPaths.rootPath, 'src-map-editor');
const outDir = path.join(mapEditorPath, 'pkg');
const wasmPath = path.join(
  webpackPaths.rootPath,
  'target',
  'wasm32-unknown-unknown',
  profile,
  'ja2_map_editor.wasm',
);

// The wasm-bindgen CLI has to match the version of the wasm-bindgen crate exactly
const cargoToml = fs.readFileSync(
  path.join(mapEditorPath, 'Cargo.toml'),
  'utf8',
);
const pinnedVersion = cargoToml.match(/^wasm-bindgen = "=([^"]+)"/m)?.[1];
if (!pinnedVersion) {
  throw new Error('Could not find pinned wasm-bindgen version in Cargo.toml');
}

function wasmBindgenVersion(command: string): string | null {
  try {
    const output = execFileSync(command, ['--version'], { encoding: 'utf8' });
    return output.trim().split(' ')[1] ?? null;
  } catch {
    return null;
  }
}

// `cargo install` puts binaries into $CARGO_HOME/bin, which is not always on the PATH
const exe = process.platform === 'win32' ? '.exe' : '';
const cargoHome = process.env.CARGO_HOME ?? path.join(os.homedir(), '.cargo');
const candidates = [
  `wasm-bindgen${exe}`,
  path.join(cargoHome, 'bin', `wasm-bindgen${exe}`),
];
const wasmBindgen = candidates.find(
  (candidate) => wasmBindgenVersion(candidate) === pinnedVersion,
);
if (!wasmBindgen) {
  const found = candidates
    .map((candidate) => wasmBindgenVersion(candidate))
    .find((version) => version !== null);
  console.error(
    chalk.whiteBright.bgRed.bold(
      `wasm-bindgen ${pinnedVersion} is required to build the map editor${found ? `, found ${found}` : ''}. Install it with:`,
    ),
  );
  console.error(
    `cargo install wasm-bindgen-cli --version ${pinnedVersion} --locked`,
  );
  process.exit(1);
}

execSync(
  // Always pass `-p`, the Neon addon cannot be built for wasm
  `cargo build --profile ${profile} -p ja2-map-editor --target wasm32-unknown-unknown`,
  {
    cwd: webpackPaths.rootPath,
    stdio: 'inherit',
  },
);
execFileSync(wasmBindgen, ['--target', 'web', '--out-dir', outDir, wasmPath], {
  cwd: webpackPaths.rootPath,
  stdio: 'inherit',
});
