---
topic: lifecycle-metadata
type: task
status: active
related:
  - ../../../crates/lilac-core/src/record.rs
  - ../../../crates/lilac-core/src/storage.rs
  - ../../../crates/lilac-host/src/lib.rs
  - ../../../extension/background.js
  - ../../Markdown/Design/record-format.md
---

# T14 — Automatic created / last opened / last modified

Shipped:

- `write_record` stamps `created` and `modified` on every save. An existing
  file's `created` wins; `modified` moves only when the payload actually
  changes; identical output is not rewritten.
- `storage::touch_opened` / host `touch_opened` update only `opened`. The
  extension calls it after reopening a record, and for every page a reopened
  collection opens.
- `IndexEntry` carries `created`, `modified` and `opened`; the host index dump
  and Orchard's `RecordView` expose them, and the expanded record shows the
  three values.
- `backfill_lifecycle` fills missing values from file creation and modification
  times without touching the body or unknown fields; Orchard runs it once at
  startup and reports through the WAL. A second run updates nothing.

## Goal

Every record carries three timestamps that update themselves, so time-based
queries and sorting work without the user maintaining fields by hand:

- `created` — when the record was created;
- `modified` — when its content or metadata last changed;
- `opened` — when it was last opened **as a tab from Lilac** (the brief's "last
  opened"; the reserved field name is `opened`).

## Requirements

- `created` is set once on creation and never rewritten by a normal save.
  Existing records without it are backfilled from file birth time (falling back
  to modification time) in a one-time, non-destructive pass.
- `modified` is updated by every write that changes content or metadata. Reading,
  indexing, previewing and opening do **not** count.
- `opened` is updated when Lilac opens or reopens a record as a tab. That
  write updates only `opened`; it must not touch `modified`, otherwise
  merely browsing would make every record look edited.
- Timestamps are ISO 8601 UTC. Comparisons and sorting use the parsed instant,
  not the string, so mixed formats and offsets do not sort wrongly.
- One write path sets them. A caller cannot forget: the storage layer fills
  missing timestamps before writing. Unknown frontmatter fields keep their
  position and value.
- No-op writes are skipped when only the timestamp would change, to avoid
  gratuitous file churn and watcher loops.
- Index entries carry the three values so listing and sorting do not re-read
  files; a rebuild repopulates them from the records.
- Orchard shows the values in the record header and the list can sort by them.

## Design notes

`modified` is derived from the file content, but `opened` is not. `write_record`
ignores `opened` when deciding whether content changed, so `touch_opened` can
use the same writer and still leave `modified` alone. That is why opening needs a
dedicated command rather than a general "touch".

Clock skew and same-second edits are handled by storing seconds (or
milliseconds) consistently and only bumping when the new value is greater.

## Done when

- Creating a record writes all three fields. **Done.**
- Editing a record changes `modified` and not `created`. **Done.**
- Opening a record as a tab changes only `opened`. **Done.**
- Records written before this change gain `created` without losing fields.
  **Done.**
- Sorting by any of the three is correct across mixed timestamp strings.
  Index sort still compares the stored strings; parsed-instant sorting is the
  remaining piece.

## Open questions

- Should `opened` also be set when Orchard opens a record in its own editor?
  The requirement names Lilac tab opens; keeping Orchard's own opens separate
  avoids ambiguity.
