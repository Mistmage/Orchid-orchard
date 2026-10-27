# Phase 4 — Browse & Search

**Goal:** Make saved records findable and editable: a popup with Recent / Collections / Tags / Search, plus metadata editing. This is where Lilac becomes more useful than OneTab (tag search, notes on records).

**Milestone:** M3. **Depends on:** Phases 1–3.

## Scope

In scope:
- Popup navigation: Recent, Collections, Tags, Search views.
- Search syntax (`tag:rust`, `type:page`, `status:active`, free-text over title/url/body).
- Metadata editing (tags, status, project, notes) via popup → `update_record`.
- Tag aggregation view (counts).
- File-watcher / staleness handling for external edits.
- Settings page (records root display, host status).

Out of scope: advanced query language, saved queries/views, graph view (Wisteria territory).

## Popup structure

```
Popup
├── Header: search box + "Save tab" / "Save selected" buttons
├── Tabs: Recent | Collections | Tags | Search
├── Recent: list of recent records (title, url, tags, opened time) with Open/Delete
├── Collections: list (Phase 3)
├── Tags: tag → count, click to filter
└── Search: results for query
```

- Records shown with title (fallback to URL host), favicon (optional, from url), tags as chips.
- Actions per record: Open, Edit, Delete, Copy link (record id / file path).
- Delete confirms; removes file + index entry.

## Search syntax

Minimal, implementation-defined (per spec: "Query syntax remains implementation-defined"). Support:

- `tag:rust` — records with tag rust.
- `type:page` / `type:collection` — by type.
- `status:active` — by status.
- Free text — substring match over title, url, body, tags.
- Combine with spaces = AND (e.g., `tag:rust type:page seaorm`).
- Quoted phrases for multi-word: `tag:"machine learning"`.

Keep it simple and fast (in-memory index filter). No full-text ranking in v0.1; sort by modified desc.

## Metadata editing

- Edit view (modal or inline): title, tags (add/remove), status (dropdown from existing status records or free text), project (ref), notes (body).
- Save via `update_record` with a patch; host writes atomically; index updated.
- Wikilink entry: user types `[[Orchid]]` or picks from existing records; stored as value; normalized on read.
- Body (notes) editable as markdown text.

Principle: editing a record never changes its `id` or filename (frozen slug). Only content changes.

## Tag aggregation

- Tags view groups records by tag with counts.
- Clicking a tag runs `tag:<name>` search.
- Unknown tags (free-text) still appear; no controlled vocabulary in v0.1.

## External edits & staleness

- On popup open and before search: check index staleness (compare index mtime vs records root mtime or a marker). If stale, rebuild index (or prompt).
- Optional: file watcher in host (notify on changes) for live updates — nice-to-have; if not implemented, rebuild-on-focus is enough.
- "This file changed outside Lilac" is not a special state; just re-read from disk.

## Settings page

- Show config root path and records root path.
- "Change records root" (validates path, writes config).
- Host status (connected/disconnected, version).
- "Rebuild index" button.
- "Open records folder" (if OS allows via host; else show path).
- Clear logs.

## Acceptance criteria (M3)

- Popup shows Recent/Collections/Tags/Search; navigation works.
- `tag:rust` returns correct records; free text finds by title/url.
- Edit a record's tags/status/notes → file updated; re-read shows changes.
- External edit (vim) → popup reflects after reopen/rebuild.
- Delete record → file + index entry removed; search no longer returns it.
- Tag view counts match.
- Works in Chrome + Firefox.

## Tasks

- [ ] Popup multi-view UI (Recent, Collections, Tags, Search).
- [ ] Search parser + filter over in-memory index.
- [ ] Record edit UI + `update_record` integration.
- [ ] Tag aggregation view.
- [ ] Staleness check + rebuild-on-focus.
- [ ] Settings page + records root change.
- [ ] File watcher (optional, if time).
- [ ] Cross-browser test.

## Risks

- Service worker lifecycle + popup state: popup is ephemeral; state lives in host/index, not extension. Keep extension stateless (Q1 principle).
- Editing conflicts: if record changed externally between load and save, last-write-wins in v0.1. Acceptable; conflict philosophy favors visible files. Could add mtime check later.
- Search performance on large roots: in-memory filter over index; fine for v0.1 scale.

## Deliverables

- Full browse/search/edit loop.
- M3 demo: save tabs, tag them, search `tag:rust`, edit a note, find it again.
- Settings + rebuild index.
