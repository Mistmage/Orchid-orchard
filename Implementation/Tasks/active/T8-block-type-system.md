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

Verified: 53 core + 10 roundtrip + 5 probe tests pass, frontend and Tauri debug
builds succeed, and a live record shows `---` sections containing `***`
sub-sections with headings nested at rank 30+.

Remaining:

- Extract declared `MetaField` metadata into `Block` (not only diagnostics).
- Load runtime `Custom` block types from config.
- Make matchers document-context aware where needed (tables across blank lines,
  frontmatter only at offset 0).
- Render each block type with its own leaf component while keeping Rust as the
  authority for structure.
