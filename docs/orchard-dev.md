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

This produces `dist/orchard.exe` and `dist/lilac-extension.zip`. The script also
regenerates the multi-size application icon with `scripts/make-icon.ps1`; the
icon is embedded in the exe (window/taskbar) and used in the app UI.

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

Defaults are the standard record types: `page`, `collection`, `bookmark`,
`snapshot`, `note`, `snippet`. They are created under
`<records_root>/Records/<folder>` on load. A config that cannot be parsed (for
example the old installer's concatenated file) is salvaged for its last
`records_root` value and rewritten cleanly; folder entries that are absolute or
contain `..` are ignored.

## Architecture

- Rust is sole authority: `lilac-core` provides storage, index, search, block
  model, block type registry, naming, rules, WAL and record sessions.
- Block types are declared in `crates/lilac-core/src/block_types.rs`. The parser
  asks the registry for syntax and hierarchy; it has no per-type nesting
  branches. `validate_registry()` and `validate_tree()` enforce the invariants.
- The frontend is SolidJS 2.0 RC + Vite (`apps/orchard/src/App.tsx`), a thin
  renderer that invokes Tauri commands and paints the returned block tree.
- No document structure or parsing lives in JS.
- `RecordSession` (`crates/lilac-core/src/session.rs`) owns the open document:
  it replays WAL edits on open, detects and reconciles external changes, applies
  edits by position path (`[0, 2]`, never `id!`), keeps undo history, and
  refuses an edit when the addressed block's content no longer matches what the
  editor started from.

## Commands exposed

`list_records`, `search`, `load_record`, `get_blocks`,
`apply_block_edit_cmd`, `insert_block_cmd`, `delete_block_cmd`,
`undo_block_edit_cmd`, `records_root_path`,
`register_native_host`, `get_block_types`, `get_settings`, `open_folder`,
`set_registered_folders_cmd`, `set_records_root_cmd`, `set_naming_cmd`,
`preview_naming_cmd`.

`get_block_types` returns `{ types: [...], valid: bool }` from
`lilac_core::block_types::describe()`. The Settings dialog uses it to render
the registry and its validity.

`get_blocks` returns a session view:
`{ blocks, body, recovered_edits, orphaned_wal, external_changed }`. The tree is
re-read from the session and external changes are reconciled on every call.
`apply_block_edit_cmd` takes `{ id, blockPath, newContent, expectedContent }`;
`expectedContent` is the text the editor started from and guards against
positional paths drifting after an external edit. `undo_block_edit_cmd` takes
`{ id }` and returns the same view.

`insert_block_cmd` takes `{ id, afterPath, content }` and inserts raw Markdown.
`afterPath: null` appends to the body; a section path inserts after its whole
subtree, so the new block is placed after the section rather than inside its
first child slot. Final hierarchy still follows the declared ranks: a deeper
heading stays inside the section, an equal or shallower one becomes a sibling.
`content` is complete Markdown (`## Heading`, `- item`, a fenced block), which
keeps empty blocks impossible: there is no draft state in the file.
`delete_block_cmd` takes `{ id, blockPath, expectedContent }`; it removes the
block and its children, and the same `expectedContent` guard refuses a delete
when the block changed on disk. The UI composes new blocks from a type select
(paragraph, headings, bullet, quote, code, divider) plus text.

`get_settings` includes the live `naming` config. `set_naming_cmd({ naming })`
saves it to `config.yaml`; `preview_naming_cmd({ naming, title, url })` derives a
title without writing a record.

## Naming

`lilac_core::naming` is the single naming engine. `NamingConfig` holds ordered
global `rules` plus `domains` overrides. `rule_for_host` normalises the host
(lowercase, trailing dot and `www.` removed) and returns the first match;
`github.com` matches exactly, `*.example.com` matches the root and subdomains.
A matched override replaces the global rules rather than adding to them.

The host's `derive_title` uses the saved config when the message has no `rule`,
so Settings is authoritative for the extension. An explicit `rule` from an
older client is still honoured.
