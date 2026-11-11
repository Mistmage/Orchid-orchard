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

There are two entry points, and the difference is visible:

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

## Actions stay data-only

`set`, `add tag`, `remove tag`, `move` and `delete` change frontmatter. `delete`
sets `_deleted: true` instead of removing a file during a rule pass; actual
deletion remains an explicit user action. Rules cannot run commands, fetch URLs
or edit bodies. That keeps a rule file safe to copy between machines and keeps
the failure model small.

## Limits

- Rules do not chain by dependency; they run in file order and each sees the
  previous rule's changes.
- There is no per-rule enable switch yet; comment it out or delete it.
- Formula expressions over numeric metadata are a later T2 step.
