# Lilac Plan — General Overview

## What Lilac is

Lilac is **not a browser**. It is a **browser-record storage system**: OneTab + Markdown + Git + cross-browser support, "and stop there."

Browsers (Firefox, Chrome, Edge, Brave, Vivaldi, …) are **clients**. Lilac is the **store**. Every tab, bookmark, session, snippet, and note is a **record** stored as an ordinary file on disk. The filesystem is the database; indexes are caches; nothing is authoritative except the files.

This plan turns the v0.1 spec into an executable roadmap. It is deliberately scoped to v0.1 parity with OneTab plus the storage foundations that make later (Wisteria) growth additive rather than a rewrite.

## Decisions locked (from discussion)

| # | Decision | Chosen |
|---|----------|--------|
| Q1 | Architecture | **A** — Rust native messaging host (owns record store, YAML, FS, index) + thin MV3 TypeScript extension (browser APIs, UI, tab→record mapping). |
| Q2 | MVP capture semantics | **A, aware of B** — metadata-only records in v0.1, but structured so adding an MHTML snapshot later is **additive** (optional `snapshot` asset ref), never a rewrite. Slightly slower to add, much cheaper to evolve. |
| Q3 | Storage layout | **`.lilac/` config folder** (hidden, app-owned, regenerable) + **user-defined records root** (portable, syncable). Not a single `Lilac/` tree. Optionally nested under `.orchard/.lilac/`. |

## Architecture (Q1 = A)

```
┌────────────────────────────┐         native messaging (JSON, length-prefixed)
│  Browser (any)             │ ◄──────────────────────────────────────► ┌──────────────────────────┐
│  MV3 extension (TS)        │                                           │  Rust native host         │
│  - tabs API, UI, popup     │                                           │  - record store (YAML)    │
│  - tab→record mapping      │                                           │  - atomic file I/O        │
│  - connectNative           │                                           │  - index rebuild          │
└────────────────────────────┘                                           │  - search                 │
                                                                         └────────────┬─────────────┘
                                                                                      │
                                                                         ┌────────────┴─────────────┐
                                                                         │  .lilac/   (config root) │
                                                                         │  records root (user dir) │
                                                                         └──────────────────────────┘
```

- The extension never touches the filesystem; Chrome MV3 forbids it. The native host is mandatory regardless of language — Rust is chosen per project direction ("we are rust").
- Tab→record mapping lives in the extension (it has the tab objects); the host owns persistence and search.

## Storage layout (Q3 = `.lilac` + user-defined records root)

Two roots, deliberately separated:

```
~/.lilac/                      (or ~/.orchard/.lilac/)   ← CONFIG ROOT (hidden, app-owned, regenerable, machine-specific)
├── config.yaml                # records root path, prefs, native host version
├── index.json                 # disposable cache; rebuildable from records root
├── logs/                      # helper logs
└── tmp/                       # atomic-write scratch

<records root>/                ← USER-DEFINED (portable, syncable, human-readable)  default: ~/Lilac  (or chosen folder)
├── Records/
│   ├── page/
│   ├── collection/
│   ├── bookmark/
│   ├── snapshot/              # reserved; unused in v0.1
│   ├── note/
│   └── snippet/
└── Assets/                    # referenced by records (snapshots, images); content-addressed preferred
```

**Why split:** config is machine-specific and regenerable (indexes, prefs, logs) — safe to gitignore or lose. Records + assets are the actual user data — syncable, diffable, editable in any editor. This matches "browser storage contains only UI preferences; all user data lives in portable files" and "indexes are caches."

**User-defined records root:** the records folder path is stored in `.lilac/config.yaml` and is fully user-configurable (default suggested: `~/Lilac`). Users may point it at a Syncthing/Git folder, an Obsidian vault subfolder, etc. Multiple profiles = multiple `.lilac` configs pointing at different roots.

## Record model (recap)

A record = **YAML frontmatter** + **optional markdown body**.

- **Types:** `page`, `collection`, `bookmark`, `snapshot`, `note`, `snippet`.
- **Reserved fields:** identity (`id`, `type`, `version`); display (`title`, `icon`, `color`, `description`); timestamps (`created`, `modified`, `opened`, `last_viewed`); references (`parent`, `related`, `links`); classification (`tags`, `categories`, `status`).
- **Open metadata:** any user key round-trips untouched (`rating`, `author`, `foo`, `my_company`, …).
- **References:** values may be wikilinks (`[[Reading]]`) or plain strings (`reading`); property names are implicitly records. Pragmatic on-disk form: `status: [[Reading]]` (quoted where YAML requires — `[[...]]` is a flow indicator risk; Lilac normalizes on read).
- **Wisteria-compat (reserved, no-op in v0.1):** `authority`, `lineage`, `policy`, `state`, `forked_from`.
- **Snapshot-awareness (Q2):** `snapshot` (asset ref) + `snapshot_format` reserved on `page` records; absent in v0.1, populated additively later. Assets referenced by relative path or content hash, never embedded in the record.

## Phases

| # | Phase | Milestone |
|---|-------|-----------|
| 1 | Record core (Rust) — model, YAML, atomic I/O, index rebuild | M1 |
| 2 | Native host + save current tab | M1 |
| 3 | Collections + reopen | M2 |
| 4 | Browse / search / metadata edit | M3 |
| 5 | Forward-compat & later (Wisteria fields, snapshot type, sync hardening) | — |

- **M1 — "tab → file":** save a tab, get a readable Markdown file. (Phases 1–2)
- **M2 — "session → reopen":** save selected tabs as a collection, reopen as tabs. (Phase 3)
- **M3 — "find & edit":** search by tag, edit metadata. (Phase 4)

## Cross-cutting concerns

- **Sync:** Lilac never syncs. Files are ordinary; git/syncthing/dropbox/etc. work. Conflict philosophy: a conflicted file is preferable to a hidden merge; users inspect with ordinary tools.
- **Cross-browser rule:** records never store `window_id`/`tab_id`; only `url`/`title` (plus user metadata). Firefox/Chrome/Edge read the identical file.
- **Native messaging:** Chrome uses length-prefixed JSON over stdin/stdout; Firefox uses the same with a manifest. Host must be registered per-OS (Windows: registry; Linux/macOS: manifest in native-messaging-hosts dir).
- **Privacy:** no telemetry; all data local; records root is user-chosen (can be encrypted volume).
- **Extensibility:** unknown record types and unknown fields must survive round-trips untouched (open metadata principle).

## Risks / open questions

1. **YAML wikilink validity:** `status: [[Reading]]` unquoted may be parsed as a flow sequence. Needs verification in the Rust YAML library; fallback is quoted form `status: "[[Reading]]"` with normalization on read. (Phase 1 task.)
2. **File naming churn:** title edits must not rename files (would break links/sync). Resolve with frozen slug + immutable id.
3. **Index drift under external edits:** file watcher → reparse → reconcile. Debounce; mark "changed outside." (Phase 1/4.)
4. **MV3 service worker lifetime:** native port connections and message handling must tolerate worker restarts.
5. **Cross-platform native host registration:** Windows registry vs. Unix manifest files — needs install/uninstall scripts.

## Success criteria (v0.1 done when)

- Save current tab → a human-readable `.md` record exists in the records root, openable in any editor.
- Save selected tabs → a `type: collection` record whose body lists the saved records/URLs.
- Reopen a collection → tabs open in the browser.
- Search `tag:rust` (and similar) returns matching records.
- Delete `index.json` → rebuild restores full searchability.
- A record edited in vim/Obsidian survives a Lilac round-trip with user fields intact.
- Same records root works across at least two browsers (Firefox + Chrome) without conversion.

## Related notes

- `Phase 1 - Record core.md`
- `Phase 2 - Native host and save tab.md`
- `Phase 3 - Collections and reopen.md`
- `Phase 4 - Browse and search.md`
- `Phase 5 - Forward-compat and later.md`
- `Orchard draft.md`, `Orchid draft.md`, `Wisteria draft.md`
- Source specs: `../Orchid project.md`, `../Lilac tab recorder/Lilac 0.1 spec.md`
