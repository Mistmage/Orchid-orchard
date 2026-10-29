use lilac_core::{apply_block_edit, parse_blocks, read_record, rebuild_index, search_records, write_record, Record};
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

#[tauri::command]
fn register_native_host(extension_id: String, host_binary_path: Option<String>) -> Result<String, String> {
    // Resolve host binary path.
    let host_bin = if let Some(p) = host_binary_path {
        std::path::PathBuf::from(p)
    } else {
        // default: sibling of orchard.exe
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("lilac-host.exe")))
            .unwrap_or_else(|| std::path::PathBuf::from("lilac-host.exe"))
    };
    if !host_bin.exists() {
        return Err(format!("host binary not found: {}", host_bin.display()));
    }
    let manifest_dir = dirs_local_app_data().join("Lilac");
    std::fs::create_dir_all(&manifest_dir).map_err(|e| e.to_string())?;
    let manifest_path = manifest_dir.join("lilac_host.json");
    // wrapper so the browser invokes orchard.exe in host mode
    let wrapper_path = manifest_dir.join("lilac-host.bat");
    let wrapper = format!("@echo off\r\n\"{}\" --host", host_bin.to_string_lossy());
    std::fs::write(&wrapper_path, wrapper).map_err(|e| e.to_string())?;

    let allowed = if extension_id.is_empty() {
        "[]".to_string()
    } else {
        format!("[\"chrome-extension://{}/\"]", extension_id)
    };
    let manifest = format!(
        r#"{{"name":"lilac_host","description":"Lilac record storage native host","path":"{}","type":"stdio","allowed_origins":{},"allowed_extensions":["lilac@orchid"]}}"#,
        wrapper_path.to_string_lossy().replace('\\', "\\\\"),
        allowed
    );
    std::fs::write(&manifest_path, manifest).map_err(|e| e.to_string())?;

    // Windows registry
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        for (root, sub) in [
            ("HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts", "lilac_host"),
            ("HKCU\\Software\\Mozilla\\NativeMessagingHosts", "lilac_host"),
        ] {
            let key = format!("{}\\{}", root, sub);
            let _ = Command::new("reg")
                .args(["add", &key, "/ve", "/d", manifest_path.to_str().unwrap_or(""), "/f"])
                .output();
        }
    }
    // Unix: write manifest files
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").map(std::path::PathBuf::from).unwrap_or_default();
        let chrome_dir = home.join(".config/google-chrome/NativeMessagingHosts");
        let _ = std::fs::create_dir_all(&chrome_dir);
        let _ = std::fs::write(chrome_dir.join("lilac_host.json"), std::fs::read_to_string(&manifest_path).unwrap_or_default());
        let ff_dir = home.join(".mozilla/native-messaging-hosts");
        let _ = std::fs::create_dir_all(&ff_dir);
        let _ = std::fs::write(ff_dir.join("lilac_host.json"), std::fs::read_to_string(&manifest_path).unwrap_or_default());
    }
    Ok(format!("Registered host. Manifest: {}", manifest_path.display()))
}

fn dirs_local_app_data() -> std::path::PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(std::path::PathBuf::from))
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

#[tauri::command]
fn get_blocks(id: String) -> Result<String, String> {
    let root = records_root();
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (_rec, body) = read_record(&path).map_err(|e| e.to_string())?;
    let body = body.unwrap_or_default();
    let blocks = parse_blocks(&body);
    Ok(serde_json::to_string(&blocks).unwrap_or_default())
}

#[tauri::command]
fn apply_block_edit_cmd(id: String, block_id: String, new_content: String) -> Result<String, String> {
    let root = records_root();
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (mut rec, body) = read_record(&path).map_err(|e| e.to_string())?;
    let body = body.unwrap_or_default();
    let new_body = apply_block_edit(&body, &block_id, &new_content).map_err(|e| e)?;
    rec.modified = Some(chrono::Utc::now().to_rfc3339());
    write_record(&path, &rec, Some(&new_body)).map_err(|e| e.to_string())?;
    Ok(new_body)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_records,
            search,
            load_record,
            get_blocks,
            apply_block_edit_cmd,
            records_root_path,
            register_native_host
        ])
        .run(tauri::generate_context!())
        .expect("error while running Orchard");
}