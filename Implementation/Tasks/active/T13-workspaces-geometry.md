---
topic: workspaces-geometry
type: task
status: active
related:
  - ../../../extension/background.js
  - ../../../crates/lilac-core/src/workspace.rs
  - ../../../crates/lilac-core/src/collection.rs
  - ../../Tasks/active/T12-tabs-groups-windows-capture.md
  - ../../../docs/orchard-dev.md
---

# T13 — Workspaces and named windows, reopen at coordinates

Shipped:

- `type: workspace` records with a title, description, ordered `items`,
  `tab_groups` and a typed `geometry` (`left`, `top`, `width`, `height`,
  `state`, `display_scale`, `display` work area). All geometry fields are
  optional; a browser that reports none still produces a usable record.
- Capture: the popup's **Workspaces** tab saves the current window under a name,
  and **Update** replaces an existing workspace from the current window. New
  tabs get page records; tabs whose URL is already in the workspace reuse the
  existing page record, so updating does not duplicate pages.
- Geometry rules live in `crates/lilac-core/src/workspace.rs` and are exposed to
  the extension as the native-host methods `describe_display` and
  `place_window`. `describe_display` records the work area and scale of the
  display a window overlaps most, never its id. `place_window` clamps a captured
  rectangle to a currently visible display: it shrinks an oversized window,
  moves a partly off-screen window into view, centres a window that touches no
  display, and returns `{ clamped, reason }`.
- Reopen defaults to a new window at the clamped bounds. `maximized` and
  `fullscreen` are applied after the tabs exist (so the window does not jump),
  `minimized` is not restored, and `locked-fullscreen` maps to `fullscreen`.
  A refused position retries without bounds; missing display information uses
  the raw bounds. Every degradation is reported in the popup.
- `group` and `into` reconnect the workspace as a tab group in the current
  window, sharing `openTabSet` with collections so the two cannot drift.
- No window, tab, group or display id is stored; a round-trip test asserts the
  serialized shape contains none of them.
- Orchard shows a workspace with the same entry list as a collection, plus a
  geometry badge (`1440×900 at 40,30 · maximized`). The index carries
  `geometry` so the popup list can show bounds without reading files.

Verified: 156 core + 12 roundtrip + 5 probe + 7 host tests pass; clippy, `npx
tsc --noEmit` and `node --check` on both extension scripts are clean. Live
Chrome behaviour (window creation at clamped bounds, restoring into a group,
display scale) still needs a manual check; CDP cannot drive the extension here.

## Goal

A workspace is a named window state: its tabs, its groups, and the geometry it
was captured with. Saving one preserves enough to reopen the same layout on the
same or another display.

## Requirements

- Save the current window as a workspace, or update an existing named workspace.
  A workspace has a stable identity, a human name and a description.
- Capture geometry: outer `x`, `y`, `width`, `height`, window state
  (normal/maximized/fullscreen), display scale, and the display it was on. Use
  outer bounds so reopening does not depend on browser chrome height.
- Capture layout: tabs with order, pinned, group name/colour, group collapsed,
  and group order. Entries reuse the tab-set model (T12/T15) instead of a second
  list format.
- Reopen:
  - as a new window at the captured coordinates;
  - clamped to a currently visible display, because the original monitor may be
    gone or resized;
  - as a tab group inside an existing window when coordinates make no sense;
  - state is applied after tabs are created, so a maximised window does not
    jump.
- If the browser does not expose window positioning (or the OS refuses it), the
  window still opens and the result reports that geometry was not applied.
- No window ids are stored; a workspace is a file-backed description.

## Design notes

Coordinates are hints like every other browser field: an OS may move the window.
Store them, try to apply them, never fail the open because of them. A hidden
"reopen where it was last opened" can be added later using the same fields.

## Done when

- Saving a window writes one workspace record whose tab order, groups and bounds
  match the live window (within the browser's reported values). **Done**; live
  Chrome check pending.
- Reopening creates the tabs in order and reports the group result. **Done**;
  live Chrome check pending.
- Bounds that fall outside every display are clamped, with a warning. **Done**
  in `place_window`, with unit tests and a host test.
- The record contains no browser window or tab ids. **Done** — a round-trip
  test asserts it, and displays are described rather than identified.

## Open questions

- Are workspaces a subtype of `collection` or their own `workspace` type?
  **Resolved:** their own type, since geometry and naming are not collection
  concerns. Collections and workspaces already share the entry model and the
  open path.
- Should display scale be captured? **Resolved:** yes, via `system.display`;
  it is stored as a hint and used by `describe_display`. Reopening does not
  depend on it because bounds are already in the browser's coordinate space.
