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

Shipped:

- `CollectionEntry` in `collection.rs`: `{ id, url, title, group, group_color,
  pinned }` plus key-based de-duplication. `collection_entries` prefers
  `record.items` and falls back to the legacy body; `set_collection_entries`
  updates the list and renders the body from it in one step.
- `Record.items` is a typed, skip-if-empty frontmatter list, so it round-trips
  like any other field.
- The extension builds one entry per saved tab (title, group, colour, pinning),
  writes the collection body from those entries, and names the set after the tab
  group or the local save time. Reopening prefers `items`, fills missing URLs
  from page records, creates tabs in order with pinning, and re-groups by
  `tab_group` or per-entry group.
- The index carries `item_count`; the popup shows the count and group, and
  Orchard's list shows the count and its expanded collection shows the ordered
  entries with group badges via `collection_items_cmd`.

Verified: 147 core + 10 roundtrip + 5 probe + 5 host tests pass, clippy and
`npx tsc --noEmit` are clean, and a live CDP run against a throwaway records root
confirmed the structured count, ordered entries with group badges, the legacy
body fallback with titles filled from page records, and an `item_count` of 2 for
a collection that has entries only in its body.

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
  back. **Done.**
- Legacy collections still list and reopen. **Done.**
- Removing an entry changes both frontmatter and body. **Not yet** — there is no
  collection entry editor command/UI; entries are readable and reopenable but
  not yet removable outside the extension's body text.
- Orchard renders a collection as entries, not as a Markdown body. **Done** for
  display; the raw block editor remains below the entry list for body edits.
- Tests cover legacy fallback, duplicate entries and empty collections.
  **Partly** — legacy fallback, duplicate de-duplication and ordering are
  covered; empty-body count is covered by the legacy path.

## Remaining

- Add/remove/reorder commands that mutate `items` and re-render the body from
  one list, plus row controls in Orchard and the popup.
- Hide the raw block editor for collections once the entry editor can add and
  edit a page, so a collection is never edited as two representations.

## Open questions

- Is `collection` still the right type name once workspaces exist (T13)? Current
  intent: collections are tab sets; workspaces add geometry and a name.
