---
topic: formulas
type: design
status: active
related:
  - ../../../crates/lilac-core/src/formula.rs
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../crates/lilac-core/src/lib.rs
  - ../../../crates/lilac-host/src/lib.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
  - ../../../docs/orchard-dev.md
---

# Formulas: a small expression language for records

## Why an expression language at all

A rule can set a fixed value. It cannot say "produce a title from the parts" or
"score this record from its own metadata" without a second mechanism. The two
honest options were to grow the action syntax until it became a programming
language, or to add a separate expression evaluator that actions and views can
share. The second keeps both surfaces small: rules stay declarative, and a
formula is parsed once and reused.

The target users are producers, not programmers, so the grammar follows the
formulas they may already have seen in Obsidian Bases: arithmetic, comparisons,
`if`, `&&` / `||`, string methods and dotted properties. Nothing needs importing,
and an unknown field is null rather than an error.

## Parse once, evaluate many

`parse_formula` returns an `Expr` tree. `eval_formula` walks it against a
`Record`. Rules parse their `compute` expressions when the rules file is loaded,
so a typo is a load-time error and a rule pass over many records only evaluates.
The same split lets a future `.base` view validate a file once and render a table
without touching the parser per row.

The module is pure. It reads the record and, for `now()` / `today()` only, the
clock. It never reads or writes files, never opens a network connection and
cannot call arbitrary code. A `.base` file shared by someone else is therefore
data, not a program.

## Values and coercion

Values are null, boolean, number, string and list. Metadata is read lazily from
typed fields and `extra`, so unknown keys are usable without a schema. `+`
concatenates when either side is a string, otherwise adds; numeric strings coerce
for arithmetic (`"10" * 2` is `20`). Equality stays strict about type, matching
the rules engine: `"1" == 1` is false. Comparisons compare strings
lexicographically and everything else numerically.

Lists are real values. `tags.length`, `tags[0]`, `tags[-1]` and
`tags.contains("rust")` work without a special tag syntax. A `compute` that
writes a list into a typed list field is joined with `", "` and goes through the
same comma-splitting path as `set tags`.

## Failure is local

Two kinds of failure are distinct:

- **Syntax** is a parse error. The rules file is rejected as a whole by the
  editor and the host logs it; the old file stays active.
- **Evaluation** is a runtime error such as division by zero or arithmetic on a
  non-numeric string. It is reported per action, the action is not counted as
  fired, and later actions still run.

`apply_rules_reported` returns `(fired, errors)`; `apply_rules` is the older
convenience wrapper that discards errors. The host appends compute errors to the
WAL instead of failing a browser save, and Orchard shows them in the notice line
after `Apply rules`. This mirrors the rest of the system: a cache or a
convenience feature must never take a file hostage.

## What is deliberately missing

- **No assignments, loops or user-defined functions.** Those turn a formula into
  a language with control flow and a debugging surface. Repetition belongs in a
  `.base` query, not in an expression.
- **No filesystem, date arithmetic or record lookups yet.** Looking up another
  record is a real need (template placeholders already hint at it), but it needs
  an index-aware context and a cost model, not a global escape hatch.
- **No implicit stringification in equality.** Silent coercion hides mistakes in
  automation, which is worse than a visible false.
- **No per-field schema.** Typed fields and `extra` share one namespace; a schema
  would need a migration story and belongs to the `MetaField` work in T8.

## Integration points

- `rules.rs`: `Action::Compute(field, Expr)`; parsing is `compute field = ...`.
- `lilac-host`: auto-apply logs compute errors to the WAL; the `apply_rules`
  host command returns them as `errors`.
- `apps/orchard/src-tauri`: `apply_rules_cmd` returns `errors` alongside `fired`;
  the UI surfaces them.
- Future `.base` views: parse a formula once, evaluate per row, and treat a null
  as an empty cell instead of an error.
