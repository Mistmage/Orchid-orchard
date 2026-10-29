---
topic: tauri-playwright
type: future
status: deferred
related:
  - docs/orchard-dev.md
---

# tauri-playwright (future)

End-to-end testing for the Tauri webview via Playwright/WebDriver.

Currently overkill (manual smoke + Rust unit tests cover the logic). Revisit when
the frontend grows beyond trivial rendering and regressions become costly.

Adoption criteria:
- Frontend has enough interactive state that manual smoke misses bugs.
- CI needs automated frontend checks.
- A stable Playwright driver for the Tauri webview is available for our version.