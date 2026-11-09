---
topic: exact-folder-placement
type: task
status: active
related:
  - crates/lilac-core/src/storage.rs
  - apps/orchard/src-tauri/src/lib.rs
  - apps/orchard/src/App.tsx
  - extension/popup/popup.js
---

# T1 — Exact folder placement + subtrees

Shipped:

- `folder` field on records; `record_path` honors it (sanitized, no traversal);
  rules' `move` action sets it; the browser popup and settings expose it.
- `normalize_folder` is the single user-input rule: blank means the type folder,
  nested `a/b` is allowed, backslashes are accepted, and absolute paths,
  `.`/`..` segments and drive/stream separators are rejected instead of being
  silently rewritten.
- `move_record_to_folder` writes the destination before removing the source and
  clears a `folder` equal to the record's own type folder. `find_record_by_id`
  lives in core so moving and opening use the same lookup.
- `move_records_cmd` handles one record or a batch; it validates the target
  once, reports `{ moved, errors }`, and drops the stale editor session for each
  moved record.
- The list view has per-record selection, select-all, a move bar with
  registered-folder suggestions, and a `Records/...` folder badge. The expanded
  record has a `Move…` action.
- `IndexEntry.folder` exposes placement in list/search without re-reading files.

Remaining:

- Present nested folders as a tree in the list view (the list is still flat,
  with the full folder path shown as a badge).
- Drag-and-drop placement and a folder picker that can create folders inline.
