---
topic: tabs-groups-windows-capture
type: task
status: active
related:
  - ../../../extension/background.js
  - ../../../extension/popup/popup.js
  - ../../../crates/lilac-core/src/collection.rs
  - ../../Tasks/finished/T3-tab-groups-windows.md
  - ../../Tasks/active/T15-collections.md
---

# T12 — Save tabs, tab groups and windows

Shipped:

- Three scopes in `extension/background.js`, all through one `saveTabSet`:
  selected tabs, the active tab's group, and the whole current window. Saving a
  group while the active tab is ungrouped is an error rather than a whole-window
  save.
- Captured set metadata: `saved_from` (`selection`, `group`, `window`), ordered
  `tab_groups` (`{ name, color, collapsed }`), and per-tab `pinned`, `active`,
  `muted`, `group`, `group_color`. A window capture also stores outer
  `left`/`top`/`width`/`height` and `state` in `window_hint`.
- `CollectionEntry` carries `active` and `muted` (both skip-if-false). `audible`
  is deliberately not captured: it is transient, not restorable.
- Reopen accepts `{ id, mode, groupId }`: `group` (default, current window),
  `window` (new window, tabs created in order) and `into` (the user picks a live
  group from `listTabGroups`). Pinned and muted state are restored, the captured
  active tab is focused last, and groups are rebuilt in captured order with
  collapsed state.
- `rebuildGroups` reports `{ grouped, warning }`; the popup shows a degradation
  message instead of silently claiming the shape was restored.
- The index exposes `item_count`, `saved_from`, `group_name` and `group_color`
  so the popup list renders counts and group badges from `search_records`
  instead of reading every file.
- No browser id is stored: `tab_groups` and entries have no id field, group ids
  live only inside the extension, and a test asserts the serialized shape has no
  `window_id`/`tab_id`/`groupId`.

Verified: 147 core + 11 roundtrip + 5 probe + 6 host tests pass, clippy and
`npx tsc --noEmit` are clean, and both extension scripts pass `node --check`.
Live browser behaviour (group capture, window creation, reopen into an existing
group) needs a manual check in Chrome; CDP cannot drive the extension here.

## Goal

A user can capture not just a page but a working set: selected tabs, a whole tab
group, or a whole window, with the metadata that describes the set and enough
information to reopen it in the same shape.

## Requirements

- Save scopes: selected tabs, one tab group, all groups in a window, and one
  window. Each produces a set record (T15) plus page records for its entries.
- Metadata captured **where it was saved from**:
  - tab set: source (`selection`, `group`, `window`), captured time, order,
    pinned, active, muted, audible;
  - tab group: name, colour, collapsed, index in the window;
  - window: outer bounds and state, display scale, tab count, incognito hint;
  - page: title, URL, group name/colour at capture time.
- No `window_id` or `tab_id` is ever written. Browser data is a hint; reopening
  is a reconstruction, not a restore of handles.
- Reopen forms:
  - as a new window;
  - as a new tab group;
  - **into an existing tab group** (the user picks the target group).
  Order and pinned state are preserved where the browser allows it.
- Unsupported combinations degrade visibly: for example, a browser without tab
  groups still reopens the tabs and reports that the group could not be made.
- Preview (T9) lists the set and its entries; entries can be excluded, which
  updates the set record too.

## Design notes

`chrome.tabGroups` is Chromium-only. Feature-detect it, keep the record shape
browser-neutral, and treat group colour as a string hint rather than an enum
owned by one browser.

A window capture is a workspace without a name (T13): share the geometry and
ordering code instead of writing two capture paths.

## Done when

- Saving a group writes one set record whose entries match the group order and
  whose metadata matches the group name and colour. **Done in the extension**;
  manual browser check pending.
- Reopening into an existing group puts every tab in that group and preserves
  order. **Done in the extension**; manual browser check pending.
- The record contains no browser ids (asserted in a test). **Done** — the
  serialized set shape has no `window_id`, `tab_id` or `groupId`.
- Excluding an entry in the preview removes it from both the page save and the
  set. **Blocked on T9**, which owns the preview plan.

## Open questions

- Should a `window` scope save every group as separate sets, or one set with
  group annotations? Current intent: one set with group annotations, since it
  reopens as one window.
