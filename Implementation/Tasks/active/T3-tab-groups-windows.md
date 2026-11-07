---
topic: tab-groups-windows
type: task
status: active
related:
  - extension/background.js
  - extension/popup/popup.js
---

# T3 — Tab groups and windows

Done: capture group name/color into `tab_group` metadata on save (single +
collection); restore tabs as a group with title/color on reopen; cross-browser
rule respected (hints, not truth). Single-record preview via `previewSingle`
setting.

Remaining (why this is active again): capture window context as hints, and
verify/fix the "save selected" path after the recent folder-routing changes.