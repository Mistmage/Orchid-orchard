---
topic: block-type-system
type: design
status: active
related:
  - ../../../crates/lilac-core/src/block_types.rs
  - ../../../crates/lilac-core/src/block.rs
  - ../Fact/block-dividers.md
  - ../../Tasks/active/T8-block-type-system.md
---

# Block type system (design)

## Why a registry

Earlier the parser recognised `---` and `***` as one `HorizontalRule` kind and
nested blocks with per-type branches. That made every new block type a chance
to introduce a hierarchy bug, and the hierarchy itself was implicit.

The block type system replaces that with one declared table. A block type owns
its syntax, its place in the hierarchy, its identity policy and its metadata
fields. The parser and tree builder are generic: they ask the registry, they do
not branch on specific types.

## The declaration

`crates/lilac-core/src/block_types.rs` defines `BlockTypeDef`:

```rust
pub struct BlockTypeDef {
    pub name: &'static str,       // stable key, also diagnostics name
    pub title: &'static str,      // human label
    pub syntax: Syntax,           // recogniser
    pub nesting: Nesting,         // hierarchy rule
    pub id: IdPolicy,             // identity on parse
    pub metadata: &'static [MetaField],
}
```

### Nesting

```rust
pub enum Nesting {
    Root,                    // closes every open section, always top level
    Section(u8),             // opens a section at a fixed rank
    Heading { base: u8 },    // rank = base + level - 1
    Leaf,                    // attaches to the innermost open section
}
```

Ranks are spaced so new section types can be inserted without renumbering:

| Rank | Type |
| --- | --- |
| 0 | frontmatter (root) |
| 10 | divider (`---`) |
| 20 | subdivider (`***`, `___`) |
| 30-80 | headings (`#` = 30, `##` = 40, ...) |
| 255 | leaves |

The tree builder keeps a stack of open sections:

- a section first closes every open section whose rank is `>=` its own,
- it then becomes a child of the stack top (or a root),
- a leaf becomes a child of the stack top (or a root).

That single rule produces the intended semantics: `--- *** *** ---` is a
top-level divider with two sub-divider children, and the second `---` is a new
top-level sibling.

### Syntax

`Syntax` selects a matcher: frontmatter, heading, thematic break, sub-divider,
fence, quote, callout, list, table, math, HTML, image, paragraph (fallback).
Order matters: more specific recognisers precede general ones, so a callout is
matched before a quote. `Custom` is reserved for future runtime-defined types.

### Identity

`IdPolicy` decides what id a block gets on parse:

- `EagerFixed("frontmatter")` - one known root block.
- `EagerSlug` - headings get a slug, deduplicated as `same`, `same-2`, ...
- `Lazy` - no id until a reference needs one.
- `ContentHash` - hash fallback for dedup/change detection.

### Metadata

`MetaField { name, kind, required }` declares what a type may carry, for
diagnostics and future tooling. It does not change parsing yet; unknown record
frontmatter still round-trips untouched.

## Invariants

`validate_registry()` checks:

- unique type names,
- exactly one root type,
- a paragraph fallback exists,
- section ranks are non-zero and below the leaf rank,
- no two section types share a rank,
- headings produce ranks that do not collide with section ranks,
- heading ranks stay below the leaf rank.

`validate_tree()` checks a parsed tree:

- a leaf never has children,
- a section is strictly deeper than its parent section.

`parse_blocks()` asserts `validate_tree()` in debug builds, so a hierarchy bug
is loud during development instead of silently corrupting the UI.

## Adding a block type

1. Add a `BlockTypeDef` entry with a unique `name`, the right `syntax`,
   `nesting` and `id` policy.
2. If it is a new syntax, add a `Syntax` variant and a matcher branch; add a
   `BlockKind` variant and map it in `classify_block`.
3. Run `cargo test -p lilac-core`; the registry and tree tests fail if the
   declaration violates an invariant.
4. Open Orchard Settings: the registry section shows the new type, its rank,
   nesting and id policy, and whether the registry is valid.

## Diagnostics

`describe()` serialises the whole registry to JSON. Tauri exposes it as
`get_block_types`; the Settings dialog renders the list. This is deliberately
the first consumer: it makes the declaration observable, so debugging a block
type starts from data rather than from reading parser branches.

## Limits and next steps

- Syntax matchers are still line-based. A future pass can make them
  document-context aware (e.g. frontmatter only at offset 0, tables that span
  blank lines).
- Metadata is declared but not yet extracted into `Block`. Wiring it in lets
  views query typed block metadata instead of re-parsing content.
- Runtime `Custom` types are reserved, not loaded from config yet.
