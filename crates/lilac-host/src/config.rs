use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use lilac_core::NamingConfig;

/// Standard record-type folders. These are the defaults Lilac saves into when
/// no folder is chosen, and they are created under `<records_root>/Records`.
pub const DEFAULT_REGISTERED_FOLDERS: &[&str] = &[
    "page",
    "collection",
    "bookmark",
    "snapshot",
    "note",
    "snippet",
];

pub fn default_registered_folders() -> Vec<String> {
    DEFAULT_REGISTERED_FOLDERS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub records_root: Option<String>,
    #[serde(default)]
    pub registered_folders: Vec<String>,
    #[serde(default)]
    pub naming: NamingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrchidConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub registered_folders: Vec<String>,
    #[serde(default)]
    pub extension_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            records_root: None,
            registered_folders: Vec::new(),
            naming: NamingConfig::default(),
        }
    }
}

/// Resolve the lilac config root directory: ~/.lilac (or ~/.orchard/.lilac if present).
pub fn config_root() -> PathBuf {
    // Allow override via env for testing.
    if let Ok(p) = std::env::var("LILAC_CONFIG_ROOT") {
        return PathBuf::from(p);
    }
    home_dir().join(".orchard").join(".lilac")
}

pub fn orchid_config_root() -> PathBuf {
    if let Ok(p) = std::env::var("ORCHID_CONFIG_ROOT") {
        return PathBuf::from(p);
    }
    home_dir().join(".orchard").join(".orchid")
}

pub fn home_dir() -> PathBuf {
    if let Ok(p) = std::env::var("USERPROFILE") {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("HOME") {
        return PathBuf::from(p);
    }
    PathBuf::from(".")
}

pub fn default_records_root() -> PathBuf {
    home_dir().join("Documents").join("Lilac")
}

/// Strip a UTF-8 BOM and normalise line endings before handing text to YAML.
/// PowerShell's `Set-Content -Encoding UTF8` writes a BOM, which serde_yaml
/// does not accept at the start of a document.
fn clean_yaml(raw: &str) -> String {
    raw.trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

/// Read the last scalar value for `key`. Used only to salvage a config that is
/// not valid YAML (for example a file concatenated by an old installer).
fn yaml_scalar_after(text: &str, key: &str) -> Option<String> {
    let mut search = 0;
    let mut last = None;
    while let Some(pos) = text[search..].find(key) {
        let key_start = search + pos;
        let before = text[..key_start].chars().next_back();
        let is_word_start = before
            .map(|c| !c.is_alphanumeric() && c != '_')
            .unwrap_or(true);
        let after_key = key_start + key.len();
        if is_word_start {
            let rest = text[after_key..].trim_start();
            if let Some(rest) = rest.strip_prefix(':') {
                let rest = rest.trim_start();
                let value = if let Some(r) = rest.strip_prefix('"') {
                    r.find('"').map(|end| r[..end].to_string())
                } else if let Some(r) = rest.strip_prefix('\'') {
                    r.find('\'').map(|end| r[..end].replace("''", "'"))
                } else {
                    let end = rest
                        .find(|c: char| c == '\n' || c == '\r' || c.is_whitespace())
                        .unwrap_or(rest.len());
                    Some(rest[..end].to_string())
                };
                if let Some(value) = value.filter(|v| !v.is_empty()) {
                    last = Some(value);
                }
            }
        }
        search = after_key;
    }
    last
}

fn parse_config(raw: &str) -> (Config, bool) {
    let text = clean_yaml(raw);
    match serde_yaml::from_str::<Config>(&text) {
        Ok(cfg) => (cfg, true),
        Err(_) => {
            let cfg = Config {
                records_root: yaml_scalar_after(&text, "records_root"),
                ..Config::default()
            };
            (cfg, false)
        }
    }
}

/// Create the standard `<records_root>/Records/<folder>` directories. Entries
/// that are absolute or contain `..` are ignored so a config value can never
/// escape the records root.
fn ensure_record_folders(records_root: &Path, folders: &[String]) {
    let records = records_root.join("Records");
    let _ = fs::create_dir_all(&records);
    for folder in folders {
        let p = Path::new(folder);
        if p.is_absolute()
            || p.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
        {
            continue;
        }
        let _ = fs::create_dir_all(records.join(p));
    }
}

pub fn load_orchid_config() -> OrchidConfig {
    let root = orchid_config_root();
    let _ = fs::create_dir_all(&root);
    let path = root.join("config.yaml");
    if path.exists() {
        let raw = fs::read_to_string(&path).unwrap_or_default();
        let text = clean_yaml(&raw);
        serde_yaml::from_str(&text).unwrap_or_else(|_| OrchidConfig {
            version: 1,
            registered_folders: Vec::new(),
            extension_id: yaml_scalar_after(&text, "extension_id"),
        })
    } else {
        OrchidConfig::default()
    }
}

pub fn save_orchid_config(cfg: &OrchidConfig) -> Result<(), String> {
    let root = orchid_config_root();
    let _ = fs::create_dir_all(&root);
    let path = root.join("config.yaml");
    let yaml = serde_yaml::to_string(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, yaml).map_err(|e| e.to_string())?;
    Ok(())
}

/// One-time migration from the pre-0.2 location `~/.lilac/`.
/// Runs only when the new root has no config yet, and never when the root was
/// overridden via `LILAC_CONFIG_ROOT` (tests must stay isolated).
fn migrate_legacy(new_root: &Path) {
    if std::env::var("LILAC_CONFIG_ROOT").is_ok() {
        return;
    }
    let new_cfg = new_root.join("config.yaml");
    if new_cfg.exists() {
        return;
    }
    let old_root = home_dir().join(".lilac");
    let old_cfg = old_root.join("config.yaml");
    if !old_cfg.exists() {
        return;
    }
    let _ = fs::copy(&old_cfg, &new_cfg);
    let old_rules = old_root.join("rules.txt");
    let new_rules = new_root.join("rules.txt");
    if old_rules.exists() && !new_rules.exists() {
        let _ = fs::copy(old_rules, new_rules);
    }
}

pub fn load_config() -> (Config, PathBuf) {
    let root = config_root();
    let _ = fs::create_dir_all(&root);
    migrate_legacy(&root);
    let path = root.join("config.yaml");
    let raw = if path.exists() {
        fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    let normalised = clean_yaml(&raw);
    let had_folders_key = normalised.contains("registered_folders");
    let (mut cfg, parsed) = parse_config(&raw);
    if cfg.version == 0 {
        cfg.version = 1;
    }
    // Standard folders are defaults, not a hidden fallback: Lilac and Orchid
    // both need concrete destinations. An explicit empty list is respected.
    if cfg.registered_folders.is_empty() && (!had_folders_key || !parsed) {
        cfg.registered_folders = load_orchid_config().registered_folders;
        if cfg.registered_folders.is_empty() {
            cfg.registered_folders = default_registered_folders();
        }
    }
    // Naming has a useful starter set; an entirely empty config gets it. A
    // config with rules or domain overrides is left untouched.
    if cfg.naming.rules.is_empty() && cfg.naming.domains.is_empty() {
        cfg.naming = NamingConfig::starter();
    }
    if !parsed && !normalised.is_empty() {
        // Repair the file once so the next start is clean. Do this after the
        // defaults are resolved, otherwise the repaired file would pin an
        // empty folder list.
        let _ = save_config(&cfg);
    }
    // ensure records root
    let records_root = cfg
        .records_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(default_records_root);
    let _ = fs::create_dir_all(&records_root);
    ensure_record_folders(&records_root, &cfg.registered_folders);
    (cfg, records_root)
}

pub fn save_config(cfg: &Config) -> Result<(), String> {
    let root = config_root();
    let _ = fs::create_dir_all(&root);
    let path = root.join("config.yaml");
    let yaml = serde_yaml::to_string(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, yaml).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_records_root(new: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(new);
    // validate
    if !p.exists() {
        fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    }
    if !p.is_dir() {
        return Err("records root is not a directory".into());
    }
    let mut cfg = load_config().0;
    cfg.records_root = Some(new.to_string());
    save_config(&cfg)?;
    Ok(p)
}

pub fn logs_dir() -> PathBuf {
    config_root().join("logs")
}

pub fn rules_path() -> PathBuf {
    config_root().join("rules.txt")
}

pub fn _ensure_parent(_p: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_yaml_strips_bom_and_normalises_newlines() {
        let raw = "\u{feff}version: 1\r\nrecords_root: C:/tmp\r\n";
        assert_eq!(clean_yaml(raw), "version: 1\nrecords_root: C:/tmp\n");
    }

    #[test]
    fn parse_accepts_bom() {
        let raw = "\u{feff}version: 1\nrecords_root: C:/tmp/lilac\n";
        let (cfg, parsed) = parse_config(raw);
        assert!(parsed);
        assert_eq!(cfg.records_root.as_deref(), Some("C:/tmp/lilac"));
    }

    #[test]
    fn salvage_concatenated_config_takes_last_value() {
        // The shape produced by the old installer bug: no newlines at all.
        let raw = "version: 1records_root: \"C:/old\"records_root: \"C:/new/lilac\"";
        let (cfg, parsed) = parse_config(raw);
        assert!(!parsed);
        assert_eq!(cfg.records_root.as_deref(), Some("C:/new/lilac"));
    }

    #[test]
    fn default_folders_are_the_record_types() {
        let folders = default_registered_folders();
        assert!(folders.iter().any(|f| f == "page"));
        assert!(folders.iter().any(|f| f == "note"));
    }
}
