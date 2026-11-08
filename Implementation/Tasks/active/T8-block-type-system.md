---
topic: block-type-system
type: task
status: active
related:
  - ../../../crates/lilac-core/src/block_types.rs
  - ../../../crates/lilac-core/src/block.rs
  - ../../Markdown/Design/block-type-system.md
  - ../../Markdown/Fact/block-dividers.md
---

# T8 - Declared block type system and ranked dividers

Shipped:

- `crates/lilac-core/src/block_types.rs`: declared `BlockTypeDef` registry with
  `Syntax`, `Nesting`, `IdPolicy`, `MetaField`, `validate_registry()` and
  `describe()`.
- `crates/lilac-core/src/block.rs`: registry-driven parsing and tree building.
  Added `Divider { rank }`, callouts, tables, math, HTML, images and a `Custom`
  escape hatch. Removed per-type hierarchy branching.
- Ranks: `---` = 10, `***` / `___` = 20, headings = 30 + (level - 1), leaves =
  255. Frontmatter is a root block and is only recognised when the delimited
  YAML is a non-empty mapping.
- Duplicate heading ids are deduplicated (`same`, `same-2`, ...).
- `validate_tree()` checks leaves have no children and sections are strictly
  deeper than their parent; debug parses assert it.
- Tauri `get_block_types` command; Settings lists every type with rank, nesting,
  id policy and registry validity; the record view renders the block tree
  recursively with rank labels.
- Record editing is path-addressed (`blockPath`) and runs through
  `RecordSession` (`crates/lilac-core/src/session.rs`): WAL replay, external
  change reconciliation, undo, and an `expectedContent` guard against positional
  paths drifting. See `../../Markdown/Design/record-session.md`.
- The record view has a user-toggleable hierarchy indent (default on); with it
  off, all blocks share the same left edge and hierarchy stays internal.
- Settings is split into Orchid / Storage / Lilac / Block types tabs, with styled
  buttons and inputs.

Verified: 58 core + 10 roundtrip + 5 probe + 4 config tests pass, clippy is
clean, frontend/Tauri/release builds succeed, and live CDP checks confirm block
editing, external-change refusal, undo, WAL replay, config repair and folder
creation.

Remaining:

- Extract declared `MetaField` metadata into `Block` (not only diagnostics).
- Load runtime `Custom` block types from config.
- Make matchers document-context aware where needed (tables across blank lines,
  frontmatter only at offset 0).
- Render each block type with its own leaf component while keeping Rust as the
  authority for structure.
