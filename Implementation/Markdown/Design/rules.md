---
topic: rules
type: design
status: active
related:
  - ../../../crates/lilac-core/src/rules.rs
  - ../../../crates/lilac-core/src/naming.rs
  - ../../../crates/lilac-host/src/lib.rs
  - ../../../crates/lilac-host/src/config.rs
  - ../../../apps/orchard/src-tauri/src/lib.rs
  - ../../../apps/orchard/src/App.tsx
  - ../../../docs/orchard-dev.md
---

# Rules: declared automation over ordinary metadata

## The file stays the program

Rules are a plain text file, `~/.orchard/.lilac/rules.txt`, not a database table
or a hidden settings blob. It is readable, diffable, portable and survives an
Orchard reinstall. The engine is in core; the host, Orchard and the browser
extension all call the same parser and evaluator.

The syntax is deliberately small. A rule is one `when` line followed by one or
more `then` lines, and a blank line ends it:

```text
rule "YouTube" for "*.youtube.com":
when tag "video"
then set status = "Video"
then add tag "watch-later"
```

`rule "name"` and `for "<pattern>"` are optional. A second form, a standalone
`for "pattern"` line, exists so an unnamed rule can be scoped too. Anything the
parser does not understand is an error; it never guesses.

## Scopes use hosting semantics, not new syntax

A scope is the same idea as a naming domain override, so it uses the same
matcher: lower-case, trailing dot removed, `www.` ignored, `example.com` exact,
`*.example.com` includes the root and every subdomain. Reusing `host_matches`
means a user learns one rule about domains, not two.

A scoped rule is inert without a URL. That is deliberate: there is no "current
site" state anywhere. The record either carries a URL that matches or the rule
does not run, so replaying the same file on another machine produces the same
result.

## Derived fields are read-only projections

Stored frontmatter is the only truth. Conditions may also read:

| Field | Source |
|---|---|
| `host`, `domain`, `site` | host of `url`, normalised |
| `folder` | placement relative to `Records/` |
| `tags` | tags joined with `, ` |
| `created`, `modified` | timestamps |
| `type`, `id` | record identity |

None of them are persisted. Storing `host` would create a second copy of the URL
that goes stale when a page moves or a user edits the URL, so `record_path` and
the index never learn about it. `matches`/`like` add case-insensitive globbing
(`*`, `?`) for cases a scope cannot express, such as matching both a host and a
path fragment in another field. Prose comparisons (`contains`, `has`) ignore
case; equality stays exact, so `status == "Reading"` does not silently match
`reading`.

## Scheduling is explicit

There are two application entry points, and the difference is visible:

- **Browser save.** `save_record` applies the rules when
  `rules.auto_apply` is true. It runs before `record_path`, so a `move` action
  picks the destination rather than moving the file after it was written. The
  switch defaults to off; a new install never rewrites records before the user
  has seen the rules.
- **On demand.** `apply_rules_cmd` in Orchard reads one stored record, applies
  the file, writes metadata and moves the file when needed. The live session is
  dropped, because the file it owns may have changed path.

A broken rules file must not break saving. On browser save the parse error is
written to the WAL and the record is stored unchanged. In the Orchard editor,
`set_rules_cmd` refuses to write an invalid file at all, so the previous working
rules stay active. The two choices are different on purpose: a save is an
external event that cannot be retried by the user, while editing rules is an
interactive act that can be corrected immediately.

## Turning a rule off without deleting it

A user who is unsure about a rule should not have to choose between trusting it
and losing the text. `disabled rule "name":` keeps the rule in the file, keeps it
visible in the editor and keeps its scope and actions documented, but the engine
skips it. `Rule.enabled` is parsed once; `apply_rules` and `preview_rules` both
honour it. Removing the word `disabled` restores the rule exactly as written.

`disabled` is only accepted before `rule`. A `disabled when ...` line is an error
rather than a silently ignored rule, because a typo must not look like a working
statement.

## Dry run before a real pass

`preview_rules` answers "what would happen?" without writing a byte. It returns
one `RulePreview` per rule — position, name, scope, enabled, matched, the action
labels that would run, and any runtime errors — and applies the actions to a
clone of the record. Two consequences follow:

- Later rules see what earlier rules would compute, so the preview is faithful
  rather than a per-rule lookup table.
- The caller's record, the file and the index are untouched; preview cannot move
  or delete anything.

Orchard exposes this as **Preview rules** under an expanded record or base.
`apply_rules_cmd` and `preview_rules_cmd` share the same engine, so the preview
cannot drift from the real pass; only the write step differs.

## Actions stay data-only

`set`, `add tag`, `remove tag`, `move` and `delete` change frontmatter. `delete`
sets `_deleted: true` instead of removing a file during a rule pass; actual
deletion remains an explicit user action. `compute field = expression` evaluates
the pure formula engine and writes the result to a field. Rules cannot run
commands, fetch URLs or edit bodies. That keeps a rule file safe to copy between
machines and keeps the failure model small.

## Limits

- Rules do not chain by dependency; they run in file order and each sees the
  previous rule's changes.
- `disabled` is a file-level word, not a per-record condition; a rule that should
  vary by site uses a scope instead.
- A preview reports action labels and runtime errors, not a byte-level diff of
  the record. The real pass computes the same values through the same engine.
