# Orchard (Tauri) Development

## Prerequisites

- Rust toolchain (stable) with the MSVC target on Windows.
- Node is optional (frontend is vanilla JS for now; SolidJS migration planned).

## Build & run

```
cd apps/orchard/src-tauri
cargo run
```

The app reads records from the Lilac records root (`~/.lilac/config.yaml` → `records_root`, default `~/Documents/Lilac`).

## Architecture

- Rust is sole authority: `lilac-core` provides storage, index, search, block model, naming, rules, WAL.
- The frontend (`src/index.html`) is a thin renderer: it calls Tauri commands (`list_records`, `search`, `get_blocks`, `apply_block_edit_cmd`, …) and paints the returned scene graph (block tree).
- No document structure or parsing lives in JS.

## Commands exposed

`list_records`, `search`, `load_record`, `get_blocks`, `apply_block_edit_cmd`, `records_root_path`.

## SolidJS migration (planned)

The frontend will move to SolidJS + Vite, emitting the same Tauri commands. Until then, vanilla JS keeps the no-build workflow.
