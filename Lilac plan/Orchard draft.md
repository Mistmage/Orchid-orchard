# Orchard Draft — General Vision (Umbrella Project)

**Status:** Design only. No code. This note captures the umbrella vision so Lilac's plan stays aligned with the long-term shape without inheriting its scope.

## What Orchard is

Orchard is the **umbrella project**: a personal knowledge/workspace OS built on one recursive idea — **everything is a record/block**, and **views are projections** over a shared store. Lilac is the first brick (browser records). Orchid is the note app. Wisteria is the browser. Magic Paper is the canvas view. All are views over the same kernel.

## The through-line (from `Orchid project.md`)

One model, recursive:
- **Root** = access policy (graded permission ladder, level 1–7).
- **Stem** = queries over the record graph (link/query duality: a link is a query with cardinality 1).
- **Petals** = atomic record/block unit (stable IDs, metadata, hierarchy, de-rendering).
- **Leaves** = composable sequential containers (blocks in order; columns).
- **Flowers** = terminal views with their own rendering authority (bases, terminals, web pages, clocks).
- **Fruits** = UI as records (panels, cards, sidebars — rearrangeable chrome).
- **Flavours** = swappable parsers (grape markdown canonical; per-block flavour).
- **Wisteria** = the browser as a record source (web pages as server-authoritative flowers).

## The kernel (the only thing that must be right)

Four things, per the AI review in `Orchid project.md`:
1. **entity** — stable ID, type, creation event, lineage.
2. **version** — content-addressed snapshots.
3. **events** — append-only log per entity.
4. **edges** — typed relations (references, transclusions, isotopes, annotations).

Everything else (views, parsers, UI, browser) is specialization via per-type policy.

## Permission ladder (Root)

Levels 1–7, security as a spectrum with explicit consent:
- 1: single folder fully indexed (Obsidian-like).
- 2: multiple indexed folders that can communicate.
- 3: separate files linkable by system name.
- 4: link-index files/folders outside if inside garden (default).
- 5: link-index outside if not in forbidden list.
- 6: anything linkable (filesystem read-write).
- 7: 6 + more permission (code execution) — different threat model, separate UX.

Lilac maps to a narrow slice: read-write to a user-chosen records root + config root. No code execution. Effectively level 6 scoped to two roots.

## The hard problems (named, deferred)

From the AI review, the genuinely hard sub-problems — all deferred past Lilac:
1. Block identity in plain text (in-band vs sidecar vs content-addressed; hybrid).
2. Filesystem doesn't do surgical edits (WAL + atomic rename + compaction).
3. Offset drift under external edits (reparse + reconcile).
4. Preserving untouched bytes (byte-span tracking in parser).

Lilac v0.1 deliberately avoids these: records are whole-file units (frontmatter + body), written atomically. Block-level surgery is Orchid territory.

## Relationship to Lilac

Lilac is **Orchard's first brick**. It proves the record primitive in the smallest possible setting (browser tabs → files), with plain-text-canonical storage, open metadata, and first-class links. Everything Lilac gets right (records, references, open metadata, file-as-DB, sync via ordinary files) becomes substrate for Orchid and Wisteria. Lilac must not inherit Wisteria's scope ("and stop there").

## Refined architecture (from discussion)

- **orchard-core** is the shared Rust core (records, storage, index, block model, naming, …). Used by Lilac, Orchid, and Wisteria.
- **Orchard** is the app shell / UI that hosts features and controls Lilac.
- **Orchid** = note-taking feature on orchard-core (Tauri, not Electron). Starts as the GUI for the Lilac host, then grows note features.
- **Wisteria** = browser feature, deferred; later embeds a Chromium backend feeding into Orchard's Tauri.
- **Lilac** = browser-record storage. Built now to defer Wisteria with peace of mind; later remains useful as a standalone tool for separate browsers, and is also controlled by Orchard.
- **Magic Paper** = a canvas view type over the same store (not a separate app).

Core grows block awareness (Lilac benefits too). Timing: Orchid storage waits for Lilac's block-level evolution; Orchid editor/parser can start against a mock store now.

## Status

Design discussion only. No implementation. Lilac v0.1 is the first concrete deliverable under this umbrella.
