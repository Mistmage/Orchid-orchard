---
topic: markdown-block-dividers
type: fact
status: active
related:
  - crates/lilac-core/src/block.rs
---

# Markdown block dividers (fact)

YAML frontmatter is a **type of block** for us (kind: `Frontmatter`), not just
metadata. It is a first-class block in the block tree.

The two most common block dividers are:

1. `---` (thematic break / horizontal rule).
2. `#` (any heading), with nesting of lower headings:
   `# main` → `## topic` → `### Subtopic` → `## topic` etc.

`***` is equivalent to `---` with one exception: it is **one rank lower**.
So `--- *** *** ---` is NOT 5 equal blocks. It is 3 blocks delimited by `---`,
and the middle `---` block is split into 3 sub-blocks by `***`.

In other words, `---` is a top-level divider; `***` is a sub-divider within
a `---`-delimited block.

Implementation note: the current block parser treats `---`/`***`/`___` as a
single `HorizontalRule` kind and groups consecutive non-blank lines. The
ranked-divider semantics above are a future refinement (tracked as T8).