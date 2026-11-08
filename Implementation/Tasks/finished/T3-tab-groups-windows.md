---
topic: tab-groups-windows
type: task
status: finished
related:
  - extension/background.js
  - extension/popup/popup.js
  - docs/record-format.md
---

# T3 — Tab groups and windows

Shipped:

- `tab_group` (`{ title, color }`) is captured on single, snapshot and
  collection saves, and restored as a real browser group when a collection is
  reopened. Cross-browser rule respected: hints, not truth.
- `window_hint` context is captured as non-authoritative hints:
  `tab_count`, `tab_index`, `pinned`, `incognito` per page, plus
  `selected_count` on collections. No `window_id`/`tab_id` is ever written.
- "Save selected" now saves exactly the highlighted tabs (filtered to
  `http(s)`), and refuses an empty selection instead of silently falling back
  to every tab in the window.
- The chosen page folder applies only to the saved pages. The collection keeps
  its own `Records/collection` location instead of being buried in the page
  folder.
- Single-record preview via the `previewSingle` setting.

Verified with a mocked-browser harness: folder routing, hint contents (no ids),
collection body linking only the saved pages, and the empty-selection refusal.
The host round-trip was checked against real files: `window_hint` survives
YAML and `load_record`, and the collection lands in the collection folder.
