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
6. The previous body is pushed onto an in-memory undo stack (64 entries) and the
   redo stack is cleared.

If the write fails, the session restores the previous body and clears the WAL.
If the process dies between steps 4 and 5, the next open replays the edit.

## Undo and redo

Undo and redo move through the same in-memory list of bodies. Undo pops the
previous body, writes it, and pushes the body it replaced onto the redo stack;
redo pops that stack and pushes the current body back onto the undo stack. Both
check the file fingerprint first, so an undo or redo after an external change is
refused rather than silently overwriting it. A new edit, an insert, a delete or
an external refresh clears the redo path: once history branches, the old redo
text no longer follows from the file. Metadata saves change the frontmatter but
leave both stacks alone, because they do not touch the body.

## Insert and delete

New blocks are raw Markdown, never empty placeholders: the file never contains a
draft line, so a crash cannot lose structure and the parser has nothing to
guess. `insert_after_path()` finds the end of the target's subtree and splices
the text there, so the new block lands after the section rather than in its
first child slot. The declared ranks then decide the final level: a deeper
heading remains inside the section, an equal or shallower one becomes a
sibling. `append_block()` does the same at end of body. Both keep a blank line
between blocks and leave the rest of the file byte-identical.

`delete_at_path()` removes the block span together with its children in one
splice. Deletes use the same positional path plus `expectedContent` guard as
edits, so deleting a block that changed on disk is refused rather than applied
to the wrong text.

WAL entries are `insert` (path of the anchor plus content) and `delete` (path).
If any replay step fails (for example, the anchor path no longer exists), the
session restores the body it read before replay and discards the whole WAL, so a
half-applied sequence never corrupts the document. Insert and delete each push
the previous body onto the undo stack like an edit.

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

## Frontmatter editing

The record view has a `Metadata` panel for `title`, `status`, `url`, `tags` and
`description`. `RecordSession::update_record()` refreshes the file, verifies the
fingerprint the editor started from, changes only the `Record` fields, and writes
with the session's current body. A metadata save therefore cannot roll back a
block edit, and a stale panel cannot overwrite another program's change.
Metadata writes update `modified`; they are not on the block undo stack.

## Limits

- Undo and redo are per session and in memory (64 entries each); they end when
  the record is closed. Insert and delete share the same stacks; metadata changes
  are not on them.
- A WAL is one file per record, not per window. Multiple Orchard windows on the
  same record are not coordinated yet.
- Frontmatter fields outside the five edited ones are preserved but not exposed
  in the panel; edit them in the file.
