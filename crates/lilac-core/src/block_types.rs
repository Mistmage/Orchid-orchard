//! Declarative block type registry.
//!
//! Every block type declares, in one place:
//! - how it is recognised (`Syntax`),
//! - where it sits in the hierarchy (`Nesting`),
//! - how it gets an identity (`IdPolicy`),
//! - what metadata it may carry (`MetaField`).
//!
//! The parser and the tree builder never hardcode a block type: they ask this
//! registry. Adding a new block type is therefore one `BlockTypeDef` entry
//! (plus one `BlockKind` variant and its mapping), and the invariants checked
//! by [`validate_registry`] make hierarchy mistakes fail loudly instead of
//! producing a subtly wrong tree.
//!
//! ## Ranks
//!
//! Sections nest by *rank*: a section with a higher rank becomes a child of the
//! nearest open section with a lower rank; a section with rank <= the current
//! one closes it. Leaves attach to the innermost open section.
//!
//! ```text
//! ---        divider      rank 10   (top-level section)
//! ***        subdivider   rank 20   (one rank lower: splits a divider)
//! # heading  heading      rank 30   (base, level added: # = 30, ## = 40, ...)
//! paragraph  leaf         attaches to the innermost open section
//! ```

use serde::{Deserialize, Serialize};

/// Ranks are deliberately spaced so new section types can be inserted later
/// without renumbering existing ones.
pub const RANK_DIVIDER: u8 = 10;
pub const RANK_SUBDIVIDER: u8 = 20;
pub const RANK_HEADING_BASE: u8 = 30;
pub const RANK_LEAF: u8 = u8::MAX;

/// How a block type participates in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nesting {
    /// Always a root-level block; closes every open section (frontmatter).
    Root,
    /// Opens a section at a fixed rank.
    Section(u8),
    /// Opens a section whose rank is `base + heading level - 1`.
    Heading { base: u8 },
    /// Never a parent; attaches to the innermost open section.
    Leaf,
}

impl Nesting {
    pub fn rank(self, heading_level: u8) -> u8 {
        match self {
            Nesting::Root => 0,
            Nesting::Section(r) => r,
            Nesting::Heading { base } => base + heading_level.saturating_sub(1),
            Nesting::Leaf => RANK_LEAF,
        }
    }

    pub fn is_section(self) -> bool {
        matches!(self, Nesting::Section(_) | Nesting::Heading { .. })
    }

    /// Stable, human-readable form used by the diagnostics UI.
    pub fn label(self) -> String {
        match self {
            Nesting::Root => "root".to_string(),
            Nesting::Section(r) => format!("section({})", r),
            Nesting::Heading { base } => format!("heading(base {})", base),
            Nesting::Leaf => "leaf".to_string(),
        }
    }
}

/// How a block gets an id on parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdPolicy {
    /// `slug(text)`; deduplicated with `-2`, `-3`, ...
    EagerSlug,
    /// A constant id (frontmatter).
    EagerFixed(&'static str),
    /// No id until a reference needs one (sidecar index).
    Lazy,
    /// Content hash fallback for dedup/change detection.
    ContentHash,
}

impl IdPolicy {
    pub fn label(self) -> String {
        match self {
            IdPolicy::EagerSlug => "eager:slug".to_string(),
            IdPolicy::EagerFixed(s) => format!("eager:\"{}\"", s),
            IdPolicy::Lazy => "lazy".to_string(),
            IdPolicy::ContentHash => "content-hash".to_string(),
        }
    }
}

/// Recogniser for a block type. `Custom` is the escape hatch for user-defined
/// block types discovered at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Syntax {
    Frontmatter,
    Heading,
    ThematicBreak,
    SubDivider,
    Fence,
    Quote,
    Callout,
    List,
    Table,
    Math,
    Html,
    Image,
    Paragraph,
    Custom,
}

impl Syntax {
    pub fn label(self) -> &'static str {
        match self {
            Syntax::Frontmatter => "frontmatter",
            Syntax::Heading => "heading",
            Syntax::ThematicBreak => "thematic-break",
            Syntax::SubDivider => "sub-divider",
            Syntax::Fence => "fence",
            Syntax::Quote => "quote",
            Syntax::Callout => "callout",
            Syntax::List => "list",
            Syntax::Table => "table",
            Syntax::Math => "math",
            Syntax::Html => "html",
            Syntax::Image => "image",
            Syntax::Paragraph => "paragraph",
            Syntax::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetaKind {
    Text,
    Number,
    Bool,
    List,
    BlockRef,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MetaField {
    pub name: &'static str,
    pub kind: MetaKind,
    pub required: bool,
}

const FRONTMATTER_FIELDS: &[MetaField] = &[
    MetaField {
        name: "id",
        kind: MetaKind::Text,
        required: true,
    },
    MetaField {
        name: "type",
        kind: MetaKind::Text,
        required: true,
    },
    MetaField {
        name: "title",
        kind: MetaKind::Text,
        required: false,
    },
    MetaField {
        name: "tags",
        kind: MetaKind::List,
        required: false,
    },
    MetaField {
        name: "status",
        kind: MetaKind::Text,
        required: false,
    },
    MetaField {
        name: "created",
        kind: MetaKind::Text,
        required: false,
    },
    MetaField {
        name: "modified",
        kind: MetaKind::Text,
        required: false,
    },
    MetaField {
        name: "folder",
        kind: MetaKind::Text,
        required: false,
    },
    MetaField {
        name: "url",
        kind: MetaKind::Text,
        required: false,
    },
];

const HEADING_FIELDS: &[MetaField] = &[
    MetaField {
        name: "level",
        kind: MetaKind::Number,
        required: true,
    },
    MetaField {
        name: "id",
        kind: MetaKind::Text,
        required: false,
    },
];

const DIVIDER_FIELDS: &[MetaField] = &[MetaField {
    name: "rank",
    kind: MetaKind::Number,
    required: true,
}];

const CODE_FIELDS: &[MetaField] = &[MetaField {
    name: "lang",
    kind: MetaKind::Text,
    required: false,
}];

const LIST_FIELDS: &[MetaField] = &[MetaField {
    name: "indent",
    kind: MetaKind::Number,
    required: false,
}];

/// One declared block type.
#[derive(Debug, Clone, Copy)]
pub struct BlockTypeDef {
    /// Stable machine name (also the serde/type-system key).
    pub name: &'static str,
    /// Human label for UI/debugging.
    pub title: &'static str,
    pub syntax: Syntax,
    pub nesting: Nesting,
    pub id: IdPolicy,
    pub metadata: &'static [MetaField],
}

/// The registry. Order matters for classification: first match wins, so
/// specific syntaxes come before general ones (callout before quote).
pub const BLOCK_TYPES: &[BlockTypeDef] = &[
    BlockTypeDef {
        name: "frontmatter",
        title: "YAML frontmatter",
        syntax: Syntax::Frontmatter,
        nesting: Nesting::Root,
        id: IdPolicy::EagerFixed("frontmatter"),
        metadata: FRONTMATTER_FIELDS,
    },
    BlockTypeDef {
        name: "callout",
        title: "Callout",
        syntax: Syntax::Callout,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "divider",
        title: "Section divider (---)",
        syntax: Syntax::ThematicBreak,
        nesting: Nesting::Section(RANK_DIVIDER),
        id: IdPolicy::Lazy,
        metadata: DIVIDER_FIELDS,
    },
    BlockTypeDef {
        name: "subdivider",
        title: "Sub-divider (***, ___)",
        syntax: Syntax::SubDivider,
        nesting: Nesting::Section(RANK_SUBDIVIDER),
        id: IdPolicy::Lazy,
        metadata: DIVIDER_FIELDS,
    },
    BlockTypeDef {
        name: "heading",
        title: "Heading",
        syntax: Syntax::Heading,
        nesting: Nesting::Heading {
            base: RANK_HEADING_BASE,
        },
        id: IdPolicy::EagerSlug,
        metadata: HEADING_FIELDS,
    },
    BlockTypeDef {
        name: "code",
        title: "Fenced code block",
        syntax: Syntax::Fence,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: CODE_FIELDS,
    },
    BlockTypeDef {
        name: "math",
        title: "Math block",
        syntax: Syntax::Math,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "table",
        title: "Table",
        syntax: Syntax::Table,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "quote",
        title: "Blockquote",
        syntax: Syntax::Quote,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "list",
        title: "List item",
        syntax: Syntax::List,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: LIST_FIELDS,
    },
    BlockTypeDef {
        name: "image",
        title: "Image",
        syntax: Syntax::Image,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "html",
        title: "Inline HTML",
        syntax: Syntax::Html,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
    BlockTypeDef {
        name: "paragraph",
        title: "Paragraph",
        syntax: Syntax::Paragraph,
        nesting: Nesting::Leaf,
        id: IdPolicy::Lazy,
        metadata: &[],
    },
];

pub fn registry() -> &'static [BlockTypeDef] {
    BLOCK_TYPES
}

pub fn by_name(name: &str) -> Option<&'static BlockTypeDef> {
    BLOCK_TYPES.iter().find(|t| t.name == name)
}

/// First type whose syntax recognises this line. `Paragraph` is the fallback.
pub fn classify_line(line: &str) -> &'static BlockTypeDef {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    BLOCK_TYPES
        .iter()
        .find(|t| matches_syntax(t.syntax, trimmed))
        .unwrap_or_else(|| {
            BLOCK_TYPES
                .iter()
                .find(|t| t.syntax == Syntax::Paragraph)
                .expect("registry must declare a paragraph fallback")
        })
}

fn matches_syntax(syntax: Syntax, line: &str) -> bool {
    let t = line.trim();
    match syntax {
        Syntax::Frontmatter => false, // handled at document start, not per line
        Syntax::Heading => heading_level(t).is_some(),
        Syntax::ThematicBreak => is_thematic_break(t, '-'),
        Syntax::SubDivider => is_thematic_break(t, '*') || is_thematic_break(t, '_'),
        Syntax::Fence => fence_marker(t).is_some(),
        Syntax::Callout => callout_kind(t).is_some(),
        Syntax::Quote => t.starts_with('>') && callout_kind(t).is_none(),
        Syntax::List => list_marker(t).is_some(),
        Syntax::Table => t.starts_with('|'),
        Syntax::Math => t == "$$",
        Syntax::Html => t.starts_with('<') && t.ends_with('>'),
        Syntax::Image => t.starts_with("!["),
        Syntax::Paragraph => false,
        Syntax::Custom => false,
    }
}

pub fn heading_level(line: &str) -> Option<u8> {
    let t = line.trim();
    let hashes = t.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &t[hashes..];
    if rest.is_empty() || rest.starts_with(' ') {
        Some(hashes as u8)
    } else {
        None
    }
}

pub fn is_thematic_break(line: &str, marker: char) -> bool {
    let t = line.trim();
    t.len() >= 3 && t.chars().all(|c| c == marker)
}

pub fn fence_marker(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with("```") {
        Some("```")
    } else if t.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

pub fn callout_kind(line: &str) -> Option<&str> {
    let t = line.trim();
    let rest = t.strip_prefix("> [")?;
    let end = rest.find(']')?;
    Some(&rest[..end])
}

pub fn list_marker(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        return Some(indent);
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && t[digits..].starts_with(". ") {
        return Some(indent);
    }
    None
}

/// Structural starts: a line that begins a new block even without a blank line
/// separating it from the previous one.
pub fn starts_new_block(line: &str) -> bool {
    let t = line.trim();
    heading_level(t).is_some()
        || is_thematic_break(t, '-')
        || is_thematic_break(t, '*')
        || is_thematic_break(t, '_')
        || fence_marker(t).is_some()
        || t == "$$"
}

/// Invariants that make hierarchy mistakes loud rather than silent.
pub fn validate_registry() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut names = std::collections::HashSet::new();
    let mut section_ranks = std::collections::HashMap::new();
    let mut heading_bases = Vec::new();
    let mut has_paragraph = false;
    let mut root_count = 0;

    for def in BLOCK_TYPES {
        if !names.insert(def.name) {
            errors.push(format!("duplicate block type name: {}", def.name));
        }
        if def.syntax == Syntax::Paragraph {
            has_paragraph = true;
        }
        match def.nesting {
            Nesting::Root => root_count += 1,
            Nesting::Section(rank) => {
                if rank == 0 || rank == RANK_LEAF {
                    errors.push(format!(
                        "{}: section rank {} is out of range",
                        def.name, rank
                    ));
                }
                if let Some(prev) = section_ranks.insert(rank, def.name) {
                    errors.push(format!(
                        "{} and {} share section rank {}",
                        prev, def.name, rank
                    ));
                }
            }
            Nesting::Heading { base } => heading_bases.push((def.name, base)),
            Nesting::Leaf => {}
        }
    }

    if !has_paragraph {
        errors.push("registry must declare a paragraph fallback".to_string());
    }
    if root_count != 1 {
        errors.push(format!(
            "expected exactly one root type, found {}",
            root_count
        ));
    }

    for (name, base) in heading_bases {
        let max = base as u16 + 5;
        if max >= RANK_LEAF as u16 {
            errors.push(format!("{}: heading levels exceed rank range", name));
        }
        for level in 1..=6u16 {
            let rank = (base as u16 + level - 1) as u8;
            if section_ranks.contains_key(&rank) {
                errors.push(format!(
                    "{}: heading level {} collides with a section rank ({})",
                    name, level, rank
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// JSON description for the diagnostics UI / external tools.
pub fn describe() -> serde_json::Value {
    let types: Vec<serde_json::Value> = BLOCK_TYPES
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "title": t.title,
                "syntax": t.syntax.label(),
                "nesting": t.nesting.label(),
                "rank": match t.nesting {
                    Nesting::Root => 0,
                    Nesting::Section(r) => r,
                    Nesting::Heading { base } => base,
                    Nesting::Leaf => RANK_LEAF,
                },
                "id": t.id.label(),
                "metadata": t.metadata.iter().map(|m| serde_json::json!({
                    "name": m.name,
                    "kind": format!("{:?}", m.kind).to_lowercase(),
                    "required": m.required,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({ "types": types, "valid": validate_registry().is_ok() })
}
