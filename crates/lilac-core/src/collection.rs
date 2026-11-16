//! Collection body parsing/rendering.
//!
//! A `type: collection` record stores its members in two places that must agree:
//! an ordered `items` list in frontmatter (the structured form) and a Markdown
//! list in the body (the portable, human-readable form). The structured list is
//! authoritative when present; bodies from older builds are still parsed, so a
//! collection written before `items` existed keeps working.
//!
//! This module is the single source of truth for both forms so the browser
//! client and the core agree.

use crate::record::Record;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionItem {
    Ref(String),
    Url(String),
}

/// One member of a collection. `id` is absent only until the page record has
/// been written (or for a raw URL the user added by hand). Group and pinning
/// are hints captured at save time; reopening uses them to rebuild the shape.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_color: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pinned: bool,
}

impl CollectionEntry {
    pub fn from_ref(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..Self::default()
        }
    }

    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            ..Self::default()
        }
    }

    /// Identity used for de-duplication: a record id when saved, otherwise the
    /// URL. Two entries with the same URL but different ids are still separate
    /// records, so the id wins.
    pub fn key(&self) -> String {
        match &self.id {
            Some(id) => format!("id:{id}"),
            None => format!("url:{}", self.url.trim().to_lowercase()),
        }
    }
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

/// Body lines for structured entries. Mirrors `render_collection_body` so a
/// file edited outside Orchard keeps its readable shape, while the frontmatter
/// keeps the details the body omits (title, group, pinning).
pub fn render_collection_entries(entries: &[CollectionEntry]) -> String {
    let items: Vec<CollectionItem> = entries
        .iter()
        .map(|entry| match &entry.id {
            Some(id) => CollectionItem::Ref(id.clone()),
            None => CollectionItem::Url(entry.url.clone()),
        })
        .collect();
    render_collection_body(&items)
}

/// Parse a legacy body into structured entries. Titles and groups are unknown
/// here; callers can fill them from the referenced records.
pub fn parse_collection_entries(body: &str) -> Vec<CollectionEntry> {
    parse_collection_body(body)
        .into_iter()
        .map(|item| match item {
            CollectionItem::Ref(id) => CollectionEntry::from_ref(id),
            CollectionItem::Url(url) => CollectionEntry::from_url(url),
        })
        .collect()
}

/// The entries a collection actually has: the structured list when present,
/// otherwise the legacy body. `record.items` is authoritative so removing an
/// item from it always wins over a stale body line.
pub fn collection_entries(record: &Record, body: Option<&str>) -> Vec<CollectionEntry> {
    if !record.items.is_empty() {
        return record.items.clone();
    }
    body.map(parse_collection_entries).unwrap_or_default()
}

/// Drop later entries whose key already appeared, keeping first-seen order.
pub fn dedupe_entries(entries: Vec<CollectionEntry>) -> Vec<CollectionEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for entry in entries {
        if seen.insert(entry.key()) {
            out.push(entry);
        }
    }
    out
}

/// Keep the frontmatter list and the body in sync from one in-memory list.
pub fn set_collection_entries(record: &mut Record, entries: Vec<CollectionEntry>) -> String {
    let entries = dedupe_entries(entries);
    record.items = entries.clone();
    render_collection_entries(&entries)
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
    fn structured_entries_render_to_a_readable_body_and_round_trip() {
        let entries = vec![
            CollectionEntry {
                id: Some("rec_a".into()),
                url: "https://a.example".into(),
                title: Some("A".into()),
                group: Some("Design".into()),
                group_color: Some("blue".into()),
                pinned: true,
            },
            CollectionEntry::from_url("https://b.example"),
        ];
        let body = render_collection_entries(&entries);
        assert_eq!(body, "- [[rec_a]]\n- https://b.example");
        let parsed = parse_collection_entries(&body);
        assert_eq!(parsed[0].id.as_deref(), Some("rec_a"));
        assert_eq!(parsed[1].url, "https://b.example");
        assert!(parsed[1].id.is_none());
    }

    #[test]
    fn structured_items_win_over_a_stale_body() {
        let mut record = Record::new("rec_col", "collection");
        record.items = vec![CollectionEntry::from_ref("rec_keep")];
        let body = "- [[rec_old]]";
        let entries = collection_entries(&record, Some(body));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.as_deref(), Some("rec_keep"));

        let legacy = Record::new("rec_legacy", "collection");
        let entries = collection_entries(&legacy, Some(body));
        assert_eq!(entries[0].id.as_deref(), Some("rec_old"));
    }

    #[test]
    fn syncing_entries_dedupes_and_keeps_one_list() {
        let mut record = Record::new("rec_col", "collection");
        let entries = vec![
            CollectionEntry::from_ref("rec_a"),
            CollectionEntry::from_ref("rec_a"),
            CollectionEntry::from_url("https://B.example"),
            CollectionEntry::from_url("https://b.example"),
        ];
        let body = set_collection_entries(&mut record, entries);
        assert_eq!(record.items.len(), 2);
        assert_eq!(body, "- [[rec_a]]\n- https://B.example");
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
