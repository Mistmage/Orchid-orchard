# Orchard (Tauri) Development

## Prerequisites

- Rust toolchain (stable) with the MSVC target on Windows.
- Node 20+ and npm (SolidJS + Vite frontend).

## Build & run

```
cd apps/orchard
npm install
npm run tauri dev
```

For a plain debug build of the shell (frontend must already be built):

```
cd apps/orchard
npm run build
cd src-tauri
cargo build
```

Release artifacts:

```
powershell -File scripts/build-release.ps1
```

This produces `dist/orchard.exe` and `dist/lilac-extension.zip`.

## Single executable

`orchard.exe` is both the GUI and the native messaging host. When the browser
launches it, it starts in host mode; the detection lives in
`apps/orchard/src-tauri/src/main.rs`:

- `--host` (explicit),
- `--parent-window...` / `-parent-window...` (Chrome / Firefox),
- stdin and stdout are both pipes (covers launchers that pass no arguments).

There is no separate `lilac-host.exe`.

## Config

Both the GUI and the host use `lilac_host::config::load_config()`, so there is
one source of truth:

- `~/.orchard/.lilac/config.yaml` - records root, registered folders.
- `~/.orchard/.orchid/config.yaml` - Orchid app settings (registered folders,
  extension id).
- Legacy `~/.lilac/config.yaml` is migrated once on first load.

## Architecture

- Rust is sole authority: `lilac-core` provides storage, index, search, block
  model, block type registry, naming, rules, WAL.
- Block types are declared in `crates/lilac-core/src/block_types.rs`. The parser
  asks the registry for syntax and hierarchy; it has no per-type nesting
  branches. `validate_registry()` and `validate_tree()` enforce the invariants.
- The frontend is SolidJS 2.0 RC + Vite (`apps/orchard/src/App.tsx`), a thin
  renderer that invokes Tauri commands and paints the returned block tree.
- No document structure or parsing lives in JS.

## Commands exposed

`list_records`, `search`, `load_record`, `get_blocks`, `apply_block_edit_cmd`,
`records_root_path`, `register_native_host`, `get_block_types`,
`get_settings`, `set_registered_folders_cmd`, `set_records_root_cmd`.

`get_block_types` returns `{ types: [...], valid: bool }` from
`lilac_core::block_types::describe()`. The Settings dialog uses it to render
the registry and its validity.
