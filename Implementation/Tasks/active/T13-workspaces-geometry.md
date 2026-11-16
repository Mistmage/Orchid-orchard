---
topic: workspaces-geometry
type: task
status: active
related:
  - ../../../extension/background.js
  - ../../../crates/lilac-core/src/collection.rs
  - ../../Tasks/active/T12-tabs-groups-windows-capture.md
  - ../../Markdown/Design/windows.md
---

# T13 — Workspaces and named windows, reopen at coordinates

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
  match the live window (within the browser's reported values).
- Reopening creates the tabs in order and reports the group result.
- Bounds that fall outside every display are clamped, with a warning.
- The record contains no browser window or tab ids.

## Open questions

- Are workspaces a subtype of `collection` or their own `workspace` type? Current
  intent: their own type, since geometry and naming are not collection concerns.
