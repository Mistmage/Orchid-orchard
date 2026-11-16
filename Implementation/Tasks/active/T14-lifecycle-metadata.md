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

## Goal

Every record carries three timestamps that update themselves, so time-based
queries and sorting work without the user maintaining fields by hand:

- `created` — when the record was created;
- `modified` — when its content or metadata last changed;
- `last_opened` — when it was last opened **as a tab from Lilac**.

## Requirements

- `created` is set once on creation and never rewritten by a normal save.
  Existing records without it are backfilled from file birth time (falling back
  to modification time) in a one-time, non-destructive pass.
- `modified` is updated by every write that changes content or metadata. Reading,
  indexing, previewing and opening do **not** count.
- `last_opened` is updated when Lilac opens or reopens a record as a tab. That
  write updates only `last_opened`; it must not touch `modified`, otherwise
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

`modified` is derived from the file content, but `last_opened` is not, so the
two cannot share one "touch" helper. A dedicated `touch_last_opened` write is
the only way to update the open time; it is also the only writer allowed to skip
`modified`.

Clock skew and same-second edits are handled by storing seconds (or
milliseconds) consistently and only bumping when the new value is greater.

## Done when

- Creating a record writes all three fields.
- Editing a record changes `modified` and not `created`.
- Opening a record as a tab changes only `last_opened`.
- Records written before this change gain `created` without losing fields.
- Sorting by any of the three is correct across mixed timestamp strings.

## Open questions

- Should `last_opened` also be set when Orchard opens a record in its own editor?
  The requirement names Lilac tab opens; keeping Orchard's own opens separate
  avoids ambiguity.
