# Orchard (Tauri) Development

## Prerequisites

- Rust toolchain (stable) with the MSVC target on Windows.
- Node 20+ and npm (SolidJS + Vite frontend).

## Build & run

```
cd apps/orchard
npm install
npm run tauri dev
```

For a plain debug build of the shell (frontend must already be built):

```
cd apps/orchard
npm run build
cd src-tauri
cargo build
```

Release artifacts:

```
powershell -File scripts/build-release.ps1
```

This produces `dist/orchard.exe` and `dist/lilac-extension.zip`. The script also
regenerates the multi-size application icon with `scripts/make-icon.ps1`; the
icon is embedded in the exe (window/taskbar) and used in the app UI.

## Single executable

`orchard.exe` is both the GUI and the native messaging host. When the browser
launches it, it starts in host mode; the detection lives in
`apps/orchard/src-tauri/src/main.rs`:

- `--host` (explicit),
- `--parent-window...` / `-parent-window...` (Chrome / Firefox),
- stdin and stdout are both pipes (covers launchers that pass no arguments).

There is no separate `lilac-host.exe`.

## Config

Both the GUI and the host use `lilac_host::config::load_config()`, so there is
one source of truth:

- `~/.orchard/.lilac/config.yaml` - records root, registered folders.
- `~/.orchard/.orchid/config.yaml` - Orchid app settings (registered folders,
  extension id).
- Legacy `~/.lilac/config.yaml` is migrated once on first load.

Defaults are the standard record types: `page`, `collection`, `bookmark`,
`snapshot`, `note`, `snippet`. They are created under
`<records_root>/Records/<folder>` on load. A config that cannot be parsed (for
example the old installer's concatenated file) is salvaged for its last
`records_root` value and rewritten cleanly; folder entries that are absolute or
contain `..` are ignored.

## Architecture

- Rust is sole authority: `lilac-core` provides storage, index, search, block
  model, block type registry, naming, rules, WAL and record sessions.
- Block types are declared in `crates/lilac-core/src/block_types.rs`. The parser
  asks the registry for syntax and hierarchy; it has no per-type nesting
  branches. `validate_registry()` and `validate_tree()` enforce the invariants.
  A type's declared `MetaField`s are extracted into `Block::meta` as typed
  `MetaValue`s: frontmatter from its YAML, and `level`/`id`, `rank`, `lang` and
  `indent` from the parsed shape. `missing_metadata()` reports required fields a
  file does not provide, and the record view shows the extracted values next to
  each block.
- The frontend is SolidJS 2.0 RC + Vite (`apps/orchard/src/App.tsx`), a thin
  renderer that invokes Tauri commands and paints the returned block tree.
- No document structure or parsing lives in JS.
- `RecordSession` (`crates/lilac-core/src/session.rs`) owns the open document:
  it replays WAL edits on open, detects and reconciles external changes, applies
  edits by position path (`[0, 2]`, never `id!`), keeps undo history, and
  refuses an edit when the addressed block's content no longer matches what the
  editor started from.

## Commands exposed

`list_records`, `search`, `load_record`, `get_blocks`,
`apply_block_edit_cmd`, `insert_block_cmd`, `delete_block_cmd`,
`move_records_cmd`, `update_record_fields_cmd`, `list_templates_cmd`,
`create_from_template_cmd`, `save_as_template_cmd`, `undo_block_edit_cmd`,
`redo_block_edit_cmd`, `records_root_path`,
`register_native_host`, `get_block_types`, `get_settings`, `open_folder`,
`set_registered_folders_cmd`, `set_records_root_cmd`, `set_naming_cmd`,
`preview_naming_cmd`.

`get_block_types` returns `{ types: [...], valid: bool }` from
`lilac_core::block_types::describe()`. The Settings dialog uses it to render
the registry and its validity.

`get_blocks` returns a session view:
`{ blocks, body, record, fingerprint, recovered_edits, orphaned_wal, external_changed }`.
The tree is re-read from the session and external changes are reconciled on every
call. `fingerprint` is the FNV-1a version of the whole file, sent as a decimal
string because a 64-bit value does not fit JavaScript's safe integer range.
`apply_block_edit_cmd` takes `{ id, blockPath, newContent, expectedContent }`;
`expectedContent` is the text the editor started from and guards against
positional paths drifting after an external edit. `undo_block_edit_cmd` and
`redo_block_edit_cmd` take `{ id }` and return the same view. Redo replays the
most recently undone body; a new edit or an external change discards the redo
path, so redo never applies text that no longer follows from the current file.

`insert_block_cmd` takes `{ id, afterPath, content }` and inserts raw Markdown.
`afterPath: null` appends to the body; a section path inserts after its whole
subtree, so the new block is placed after the section rather than inside its
first child slot. Final hierarchy still follows the declared ranks: a deeper
heading stays inside the section, an equal or shallower one becomes a sibling.
`content` is complete Markdown (`## Heading`, `- item`, a fenced block), which
keeps empty blocks impossible: there is no draft state in the file.
`delete_block_cmd` takes `{ id, blockPath, expectedContent }`; it removes the
block and its children, and the same `expectedContent` guard refuses a delete
when the block changed on disk. The UI composes new blocks from a type select
(paragraph, headings, bullet, quote, code, divider) plus text.

`get_settings` includes the live `naming` config. `set_naming_cmd({ naming })`
saves it to `config.yaml`; `preview_naming_cmd({ naming, title, url })` derives a
title without writing a record.

`move_records_cmd({ ids, folder })` moves one or many records to a folder
relative to `Records/` (blank means the record's type folder). The target is
validated once before any file moves; `normalize_folder` rejects absolute paths,
`.`/`..` segments and drive/stream separators, while accepting nested paths and
backslashes. `move_record_to_folder` writes the destination before removing the
source, so a failed move never loses the record; moving into the type's own
folder clears the redundant `folder` field. The command drops any open session
for a moved record, and the next edit reopens it at the new path. The UI exposes
per-record checkboxes, select-all, a folder input with registered-folder
suggestions, a folder badge on each card, and a `Move…` action in the record
view. `IndexEntry` carries `folder` so list views do not re-read each file.

`update_record_fields_cmd({ id, fields, expectedFingerprint })` replaces the
editable YAML fields (`title`, `tags`, `status`, `url`, `description`). It
writes through the live session so the body is preserved exactly, refreshes the
file version first, and refuses the write when `expectedFingerprint` no longer
matches. Empty strings clear a field; tags are trimmed, de-duplicated and empty
entries dropped. Moving a record or editing its body changes the fingerprint, so
the view returns the new one with every session result and the UI keeps it
current. Metadata changes update `modified` but are not on the block undo stack.

## Templates

A template is an ordinary record with `type: template` and a required
`target_type`. `list_templates` returns `{ id, title, target_type,
description, path }` for every template under `Records/`, including one with a
missing target so the UI can report the broken declaration.

`create_from_template_cmd({ templateId, title, url })` copies the template body
and metadata into a new record of `target_type`. Identity, timestamps and folder
are fresh; placement uses the target type's folder. `save_as_template_cmd({
sourceId, title })` copies an existing record into a template for its own type
and leaves the source untouched. A template cannot be saved as another template,
which would make the target ambiguous.

Placeholders are plain text: `{{id}}`, `{{title}}`, `{{url}}`, `{{host}}`,
`{{slug}}`, `{{date}}` and `{{now}}`. They are substituted in the body and in
string metadata, including unknown fields. An unknown or unclosed placeholder is
left as written. The UI exposes `New from template` (template, title, URL) and
`Save as template` inside a record.

### References to other records

`{{record:<id>.<expression>}}` reads another record without modifying it. The
first `.` splits the id from a formula, so `{{record:rec_book.title}}` copies a
title and `{{record:rec_book.pages_total - record.pages_read}}` computes a value
from that record. `file.name`, `file.folder` and every other formula field work.
References are resolved lazily and cached per application, so the same record is
read at most once.

A reference that cannot be resolved (missing record, invalid formula, runtime
error) is left in the text exactly as written and reported as a warning.
`create_from_template_cmd` returns those in `warnings`; Orchard shows them next
to the notice. The new record is always created: one broken reference must not
make a template unusable. `create_from_template_reported` is the function behind
the command; the older `create_from_template` discards warnings.

## Naming

`lilac_core::naming` is the single naming engine. `NamingConfig` holds ordered
global `rules` plus `domains` overrides. `rule_for_host` normalises the host
(lowercase, trailing dot and `www.` removed) and returns the first match;
`github.com` matches exactly, `*.example.com` matches the root and subdomains.
A matched override replaces the global rules rather than adding to them.

The host's `derive_title` uses the saved config when the message has no `rule`,
so Settings is authoritative for the extension. An explicit `rule` from an
older client is still honoured.

## Rules

`lilac_core::rules` parses `~/.orchard/.lilac/rules.txt` and applies actions to
record metadata. A rule is one `when` line and one or more `then` lines, separated
from the next rule by a blank line:

```text
rule "YouTube" for "*.youtube.com":
when tag "video"
then set status = "Video"
then add tag "watch-later"

when host == "github.com" and status != "Archived"
then add tag "code"
```

`rule "name"` is optional, as is `for "<host pattern>"`. The scope uses the same
matching as naming: `www.` is ignored, `example.com` is exact, and
`*.example.com` includes the root and every subdomain. A scoped rule never runs
for a record without a URL.

Write `disabled rule "name":` instead of `rule "name":` to keep a rule in the
file without applying it. A disabled rule is still parsed, counted and listed, so
it stays documented and comes back with one word removed. `disabled` is only
valid before `rule`; putting it on a `when` line is a parse error.

Conditions support `and`, `or`, `not`, parentheses, `tag "x"`, `==`, `!=`,
`contains`, `has`, `matches` and `like`. `contains` and `has` compare text
without regard to case. `matches` and `like` are case-insensitive globs with `*`
and `?`. Besides stored frontmatter, conditions can read derived fields:
`host` / `domain` / `site` (from `url`), `folder`, `tags`, `created` and
`modified`. Derived fields are never written, so the URL remains the only source
of truth for a site.

Actions are `set field = value`, `add tag "x"`, `remove tag "x"`,
`move "folder"`, `delete` (which marks `_deleted: true` instead of removing a
file during a rule pass) and `compute field = <expression>`. A `set` on a list
field (`tags`, `categories`, `related`, `links`) splits the value on commas;
`set folder` is equivalent to `move`. Rules run in file order; a later rule sees
the changes made by an earlier one.

`compute` evaluates an expression against the record after earlier actions have
run, so a second `compute` can read the field set by the first:

```text
when tag "book"
then compute progress = if(status == "Finished", 100, 0)
then compute reading_time = round(320 / 200, 1)
```

Formulas are described in the next section. A `compute` that fails at runtime
(division by zero, a non-numeric string) is reported through
`apply_rules_reported`; the remaining actions still run and a browser save is
never blocked. The free `apply_rules` function keeps the older signature and
discards those errors.

There are two scheduling paths:

- **On browser save.** When `rules.auto_apply` is true in the Lilac config, the
  host applies `rules.txt` to each newly saved record before choosing its path,
  so a `move` action decides where the file lands. The switch is off by default
  and is edited in **Settings → Rules**. A rules file that fails to parse never
  blocks a save; the error is appended to the WAL and the record is stored
  unchanged. Compute errors from a valid file are logged the same way.
- **On demand.** `apply_rules_cmd({ id })` applies the file to one stored record
  from Orchard, writes the metadata, and moves the file when a `move` action
  fires. The open session is dropped so the next view reads the new path.
- **Dry run.** `preview_rules_cmd({ id })` returns one `RulePreview` per rule —
  `index`, `name`, `scope`, `enabled`, `matched`, `actions` and `errors` — without
  writing anything. It applies the rules to a clone of the record, so a later rule
  sees the values earlier rules would compute, exactly as a real pass would. The
  record and its file are unchanged. Orchard shows the result under the expanded
  record or base from **Preview rules**.

The GUI reads and writes the same file through `get_rules_cmd`, `set_rules_cmd`
and `set_auto_apply_rules_cmd`. `get_rules_cmd` returns both `rules` (parsed) and
`active` (enabled), so the editor can show how many are switched off.
`set_rules_cmd` validates before writing, so a typo in the editor cannot disable
the old rules. The extension popup's `Apply rules` sends its record to the host
`apply_rules` command, which uses the saved file unless an explicit `rules` string
is supplied. Both the host response and `apply_rules_cmd` include an `errors` list
for actions that ran but failed.

## Formulas

`lilac_core::formula` is a small expression language used by `compute` actions
and, later, by `.base` views. It is parsed once (`parse_formula`) and evaluated
against a record (`eval_formula`), so a rule load validates the syntax and a
hundred records cost only evaluation.

Values are null, booleans, numbers, strings and lists. Unknown fields evaluate to
null rather than failing, which is what makes `coalesce(missing, "fallback")`
and `if(done, "yes", "no")` useful on half-filled records.

```text
2 + 3 * 4                        14
price * quantity                 numeric, coercion from numeric text
title + " (" + status + ")"      string concatenation
tags.length                      list length
tags[0], tags[-1]                indexing, negative from the end
done ? "yes" : "no"              ternary
not status == "Archived"         `not` has low precedence
if(true, 1, unknownFn())         1; the unused branch is never evaluated
```

Fields: every frontmatter key, plus derived `host` / `domain` / `site` (from
`url`, `www.` ignored), `folder`, `created`, `modified`, `type`, `id`,
`target_type` and list fields. `file.name`, `file.folder`, `file.path`,
`file.ext` expose naming. Unknown metadata is read from `extra`.

Functions: `if`, `coalesce` / `default`, `now`, `today`, `min`, `max`, `sum`,
`average`, `abs`, `floor`, `ceil`, `round`, `sqrt`, `length`, `lower` / `upper`,
`trim`, `string`, `number`, `boolean`, `list`, `contains`, `startsWith`,
`endsWith`, `replace`, `join`, `pow`. Methods: `.toUpperCase()`,
`.toLowerCase()`, `.trim()`, `.toFixed(n)`, `.includes()`, `.startsWith()`,
`.endsWith()`, `.replace()`, `.split()`, `.join()`, `.slice()`, `.at()`,
`.isEmpty()`, plus `.length` and `.first` / `.last` properties.

Evaluation is pure apart from the clock in `now()` / `today()`. There is no
filesystem or network access, so a shared `.base` file cannot execute anything.

## Bases

A base is a terminal view with its own rendering authority: the first *flower*.
It is still an ordinary record with `type: base`, so it lives under
`Records/base/`, is indexed like everything else, and needs no second database.
Its body is a YAML query document; frontmatter remains ordinary metadata.

```yaml
source:
  type: book
  tag: reading
  folder: Reading
where: pages_total > pages_read
columns:
  - label: Title
    expr: file.name
  - status
  - label: Pages left
    expr: pages_total - pages_read
sort: "-modified"
limit: 100
```

`source` filters the index before any file is read. All fields are optional:
`type` (exact, case-insensitive), `tag`, `folder` (the folder and its
subfolders) and `path` (a case-insensitive substring of the stored path).

`where`, every column `expr` and `sort` use the same formula language as rules.
A bare string column (`- status`) uses the expression as its label; the object
form adds a human label. `sort` is ascending unless it starts with `-`. `limit`
must be greater than zero. A record whose cell fails (division by zero, a
non-numeric string) still appears, with an empty cell and a warning, so one bad
row cannot hide the rest.

Omitting `columns` gives `file.name`, `status` and `file.folder`. Omitting the
whole body gives a base over every record. Edits are validated before they are
written: `save_base_source_cmd` refuses invalid YAML or an invalid expression and
leaves the old definition in place.

Commands:

- `render_base_cmd({ id })` returns `{ source, view }`, where `view` is
  `{ labels, rows, matched, scanned, warnings }`.
- `save_base_source_cmd({ id, text, expectedSource })` validates, writes the
  body, and returns the new view. `expectedSource` guards against overwriting a
  definition changed by another program.
- `create_base_cmd({ title })` creates a `type: base` record with a starter
  document and returns its id.

A base is read-only over other records: rendering never writes a file. That is
the flower contract: it owns its presentation, not the data it projects.
