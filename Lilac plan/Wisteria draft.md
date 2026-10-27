# Wisteria Draft — Record-Native Browser

**Status:** Far-future design. No code. Wisteria is the browser that makes tabs into records; Lilac's record model is its seed.

## What Wisteria is

Wisteria is a **record-native browser**: tabs, bookmarks, history, sessions, and tab groups collapse into **one store of records** with policies and views. Memory scales down because live instances become a small, policy-controlled set; everything else is cheap records.

Core move (from `Orchid project.md`): **tabs become records in a queryable DB; live instances become a pooled resource; isotopes are explicit forks.** Default policy: one live instance per (site, profile, container, account); opening another reuses/focuses; forking is explicit.

## The record model (seeded by Lilac)

Lilac v0.1 already defines the primitives Wisteria needs:
- `type: page` for a web page (server-authoritative record).
- `type: collection` for tab sessions.
- Reserved fields: `id`, `type`, `version`, display, timestamps, references, classification.
- Wisteria-compat fields reserved in v0.1: `authority`, `lineage`, `forked_from`, `policy`, `state`.
- First-class links (`[[...]]`) and open metadata.

Wisteria additions (future, additive to Lilac's schema):
- `authority: server` for pages (remote owns mutations; local observes via reload/snapshot).
- `lineage` / `forked_from` for isotope tracking.
- `policy: { singleton, auto_snapshot, ttl, ... }`.
- `state: { live, evicted, snapshot_ref, ... }`.
- View definitions (stacks, wheels, timelines, kanban) as saved queries over records.

## Key concepts

- **Record:** URL, title, site, container/account, tags, view count, history events.
- **Live instance:** actual renderer/process tree (pooled, refcounted).
- **View:** tab-strip item / stack slot / wheel slot pointing at a live instance or dormant record.
- **Isotope:** explicit fork — new live instance + new lineage.
- **Snapshot:** local capture (MHTML/WARC) for offline reading; referenced by hash.

## Why Wisteria needs Lilac first

1. **Shared store:** pages and notes are the same kind of object (record with typed content + edges). Lilac proves the file-based record store works cross-browser.
2. **Plain-text export:** records serialize to Markdown+YAML/JSON; snapshots as MHTML/WARC referenced by hash. Lilac's format is the starting point.
3. **Policy engine inputs:** Wisteria's rules operate on record metadata (domain, tag, view count, dirty state). Lilac's open metadata + tags provide that substrate.
4. **Sync:** metadata-level sync (cheap) vs. live-instance sync (none). Lilac's file-based sync model carries over.

## Relationship: Lilac → Wisteria

Lilac is the **storage layer** Wisteria will read and write. Wisteria adds the **browser kernel** (renderer pooling, isotope policy, views) on top. Nothing in Lilac v0.1 should need to change for that future except additive fields and new record types. Lilac must keep its scope small ("and stop there") so it stays a clean substrate.

## Refined role (from discussion)

- Wisteria is the **browser feature** on orchard-core, deferred.
- Later: embed a Chromium backend feeding into Orchard's Tauri (not a standalone engine fork initially).
- Lilac is built now to defer Wisteria with peace of mind; later Lilac remains useful as an additional tool for separate browsers.
- Lilac's record model is the seed Wisteria will read/write.

## Status

Far-future design. Depends on Lilac v0.1 (substrate) + a custom browser engine decision. Not actionable now; tracked here for alignment only.
