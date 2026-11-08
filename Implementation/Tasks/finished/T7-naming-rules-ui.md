---
topic: naming-rules-ui
type: task
status: finished
related:
  - crates/lilac-core/src/naming.rs
  - crates/lilac-host/src/lib.rs
  - apps/orchard/src/App.tsx
  - extension/background.js
  - extension/popup/popup.js
---

# T7 — Naming rules UI

Shipped:

- `NamingRule` / `NamingConfig` engine in `crates/lilac-core/src/naming.rs`, plus
  `DomainNaming` overrides and `NamingConfig::starter()`.
- Per-domain matching: `rule_for_host()` ignores case, a trailing dot and
  `www.`, matches an exact host or a `*.example.com` subtree, and the first
  match wins. A matched override replaces the global rules for that host.
- Settings → Naming tab: editable global rule cards (strip, replace,
  prefix/suffix templates, max length) and domain override cards, plus a live
  preview against a title and URL.
- Persistence in `~/.orchard/.lilac/config.yaml` under `naming`; fresh configs
  get the starter rule (strips ` - GitHub`, ` | Hacker News`, ` - YouTube`,
  ` - Stack Overflow`; `max_len: 120`).
- Host `derive_title` uses the saved config when no `rule` is supplied, so
  Orchard Settings is authoritative. An explicit legacy `rule` still wins.
- The extension popup now points at Orchard Settings and sends no rule, so
  "Auto-name all" uses the same config.

Verified live: starter rules load, a `github.com` override and
`*.example.com` wildcard save and reload, preview returns `GH: Repo - GitHub`
for GitHub and the stripped title elsewhere, and the host returns
`GH: …` / stripped / `LEGACY: …` for config / global / explicit-rule messages.
