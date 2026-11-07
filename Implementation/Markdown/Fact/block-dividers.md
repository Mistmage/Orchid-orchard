---
topic: markdown-block-dividers
type: fact
status: active
related:
  - ../../../crates/lilac-core/src/block_types.rs
  - ../../../crates/lilac-core/src/block.rs
  - ../Design/block-type-system.md
---

# Markdown block dividers (fact)

YAML frontmatter is a **type of block** for us (kind: `Frontmatter`), not just
metadata. It is a first-class block in the block tree.

The two most common block dividers are:

1. `---` (thematic break / horizontal rule).
2. `#` (any heading), with nesting of lower headings:
   `# main` -> `## topic` -> `### subtopic` -> `## topic` etc.

`***` is equivalent to `---` with one exception: it is **one rank lower**. So
`--- *** *** ---` is NOT 5 equal blocks. It is 2 top-level `---` sections, and
the first `---` section is split into 2 sub-sections by `***`.

In other words, `---` is a top-level divider; `***` is a sub-divider within a
`---`-delimited block.

Implementation: declared in `crates/lilac-core/src/block_types.rs`
(`divider` = rank 10, `subdivider` = rank 20) and applied generically by the
tree builder in `crates/lilac-core/src/block.rs`. See
`../Design/block-type-system.md`.
