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
- Per-rule enable/disable: `disabled rule "name":` parses and lists a rule but
  never applies it. `get_rules_cmd` reports both parsed and active counts.
- Dry run: `preview_rules` / `preview_rules_cmd` return one `RulePreview` per
  rule (index, name, scope, enabled, matched, actions, errors) against a clone of
  the record, so later rules see earlier computed values. Orchard shows the
  result from **Preview rules** on a record or base; nothing is written.
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
- Bases: a record with `type: base` whose YAML body declares `source` filters,
  a `where` formula, columns, `sort` and `limit`. `crates/lilac-core/src/base.rs`
  parses/validates once and renders against the index; it reads other records but
  never writes them. Orchard renders the table, a warnings list and a definition
  editor; `New base` creates one. This is the first Flower.
- First-class `.base` files: the same YAML without record frontmatter, scanned
  under `Records/` rather than indexed as records. `scan_base_files`,
  `render_base_file`, `save_base_file` and `create_base_file` in core back
  `list_base_files_cmd`, `render_base_file_cmd`, `save_base_file_cmd` and
  `create_base_file_cmd`. A broken file is listed with its error; paths are
  confined to `Records/` and must end in `.base`. Orchard has a **Base files**
  section with the same table, definition editor and atomic save.
- Template references: `{{record:<id>.<expression>}}` reads another record with
  the formula engine. `ReferenceCache` reads each id once per application; a
  failed reference keeps the placeholder and is returned through
  `create_from_template_reported` as a warning, so the record is still created.

Remaining:

- Multiple named views per base and explicit view types (table is the only one).
- Formula lookups across records (needs an index-aware evaluation context).
