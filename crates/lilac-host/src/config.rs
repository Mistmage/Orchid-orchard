use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub records_root: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            records_root: None,
        }
    }
}

/// Resolve the lilac config root directory: ~/.lilac (or ~/.orchard/.lilac if present).
pub fn config_root() -> PathBuf {
    // Allow override via env for testing.
    if let Ok(p) = std::env::var("LILAC_CONFIG_ROOT") {
        return PathBuf::from(p);
    }
    let home = home_dir();
    let orchard = home.join(".orchard").join(".lilac");
    if orchard.exists() {
        return orchard;
    }
    home.join(".lilac")
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

pub fn load_config() -> (Config, PathBuf) {
    let root = config_root();
    let _ = fs::create_dir_all(&root);
    let path = root.join("config.yaml");
    let cfg: Config = if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_yaml::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Config::default()
    };
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
