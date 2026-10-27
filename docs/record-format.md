# Lilac Record Format

Canonical reference for the on-disk record format. The filesystem is the
database; indexes are caches. Everything must be reconstructable from files
alone.

## File layout

```
<records root>/
├── Records/
│   ├── page/
│   ├── collection/
│   ├── bookmark/
│   ├── snapshot/
│   ├── note/
│   └── snippet/
└── Assets/          # referenced by records (snapshots, images); content-addressed preferred
```

Config lives separately in the config root (`~/.lilac/`): `config.yaml`, `index.json`, logs.

## Record file

A record is **YAML frontmatter** + **optional Markdown body**, delimited by `---`:

```markdown
---
id: rec_89a13f2
type: page

title: SeaORM

url: https://sea-orm.org

created: 2026-10-26T12:00:00Z
modified: 2026-10-26T12:30:00Z

tags:
- rust
- orm

status: '[[Reading]]'

project: '[[Orchid]]'
---

Research candidate.
```

## Reserved fields

- Identity: `id`, `type`, `version`
- Display: `title`, `icon`, `color`, `description`
- Timestamps: `created`, `modified`, `opened`, `last_viewed` (RFC3339)
- References: `parent`, `related`, `links`
- Classification: `tags`, `categories`, `status`
- Page: `url`
- Browser metadata (optional, ephemeral, treated as hints): `open`, `position`
- Wisteria-compat (reserved, no-op in v0.1): `authority`, `lineage`, `forked_from`, `policy`, `state`
- Snapshot-awareness: `snapshot` (asset ref), `snapshot_format`, `snapshot_created`
- Views (future): `views`

All other keys are **open metadata** and round-trip untouched.

## References / wikilinks

Values may be wikilinks or plain strings:

- Preferred: `status: '[[Reading]]'`
- Allowed: `status: reading`
- Preferred: `project: '[[Orchid]]'`
- Allowed: `project: Orchid`
- Preferred: `tags: ['[[Rust]]']`
- Allowed: `tags: [rust]`

Property names are implicitly records internally (`status:` ≡ `[[status]]:`). On disk, plain keys are used (YAML-safe, Obsidian-compatible).

**Wikilink values must be quoted** on disk (`'[[Reading]]'`) because unquoted `[[...]]` is a YAML flow indicator. Lilac's reader accepts both; writer emits quoted form.

## Record types

`page`, `collection`, `bookmark`, `snapshot`, `note`, `snippet`. Types are conventions, not hardcoded.

## Collection body

A collection stores its members as a Markdown list in the body:

```markdown
---
type: collection
title: ORM Research
---
- [[rec_seaorm]]
- [[rec_diesel]]
- https://sqlx.rs
```

Each item is a record ref (`[[id]]`) or a raw URL.

## File naming

`<slug>--<id>.md`, slug frozen at creation from title (or URL host). Directory: `Records/<type>/`.

## Index

`<config root>/index.json` — disposable cache. Deleting it must never destroy information; `rebuild_index` restores it from files.

## Principles

- Files are the database; indexes are caches.
- Browsers are clients; no browser is authoritative.
- Metadata is open; arbitrary user keys survive round-trips.
- Cross-browser: never store `window_id`/`tab_id`; only `url`/`title`.
- Conflict philosophy: a conflicted file is preferable to a hidden merge.
