# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Electron desktop app (React 19 + Ant Design + Redux Toolkit) for creating mods for JA2 Stracciatella. Most heavy lifting (file I/O, VFS, schemas, image/sound decoding, mod management) is done in a Rust native module (`src-rust/`, built with Neon) that depends on the `stracciatella` crate from the main game repo, pinned to a git rev in `src-rust/Cargo.toml`. The tactical map viewer (`src-map-editor/`, Bevy compiled to wasm, see its README) runs inside the renderer. Both crates are members of the Cargo workspace in the repo root and share `Cargo.lock`, the `stracciatella` dependency and the toolchain (`rust-toolchain.toml`). The build is based on electron-react-boilerplate; webpack configs and helper scripts live in `.erb/`.

## Commands

- `npm install`: install deps (postinstall builds the renderer DLL bundle)
- `npm start`: dev mode (rebuilds Rust, builds main, serves renderer with hot reload; electronmon restarts on `src/main/**` changes)
- `npm run rebuild:rust`: `cargo build --release` in `src-rust/`, then copies the cdylib to `src-rust/target/stracciatella-toolset.node`. Rerun this after any Rust change.
- `npm run rebuild:map-editor`: builds the map viewer wasm and its wasm-bindgen glue into `src-map-editor/pkg/` (needs the pinned `wasm-bindgen-cli`). Rerun this after any change in `src-map-editor/`. `prestart` and `build:renderer` run it.
- `cargo run -p ja2-map-editor --example bench --features bench --release -- --scenario idle --duration 60`: native GPU memory and frame time benchmark of the map viewer on a synthetic map, prints JSON lines (see `src-map-editor/README.md`, `--list` for the scenarios)
- `npm run tsc:check`: typecheck the app and `.erb/`
- `npm run lint`: ESLint with Prettier (single quotes)
- `npm test` / `npm run test:watch`: Vitest (jsdom)
- Single test: `npx cross-env NODE_ENV=development vitest run src/__tests__/renderer/state/files.test.tsx` (add `-t "<name>"` to filter)
- `npm run package`: build and package with electron-builder

CI (`.github/workflows/test.yml`) runs lint, tsc:check, test and package on macOS, Windows and Linux.

## Architecture

### IPC "invokables" (the core pattern)

All renderer → backend calls go through a single IPC channel (`invoke`) with a `{ name, input }` payload, where names look like `category/action` (e.g. `json/read`).

1. **Definition** (`src/common/invokables/<category>.ts`): an `InvokableDefinition` with zod `inputSchema` / `outputSchema`. It must be added to `ALL_INVOKABLES` in `src/common/invokables/index.ts`, which drives the typing of `invoke()`.
2. **Renderer** (`src/renderer/lib/invoke.tsx`): `invoke(name, input)` calls `window.electronAPI.invoke` (exposed in `src/main/preload.ts`) and validates the result against `outputSchema`.
3. **Main** (`src/main/invokables/index.ts`, `handleInvoke`): first checks the JS invokables in `src/main/invokables/` (Electron-only things such as dialogs and window close). Anything else is forwarded as JSON (`{ func, params }`) to the Rust `invoke`.
4. **Rust** (`src-rust/src/invokables/`): each invokable is a serde-deserializable struct implementing the `Invokable` trait (`name()`, optional `validate()`, `invoke(&AppState)`). It must be registered in `Invokables::new()` in `invokables/mod.rs`. It runs on a Neon task thread and returns a JS promise.

A new backend call therefore touches the common definition, `ALL_INVOKABLES`, and either a main-process JS invokable or a Rust struct plus its registration. Rust and TS schemas are kept in sync by hand.

Main → renderer events work the same way: definitions live in `src/common/events/`, main sends with `sendEventToRenderer` (`src/main/events.ts`) on channel `event/<name>`, and the renderer subscribes via the `Listen` components.

### Rust state

`AppState` (`src-rust/src/state.rs`) is created once through `newAppState` and passed as a boxed handle on every invoke. It holds the toolset config (read from the user config dir, `config.rs`) and the currently opened mod (`OpenedMod`: a `stracciatella` `Vfs` layered over vanilla data plus the mod). JSON schemas and vanilla data come from the `stracciatella` crate's `SchemaManager` and VFS.

### JSON editing model

- A mod can override a vanilla JSON file with a full replacement (`foo.json`) or a JSON patch (`foo.patch.json`, RFC 6902). `json/read` returns `schema`, `vanilla`, and the mod's `value` or `patch`. `json/persist` writes one of the two.
- `src/renderer/state/files.tsx` (Redux slice) tracks `disk` (loaded data, wrapped in `Persistable`/loadable) and `open` (editor buffers). Each open file has a `saveMode` (`patch` | `replace`) and an `editMode` (`visual` | `text`). In visual patch mode, the patch is generated with `fast-json-patch` `compare` against vanilla on save.
- Components read and write state through the many small `useFile*` hooks in `src/renderer/hooks/`, not directly through the store.

### Editor UI

`src/renderer/EditorRoutes.tsx` defines the sidebar `MENU`: each JSON file is mapped via `makeFileItem` to an editor component (`JsonForm`, `JsonItemsForm`, `JsonStrategicMapForm`, or custom forms) plus an RJSF `uiSchema`. Forms are rendered with `@rjsf/antd` from the schema supplied by Rust. Cross-references between files use custom widgets in `components/visual/form/` (`StringReferenceWidget`, `ResourceReferenceWidget`, sector selectors). Adding support for a new JSON file usually means adding a `makeFileItem` entry here.

### Tests

Renderer tests live in `src/__tests__/renderer/`. `window.electronAPI` is stubbed with `vi.fn()` in `src/__tests__/vitest.setup.ts`. Use `InvokeMock` (`test-utils/invoke.tsx`) to declare expected invoke calls and their results, and `renderWithTestProviders` (`test-utils/render.tsx`) to render with a fresh Redux store. Unexpected invoke calls throw. The map viewer has Rust unit tests (`cargo test -p ja2-map-editor`), which run natively; the Neon module has none.

## Project rules (from `.rules`)

- Don't add new dependencies unless asked to.
- Run typechecking, linting, building and tests before committing. Only commit to the current branch.
- Run `npm install` after changing dependencies in `package.json`.
- React: use `useMemo` / `useCallback` for performance.
- Rust: return errors (`anyhow`) instead of panicking.
