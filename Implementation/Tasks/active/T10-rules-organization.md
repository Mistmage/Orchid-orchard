---
topic: rules-organization
type: task
status: active
related:
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../crates/lilac-core/src/formula.rs
  - ../../../crates/lilac-host/src/lib.rs
  - ../../Markdown/Design/rules.md
  - ../../Markdown/Design/formulas.md
---

# T10 — De-duplication, folder sorting, post-processing

## Goal

Rules decide what a save means, not only which tags it gets: whether it is a
duplicate, where it belongs, and what happens after it is written. Expressions
follow Obsidian's formula syntax where practical, evaluated by our own pure
engine.

## Requirements

- **De-duplication**, partly rule-based. A rule declares a match key and an
  action:
  - key: exact URL, canonical URL (lowercase host, strip `www.`, fragment,
    common tracking parameters), type + title, content hash, or a formula;
  - action: skip, update the existing record, merge selected fields/tags, create
    anyway, or ask in the preview.
- The preview marks a draft as a duplicate of an existing record and excludes it
  by default. The user can still force a save.
- **Folder sorting**: rules propose or set the folder using the existing
  `folder` field and `move` action, with an explicit precedence: a folder the
  user chose wins unless a rule is marked as overriding. First matching rule wins
  unless a rule is marked `continue`.
- **Post-processing** runs after the record is written, sees both the previous
  and the new record, and can update fields, add tags, append to a collection,
  or run a notification. Actions must be idempotent and failures are logged, not
  fatal.
- Every stage is dry-runnable and reports which rule fired and why. Rule order is
  deterministic (file order), never hash or map order.
- Expression syntax: follow Obsidian formula forms (`file.name`, `note.field`,
  `date(...)`, list functions) so a formula copied from Obsidian works, while the
  evaluator stays pure and sandboxed.

## Design notes

Rules already parse into `Rule` with scopes and typed actions. Extend the action
enum rather than adding a parallel rule file. De-duplication needs an index
lookup, so it runs in the host (where the index lives), not in the extension
page.

## Done when

- A second save of the same URL produces the configured duplicate behaviour.
- Folder rules and manual folder selection have a tested precedence.
- Post-processing errors appear as warnings and never roll back a written record.
- Preview shows the matched rule name for each action.

## Open questions

- Canonical URL: which tracking parameters are stripped by default? Start with a
  documented short list, not a guess.
- Should `continue` be per action or per rule?
