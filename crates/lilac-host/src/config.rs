use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub records_root: Option<String>,
    #[serde(default)]
    pub registered_folders: Vec<String>,
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

pub fn load_orchid_config() -> OrchidConfig {
    let root = orchid_config_root();
    let _ = fs::create_dir_all(&root);
    let path = root.join("config.yaml");
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_yaml::from_str(&s).ok())
            .unwrap_or_default()
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
    let mut cfg: Config = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_yaml::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Config::default()
    };
    // pull registered folders from orchid config if lilac's is empty
    if cfg.registered_folders.is_empty() {
        cfg.registered_folders = load_orchid_config().registered_folders;
    }
    // ensure records root
    let records_root = cfg
        .records_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(default_records_root);
    let _ = fs::create_dir_all(&records_root);
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
