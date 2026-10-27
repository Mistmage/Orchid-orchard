//! Collection body parsing/rendering.
//!
//! A `type: collection` record stores its members as a Markdown list in the
//! body. Each item is either a reference to another record (`[[id]]`) or a
//! raw URL. This module is the single source of truth for that format so the
//! browser client and the core agree.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionItem {
    Ref(String),
    Url(String),
}

/// Parse a collection body into an ordered list of items.
pub fn parse_collection_body(body: &str) -> Vec<CollectionItem> {
    let mut out = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // strip list marker
        let item = line
            .trim_start_matches("- ")
            .trim_start_matches("* ")
            .trim_start_matches("+ ")
            .trim();
        if item.is_empty() {
            continue;
        }
        // wikilink ref
        if let Some(rest) = item.strip_prefix("[[")
            && let Some(end) = rest.find("]]")
        {
            let id = rest[..end].trim();
            if !id.is_empty() {
                out.push(CollectionItem::Ref(id.to_string()));
                continue;
            }
        }
        // inline wikilink anywhere in the line
        if let Some(start) = item.find("[[")
            && let Some(end) = item[start..].find("]]")
        {
            let id = item[start + 2..start + end].trim();
            if !id.is_empty() {
                out.push(CollectionItem::Ref(id.to_string()));
                continue;
            }
        }
        // raw url
        let candidate = item.trim_matches('`').trim();
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            out.push(CollectionItem::Url(candidate.to_string()));
            continue;
        }
        // fallback: treat as url if it looks like one
        if candidate.contains("://") {
            out.push(CollectionItem::Url(candidate.to_string()));
        }
        // otherwise skip (unknown line)
    }
    out
}

/// Render items back into the canonical body form.
pub fn render_collection_body(items: &[CollectionItem]) -> String {
    let mut out = String::new();
    for it in items {
        match it {
            CollectionItem::Ref(id) => out.push_str(&format!("- [[{}]]\n", id)),
            CollectionItem::Url(u) => out.push_str(&format!("- {}\n", u)),
        }
    }
    out.trim_end_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mixed() {
        let body = "- [[rec_a]]\n- https://example.com\n* [[rec_b]]\n";
        let items = parse_collection_body(body);
        assert_eq!(
            items,
            vec![
                CollectionItem::Ref("rec_a".into()),
                CollectionItem::Url("https://example.com".into()),
                CollectionItem::Ref("rec_b".into()),
            ]
        );
    }

    #[test]
    fn parse_empty() {
        assert!(parse_collection_body("").is_empty());
        assert!(parse_collection_body("   \n\n").is_empty());
    }

    #[test]
    fn roundtrip() {
        let items = vec![
            CollectionItem::Ref("rec_x".into()),
            CollectionItem::Url("https://y".into()),
        ];
        let body = render_collection_body(&items);
        assert_eq!(parse_collection_body(&body), items);
    }
}
