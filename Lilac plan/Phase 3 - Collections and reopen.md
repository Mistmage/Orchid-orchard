# Phase 3 — Collections + Reopen

**Goal:** OneTab parity for the core loop: **save selected tabs as a collection**, then **reopen** them. This is the milestone where Lilac becomes a usable tab manager, not just a saver.

**Milestone:** M2. **Depends on:** Phases 1–2.

## Scope

In scope:
- `type: collection` records with a body list of record refs and/or raw URLs.
- "Save selected tabs" (and "save all tabs in window") command.
- Reopen flow: collection → URLs → open tabs.
- Popup listing of collections with counts and reopen actions.
- Basic collection metadata (title, tags, notes).

Out of scope: dedup/isotope, smart collections, queries as views, session restore of scroll position.

## Collection record format

```markdown
---
id: rec_collection_01
type: collection

title: ORM Research

created: 2026-10-26T12:00:00Z
modified: 2026-10-26T12:00:00Z

tags:
- rust
- orm
---

- [[SeaORM]]
- [[Diesel]]
- [[SQLx]]
- https://docs.rs/sea-orm
```

Rules:
- Body is a markdown list. Each item is either a wikilink to an existing record (`[[...]]`) or a raw URL.
- When saving selected tabs, each tab becomes a `type: page` record first, then the collection references them by `[[id-or-title]]` (prefer id-based ref to survive renames; display title resolved at read time).
- If a referenced record is deleted, the collection keeps the raw URL as fallback (store both? recommended: store the page record ref; on reopen, resolve ref → url; if missing, skip with a warning). Keep it simple: store refs; reopen resolves; missing refs are reported.
- Collection title defaults to a timestamp or window name; user can rename via metadata edit (Phase 4).

## "Save selected tabs" flow

1. Popup offers "Save selected" (uses `chrome.tabs.query({ highlighted: true, currentWindow: true })`) and "Save all in window" (`chrome.tabs.query({ currentWindow: true })`).
2. For each tab: build `type: page` record, `save_record` → get id.
3. Build `type: collection` record whose body lists `[[<id>]]` for each (or raw URL if save-as-urls mode).
4. `save_record` for the collection.
5. UI: "Saved N tabs to <collection title>" with reopen link.

Options (settings, Phase 4+): "save as records" (default) vs "save as URLs only" (lighter, OneTab-like, no per-tab records). Both valid; record-ref is the Lilac-native form and enables metadata/search later.

## Reopen flow

1. User clicks "Open" on a collection (or a single page record).
2. Resolve collection → list of URLs:
   - For each body item: if `[[ref]]`, load record → url; if raw URL, use as-is.
   - Skip missing records; collect warnings.
3. Open each URL: `chrome.tabs.create({ url, active: false })` (or `browser.tabs.create`), optionally in a new window if "open in new window" chosen.
4. Optionally mark collection `opened: <timestamp>` and `last_viewed`.

Behavior:
- Reopen does not delete the collection (records are durable; "close" ≠ "lost" — see Orbis/Wisteria philosophy, but for Lilac: dormant, not destroyed).
- Reopen of a single `page` record opens that URL.
- Position hints (`position: {window, tab}`) are ignored in v0.1 (hints, not truth).

## Popup UI additions

- Collections list: title, count, tags, created date, "Open" / "Open in new window" / "Delete".
- Empty state: "No saved collections yet."
- Single page records also listed (Recent) with "Open".

## Dedup / isotope (deferred)

No dedup in v0.1: saving the same URL twice creates two records. This is acceptable and matches "close = dormant, not lost." Dedup/isotope semantics belong to Wisteria (policy engine). Lilac may add a simple "this URL already saved" hint later, but not auto-merge.

## Acceptance criteria (M2)

- Select 3 tabs → "Save selected" → one collection record + 3 page records exist; collection body lists all 3.
- Click "Open" on collection → 3 tabs open with correct URLs.
- Collection with a raw URL (no page record) still reopens.
- Delete `index.json` → reopen still works (reads files directly).
- Collection edited in an external editor (add/remove list items) → reopen reflects edits.
- Works in both Chrome and Firefox.

## Tasks

- [ ] Implement collection body parse (list of refs/URLs) in core.
- [ ] Implement "save selected tabs" in extension (tab query + sequential save).
- [ ] Implement collection creation referencing page records by id.
- [ ] Implement reopen resolution + tab creation.
- [ ] Popup UI: collections list, open actions, empty states.
- [ ] Error handling: missing refs, partial failures, report to UI.
- [ ] Cross-browser test (Chrome + Firefox).

## Risks

- Sequential saves during "save selected" (N tabs) can be slow for large selections; acceptable for v0.1; batch API could be added later.
- Service worker may be killed mid-save for very large selections; mitigate by keeping port alive and processing sequentially with awaits.
- Reopen of many tabs at once can be heavy; consider a cap/confirmation for >N tabs (UX, not correctness).

## Deliverables

- Working "save selected tabs → collection → reopen" loop in both browsers.
- M2 demo: save a research session, close browser, reopen from Lilac, all tabs back.
