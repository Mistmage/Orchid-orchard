---
topic: obsidian-bases-formulas
type: note
status: active
related:
  - ../..
  - ../../Tasks/active/T2-rules-templates-formulas.md
  - ../../Markdown/Design/formulas.md
---

# Obsidian Bases formulas (inspiration)

Reference: https://obsidian.md/help/formulas

Orchid's rule/formula system draws from Obsidian Bases:
- Formulas over note properties (status, tags, dates, etc.).
- Computed properties that update when inputs change.
- Views (table, list, kanban) as projections over filtered/sorted records.

Shipped in the formula engine (`crates/lilac-core/src/formula.rs`): arithmetic,
comparisons, `and`/`or`/`not`, ternaries, lists and indexing, string/number
functions, `if`/`coalesce`, `now`/`today`, and dotted `file.*` properties. Rules
use it through `compute field = <expression>`. The language is pure and parses
once, so a future `.base` view can validate a file and evaluate it per row.

Differences vs Orchid:
- Orchid treats formulas as rules that can also *act* (add metadata, move folder),
  not just compute.
- `.base` files supported as a type of Flower (terminal view with its own
  rendering authority). Not built yet; the evaluator is ready for it.

Tracked as refinement of T2.