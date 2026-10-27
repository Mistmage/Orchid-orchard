//! Block model for grape markdown.
//!
//! Rust is the sole authority for document structure: parsing, block identity,
//! byte spans, and hierarchy all live here. The frontend only renders.
//!
//! Block identity is type-aware:
//!     - Headings get an id eagerly (slug of their text).
//!     - YAML frontmatter gets an id eagerly ("frontmatter").
//!     - Other blocks are lazy (None) unless explicitly referenced.
//! Content hash is the universal fallback for dedup/change detection.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlockKind {
    Frontmatter,
    Heading { level: u8 },
    Paragraph,
    CodeBlock { lang: Option<String> },
    ListItem { indent: usize },
    HorizontalRule,
    Blockquote,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: Option<String>,
    pub kind: BlockKind,
    pub content: String,
    /// Byte span [start, end) in the original text.
    pub span: (usize, usize),
    pub children: Vec<Block>,
}

#[derive(Debug, Clone)]
struct FlatBlock {
    id: Option<String>,
    kind: BlockKind,
    content: String,
    span: (usize, usize),
}

/// Parse grape markdown into a block tree.
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let flat = parse_flat(text);
    build_tree(flat)
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
    let block = find_block(&blocks, block_id).ok_or_else(|| format!("block not found: {}", block_id))?;
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

fn parse_flat(text: &str) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    // First, handle YAML frontmatter if present.
    if text.starts_with("---") {
        let mut end_byte = 0;
        let mut byte = 0;
        let mut found_close = false;
        let mut close_line_idx = 0;
        for (idx, line) in lines.iter().enumerate() {
            if idx == 0 {
                byte += line.len();
                continue;
            }
            if line.trim() == "---\n" || line.trim() == "---" {
                found_close = true;
                close_line_idx = idx;
                byte += line.len();
                end_byte = byte;
                break;
            }
            byte += line.len();
        }
        if found_close {
            let fm_content: String = lines[1..close_line_idx].concat();
            out.push(FlatBlock {
                id: Some("frontmatter".to_string()),
                kind: BlockKind::Frontmatter,
                content: fm_content.trim_end_matches('\n').to_string(),
                span: (0, end_byte),
            });
            let rest_text = &text[end_byte..];
            out.extend(parse_flat_rest(rest_text, end_byte));
            return out;
        }
    }
    out.extend(parse_flat_rest(text, 0));
    out
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
        let start = lines[i].0;
        let mut j = i;
        let mut content = String::new();
        while j < lines.len() && !lines[j].1.trim().is_empty() {
            content.push_str(lines[j].1);
            j += 1;
        }
        let end = lines[j - 1].0 + lines[j - 1].1.len();
        let trimmed = content.trim_end_matches('\n').to_string();
        let kind = classify_block(&trimmed);
        let id = match &kind {
            BlockKind::Heading { .. } => Some(slug_id(&trimmed)),
            _ => None,
        };
        out.push(FlatBlock {
            id,
            kind,
            content: trimmed,
            span: (start, end),
        });
        i = j;
    }
    out
}

fn classify_block(content: &str) -> BlockKind {
    let first = content.lines().next().unwrap_or("").trim();
    if first.starts_with("# ") {
        BlockKind::Heading { level: 1 }
    } else if first.starts_with("## ") {
        BlockKind::Heading { level: 2 }
    } else if first.starts_with("### ") {
        BlockKind::Heading { level: 3 }
    } else if first.starts_with("#### ") {
        BlockKind::Heading { level: 4 }
    } else if first.starts_with("```") {
        BlockKind::CodeBlock { lang: None }
    } else if first == "---" || first == "***" || first == "___" {
        BlockKind::HorizontalRule
    } else if first.starts_with("> ") || first.starts_with(">") {
        BlockKind::Blockquote
    } else if first.starts_with("- ") || first.starts_with("* ") || first.starts_with("+ ") {
        BlockKind::ListItem { indent: 0 }
    } else {
        BlockKind::Paragraph
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
    let mut root_children: Vec<Block> = Vec::new();
    let mut stack: Vec<(u8, Block)> = Vec::new();
    for fb in flat {
        let level = match &fb.kind {
            BlockKind::Heading { level } => *level,
            _ => 0,
        };
        let block = Block {
            id: fb.id,
            kind: fb.kind,
            content: fb.content,
            span: fb.span,
            children: Vec::new(),
        };
        if level > 0 {
            while let Some((lvl, _)) = stack.last() {
                if *lvl >= level {
                    let (_, finished) = stack.pop().unwrap();
                    if let Some((_, parent)) = stack.last_mut() {
                        parent.children.push(finished);
                    } else {
                        root_children.push(finished);
                    }
                } else {
                    break;
                }
            }
            stack.push((level, block));
        } else if let Some((_, top)) = stack.last_mut() {
            top.children.push(block);
        } else {
            root_children.push(block);
        }
    }
    while let Some((_, finished)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(finished);
        } else {
            root_children.push(finished);
        }
    }
    root_children
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn nesting_under_heading() {
        let text = "# H1\n\npara1\n\n## H2\n\npara2\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].children.len(), 2);
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
        assert!(matches!(blocks[0].kind, BlockKind::HorizontalRule));
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
    fn block_hash_stable() {
        let b = Block {
            id: Some("h".into()),
            kind: BlockKind::Paragraph,
            content: "hello".into(),
            span: (0, 5),
            children: vec![],
        };
        assert_eq!(block_hash(&b), block_hash(&b));
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
        let b = find_block(&blocks, "hi").unwrap();
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
        let edits = vec![("a".to_string(), "# A1\n".to_string()), ("b".to_string(), "# B1\n".to_string())];
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
