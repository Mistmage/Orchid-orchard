---
topic: record-session
type: design
status: active
related:
  - ../../../crates/lilac-core/src/session.rs
  - ../../../crates/lilac-core/src/block.rs
  - ../../../crates/lilac-core/src/wal.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
  - ../../../apps/orchard/src/App.tsx
---

# Record sessions: path edits, WAL, external changes

## Why a session

Editing a document is not a single write. The editor needs a stable view while
the user types, crash recovery for an edit that was started but not committed,
and a rule for what happens when another program changes the same file (the
Lilac host, a sync client, an external editor). `RecordSession` owns those
concerns so Tauri commands stay thin and the frontend never parses Markdown.

## Position paths, not ids

The frontend addresses a block as `[0, 2]`: first root block, third child. Every
block is editable this way, including leaves that deliberately have no id
(paragraphs, list items). Ids remain for stable references such as headings and
frontmatter, but no UI code may assume `id!`.

Because a path is positional, it can drift when the file changes. The edit
command therefore also receives `expectedContent`, the text the editor started
from. The session refreshes from disk, compares the block at the path, and
refuses the write if it differs. The UI then reloads instead of overwriting the
wrong block.

## Write path

1. `refresh_external()` reads the file and compares a FNV-1a fingerprint.
2. If it changed, the session re-reads it and reconciles ids by content hash.
3. The new text is built with `apply_block_edit_at_path()` (surgical splice;
   everything outside the block span stays byte-identical).
4. A `begin` entry with the base fingerprint and an `edit` entry with the path
   and content go to the WAL and are flushed.
5. The record is written; the fingerprint is refreshed and the WAL is truncated.
6. The previous body is pushed onto an in-memory undo stack (64 entries).

If the write fails, the session restores the previous body and clears the WAL.
If the process dies between steps 4 and 5, the next open replays the edit.

## Opening and WAL safety

`RecordSession::open()` reads the file, fingerprints it, and replays WAL entries
only when the first entry's `base` matches the current file. A WAL from another
version is never applied: it is renamed to `*.orphan-<timestamp>.log` and the
open report tells the UI. This avoids resurrecting an edit that belonged to
text which no longer exists.

WAL files live under `<config_root>/wal/<record-id>.log`; ids are sanitised so
they cannot escape that directory.

## External changes

`get_blocks` calls `refresh_external()` every time, so switching to a record or
reopening it picks up host/sync edits and reports `external_changed`. An edit
also refreshes first. Reconciliation preserves ids where content matches, so
stable references survive a rewrite by another process.

## Limits

- Undo is per session and in memory; redo is not implemented.
- A WAL is one file per record, not per window. Multiple Orchard windows on the
  same record are not coordinated yet.
- Frontmatter is not part of the body session; metadata editing is a separate
  surface.
