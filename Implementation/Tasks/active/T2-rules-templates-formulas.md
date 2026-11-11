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
  `==`/`!=`/`contains`/`has`/`matches`/`like`, `tag`,
  `set`/`add tag`/`remove tag`/`move`/`delete`. Rules file at
  `~/.orchard/.lilac/rules.txt`.
- Per-domain rules: `rule "name" for "*.example.com":` or a standalone
  `for "*.example.com"` line. Scopes share the naming matcher (`www.` ignored,
  `*.` includes subdomains) and never fire for records without a URL.
- Derived read-only fields in conditions: `host`, `domain`, `site`, `folder`,
  `tags`, `created`, `modified`.
- Rule scheduling: `rules.auto_apply` in the Lilac config runs the file on every
  browser save before the path is chosen. Off by default. Orchard's
  **Settings → Rules** tab edits/validates the file, toggles automatic apply,
  and `Apply rules` runs them on one open record.
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
- Formula engine in `crates/lilac-core/src/formula.rs`: values, arithmetic,
  comparisons, `and`/`or`/`not`, ternaries, lists, indexing, methods, and a
  function set (`if`, `coalesce`, `min`/`max`/`sum`/`average`, string helpers,
  `now`/`today`). Expressions parse once and evaluate against a record. Purity
  is enforced by construction: no filesystem, network or arbitrary code.
- `compute field = <expression>` rule action. Evaluation errors are reported via
  `apply_rules_reported`, logged to the WAL by the host and shown in Orchard,
  but never block a save. Metadata-only results no longer rename a file.

Remaining:

- `.base` files supported as a type of Flower, rendered from a validated
  formula once and evaluated per row.
- Template variables that read another record/property (today placeholders are
  values of the new record only).
- Formula lookups across records (needs an index-aware evaluation context).
- Per-rule enable/disable and a dry-run preview showing which rules would fire.
