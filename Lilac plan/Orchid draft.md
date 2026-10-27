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
- Built on **Tauri + SolidJS** (not Electron). Tauri renders; Rust is sole authority.
- **No JS editor library owns document structure** (no ProseMirror/CodeMirror/Tiptap). Blocks are plain-text-ish with a half-hidden internal hierarchy; Rust owns the buffer, block tree, IDs, and mutations. Frontend is a thin renderer.
  - CodeMirror (minimal: view + basic keymap only) is acceptable as the *input surface* for the active block: input normalization, selection, caret, mouse/keyboard navigation. No markdown parsing, no history, no structure in JS.
- Core must grow block awareness (block identity, surgical edits, byte spans, WAL, external-edit reconciliation); Lilac benefits from this too.
- Block identity is type-aware: headings get IDs eagerly; `---` separators and other blocks only when referenced (lazy); YAML frontmatter IDs eager. Content hash as fallback.
- **Rendering contract is a scene graph, not a command/event protocol.** Rust emits the view tree (the block tree is the scene graph for v0); renderers (Tauri webview, future native) paint it. This avoids a second source of truth and makes renderer swaps trivial. (Think Blitz-style display list.)
- Timing: editor/parser can start against a mock store now; storage integration waits for Lilac's block-level evolution.
- Later, Wisteria does the same pattern (Rust authority, scene-graph renderer). Wisteria later embeds a Chromium backend (Tauri -> Chromium is the progression). "webtau" for static routing between Orchid and Wisteria views — if this is a specific tool, point me at it.

## Implementation status

Scaffolded: `apps/orchard` — Tauri v2 shell (SolidJS-less, vanilla JS renderer) wiring `lilac-core` via Tauri commands (list_records, search, load_record, records_root_path). Rust is sole authority; frontend is a thin renderer. Builds and runs on Windows. Block editor and note features are next.

## Status

Design discussion only. No implementation. Depends on Lilac v0.1 (substrate) plus the four hard storage problems being solved.
