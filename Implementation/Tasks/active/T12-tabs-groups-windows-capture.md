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
  whose metadata matches the group name and colour.
- Reopening into an existing group puts every tab in that group and preserves
  order.
- The record contains no browser ids (asserted in a test).
- Excluding an entry in the preview removes it from both the page save and the
  set.

## Open questions

- Should a `window` scope save every group as separate sets, or one set with
  group annotations? Current intent: one set with group annotations, since it
  reopens as one window.
