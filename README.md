# Orchid Orchard

A local-first personal knowledge/workspace OS built on one recursive idea:
**everything is a record/block, and views are projections** over a shared store.

Orchard is the desktop app (Tauri 2 + SolidJS). Lilac is the browser extension
that turns tabs, bookmarks, sessions and notes into the same portable
Markdown + YAML records on disk.

## What is here

| Path | What it is |
| --- | --- |
| `apps/orchard` | Orchard desktop app: SolidJS frontend + Tauri/Rust shell. |
| `crates/lilac-core` | Rust record storage core: records, YAML, atomic I/O, index, search, block model, block type registry, naming, rules, WAL. |
| `crates/lilac-host` | Shared native-messaging library, embedded in `orchard.exe`. |
| `extension/` | MV3 browser extension (Chrome + Firefox). |
| `docs/` | Record format, host protocol, development notes, sync. |
| `scripts/` | Host install/uninstall, release build. |
| `templates/` | `.gitignore` template for a records root. |
| `Implementation/` | Task, goal, design and fact notes (see its `README.md` legend). |

## Requirements

- Rust stable (MSVC target on Windows).
- Node 20+ and npm.
- WebView2 runtime (installed by default on current Windows).

## Build and run

Development app:

```powershell
cd apps\orchard
npm install
npm run tauri dev
```

Plain debug build (frontend must already be built):

```powershell
cd apps\orchard
npm run build
cd src-tauri
cargo build
```

Core tests:

```powershell
cargo test --workspace
```

Release artifacts (frontend + release build + extension zip):

```powershell
powershell -File scripts\build-release.ps1
```

Outputs:

- `dist\orchard.exe` - the single desktop executable.
- `dist\lilac-extension.zip` - the browser extension.

`dist/` is generated and gitignored.

## Single executable

`orchard.exe` is both the GUI and the native-messaging host. There is no
separate `lilac-host.exe`. When a browser launches it, host mode is detected
from `apps/orchard/src-tauri/src/main.rs`:

- `--host` (explicit),
- `--parent-window...` / `-parent-window...` (Chrome / Firefox),
- stdin and stdout are both pipes (launchers that pass no arguments).

## Register the browser host

After building, register the executable with Chrome/Firefox:

```powershell
powershell -File scripts\install-host.ps1 `
  -ExtensionId "<chrome-extension-id>" `
  -BinaryPath "dist\orchard.exe" `
  -RecordsRoot "C:\Users\<you>\Documents\Lilac"
```

This writes `%LOCALAPPDATA%\Lilac\lilac_host.json`, registers the Chrome and
Firefox native-messaging keys, and stores the records root in
`~/.orchard/.lilac/config.yaml`. Restart the browser, then load the extension
from `extension/`.

If the extension ID changes, re-run the script; otherwise the manifest's
`allowed_origins` will reject the host.

## Configuration

Both the GUI and the host load the same config, so there is one source of
truth:

- `~/.orchard/.lilac/config.yaml` - records root and registered folders.
- `~/.orchard/.orchid/config.yaml` - Orchard app settings (registered folders,
  extension id).
- Legacy `~/.lilac/config.yaml` is migrated once on first load.

The default registered folders are the standard record types: `page`,
`collection`, `bookmark`, `snapshot`, `note`, `snippet`. They are created under
`<records_root>/Records/`. An unreadable or corrupted config is salvaged for
its last `records_root` value and rewritten cleanly on load.

The GUI's Settings dialog has separate tabs for Orchid, Storage, Lilac, Naming,
Rules and the live block type registry.

## Naming

Page titles are cleaned up by naming rules before a record is saved. Rules live
in the same `config.yaml` under `naming` and are edited in **Settings → Naming**:

- **Global rules** run in order: strip substrings, replace `from=to`, add a
  `{host}` / `{title}` prefix or suffix, truncate to `max_len`.
- **Domain overrides** match a host exactly (`github.com`) or a subdomain tree
  (`*.example.com`). The first match wins and replaces the global rules for that
  host. `www.` is ignored, matching is case-insensitive.
- A starter rule strips common site suffixes (` - GitHub`, ` | Hacker News`,
  ` - YouTube`, ` - Stack Overflow`) and caps titles at 120 characters.

Orchard is authoritative. The browser extension asks the host to derive a title
and sends no rule, so editing naming in Settings changes what Lilac saves. An
older client that still sends an explicit `rule` keeps working: that rule wins.

## Rules

`Settings → Rules` edits `~/.orchard/.lilac/rules.txt` and shows parse errors
before saving. A rule has one `when` condition and one or more `then` actions:

```text
rule "YouTube" for "*.youtube.com":
when tag "video"
then set status = "Video"
then add tag "watch-later"
```

`for "*.example.com"` limits a rule to a site (the same matching as naming:
`www.` ignored, `*.` includes subdomains). Conditions combine with `and`, `or`
and `not`, and can use `tag`, `==`, `!=`, `contains`, `has`, `matches` and `like`.
`contains`/`has` ignore case; `matches` supports `*` and `?` globs.
Derived fields `host`, `domain`, `folder`, `tags`, `created` and `modified` are
available without storing them. Actions are `set`, `add tag`, `remove tag`,
`move`, `delete` and `compute`. `compute field = expression` calculates a value,
for example `compute score = 2 * 3 + 1` or
`compute summary = title + " (" + status + ")"`; see
[Formulas](docs/orchard-dev.md#formulas) for the expression language.

Turn on **Apply automatically when the browser saves a page** to run the rules
on each new record; a `move` action chooses the folder before the file is
written. `Apply rules` in an open record runs them on demand. A file that does
not parse is never saved, and a parse error or a failed calculation never blocks
a browser save.

## Records

A record is a Markdown file with YAML frontmatter:

```markdown
---
id: rec_1793141421839
type: article
title: How we built a local-first knowledge space
tags: [tauri, markdown]
status: unread
url: https://example.com/article
---

# Notes

Body text. Unknown frontmatter keys are preserved on write.
```

Files are the database. The index is a cache and can always be rebuilt from
disk. Records never store browser `window_id`/`tab_id`; browser data is only a
hint used to propose placement.

## Bases

A base is a saved query that renders as a table. It is an ordinary record with
`type: base`; the body is YAML:

```yaml
source:
  type: book
  tag: reading
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

`where`, column expressions and `sort` use the [formula
language](docs/orchard-dev.md#formulas). Rendering only reads records; a failed
cell is shown as a warning instead of hiding the row. Create one with **New
base** in the header.

## Block type system

Block structure is declared, not hardcoded. The registry lives in
`crates/lilac-core/src/block_types.rs`; each type declares:

- `syntax` - how it is recognised,
- `nesting` - its hierarchy rank (`Root`, `Section(rank)`, `Heading`, `Leaf`),
- `id` - eager slug, fixed id, or lazy,
- `metadata` - declared fields for diagnostics and future tooling.

Default ranks:

| Type | Syntax | Rank |
| --- | --- | --- |
| frontmatter | leading `---` YAML mapping | root |
| divider | `---` | 10 |
| subdivider | `***` / `___` | 20 |
| heading | `#` ... `######` | 30, 40, ... 80 |
| leaf blocks | paragraphs, lists, code, tables, quotes, callouts, math, HTML, images | 255 |

A section with a higher rank nests under the nearest open section with a lower
rank; an equal or lower rank closes it. Leaves attach to the innermost open
section. This makes `--- *** *** ---` a top-level divider containing two
sub-dividers, not five equal rules.

The parser and tree builder ask the registry; they contain no per-type
hierarchy branches. `validate_registry()` checks the invariants (unique names,
unique section ranks, one root type, a paragraph fallback, heading ranks inside
range), and `validate_tree()` checks every parsed tree. Adding a block type is
one registry entry plus its `BlockKind` mapping, and hierarchy mistakes fail
loudly instead of producing a subtly wrong tree.

The Settings dialog lists every registered type with rank, nesting and id
policy, and reports `N registered · registry valid`.

See `Implementation/Markdown/Design/block-type-system.md` for the design and
extension steps.

## Editing and placement

The record view edits blocks in place. `Add block` and the per-block `+` open a
composer: choose paragraph, heading, bullet, quote, code or divider, type the
text, and the complete Markdown is inserted (a section's `+` inserts after its
subtree). `×` deletes a block and its children. Every operation is journaled and
undoable, and a block that changed on disk is never overwritten by a stale
editor path.

Records can be moved without leaving Orchard. Select one or more cards (or use
`Move…` in an open record), type a folder relative to `Records/` — nested paths
like `Archive/2026` are fine, blank means the type's own folder — and press
`Move`. The file is renamed, the record's `folder` field is updated, and an
explicit folder equal to the type folder is dropped. Invalid targets such as
`../outside` are refused before any file moves.

`Undo` and `Redo` walk the block edit history; a new edit starts a new branch and
clears the redo path.

The `Metadata` panel in an open record edits title, status, URL, tags and
description. It writes only frontmatter and refuses a save when the file changed
since the panel loaded, so it cannot silently overwrite a block edit or another
program's change.

## Templates

`Save as template` copies an open record into a `type: template` record that
declares its `target_type`. `New from template` then creates a record of that
type from the template body and metadata. Identity, timestamps and placement are
fresh, so every record starts as its own file.

Templates support `{{title}}`, `{{url}}`, `{{host}}`, `{{slug}}`, `{{id}}`,
`{{date}}` and `{{now}}` in the body and in string metadata. Unknown
placeholders are left as written. Templates are ordinary records under
`Records/template/`: edit, tag, move or delete them like anything else.

## Principles

- Files are the database; indexes are caches.
- Rust owns document structure and logic; the frontend is a thin renderer.
- Metadata is open; arbitrary user keys survive round-trips.
- Browsers are clients; no browser is authoritative.
- Cross-browser: records never store `window_id`/`tab_id`.
- Local-first: no telemetry; all data lives in portable files.
- Conflict philosophy: a conflicted file is preferable to a hidden merge.
- Build for debuggability and extension, even when it costs more code.

## More documentation

- `docs/orchard-dev.md` - development, commands, host mode.
- `docs/record-format.md` - record/frontmatter format.
- `docs/native-host-protocol.md` - browser host messages.
- `extension/README.md` - extension install and use.
- `Implementation/README.md` - note conventions and legend.

## License

MIT (see `LICENSE`).
