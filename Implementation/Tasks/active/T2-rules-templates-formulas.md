---
topic: rules-templates-formulas
type: task
status: active
related:
  - crates/lilac-core/src/rules.rs
  - crates/lilac-host/src/main.rs
  - extension/popup/popup.js
---

# T2 — Rules, templates, formulas

Minimal rule engine done: `when`/`then`, `and`/`or`/`not`, `==`/`!=`/`contains`/`has`,
`tag`, `set`/`add tag`/`remove tag`/`move`/`delete`. Applied on save/edit via
"Apply rules" button, rules file at `~/.orchard/.lilac/rules.txt`.

Remaining: templates (new records from a template record), formula grammar
refinement (Obsidian Bases-like: https://obsidian.md/help/formulas), per-domain
rules, rule scheduling. `.base` files supported as a type of Flower.