//! Human-readable record naming rules.
//!
//! Derives a display title (and slug source) from a record using ordered
//! rules: strip noise, replace, affix, truncate. Pure logic, no I/O.

use crate::record::Record;
use crate::storage::host_of;

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
pub struct DomainNaming {
    /// Host pattern. `github.com` matches exactly; `*.github.com` also matches
    /// subdomains. A leading `www.` is ignored on both sides.
    pub domain: String,
    #[serde(default)]
    pub rule: NamingRule,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NamingConfig {
    /// Global rules, applied in order when no domain override matches.
    #[serde(default)]
    pub rules: Vec<NamingRule>,
    /// Per-domain overrides. The first matching pattern wins.
    #[serde(default)]
    pub domains: Vec<DomainNaming>,
}

impl NamingConfig {
    /// Sensible starting point for a fresh config: strip the usual site noise
    /// and keep titles short. Users can edit or delete these in Settings.
    pub fn starter() -> Self {
        Self {
            rules: vec![NamingRule {
                strip: vec![
                    " - GitHub".to_string(),
                    " | Hacker News".to_string(),
                    " - YouTube".to_string(),
                    " - Stack Overflow".to_string(),
                ],
                max_len: Some(120),
                ..NamingRule::default()
            }],
            domains: Vec::new(),
        }
    }

    /// First per-domain rule matching `host`, if any.
    pub fn rule_for_host(&self, host: &str) -> Option<&NamingRule> {
        let host = normalize_host(host);
        self.domains
            .iter()
            .find(|d| host_matches(&host, &d.domain))
            .map(|d| &d.rule)
    }
}

/// Canonical comparison form of a host: trimmed, lower-case, no trailing dot,
/// and with a leading `www.` removed. Both record URLs and user-written
/// patterns go through this, so `www.Example.com` and `example.com` agree.
pub fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    host.strip_prefix("www.").unwrap_or(&host).to_string()
}

/// Does `host` match a domain pattern?
///
/// `github.com` matches exactly. `*.github.com` matches `github.com` itself and
/// every subdomain, which is what people mean by "the GitHub family". A leading
/// `www.` is ignored on both sides.
pub fn host_matches(host: &str, pattern: &str) -> bool {
    let host = normalize_host(host);
    let pattern = normalize_host(pattern);
    if pattern.is_empty() {
        return false;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host == suffix || host.ends_with(&format!(".{}", suffix))
    } else {
        host == pattern
    }
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
        if let Some(host) = host
            && let Some(rule) = self.rule_for_host(host)
        {
            return non_empty(rule.apply(&base, Some(host)));
        }
        let mut result = base;
        for rule in &self.rules {
            result = rule.apply(&result, host);
        }
        non_empty(result)
    }
}

fn non_empty(value: String) -> String {
    if value.trim().is_empty() {
        "Untitled".to_string()
    } else {
        value
    }
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
            domains: vec![],
        };
        let r = rec(Some("sea-orm - GitHub"), Some("https://github.com/sea-orm"));
        assert_eq!(cfg.derive_title(&r), "github.com: sea-orm");
    }

    #[test]
    fn domain_override_wins_over_global() {
        let cfg = NamingConfig {
            rules: vec![NamingRule {
                prefix: Some("global: ".to_string()),
                ..Default::default()
            }],
            domains: vec![DomainNaming {
                domain: "github.com".to_string(),
                rule: NamingRule {
                    prefix: Some("gh: ".to_string()),
                    ..Default::default()
                },
            }],
        };
        let r = rec(Some("sea-orm"), Some("https://github.com/sea-orm"));
        assert_eq!(cfg.derive_title(&r), "gh: sea-orm");
    }

    #[test]
    fn domain_match_ignores_www_and_case() {
        let cfg = NamingConfig {
            rules: vec![],
            domains: vec![DomainNaming {
                domain: "github.com".to_string(),
                rule: NamingRule {
                    suffix: Some(" [gh]".to_string()),
                    ..Default::default()
                },
            }],
        };
        let r = rec(Some("Repo"), Some("https://WWW.GitHub.com/org/repo"));
        assert_eq!(cfg.derive_title(&r), "Repo [gh]");
    }

    #[test]
    fn wildcard_domain_matches_subdomains_only() {
        let cfg = NamingConfig {
            rules: vec![],
            domains: vec![DomainNaming {
                domain: "*.example.com".to_string(),
                rule: NamingRule {
                    prefix: Some("ex: ".to_string()),
                    ..Default::default()
                },
            }],
        };
        let sub = rec(Some("Page"), Some("https://docs.example.com/page"));
        let root = rec(Some("Page"), Some("https://example.com/page"));
        let other = rec(Some("Page"), Some("https://notexample.com/page"));
        assert_eq!(cfg.derive_title(&sub), "ex: Page");
        assert_eq!(cfg.derive_title(&root), "ex: Page");
        assert_eq!(cfg.derive_title(&other), "Page");
    }

    #[test]
    fn unmatched_domain_falls_back_to_global_rules() {
        let cfg = NamingConfig {
            rules: vec![NamingRule {
                strip: vec![" - Example".to_string()],
                ..Default::default()
            }],
            domains: vec![DomainNaming {
                domain: "github.com".to_string(),
                rule: NamingRule::default(),
            }],
        };
        let r = rec(Some("Page - Example"), Some("https://other.test/page"));
        assert_eq!(cfg.derive_title(&r), "Page");
    }

    #[test]
    fn starter_strips_common_suffixes() {
        let cfg = NamingConfig::starter();
        let r = rec(Some("Repo - GitHub"), Some("https://github.com/org/repo"));
        assert_eq!(cfg.derive_title(&r), "Repo");
    }
}
