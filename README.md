# Orchid Orchard

A local-first personal knowledge/workspace OS built on one recursive idea:
**everything is a record/block, and views are projections** over a shared store.

Lilac is the first brick: a cross-browser extension that stores tabs, bookmarks,
sessions and notes as portable Markdown+YAML records on disk.

## Layout

- `crates/lilac-core` — Rust record storage core (model, YAML, atomic I/O, index, search, collection parsing).
- `crates/lilac-host` — Rust native messaging host (browser ↔ record store).
- `extension/` — MV3 browser extension (Chrome + Firefox).
- `docs/` — record format, native host protocol, extension dev, sync.
- `scripts/` — host install/uninstall (Windows/Linux/macOS).
- `templates/` — `.gitignore` template for a records root.
- `Lilac plan/` — design notes and phase plans.
- `Orchid project.md` — umbrella vision.

## Build

```
cargo build -p lilac-host
cargo test --workspace
```

## Try it

See `extension/README.md` for the end-to-end steps (build → load unpacked →
register host → restart browser).

## Principles

- Files are the database; indexes are caches.
- Browsers are clients; no browser is authoritative.
- Metadata is open; arbitrary user keys survive round-trips.
- Cross-browser: records never store `window_id`/`tab_id`.
- Conflict philosophy: a conflicted file is preferable to a hidden merge.
- Local-first: no telemetry; all data lives in portable files.

## License

MIT (see `LICENSE`).
