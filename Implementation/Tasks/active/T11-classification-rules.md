---
topic: classification-rules
type: task
status: active
related:
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../crates/lilac-core/src/formula.rs
  - ../../../extension/background.js
  - ../../../extension/popup/popup.js
  - ../../Tasks/active/T9-extension-save-pipeline.md
  - ../../Tasks/active/T10-rules-organization.md
---

# T11 — Two-stage classification and page-context rules

## Goal

Rules answer two questions in order: **what is this?** (`type` and `subtype`) and
**what should it carry?** (fields). Both are evaluated in the context of the web
page, so they can use the metadata the page actually exposes, and the user is
asked only for what the rules could not determine.

## Requirements

- **Stage A — classify.** Rules set `type` and `subtype` from page context.
  A subtype is a stable second level (`page/article`, `page/product`,
  `book/reading`), stored as its own field and valid only for its declared type.
- **Stage B — enrich.** Rules add fields after the type is known, so a rule can
  be scoped to a type or subtype and can rely on type-specific fields. Later
  rules see earlier results.
- **Page context** includes more than URL and title: OpenGraph and meta tags,
  JSON-LD, canonical URL, site name, author, published date, language, favicon,
  selection text, scroll position, tab group name/color and window hints.
  Extraction happens in a content script; rules evaluate on the extracted data
  and never execute page scripts directly.
- **Form fallback.** When a required field is unknown, or classification is
  ambiguous, the popup shows a form generated from declarations (field type:
  text, number, date, choice, tags, record reference). The user can fill in more
  information even when nothing is required.
- **Provenance.** Each field records where it came from: `user`, `rule`,
  `page` or `default`. Precedence is user > rule > page > default, and the
  preview shows the source so an edited value is not silently overwritten.
- **Unknown is allowed.** A record with no confident type falls back to `page`
  and the form asks; nothing is dropped for being unrecognised.

## Design notes

The rule engine is shared with Orchard, so the type declarations and stage
boundaries live in core. The extension computes page context and asks the host to
evaluate; the popup only renders the form. This keeps the evaluator in one place
and lets Orchard validate the same rules.

## Done when

- A rule set can classify a product page and a blog post into different types and
  fill different fields in one save.
- The popup asks only for fields that remain unknown after both stages.
- Field provenance survives into the written YAML (for example under a reserved
  key) without breaking unknown-field round-trip.
- A rule scoped to a type does not fire before that type is set.

## Open questions

- Should provenance live in frontmatter or only in the preview? It is useful for
  debugging rules later, so a reserved `_sources` mapping is a candidate.
- How much page text may be sent to the AI connector from T16 (deferred)?
