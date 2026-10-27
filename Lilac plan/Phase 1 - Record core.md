# Phase 1 — Record Core (Rust)

**Goal:** A Rust library that can read and write Lilac records to disk, with no browser involved. This is the foundation everything else builds on and the place where the "filesystem is the database" invariant must be proven.

**Milestone:** M1 (part 1). **Depends on:** decisions in `00 General.md`.

## Scope

In scope:
- Record data model (types, reserved fields, open metadata, references).
- File format: YAML frontmatter + optional markdown body.
- Read record from file; write record to file (atomic).
- File naming convention.
- Index rebuild from a records root.
- Round-trip fidelity (unknown fields/types survive).
- Unit tests against plain files (no browser, no native messaging).

Out of scope (later phases): native messaging, extension, search UI, snapshots, Wisteria semantics.

## Data model

```rust
// sketch
struct Record {
    id: String,                 // e.g. "rec_89a13f2" (immutable)
    r#type: String,             // page | collection | bookmark | snapshot | note | snippet
    version: u32,               // schema version, default 1
    // display
    title: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    description: Option<String>,
    // timestamps (RFC3339)
    created: String,
    modified: String,
    opened: Option<String>,
    last_viewed: Option<String>,
    // references
    parent: Option<Ref>,
    related: Vec<Ref>,
    links: Vec<Ref>,
    // classification
    tags: Vec<String>,          // allow plain strings OR [[...]] refs; normalize
    categories: Vec<String>,
    status: Option<Ref>,
    // open metadata (round-tripped verbatim)
    extra: BTreeMap<String, serde_yaml::Value>,
    // body
    body: Option<String>,       // markdown after frontmatter
    // wisteria-compat (reserved, no-op v0.1)
    authority: Option<String>,
    lineage: Option<String>,
    // snapshot-awareness (Q2)
    snapshot: Option<String>,   // asset ref (relative path or content hash); None in v0.1
    snapshot_format: Option<String>,
}
type Ref = String; // normalized wikilink or plain value
```

Notes:
- `extra` holds every field not in the reserved set, as raw YAML value, so unknown fields round-trip byte-for-byte-ish (YAML re-serialization may change formatting — acceptable; values must be preserved).
- Reserved field set must be checked by name; everything else goes to `extra`.

## File format

```markdown
---
id: rec_89a13f2
type: page

title: SeaORM

url: https://sea-orm.org

created: 2026-10-26T12:00:00Z
modified: 2026-10-26T12:30:00Z

tags:
- rust
- orm

project: [[Orchid]]
---

Research candidate.
```

Rules:
- Frontmatter delimited by `---` on its own line at start and end.
- Body is everything after the closing `---` (trimmed of one leading newline).
- Empty body → `None`.
- A file with no frontmatter is invalid (must have at least `id` + `type`); on read failure, surface error, don't crash.

## YAML handling (investigation item #1)

Risk: `status: [[Reading]]` unquoted is a flow indicator (`[` starts a flow sequence). Must verify against the chosen Rust YAML crate before committing to bare form.

Plan:
1. Prototype parse of `status: [[Reading]]`, `project: [[Orchid]]`, `tags: [rust]`, `tags: [[Rust]]` in the chosen crate (likely `serde_yaml` or `yaml-rust2`).
2. If bare form fails or is ambiguous → store **quoted** on write (`status: "[[Reading]]"`) and **normalize** on read (strip surrounding quotes, keep inner brackets). Value semantics unchanged; only lexical form differs.
3. Document the canonical on-disk form in this note and in `00 General.md`.

Decision rule: Lilac's reader accepts both quoted and unquoted; writer emits the form that round-trips cleanly in the chosen crate. Prefer quoted if any ambiguity.

## File naming

- Filename: `<slug>--<id>.md` where `slug` is a frozen, slugified title at creation time (lowercase, hyphens, truncated ~40 chars) and `id` is immutable.
- If no title: `<id>.md`.
- Directory: `<records root>/Records/<type>/`.
- Rationale: human-readable in git/sync, stable across title edits (slug frozen), unique by id. Collision on slug is fine because id differs.
- Collection example: `orm-research--rec_abc123.md` under `Records/collection/`.

## Atomic write

- Write to `<records root>/.../.tmp/<filename>.<pid>` then `fs::rename` to final path (same filesystem). On failure, leave original intact.
- After write, update in-memory index entry; index is cache, not truth.
- Never partially overwrite an existing record.

## Index rebuild

- `rebuild_index(records_root) -> Index` scans `Records/**/*.md`, parses each, builds `Index { records: Vec<IndexEntry> }`.
- `IndexEntry`: `{ id, r#type, title, tags, path (relative), modified }`.
- Written to `<config root>/index.json` (JSON, pretty, 2-space indent) — disposable; deleting it must never destroy information.
- `search_records(query)` filters the in-memory index; rebuild if stale/missing.
- Staleness: compare index mtime vs records root mtime; or explicit `rebuild_index` call. Keep simple: rebuild on startup + on explicit request; file-watcher optional (Phase 4).

## Round-trip fidelity tests (acceptance for Phase 1)

1. Write a record with reserved + extra fields + body → read back → all values equal.
2. Unknown field (`foo: bar`, nested `my_company: {project: Orchid}`) survives.
3. Unknown type string survives.
4. `[[...]]` reference values survive (quoted or unquoted per crate).
5. Delete `index.json` → `rebuild_index` → search returns all records.
6. A record edited externally (added line in body, changed a tag) re-parses without data loss.
7. Malformed file → typed error, no panic, other records still load.

## Tasks

- [ ] Choose Rust YAML crate; prototype wikilink parsing; document canonical form.
- [ ] Define `Record`, `Ref`, field sets, open-metadata map.
- [ ] Implement frontmatter split + parse.
- [ ] Implement serialize (frontmatter + body).
- [ ] Implement atomic write + read.
- [ ] Implement slug + filename generation.
- [ ] Implement index model + rebuild + write/read index.json.
- [ ] Implement basic in-memory search (tag, type, status filters).
- [ ] Unit tests for all acceptance cases.
- [ ] (Optional) file-watcher stub for Phase 4.

## Risks

- YAML crate choice affects wikilink handling and round-trip fidelity. Decide early (prototype first).
- `extra` as raw `Value` may re-serialize with different formatting; acceptable but must preserve values. Test.
- Large records root (thousands of files) → rebuild cost. Acceptable for v0.1; optimize later (incremental index) if needed.

## Deliverables

- Rust crate `lilac-core` (or module) with public API: `read_record`, `write_record`, `delete_record`, `rebuild_index`, `search_records`, `list_records`.
- Documented canonical on-disk form.
- Passing round-trip test suite.
- No browser/native-messaging code yet.
