---
topic: bases
type: design
status: active
related:
  - ../../../crates/lilac-core/src/base.rs
  - ../../../crates/lilac-core/src/formula.rs
  - ../../../crates/lilac-core/src/storage.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
  - ../../../apps/orchard/src/App.tsx
  - ../../../docs/orchard-dev.md
---

# Bases: the first flower

## What a flower is

A leaf is a composable sequence of blocks: the file's contents, nestable, with
blocks inserted anywhere. A flower is a terminal view with its own rendering
authority: a base, a terminal, a clock, an embedded web page. The test is
whether arbitrary blocks can be inserted into its interior. For a base they
cannot: it owns its table.

## Two file forms

A base can be a record or a standalone file. A `type: base` record lives under
`Records/base/`, has an id, a title, folder and timestamps, appears in the same
index and backup, and can be moved or templated like anything else. A `.base`
file is only the YAML query: no frontmatter, no index entry, no record metadata.
It is the form for a query copied from elsewhere or edited in another program.

Both are flowers: when Orchard opens one, the block editor is replaced by the
base view. The difference is how identity and discovery work, not how the query
runs. Keeping the record form means a user who wants tags on a dashboard does not
need a second metadata system; keeping the file form means a query does not have
to pretend to be a note.

```yaml
id: base_books        # optional
title: Books          # optional; the file stem is the fallback
source:
  type: book
  tag: reading
  folder: Reading
where: pages_total > pages_read
columns:
  - label: Title
    expr: file.name
  - status
sort: "-modified"
limit: 100
```

Scanning `.base` files is separate from `rebuild_index`, because a broken view
must not disappear with a broken index. A file that does not parse is still
listed with its error. The path is confined to `Records/` and must end in
`.base`, so a command cannot read or overwrite an arbitrary file by claiming it
is a flower. Creation writes atomically, like records.

## The body is the query

For a record-backed base, frontmatter stays ordinary metadata, so a base does not
invent a second schema for identity and placement. The body is YAML:

```yaml
source:
  type: book
  tag: reading
  folder: Reading
where: pages_total > pages_read
columns:
  - label: Title
    expr: file.name
  - status
sort: "-modified"
limit: 100
```

This keeps the record contract intact: YAML frontmatter plus a body. It also
means the entire base is readable, diffable and portable without Orchard. The
body is not Markdown in this case, but the parser never sees it as blocks: the
type decides the renderer before block parsing. That is why a flower can own its
interior without violating the "files are the database" rule.

## Filter cheaply, evaluate completely

The index stores id, type, title, tags, status, folder, path and modified. A
base runs its `source` filter against the index first, so a type-scoped base does
not read unrelated files. Only the candidates are read from disk, because
formulas need fields the index does not keep (`url`, open metadata). This is the
same split as the rest of the system: the index is a cache for discovery, the
file is the truth.

`parse_base` validates every expression once. `render_base` then evaluates
`where`, each column and `sort` per record. A shared parsed expression means a
hundred rows do not reparse the query a hundred times.

## Errors are rows, not blanks

A broken cell (division by zero, arithmetic on `"nope"`) becomes an empty cell
and a warning naming the record and column. A broken `where` skips the record and
warns. A broken definition refuses to save. Those three levels are deliberate:

- **definition** errors must stop the save, because the query is the artifact;
- **per-record** errors must not abort the whole view, because one malformed
  record would otherwise hide every good one;
- **warnings** stay visible next to the table instead of disappearing into a
  log.

Sorting is deterministic across types: nulls last, then booleans, numbers,
strings and lists. Numbers sort numerically (`9` before `10`), which is the bug
people notice first in a string-sorted table.

## Read-only by construction

`render_base` only reads. There is no action, no write, no network. A base
cannot mutate the records it projects. This is the flower contract: rendering
authority over its own presentation, not over the store. Later flower types
(terminal, web page) will need explicit capability grants; a base needs only the
query permission, and that is already the same read path the UI uses.

## Limits

- Only one view per base, always a table. Multiple named views and list/kanban
  renderers belong in a later revision of the body schema.
- A `.base` file has no tags, status or folder of its own; put those on a
  `type: base` record if they matter. It is discovered by scanning, not by the
  record index.
- The first column is the only cell that could link back to a record; row
  navigation is not built yet.
- `source` cannot express arbitrary formulas. For those, use `where`; a future
  `source.query` can reuse the same evaluator.
