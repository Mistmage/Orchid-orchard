# Lilac Tasks (backlog, not forgotten)

Recorded per user request. Ordering is flexible; all need to be done.

## T1 — Exact folder placement + subtrees

- Records can be placed in a user-chosen folder/subtree, not only `Records/<type>/`.
- `folder`/`path` becomes part of record metadata; `record_path` honors it.
- Index already walks recursively; ensure search/filter by folder.
- Rules (T2) may move records between folders.

## T2 — Rules, templates, formulas (Obsidian Bases-like, more general)

- A rule engine: conditions over record metadata → actions (add metadata, move folder, tag, etc.).
- Formula syntax (not final): e.g. `IF({title}.has(cat)) -> move(folder(cat))`.
- Rules run on save/edit and can be triggered per tab group / domain.
- Templates: new records can be created from a template record.
- Neither syntax nor rules are final; idea only: set rules → metadata can be added, folder placement can change.

## T3 — Tab groups and windows

- Capture tab group metadata (Chrome `tabGroups`): name + color. Store as `tab_group: { title, color }` on the page/collection record.
- Capture window context as hints (per cross-browser rule: hints, not truth).
- Rules can trigger on groups (e.g. title: YouTube).
- Saving a tab group saves metadata and restores the full group as a group; same for window.
- Fix current "save selection" code (slightly off).
- Optional preview of the record about to be saved:
  - Single record only for now.
  - Setting to enable preview of many records later.

## T4 — Settings via GUI (not terminal)

- Extension options page: show records root, host status/version, rebuild index, change records root (already in popup; promote to options page).
- Extension ID registration guidance in-GUI (show ID, link to install script) rather than terminal-only.
- Host status indicator.

## T5 — SolidJS frontend for Orchard

- Use SolidJS (not React) for the Tauri frontend.
- Rust remains sole authority; frontend is a thin renderer fed Rust-originating data (scene graph).
- Minimal CodeMirror (input/selection/caret only) for the active block; no parsing/history/structure in JS.

## T6 — Snapshot capture (Chrome, done prototype; harden)

- Already prototyped via `pageCapture` + `save_asset`. Harden + Firefox equivalent or manual fallback.

## T7 — Naming rules UI

- Per-domain naming templates (affix/prefix/replace) configurable in settings; uses `NamingRule`/`NamingConfig` from core.
