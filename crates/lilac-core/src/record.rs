use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A Lilac record. Frontmatter only; body is handled by the storage layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,

    // display
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // timestamps (RFC3339 strings)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_viewed: Option<String>,

    // references
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,

    // classification
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    // url for page records
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    // wisteria-compat (reserved, no-op v0.1)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<String>,

    // snapshot-awareness (Q2 = A-aware-of-B)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_format: Option<String>,

    // open metadata (round-trips untouched)
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl Record {
    pub fn new(id: impl Into<String>, r#type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            r#type: r#type.into(),
            version: Some(1),
            title: None,
            icon: None,
            color: None,
            description: None,
            created: None,
            modified: None,
            opened: None,
            last_viewed: None,
            parent: None,
            related: vec![],
            links: vec![],
            tags: vec![],
            categories: vec![],
            status: None,
            url: None,
            authority: None,
            lineage: None,
            forked_from: None,
            snapshot: None,
            snapshot_format: None,
            extra: BTreeMap::new(),
        }
    }
}
