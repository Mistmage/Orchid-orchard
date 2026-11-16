---
topic: editor
type: design
status: active
related:
  - ../../../apps/orchard/src/App.tsx
  - ../../../crates/lilac-core/src/editor.rs
  - ../../../crates/lilac-core/src/block_types.rs
  - ../Tasks/active/T8-block-type-system.md
  - block-type-system.md
---

# Editor shape: text first, blocks as affordances

## The long-term view

The editor is a **text editor** for the record's Markdown body. The user types
and selects text as in any Markdown editor. Blocks are not the input model; they
are a *reading* of the text that lets the UI place affordances:

- a margin button on a block's first line (insert, delete, change type, open
  metadata);
- per-type actions only where they make sense (a code block gets a language
  control, a divider gets none, a heading gets a level control);
- block metadata shown next to the block, not as a form that replaces the text.

Structural operations still go through core: inserting a sibling, deleting a
subtree, changing a heading level are edits to the text, computed from the
declared hierarchy (T8), never from a parallel block tree that can drift from the
file.

## The current block editor is a scaffold

The present block editor — one editable surface per block, a composer for
insertion — exists to make editing work at all while the structural model is
built. It is an interim shape, not the target. Consequences:

- do not add features that only make sense if blocks are the input model;
- do not let a block object become the source of truth for content;
- keep every block operation expressible as a text edit plus a re-parse;
- UI state (which block is selected, what the margin shows) must be recoverable
  from text position.

This is why T8 declares types and hierarchy in the registry and why insertion
inserts raw Markdown: both survive the move to a text-first surface unchanged.

## Non-goals

- A pure block editor (Notion-style) is not the destination.
- Rich-text/WYSIWYG is not required; Markdown remains visible and editable.
- The margin buttons do not own content. Removing the block UI must leave a
  valid, editable file.
