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

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

fn parse_flat(text: &str) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    // First, handle YAML frontmatter if present.
    if text.starts_with("---") {
        // find closing ---
        let mut end_byte = 0;
        // skip first line
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
            // continue parsing rest from close_line_idx+1
            // Recompute lines slice
            let rest_text = &text[end_byte..];
            out.extend(parse_flat_rest(rest_text, end_byte));
            return out;
        }
    }
    // No frontmatter: parse whole text.
    out.extend(parse_flat_rest(text, 0));
    out
}

fn parse_flat_rest(text: &str, base_offset: usize) -> Vec<FlatBlock> {
    let mut out = Vec::new();
    let mut lines: Vec<(usize, &str)> = Vec::new(); // (byte_offset, line)
    let mut off = base_offset;
    for line in text.split_inclusive('\n') {
        lines.push((off, line));
        off += line.len();
    }
    // Group consecutive non-blank lines into blocks.
    let mut i = 0;
    while i < lines.len() {
        // skip blank lines
        if lines[i].1.trim().is_empty() {
            i += 1;
            continue;
        }
        let start = lines[i].0;
        // collect run of non-blank lines
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
    // Fix heading level in kind (classify set level). Already done.
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
            // finalize headings with level >= this
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
        // "Some text." nests under the heading (half-hidden hierarchy).
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
        assert_eq!(blocks[0].children.len(), 2); // para1 + H2
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
}
