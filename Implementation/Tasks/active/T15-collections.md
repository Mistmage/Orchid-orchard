---
topic: collections
type: task
status: active
related:
  - ../../../crates/lilac-core/src/collection.rs
  - ../../../extension/background.js
  - ../../../extension/popup/popup.js
  - ../../../apps/orchard/src/App.tsx
  - ../../Tasks/finished/T3-tab-groups-windows.md
  - ../../Tasks/active/T12-tabs-groups-windows-capture.md
---

# T15 — Collection record model and display

## Goal

A collection is a first-class working set, not a timestamped list of invisible
links. Its entries, their order and the set's metadata are visible and usable in
both the extension and Orchard.

## Current state

- A save writes `{ type: collection, title: "Saved tabs <ISO>", tags: [] }` plus
  a body of `- [[id]]` lines, sometimes with `tab_group` and `window_hint`.
- Opening a collection parses those lines only; raw URLs are tolerated.
- The title is not human, there is no count, no created/modified, no indication
  of the source window, and Orchard shows the record as generic blocks.
- An entry deleted from the collection body silently disappears; there is no
  structured item to hold order, pinning or group membership.

## Requirements

- Define one collection model in `collection.rs`: ordered entries, each with a
  record id (once saved), URL, title, optional group name/colour, pinned, plus a
  source hint (`selection`, `group`, `window`).
- Frontmatter carries the structured set: display title, description, source,
  entry count, group metadata and the ordered entries. The body remains a
  human-readable Markdown list of `[[id]]` links so the file is still useful
  outside Orchard.
- Reading prefers the structured entries and falls back to parsing the body, so
  collections written by older builds still open.
- Writing re-renders the body from the structured entries; an entry the user
  removes is removed from both.
- Display:
  - extension list: human title, entry count, group colour dot, relative date;
  - collection detail: rows with title, URL, group and a remove action; clicking
    a row opens that record;
  - Orchard: a collection view with the same rows instead of raw blocks, with an
    "Open all" action mirrored from the extension.
- Collection operations: add entry, remove entry, reorder, rename, reopen as
  window/group/into a group (T12), and delete with confirmation.
- Deduplicate entries by record id or canonical URL; a duplicate is reported,
  not silently dropped.

## Design notes

The body is the portable form; the frontmatter is the structured cache of the
same thing. They must be written together from one in-memory list, never edited
independently. `parse_collection_body` already handles both refs and bare URLs
and should be the only reader of legacy bodies.

## Done when

- A saved set round-trips entries, order and group metadata through YAML and
  back.
- Legacy collections still list and reopen.
- Removing an entry changes both frontmatter and body.
- Orchard renders a collection as entries, not as a Markdown body.
- Tests cover legacy fallback, duplicate entries and empty collections.

## Open questions

- Is `collection` still the right type name once workspaces exist (T13)? Current
  intent: collections are tab sets; workspaces add geometry and a name.
