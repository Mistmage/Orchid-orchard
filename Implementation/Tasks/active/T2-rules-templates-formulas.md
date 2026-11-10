---
topic: rules-templates-formulas
type: task
status: active
related:
  - crates/lilac-core/src/rules.rs
  - crates/lilac-core/src/template.rs
  - crates/lilac-host/src/lib.rs
  - extension/popup/popup.js
---

# T2 — Rules, templates, formulas

Shipped:

- Minimal rule engine: `when`/`then`, `and`/`or`/`not`,
  `==`/`!=`/`contains`/`has`, `tag`,
  `set`/`add tag`/`remove tag`/`move`/`delete`. Applied on save/edit via
  "Apply rules" button, rules file at `~/.orchard/.lilac/rules.txt`.
- Templates: a record with `type: template` and a required `target_type`.
  `create_from_template` copies the body and metadata into a new record of that
  type with fresh identity, timestamps and placement. `save_as_template` copies
  an existing record into a template for its own type without touching the
  source. `list_templates` reports declared targets and broken declarations.
- Placeholders `{{id}}`, `{{title}}`, `{{url}}`, `{{host}}`, `{{slug}}`,
  `{{date}}`, `{{now}}` substitute in the body and in string metadata, including
  unknown fields. Unknown placeholders survive as written.
- Orchard UI: `New from template` (template picker, title, URL) and
  `Save as template` inside an open record.
- `generate_record_id` lives in core and is shared by the host and Orchard, so
  every new record gets an id that no file under `Records/` already uses.

Remaining:

- Formula grammar refinement (Obsidian Bases-like:
  https://obsidian.md/help/formulas).
- Per-domain rules and rule scheduling.
- `.base` files supported as a type of Flower.
- Template variables that read another record/property (today placeholders are
  values of the new record only).
