//! Block model for grape markdown.
//!
//! Rust is the sole authority for document structure: parsing, block identity,
//! byte spans, and hierarchy all live here. The frontend only renders.
//!
//! Block types are declared in [`crate::block_types`]. This module only:
//! 1. recognises lines through the registry,
//! 2. extracts a flat sequence of blocks with byte spans, and
//! 3. nests them using each type's declared [`Nesting`].
//!
//! There is no per-type branching in the tree builder, so adding a block type
//! cannot introduce a hierarchy bug: the type declares its rank once and the
//! same generic algorithm applies.
//!
//! Block identity is type-aware:
//! - Headings get an id eagerly (slug of their text, deduplicated).
//! - YAML frontmatter gets the fixed id `frontmatter`.
//! - Other blocks are lazy (None) unless explicitly referenced.
//!
//! Content hash is the universal fallback for dedup/change detection.

use std::collections::{BTreeMap, HashMap};

use crate::block_types::{
    self, BlockTypeDef, IdPolicy, MetaField, MetaKind, MetaValue, Nesting, RANK_DIVIDER,
    RANK_SUBDIVIDER, classify_line, fence_marker, heading_level, list_marker, starts_new_block,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlockKind {
    Frontmatter,
    Heading {
        level: u8,
    },
    /// `---` (rank [`RANK_DIVIDER`]) or `***`/`___` (rank [`RANK_SUBDIVIDER`]).
    Divider {
        rank: u8,
    },
    Paragraph,
    CodeBlock {
        lang: Option<String>,
    },
    ListItem {
        indent: usize,
    },
    Quote,
    Callout,
    Table,
    Math,
    Html,
    Image,
    /// Escape hatch for user-defined block types; always a leaf.
    Custom {
        name: String,
    },
}

impl BlockKind {
    /// Registry key.
    pub fn type_name(&self) -> &str {
        match self {
            BlockKind::Frontmatter => "frontmatter",
            BlockKind::Heading { .. } => "heading",
            BlockKind::Divider { rank } if *rank == RANK_DIVIDER => "divider",
            BlockKind::Divider { .. } => "subdivider",
            BlockKind::Paragraph => "paragraph",
            BlockKind::CodeBlock { .. } => "code",
            BlockKind::ListItem { .. } => "list",
            BlockKind::Quote => "quote",
            BlockKind::Callout => "callout",
            BlockKind::Table => "table",
            BlockKind::Math => "math",
            BlockKind::Html => "html",
            BlockKind::Image => "image",
            BlockKind::Custom { name } => name,
        }
    }

    pub fn def(&self) -> Option<&'static BlockTypeDef> {
        block_types::by_name(self.type_name())
    }

    /// Declared nesting, falling back to `Leaf` for unknown custom types.
    pub fn nesting(&self) -> Nesting {
        self.def().map(|d| d.nesting).unwrap_or(Nesting::Leaf)
    }

    pub fn id_policy(&self) -> IdPolicy {
        self.def().map(|d| d.id).unwrap_or(IdPolicy::Lazy)
    }

    pub fn metadata_fields(&self) -> &'static [MetaField] {
        self.def().map(|d| d.metadata).unwrap_or(&[])
    }

    /// Effective hierarchy rank for this block.
    pub fn rank(&self) -> u8 {
        let level = match self {
            BlockKind::Heading { level } => *level,
            _ => 0,
        };
        self.nesting().rank(level)
    }
}

impl Block {
    /// A declared metadata value, if the parser could extract it.
    pub fn meta_value(&self, name: &str) -> Option<&MetaValue> {
        self.meta.get(name)
    }

    /// Text form of a declared metadata value.
    pub fn meta_text(&self, name: &str) -> Option<&str> {
        match self.meta.get(name) {
            Some(MetaValue::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// Declared fields that did not produce a value. Used by diagnostics; the
    /// parser fills every required field it derives, so a non-empty list means
    /// the file itself lacks required frontmatter.
    pub fn missing_metadata(&self) -> Vec<&'static str> {
        self.kind
            .metadata_fields()
            .iter()
            .filter(|f| f.required && !self.meta.contains_key(f.name))
            .map(|f| f.name)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: Option<String>,
    pub kind: BlockKind,
    pub content: String,
    /// Byte span [start, end) in the original text.
    pub span: (usize, usize),
    /// Values for this type's declared [`MetaField`]s, extracted by the parser.
    /// Missing or mistyped fields are absent rather than coerced.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, MetaValue>,
    pub children: Vec<Block>,
}

#[derive(Debug, Clone)]
struct FlatBlock {
    id: Option<String>,
    kind: BlockKind,
    content: String,
    span: (usize, usize),
    meta: BTreeMap<String, MetaValue>,
}

/// Parse grape markdown into a block tree.
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let flat = parse_flat(text);
    let tree = build_tree(flat);
    debug_assert!(
        validate_tree(&tree).is_ok(),
        "hierarchy invariant violated: {:?}",
        validate_tree(&tree)
    );
    tree
}

/// Find a block by id (depth-first).
pub fn find_block(blocks: &[Block], id: &str) -> Option<Block> {
    for b in blocks {
        if b.id.as_deref() == Some(id) {
            return Some(b.clone());
        }
        if let Some(found) = find_block(&b.children, id) {
            return Some(found);
        }
    }
    None
}

/// Find a block by its position path (`[0]` = first root, `[0, 2]` = third
/// child of the first root). Every block is addressable this way, including
/// leaves without ids, which is what the editor uses.
pub fn block_at_path<'a>(blocks: &'a [Block], path: &[usize]) -> Option<&'a Block> {
    let (first, rest) = path.split_first()?;
    let block = blocks.get(*first)?;
    if rest.is_empty() {
        Some(block)
    } else {
        block_at_path(&block.children, rest)
    }
}

/// Apply an edit addressed by position path. The tree is re-parsed first, so
/// the path always refers to the current document structure.
pub fn apply_block_edit_at_path(
    text: &str,
    path: &[usize],
    new_content: &str,
) -> Result<String, String> {
    let blocks = parse_blocks(text);
    let block =
        block_at_path(&blocks, path).ok_or_else(|| format!("block path not found: {:?}", path))?;
    let (start, end) = block.span;
    if start > end || end > text.len() {
        return Err(format!("invalid span for block path {:?}", path));
    }
    Ok(splice_block(text, block, new_content))
}

/// Splice new content into a block's byte span, preserving everything else.
/// Returns the full updated text. If the block's span is invalid, returns
/// the original text unchanged.
pub fn splice_block(text: &str, block: &Block, new_content: &str) -> String {
    let (start, end) = block.span;
    if start > end || end > text.len() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + new_content.len());
    out.push_str(&text[..start]);
    out.push_str(new_content);
    out.push_str(&text[end..]);
    out
}

/// Last byte of a block and all of its descendants. A section's own span ends
/// at its heading line, so inserting "after" a section must use this.
pub fn subtree_end(block: &Block) -> usize {
    block
        .children
        .iter()
        .fold(block.span.1, |acc, child| acc.max(subtree_end(child)))
}

/// Insert Markdown after a block (and its whole subtree). The new text is
/// separated by one blank line; everything before and after stays untouched.
/// The insertion point is after the subtree, so the parser's rank rules place
/// the new block at the intended level instead of inside the target's first
/// child position.
pub fn insert_after_path(text: &str, path: &[usize], content: &str) -> Result<String, String> {
    let content = content.trim_end_matches(['\r', '\n']);
    if content.trim().is_empty() {
        return Err("refusing to insert an empty block".into());
    }
    let blocks = parse_blocks(text);
    let block =
        block_at_path(&blocks, path).ok_or_else(|| format!("block path not found: {:?}", path))?;
    let at = subtree_end(block);
    if at > text.len() {
        return Err(format!("invalid span for block path {:?}", path));
    }
    // Block spans include the line ending, so the separator before the new
    // block is one newline when the prefix already ends with one.
    let before = if at == 0 || text[..at].ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let after = if at == text.len() || text[at..].starts_with('\n') {
        "\n"
    } else {
        ""
    };
    let mut out = String::with_capacity(text.len() + content.len() + 3);
    out.push_str(&text[..at]);
    out.push_str(before);
    out.push_str(content);
    out.push_str(after);
    out.push_str(&text[at..]);
    Ok(out)
}

/// Append Markdown at the end of a body, adding only the separation needed.
pub fn append_block(text: &str, content: &str) -> Result<String, String> {
    let content = content.trim_end_matches(['\r', '\n']);
    if content.trim().is_empty() {
        return Err("refusing to insert an empty block".into());
    }
    if text.trim().is_empty() {
        return Ok(format!("{}\n", content));
    }
    let mut out = text.to_string();
    if !out.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
    }
    out.push_str(content);
    out.push('\n');
    Ok(out)
}

/// Delete a block and its subtree. The line ending and a redundant blank line
/// are removed so repeated deletes do not leave growing gaps.
pub fn delete_at_path(text: &str, path: &[usize]) -> Result<String, String> {
    let blocks = parse_blocks(text);
    let block =
        block_at_path(&blocks, path).ok_or_else(|| format!("block path not found: {:?}", path))?;
    let start = block.span.0;
    let mut end = subtree_end(block);
    if start > end || end > text.len() {
        return Err(format!("invalid span for block path {:?}", path));
    }
    if text.as_bytes().get(end) == Some(&b'\n') {
        end += 1;
    }
    if start > 0 && text[..start].ends_with('\n') && text[end..].starts_with('\n') {
        end += 1;
    }
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..start]);
    out.push_str(&text[end..]);
    Ok(out)
}

/// Parse `new` text and reconcile against `old` text's block tree,
/// preserving IDs where content matches. Returns the reconciled new tree.
pub fn reconcile_text(old: &str, new: &str) -> Vec<Block> {
    let old_blocks = parse_blocks(old);
    let mut new_blocks = parse_blocks(new);
    reconcile_blocks(&old_blocks, &mut new_blocks);
    new_blocks
}

/// Content hash for a block (used for dedup and change detection).
pub fn block_hash(block: &Block) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    format!("{:?}", block.kind).hash(&mut h);
    block.content.hash(&mut h);
    format!("{:x}", h.finish())
}

/// Apply a single block edit: find the block by id, splice new content
/// into its byte span, preserving everything else byte-identical.
/// Returns the updated text, or an error if the block isn't found or the
/// span is invalid.
pub fn apply_block_edit(text: &str, block_id: &str, new_content: &str) -> Result<String, String> {
    let blocks = parse_blocks(text);
    let block =
        find_block(&blocks, block_id).ok_or_else(|| format!("block not found: {}", block_id))?;
    let (start, end) = block.span;
    if start > end || end > text.len() {
        return Err(format!("invalid span for block {}", block_id));
    }
    Ok(splice_block(text, &block, new_content))
}

/// Apply multiple block edits sequentially. Each edit re-parses the
/// (possibly changed) text, so spans stay consistent.
pub fn apply_block_edits(text: &str, edits: &[(String, String)]) -> Result<String, String> {
    let mut current = text.to_string();
    for (id, content) in edits {
        current = apply_block_edit(&current, id, content)?;
    }
    Ok(current)
}

/// Reconcile a new parse against a previous block tree, preserving IDs
/// where content matches (by id first, then by content hash).
pub fn reconcile_blocks(old: &[Block], new: &mut [Block]) {
    let mut id_by_hash: HashMap<String, String> = HashMap::new();
    let mut hash_by_id: HashMap<String, String> = HashMap::new();
    collect_old(old, &mut id_by_hash, &mut hash_by_id);
    for b in new.iter_mut() {
        reconcile_one(b, &id_by_hash, &hash_by_id);
    }
}

fn collect_old(
    blocks: &[Block],
    id_by_hash: &mut HashMap<String, String>,
    hash_by_id: &mut HashMap<String, String>,
) {
    for b in blocks {
        let h = block_hash(b);
        if let Some(id) = &b.id {
            id_by_hash.entry(h.clone()).or_insert_with(|| id.clone());
            hash_by_id.insert(id.clone(), h.clone());
        }
        collect_old(&b.children, id_by_hash, hash_by_id);
    }
}

fn reconcile_one(
    block: &mut Block,
    id_by_hash: &HashMap<String, String>,
    hash_by_id: &HashMap<String, String>,
) {
    let h = block_hash(block);
    if block.id.is_none() {
        if let Some(id) = id_by_hash.get(&h) {
            block.id = Some(id.clone());
        }
    } else if let Some(id) = &block.id {
        let _ = hash_by_id.get(id);
    }

    for c in block.children.iter_mut() {
        reconcile_one(c, id_by_hash, hash_by_id);
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn parse_flat(text: &str) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    let mut rest_start = 0;

    if let Some((end, fm_content)) = split_frontmatter(text) {
        out.push(FlatBlock {
            id: Some("frontmatter".to_string()),
            kind: BlockKind::Frontmatter,
            content: fm_content,
            span: (0, end),
            meta: BTreeMap::new(),
        });
        rest_start = end;
    }

    out.extend(parse_flat_rest(&text[rest_start..], rest_start));
    assign_ids(&mut out);
    for fb in &mut out {
        fb.meta = extract_meta(&fb.kind, fb.id.as_deref(), &fb.content);
    }
    out
}

/// Fill the values promised by a type's declared [`MetaField`]s.
///
/// Frontmatter is read from its YAML mapping; every other type derives its
/// fields from the parsed shape. A value whose YAML type does not match the
/// declaration is dropped instead of coerced, so a `Number` field can never
/// silently become text.
fn extract_meta(kind: &BlockKind, id: Option<&str>, content: &str) -> BTreeMap<String, MetaValue> {
    let mut meta = BTreeMap::new();
    if matches!(kind, BlockKind::Frontmatter) {
        let Ok(serde_yaml::Value::Mapping(map)) =
            serde_yaml::from_str::<serde_yaml::Value>(content)
        else {
            return meta;
        };
        for field in kind.metadata_fields() {
            if let Some(value) = map.get(field.name).and_then(|v| yaml_meta(v, field.kind)) {
                meta.insert(field.name.to_string(), value);
            }
        }
        return meta;
    }
    for field in kind.metadata_fields() {
        let value = match (field.name, kind) {
            ("id", _) => id.map(|s| MetaValue::Text(s.to_string())),
            ("level", BlockKind::Heading { level }) => Some(MetaValue::Number(*level as f64)),
            ("rank", BlockKind::Divider { rank }) => Some(MetaValue::Number(*rank as f64)),
            ("lang", BlockKind::CodeBlock { lang: Some(lang) }) => {
                Some(MetaValue::Text(lang.clone()))
            }
            ("indent", BlockKind::ListItem { indent }) => Some(MetaValue::Number(*indent as f64)),
            _ => None,
        };
        if let Some(value) = value {
            meta.insert(field.name.to_string(), value);
        }
    }
    meta
}

fn yaml_meta(value: &serde_yaml::Value, kind: MetaKind) -> Option<MetaValue> {
    match kind {
        MetaKind::Text | MetaKind::BlockRef => {
            value.as_str().map(|s| MetaValue::Text(s.to_string()))
        }
        MetaKind::Number => value.as_f64().map(MetaValue::Number),
        MetaKind::Bool => value.as_bool().map(MetaValue::Bool),
        MetaKind::List => value.as_sequence().map(|items| {
            MetaValue::List(
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
            )
        }),
    }
}

/// YAML frontmatter is `---` on line 1, a closing `---`, and a non-empty YAML
/// mapping in between. Requiring a mapping prevents a plain thematic break
/// (`--- *** *** ---`) from being mistaken for metadata.
fn split_frontmatter(text: &str) -> Option<(usize, String)> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\r', '\n']).trim() != "---" {
        return None;
    }

    let mut offset = first.len();
    let mut fm = String::new();
    for line in lines {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        if trimmed == "---" {
            let end = offset + line.len();
            let looks_like_mapping = matches!(
                serde_yaml::from_str::<serde_yaml::Value>(&fm),
                Ok(serde_yaml::Value::Mapping(ref m)) if !m.is_empty()
            );
            return if looks_like_mapping {
                Some((end, fm))
            } else {
                None
            };
        }
        fm.push_str(line);
        offset += line.len();
    }
    None
}

fn parse_flat_rest(text: &str, base_offset: usize) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut off = base_offset;
    for line in text.split_inclusive('\n') {
        lines.push((off, line));
        off += line.len();
    }

    let mut i = 0;
    while i < lines.len() {
        if lines[i].1.trim().is_empty() {
            i += 1;
            continue;
        }

        // Fenced blocks (code/math) consume blank lines until their closing
        // marker; they are never split by the structural-line rule.
        if let Some(marker) = closing_marker(lines[i].1) {
            let mut j = i + 1;
            while j < lines.len() {
                let t = lines[j].1.trim();
                let closes = if marker == "$$" {
                    t == "$$"
                } else {
                    t.starts_with(marker)
                };
                j += 1;
                if closes {
                    break;
                }
            }
            push_range(&mut out, &lines, i, j);
            i = j;
            continue;
        }

        let def = classify_line(lines[i].1);
        let mut j = i + 1;
        if def.nesting.is_section() {
            // Headings/dividers own exactly their own line; following content
            // becomes their children.
        } else {
            while j < lines.len() && !lines[j].1.trim().is_empty() && !starts_new_block(lines[j].1)
            {
                j += 1;
            }
        }
        push_range(&mut out, &lines, i, j);
        i = j;
    }
    out
}

fn closing_marker(line: &str) -> Option<&str> {
    if let Some(m) = fence_marker(line) {
        return Some(m);
    }
    if line.trim() == "$$" {
        return Some("$$");
    }
    None
}

fn push_range(out: &mut Vec<FlatBlock>, lines: &[(usize, &str)], i: usize, j: usize) {
    let start = lines[i].0;
    let end = lines[j - 1].0 + lines[j - 1].1.len();
    let mut content = String::new();
    for (_, line) in &lines[i..j] {
        content.push_str(line);
    }
    let content = content.trim_end_matches('\n').to_string();
    let kind = classify_block(&content);
    out.push(FlatBlock {
        id: None,
        kind,
        content,
        span: (start, end),
        meta: BTreeMap::new(),
    });
}

fn classify_block(content: &str) -> BlockKind {
    let first = content.lines().next().unwrap_or("").trim();
    let def = classify_line(first);
    match def.syntax {
        block_types::Syntax::Frontmatter => BlockKind::Frontmatter,
        block_types::Syntax::Heading => BlockKind::Heading {
            level: heading_level(first).unwrap_or(1),
        },
        block_types::Syntax::ThematicBreak => BlockKind::Divider { rank: RANK_DIVIDER },
        block_types::Syntax::SubDivider => BlockKind::Divider {
            rank: RANK_SUBDIVIDER,
        },
        block_types::Syntax::Fence => BlockKind::CodeBlock {
            lang: fence_lang(first),
        },
        block_types::Syntax::Callout => BlockKind::Callout,
        block_types::Syntax::Quote => BlockKind::Quote,
        block_types::Syntax::List => BlockKind::ListItem {
            indent: list_marker(first).unwrap_or(0),
        },
        block_types::Syntax::Table => BlockKind::Table,
        block_types::Syntax::Math => BlockKind::Math,
        block_types::Syntax::Html => BlockKind::Html,
        block_types::Syntax::Image => BlockKind::Image,
        block_types::Syntax::Paragraph => BlockKind::Paragraph,
        block_types::Syntax::Custom => BlockKind::Custom {
            name: def.name.to_string(),
        },
    }
}

fn fence_lang(line: &str) -> Option<String> {
    let t = line.trim_start();
    let rest = t
        .strip_prefix("```")
        .or_else(|| t.strip_prefix("~~~"))?
        .trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.split_whitespace().next().unwrap_or(rest).to_string())
    }
}

fn assign_ids(flat: &mut [FlatBlock]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for fb in flat {
        fb.id = match fb.kind.id_policy() {
            IdPolicy::EagerFixed(id) => Some(id.to_string()),
            IdPolicy::EagerSlug => {
                let base = slug_id(&fb.content);
                let count = seen.entry(base.clone()).or_insert(0);
                *count += 1;
                Some(if *count == 1 {
                    base
                } else {
                    format!("{}-{}", base, count)
                })
            }
            IdPolicy::Lazy | IdPolicy::ContentHash => None,
        };
    }
}

fn slug_id(text: &str) -> String {
    let t = text.trim_start_matches('#').trim();
    let mut s = String::new();
    for c in t.chars() {
        if c.is_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '-') && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_matches('-');
    if s.is_empty() {
        "heading".to_string()
    } else {
        s.to_string()
    }
}

fn build_tree(flat: Vec<FlatBlock>) -> Vec<Block> {
    let mut roots: Vec<Block> = Vec::new();
    let mut stack: Vec<(u8, Block)> = Vec::new();

    for fb in flat {
        let nesting = fb.kind.nesting();
        let rank = fb.kind.rank();
        let block = Block {
            id: fb.id,
            kind: fb.kind,
            content: fb.content,
            span: fb.span,
            meta: fb.meta,
            children: Vec::new(),
        };

        match nesting {
            Nesting::Root => {
                flush_all(&mut stack, &mut roots);
                roots.push(block);
            }
            Nesting::Section(_) | Nesting::Heading { .. } => {
                flush_while(&mut stack, &mut roots, rank);
                stack.push((rank, block));
            }
            Nesting::Leaf => {
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push(block);
                } else {
                    roots.push(block);
                }
            }
        }
    }

    flush_all(&mut stack, &mut roots);
    roots
}

fn flush_while(stack: &mut Vec<(u8, Block)>, roots: &mut Vec<Block>, rank: u8) {
    while let Some((top_rank, _)) = stack.last() {
        if *top_rank >= rank {
            let (_, finished) = stack.pop().expect("checked above");
            if let Some((_, parent)) = stack.last_mut() {
                parent.children.push(finished);
            } else {
                roots.push(finished);
            }
        } else {
            break;
        }
    }
}

fn flush_all(stack: &mut Vec<(u8, Block)>, roots: &mut Vec<Block>) {
    while let Some((_, finished)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(finished);
        } else {
            roots.push(finished);
        }
    }
}

/// Structural invariants of a parsed tree. Used in debug builds and tests, and
/// exposed so tools can validate external/imported trees.
pub fn validate_tree(blocks: &[Block]) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for b in blocks {
        validate_block(b, None, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_block(block: &Block, parent_rank: Option<u8>, errors: &mut Vec<String>) {
    let nesting = block.kind.nesting();
    if nesting == Nesting::Leaf && !block.children.is_empty() {
        errors.push(format!(
            "leaf block {:?} has {} children",
            block.kind,
            block.children.len()
        ));
    }
    if let Some(pr) = parent_rank
        && nesting.is_section()
        && block.kind.rank() <= pr
    {
        errors.push(format!(
            "section {:?} (rank {}) is not deeper than its parent (rank {})",
            block.kind,
            block.kind.rank(),
            pr
        ));
    }
    if nesting.is_section() {
        for c in &block.children {
            validate_block(c, Some(block.kind.rank()), errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_valid() {
        block_types::validate_registry().expect("registry invariants");
    }

    #[test]
    fn heading_gets_id() {
        let blocks = parse_blocks("# Hello World\n\nSome text.\n");
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::Heading { level: 1 } => {}
            _ => panic!("expected heading"),
        }
        assert_eq!(blocks[0].id.as_deref(), Some("hello-world"));
        assert_eq!(blocks[0].children.len(), 1);
    }

    #[test]
    fn duplicate_headings_get_unique_ids() {
        let blocks = parse_blocks("# Same\n\n## Same\n\n# Same\n");
        let mut ids = Vec::new();
        fn collect(bs: &[Block], ids: &mut Vec<String>) {
            for b in bs {
                if let Some(id) = &b.id {
                    ids.push(id.clone());
                }
                collect(&b.children, ids);
            }
        }
        collect(&blocks, &mut ids);
        assert_eq!(ids, vec!["same", "same-2", "same-3"]);
    }

    #[test]
    fn non_heading_lazy_id() {
        let blocks = parse_blocks("Just a paragraph.\n");
        assert_eq!(blocks[0].id, None);
    }

    #[test]
    fn frontmatter_eager_id() {
        let text = "---\nid: rec_1\ntype: page\n---\n\n# Title\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks[0].id.as_deref(), Some("frontmatter"));
        assert!(matches!(blocks[0].kind, BlockKind::Frontmatter));
    }

    #[test]
    fn divider_run_is_not_frontmatter() {
        let blocks = parse_blocks("---\n***\n***\n---\n");
        assert!(
            matches!(blocks[0].kind, BlockKind::Divider { rank: RANK_DIVIDER }),
            "first block should be a divider, got {:?}",
            blocks[0].kind
        );
        assert!(
            !blocks
                .iter()
                .any(|b| matches!(b.kind, BlockKind::Frontmatter)),
            "a thematic-break run must not be mistaken for YAML"
        );
    }

    #[test]
    fn ranked_divider_nesting() {
        // --- opens a top section; *** is one rank lower and splits it.
        let blocks = parse_blocks("---\n***\n***\n---\n");
        assert_eq!(blocks[0].kind, BlockKind::Divider { rank: RANK_DIVIDER });
        assert_eq!(blocks[0].children.len(), 2);
        for c in &blocks[0].children {
            assert_eq!(
                c.kind,
                BlockKind::Divider {
                    rank: RANK_SUBDIVIDER
                }
            );
        }
        // Every top-level section is a divider; none is a sibling of *** at
        // the same rank as the enclosing ---.
        for b in &blocks {
            assert_eq!(b.kind, BlockKind::Divider { rank: RANK_DIVIDER });
        }
        validate_tree(&blocks).expect("tree invariants");
    }

    #[test]
    fn divider_splits_sections() {
        let text = "---\n# A\n\ntext\n\n---\n# B\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].children.len(), 1);
        assert!(matches!(
            blocks[0].children[0].kind,
            BlockKind::Heading { level: 1 }
        ));
        assert_eq!(blocks[1].children.len(), 1);
    }

    #[test]
    fn nesting_under_heading() {
        let text = "# H1\n\npara1\n\n## H2\n\npara2\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].children.len(), 2);
        validate_tree(&blocks).expect("tree invariants");
    }

    #[test]
    fn heading_splits_without_blank_line() {
        let blocks = parse_blocks("# A\n## B\n### C\ntext\n");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].children.len(), 1);
        assert_eq!(blocks[0].children[0].children.len(), 1);
        assert_eq!(blocks[0].children[0].children[0].children.len(), 1);
    }

    #[test]
    fn byte_spans() {
        let text = "# Hi\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks[0].span, (0, 5));
    }

    #[test]
    fn horizontal_rule() {
        let blocks = parse_blocks("---\n");
        assert!(matches!(
            blocks[0].kind,
            BlockKind::Divider { rank: RANK_DIVIDER }
        ));
    }

    #[test]
    fn subdivider_and_underscore() {
        let stars = parse_blocks("***\n");
        let underscores = parse_blocks("___\n");
        assert_eq!(
            stars[0].kind,
            BlockKind::Divider {
                rank: RANK_SUBDIVIDER
            }
        );
        assert_eq!(
            underscores[0].kind,
            BlockKind::Divider {
                rank: RANK_SUBDIVIDER
            }
        );
    }

    #[test]
    fn code_fence_keeps_blank_lines() {
        let text = "```rust\nfn main() {}\n\n// comment\n```\n\nafter\n";
        let blocks = parse_blocks(text);
        assert!(matches!(
            blocks[0].kind,
            BlockKind::CodeBlock { lang: Some(ref lang) } if lang == "rust"
        ));
        assert!(blocks[0].content.contains("\n\n"));
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn callout_before_quote() {
        let text = "> [!note] Title\n> body\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks[0].kind, BlockKind::Callout);
    }

    #[test]
    fn other_block_types() {
        assert!(matches!(
            parse_blocks("| a | b |\n|---|---|\n")[0].kind,
            BlockKind::Table
        ));
        assert!(matches!(
            parse_blocks("$$\nx\n$$\n")[0].kind,
            BlockKind::Math
        ));
        assert!(matches!(
            parse_blocks("<div>x</div>\n")[0].kind,
            BlockKind::Html
        ));
        assert!(matches!(
            parse_blocks("![alt](x.png)\n")[0].kind,
            BlockKind::Image
        ));
        assert!(matches!(
            parse_blocks("- item\n")[0].kind,
            BlockKind::ListItem { indent: 0 }
        ));
    }

    #[test]
    fn every_kind_maps_to_registry() {
        let kinds = vec![
            BlockKind::Frontmatter,
            BlockKind::Heading { level: 2 },
            BlockKind::Divider { rank: RANK_DIVIDER },
            BlockKind::Divider {
                rank: RANK_SUBDIVIDER,
            },
            BlockKind::Paragraph,
            BlockKind::CodeBlock { lang: None },
            BlockKind::ListItem { indent: 0 },
            BlockKind::Quote,
            BlockKind::Callout,
            BlockKind::Table,
            BlockKind::Math,
            BlockKind::Html,
            BlockKind::Image,
        ];
        for kind in kinds {
            assert!(
                kind.def().is_some(),
                "no registry definition for {:?} (type_name {})",
                kind,
                kind.type_name()
            );
        }
    }

    #[test]
    fn find_block_by_id() {
        let blocks = parse_blocks("# Hi\n\npara\n");
        let found = find_block(&blocks, "hi");
        assert!(found.is_some());
        assert!(find_block(&blocks, "nope").is_none());
    }

    #[test]
    fn splice_preserves_rest() {
        let text = "# Hi\n\npara\n";
        let blocks = parse_blocks(text);
        let b = find_block(&blocks, "hi").unwrap();
        let out = splice_block(text, &b, "# Hello\n");
        assert_eq!(out, "# Hello\n\npara\n");
        // bytes outside span preserved
        assert!(out.ends_with("para\n"));
    }

    #[test]
    fn splice_invalid_span_returns_original() {
        let text = "# Hi\n";
        let mut b = parse_blocks(text)[0].clone();
        b.span = (100, 200);
        assert_eq!(splice_block(text, &b, "x"), text);
    }

    #[test]
    fn insert_after_leaf_preserves_rest() {
        let text = "# Hi\n\npara\n\n# Next\n";
        let out = insert_after_path(text, &[0, 0], "added").unwrap();
        assert_eq!(out, "# Hi\n\npara\n\nadded\n\n# Next\n");
    }

    #[test]
    fn insert_after_section_skips_its_children() {
        let text = "# A\n\nchild\n\n# B\n";
        // Inserting after the section must land after `child`, not between
        // the heading and its child.
        let out = insert_after_path(text, &[0, 0], "added").unwrap();
        assert_eq!(out, "# A\n\nchild\n\nadded\n\n# B\n");
        let blocks = parse_blocks(&out);
        assert_eq!(blocks[0].children.len(), 2);
    }

    #[test]
    fn append_to_empty_body() {
        assert_eq!(append_block("", "# Hi").unwrap(), "# Hi\n");
        assert_eq!(append_block("\n", "para").unwrap(), "para\n");
        assert_eq!(append_block("# A\n\nb\n", "c").unwrap(), "# A\n\nb\n\nc\n");
    }

    #[test]
    fn insert_rejects_empty_content() {
        assert!(insert_after_path("# A\n", &[0], "  \n").is_err());
        assert!(append_block("", "\n").is_err());
    }

    #[test]
    fn delete_leaf_collapses_blank_line() {
        let text = "# A\n\npara\n\n# B\n";
        let out = delete_at_path(text, &[0, 0]).unwrap();
        assert_eq!(out, "# A\n\n# B\n");
    }

    #[test]
    fn delete_section_removes_subtree() {
        let text = "# A\n\na-child\n\n## A2\n\na2-child\n\n# B\n\nb-child\n";
        let out = delete_at_path(text, &[0]).unwrap();
        assert_eq!(out, "# B\n\nb-child\n");
    }

    #[test]
    fn delete_only_block_leaves_empty_body() {
        let out = delete_at_path("# A\n", &[0]).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn block_hash_stable() {
        let b = Block {
            id: Some("h".into()),
            kind: BlockKind::Paragraph,
            content: "hello".into(),
            span: (0, 5),
            meta: BTreeMap::new(),
            children: vec![],
        };
        assert_eq!(block_hash(&b), block_hash(&b));
    }

    #[test]
    fn declared_metadata_is_extracted_for_each_type() {
        let blocks = parse_blocks("# Hello\n\n```rust\nfn main() {}\n```\n\n- item\n");
        let heading = &blocks[0];
        assert_eq!(heading.meta_value("level"), Some(&MetaValue::Number(1.0)));
        assert_eq!(heading.meta_text("id"), Some("hello"));
        let code = &heading.children[0];
        assert_eq!(code.meta_text("lang"), Some("rust"));
        let list = &heading.children[1];
        assert_eq!(list.meta_value("indent"), Some(&MetaValue::Number(0.0)));
    }

    #[test]
    fn frontmatter_metadata_comes_from_yaml_and_respects_declared_types() {
        let text =
            "---\nid: rec_1\ntype: page\ntitle: Title\ntags:\n- rust\n- local\n---\n\nbody\n";
        let blocks = parse_blocks(text);
        let fm = &blocks[0];
        assert_eq!(fm.meta_text("id"), Some("rec_1"));
        assert_eq!(fm.meta_text("type"), Some("page"));
        assert_eq!(fm.meta_text("title"), Some("Title"));
        assert_eq!(
            fm.meta_value("tags"),
            Some(&MetaValue::List(vec![
                "rust".to_string(),
                "local".to_string()
            ]))
        );
        assert!(fm.missing_metadata().is_empty());
    }

    #[test]
    fn frontmatter_without_required_fields_reports_them_missing() {
        let text = "---\ntitle: No identity\n---\n\nbody\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks[0].missing_metadata(), vec!["id", "type"]);
    }

    #[test]
    fn reconcile_preserves_id_on_matching_content() {
        let old = parse_blocks("# Title\n\npara\n");
        let mut new = parse_blocks("# Title\n\npara\n");
        for b in new.iter_mut() {
            b.id = None;
            for c in b.children.iter_mut() {
                c.id = None;
            }
        }
        reconcile_blocks(&old, &mut new);
        assert_eq!(new[0].id.as_deref(), Some("title"));
    }

    #[test]
    fn apply_block_edit_single() {
        let text = "# Hi\n\npara\n";
        let blocks = parse_blocks(text);
        let _b = find_block(&blocks, "hi").unwrap();
        let out = apply_block_edit(text, "hi", "# Hello\n").unwrap();
        assert_eq!(out, "# Hello\n\npara\n");
    }

    #[test]
    fn apply_block_edit_missing() {
        let text = "# Hi\n";
        assert!(apply_block_edit(text, "nope", "x").is_err());
    }

    #[test]
    fn apply_block_edits_sequential() {
        let text = "# A\n\n# B\n";
        let edits = vec![
            ("a".to_string(), "# A1\n".to_string()),
            ("b".to_string(), "# B1\n".to_string()),
        ];
        let out = apply_block_edits(text, &edits).unwrap();
        assert!(out.contains("A1"));
        assert!(out.contains("B1"));
    }

    #[test]
    fn apply_block_edit_preserves_surrounding() {
        let text = "before\n\n# Hi\n\nafter\n";
        let out = apply_block_edit(text, "hi", "# Hello\n").unwrap();
        assert!(out.starts_with("before\n\n"));
        assert!(out.ends_with("after\n"));
    }
}
