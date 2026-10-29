---
topic: ranked-dividers
type: task
status: deferred
related:
  - crates/lilac-core/src/block.rs
  - Implementation/Markdown/Fact/block-dividers.md
---

# T8 — Ranked block dividers (--- vs ***)

Deferred: `---` is a top-level divider, `***` is one rank lower (splits a
`---`-delimited block). Current parser treats both as `HorizontalRule`.
Needs parser refinement. See fact note for semantics.