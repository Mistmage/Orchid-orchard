//! Human-readable record naming rules.
//!
//! Derives a display title (and slug source) from a record using ordered
//! rules: strip noise, replace, affix, truncate. Pure logic, no I/O.

use crate::record::Record;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NamingRule {
    /// Substrings to remove (case-sensitive) from the base title.
    pub strip: Vec<String>,
    /// (from, to) replacements applied in order.
    pub replace: Vec<(String, String)>,
    /// Optional prefix template. Supports `{host}`, `{title}`.
    pub prefix: Option<String>,
    /// Optional suffix template. Supports `{host}`, `{title}`.
    pub suffix: Option<String>,
    /// Truncate final result to this many chars (append "…" if truncated).
    pub max_len: Option<usize>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NamingConfig {
    pub rules: Vec<NamingRule>,
}

impl NamingRule {
    pub fn apply(&self, base: &str, host: Option<&str>) -> String {
        let mut s = base.to_string();
        for sub in &self.strip {
            s = s.replace(sub, "");
        }
        for (from, to) in &self.replace {
            s = s.replace(from, to);
        }
        let s = s.trim().to_string();
        let host = host.unwrap_or("");
        let prefix = self
            .prefix
            .as_deref()
            .map(|t| t.replace("{host}", host).replace("{title}", &s));
        let suffix = self
            .suffix
            .as_deref()
            .map(|t| t.replace("{host}", host).replace("{title}", &s));
        let mut out = String::new();
        if let Some(p) = &prefix {
            out.push_str(p);
        }
        out.push_str(&s);
        if let Some(su) = &suffix {
            out.push_str(su);
        }
        if let Some(max) = self.max_len
            && out.chars().count() > max
        {
            let truncated: String = out.chars().take(max.saturating_sub(1)).collect();
            out = format!("{}…", truncated);
        }
        out.trim().to_string()
    }
}

impl NamingConfig {
    pub fn derive_title(&self, record: &Record) -> String {
        let base = record
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .or_else(|| {
                record
                    .url
                    .as_deref()
                    .and_then(host_of)
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Untitled".to_string());
        let host = record.url.as_deref().and_then(host_of);
        let mut result = base;
        for rule in &self.rules {
            result = rule.apply(&result, host);
        }
        if result.trim().is_empty() {
            "Untitled".to_string()
        } else {
            result
        }
    }
}

fn host_of(url: &str) -> Option<&str> {
    let after = url.split("://").nth(1).unwrap_or(url);
    let host = after.split('/').next().unwrap_or(after);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() { None } else { Some(host) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;

    fn rec(title: Option<&str>, url: Option<&str>) -> Record {
        let mut r = Record::new("rec_1", "page");
        r.title = title.map(|s| s.to_string());
        r.url = url.map(|s| s.to_string());
        r
    }

    #[test]
    fn strip_noise() {
        let rule = NamingRule {
            strip: vec![" - GitHub".to_string()],
            ..Default::default()
        };
        assert_eq!(rule.apply("foo - GitHub", Some("github.com")), "foo");
    }

    #[test]
    fn replace() {
        let rule = NamingRule {
            replace: vec![("docs.rs".to_string(), "Docs".to_string())],
            ..Default::default()
        };
        assert_eq!(rule.apply("docs.rs/sea-orm", None), "Docs/sea-orm");
    }

    #[test]
    fn affix_host() {
        let rule = NamingRule {
            prefix: Some("[{host}] ".to_string()),
            ..Default::default()
        };
        assert_eq!(
            rule.apply("SeaORM", Some("sea-orm.org")),
            "[sea-orm.org] SeaORM"
        );
    }

    #[test]
    fn truncate() {
        let rule = NamingRule {
            max_len: Some(8),
            ..Default::default()
        };
        assert_eq!(rule.apply("abcdefghij", None), "abcdefg…");
    }

    #[test]
    fn derive_from_url_host() {
        let cfg = NamingConfig::default();
        let r = rec(None, Some("https://sea-orm.org/docs"));
        assert_eq!(cfg.derive_title(&r), "sea-orm.org");
    }

    #[test]
    fn ordered_rules() {
        let cfg = NamingConfig {
            rules: vec![
                NamingRule {
                    strip: vec![" - GitHub".to_string()],
                    ..Default::default()
                },
                NamingRule {
                    prefix: Some("{host}: ".to_string()),
                    ..Default::default()
                },
            ],
        };
        let r = rec(Some("sea-orm - GitHub"), Some("https://github.com/sea-orm"));
        assert_eq!(cfg.derive_title(&r), "github.com: sea-orm");
    }
}
