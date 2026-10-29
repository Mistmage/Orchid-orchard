---
topic: config-paths
type: change
status: active
related:
  - crates/lilac-host/src/config.rs
  - extension/
---

# Config path migration

Config root moved from `~/.lilac/` to `~/.orchard/.lilac/`.

Orchid app config lives at `~/.orchard/.orchid/`.

Both are under a shared `~/.orchard/` umbrella so Orchard can control Lilac
and communicate with it, and so multiple Orchard components share one root.