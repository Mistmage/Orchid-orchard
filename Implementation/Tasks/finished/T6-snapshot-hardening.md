---
topic: snapshot-hardening
type: task
status: finished
related:
  - extension/background.js
  - extension/content.js
  - crates/lilac-host/src/main.rs
---

# T6 — Snapshot capture hardening

Done: Chrome path via `pageCapture.saveAsMHTML` + content-addressed asset
storage. Cross-browser fallback via content script (`extractHtml`) saving HTML.
Host `save_asset` method.