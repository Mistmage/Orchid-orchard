---
topic: settings-via-gui
type: task
status: finished
related:
  - extension/popup/popup.js
  - extension/popup/popup.html
  - apps/orchard/src-tauri/src/lib.rs
---

# T4 — Settings via GUI

Done: extension popup Settings view (records root, registered folders, default
save folder, preview toggle, rebuild index, check conflicts, extension ID).
Orchard app now has a real Settings dialog (gear button): icon picker, records
root, registered folders, extension ID + "Register native host". Registration
points the browser at orchard.exe itself (single executable, auto-detected host
mode) - no separate lilac-host.exe and no .bat wrapper.