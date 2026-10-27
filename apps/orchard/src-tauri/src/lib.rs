use lilac_core::{read_record, rebuild_index, search_records, Record};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct RecordView {
    id: String,
    r#type: String,
    title: Option<String>,
    tags: Vec<String>,
    status: Option<String>,
    url: Option<String>,
}

impl From<Record> for RecordView {
    fn from(r: Record) -> Self {
        Self {
            id: r.id,
            r#type: r.r#type,
            title: r.title,
            tags: r.tags,
            status: r.status,
            url: r.url,
        }
    }
}

fn records_root() -> PathBuf {
    // Reuse Lilac's config resolution by reading ~/.lilac/config.yaml.
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let cfg_path = home.join(".lilac").join("config.yaml");
    if let Ok(s) = std::fs::read_to_string(&cfg_path) {
        // crude parse: find records_root: "..."
        for line in s.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("records_root:") {
                let v = rest.trim().trim_matches('"');
                if !v.is_empty() {
                    return PathBuf::from(v);
                }
            }
        }
    }
    home.join("Documents").join("Lilac")
}

#[tauri::command]
fn list_records() -> Result<Vec<RecordView>, String> {
    let idx = rebuild_index(&records_root()).map_err(|e| e.to_string())?;
    Ok(idx.records.into_iter().map(|e| RecordView {
        id: e.id,
        r#type: e.r#type,
        title: e.title,
        tags: e.tags,
        status: e.status,
        url: None,
    }).collect())
}

#[tauri::command]
fn search(query: String) -> Result<Vec<RecordView>, String> {
    let idx = rebuild_index(&records_root()).map_err(|e| e.to_string())?;
    let res = search_records(&idx, &query);
    Ok(res.into_iter().map(|e| RecordView {
        id: e.id,
        r#type: e.r#type,
        title: e.title,
        tags: e.tags,
        status: e.status,
        url: None,
    }).collect())
}

#[tauri::command]
fn load_record(id: String) -> Result<String, String> {
    let root = records_root();
    // find file
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (rec, body) = read_record(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&serde_json::json!({"record": rec, "body": body})).unwrap_or_default())
}

fn find_record_file(dir: &std::path::Path, id: &str) -> Option<PathBuf> {
    if !dir.is_dir() { return None; }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(p) = find_record_file(&path, id) { return Some(p); }
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok((rec, _)) = read_record(&path) {
                    if rec.id == id { return Some(path); }
                }
            }
        }
    }
    None
}

#[tauri::command]
fn records_root_path() -> String {
    records_root().to_string_lossy().to_string()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_records,
            search,
            load_record,
            records_root_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running Orchard");
}