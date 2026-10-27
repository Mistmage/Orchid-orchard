# Orchid Draft — Note App Vision

**Status:** Design only. No code. The "magic paper" note app; Lilac is its first data substrate.

## What Orchid is

Orchid is the **note app** vision: an Obsidian-like editor where everything is blocks, links are queries, and views (canvas, calendar, graph, table) are projections over a shared record store. Described in `Orchid project.md` as "Obsidian clone based around separation of root of files, stem of links and viewing mechanisms."

Core metaphor: **magic paper** — an infinite non-Euclidean canvas whiteboard supporting portals, folding, bending, stretching, layers, unrolling. But magic paper is not a separate app: it's a **view type** over the same orchard (records + edges + metadata). Canvas-first vs. text-first are just different primary views.

## Core concepts (from `Orchid project.md`)

- **Root** — graded access policy (levels 1–7). Universal file editor; any file opens (bytes → text → structured → blocks → flower), parsers optional.
- **Stem** — link/query duality. A link is a query with cardinality 1; same resolution engine, same syntax. Name collisions yield options.
- **Petals** — atomic blocks with stable IDs, metadata, hierarchy; de-render off-screen; cross-file composition (transclusion at sub-block granularity).
- **Leaves** — composable sequential containers; blocks in order; column widgets; a leaf is a cached structure of references, content separate.
- **Flowers** — terminal views with rendering authority (bases, terminals, Excalidraw, clocks, web pages). Test: can arbitrary blocks be inserted into its interior? No → flower.
- **Fruits** — UI as blocks (panels, cards, sidebars, scrollback); rearrangeable, reskinnable chrome.
- **Flavours** — swappable parsers; grape markdown as canonical simplified flavour; footer YAML supported.
- **Wisteria** — browser as a record source (see `Wisteria draft.md`).

## Storage (the hard part)

Plain-text-canonical, block-level incremental writes. The four hard sub-problems (deferred, named in `Orchid project.md`):
1. Block identity in plain text (hybrid: in-band IDs only when referenced, sidecar as cache, content hash for dedup/change detection).
2. Filesystem has no surgical edits (WAL of block ops + periodic compaction; text file remains source of truth).
3. Offset drift under external edits (reparse + reconcile on file change).
4. Preserving untouched bytes (parser must track byte spans per block).

Orchid needs these; Lilac v0.1 deliberately does not (whole-file records).

## How Lilac feeds Orchid

Lilac proves the **record primitive** in the smallest setting: portable files, open metadata, first-class links, file-as-DB, sync via ordinary files. Orchid will reuse:
- The record model (entity/version/event/edge kernel).
- The reference syntax (`[[...]]`).
- The open-metadata principle (unknown fields round-trip).
- The file-as-DB + index-as-cache discipline.

When Orchid arrives, a Lilac record is just a record with `type: page`/`collection`. No migration needed.

## Refined role (from discussion)

- Orchid is the **note-taking feature** on orchard-core, not a separate product.
- Built on Tauri (not Electron). Starts as the GUI for the Lilac host (more valuable than Orchid-as-product initially), then grows note features around it.
- Core must grow block awareness; Lilac benefits from this too.
- Timing: editor/parser can start against a mock store now; storage integration waits for Lilac's block-level evolution.

## Status

Design discussion only. No implementation. Depends on Lilac v0.1 (substrate) plus the four hard storage problems being solved.
