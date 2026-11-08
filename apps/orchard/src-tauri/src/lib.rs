use lilac_core::session::{OpenReport, RecordSession};
use lilac_core::{read_record, rebuild_index, search_records, Record, Wal};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Open editor sessions, one per record id. A session owns the record text and
/// its WAL while the editor is open, so every edit is a surgical block replace
/// verified against the file version on disk.
#[derive(Default)]
struct Sessions(Mutex<HashMap<String, RecordSession>>);

fn wal_path_for(id: &str) -> PathBuf {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    lilac_host::config::config_root()
        .join("wal")
        .join(format!("{}.log", safe))
}

/// Return the live session for `id`, opening it (and replaying any WAL) when
/// there is none yet or when the record moved to a different file.
fn ensure_session<'a>(
    map: &'a mut HashMap<String, RecordSession>,
    id: &str,
    path: &Path,
) -> Result<(&'a mut RecordSession, Option<OpenReport>), String> {
    let needs_open = map.get(id).map(|s| s.path() != path).unwrap_or(true);
    if needs_open {
        let (session, report) =
            RecordSession::open(path, Wal::new(wal_path_for(id))).map_err(|e| e.to_string())?;
        map.insert(id.to_string(), session);
        return Ok((map.get_mut(id).expect("just inserted"), Some(report)));
    }
    Ok((map.get_mut(id).expect("present"), None))
}

fn session_view(
    session: &RecordSession,
    report: Option<&OpenReport>,
    external_changed: bool,
) -> serde_json::Value {
    serde_json::json!({
        "blocks": session.blocks(),
        "body": session.body(),
        "recovered_edits": report.map(|r| r.recovered_edits).unwrap_or(0),
        "orphaned_wal": report.and_then(|r| r.orphaned_wal.as_ref()).map(|p| p.to_string_lossy().to_string()),
        "external_changed": external_changed,
    })
}

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
    // Single source of truth: the same loader the native messaging host uses.
    let (_cfg, root) = lilac_host::config::load_config();
    root
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
fn get_block_types() -> serde_json::Value {
    lilac_core::block_types::describe()
}

#[tauri::command]
fn open_folder(which: String) -> Result<String, String> {
    let root = records_root();
    let path = match which.as_str() {
        "root" => root,
        "records" => root.join("Records"),
        "config" => lilac_host::config::config_root(),
        _ => return Err(format!("unknown folder: {}", which)),
    };
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(&path).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn records_root_path() -> String {
    records_root().to_string_lossy().to_string()
}

#[tauri::command]
fn get_settings() -> serde_json::Value {
    let (cfg, root) = lilac_host::config::load_config();
    let orchid = lilac_host::config::load_orchid_config();
    let mut folders = cfg.registered_folders.clone();
    if folders.is_empty() {
        folders = orchid.registered_folders.clone();
    }
    serde_json::json!({
        "records_root": root.to_string_lossy(),
        "config_root": lilac_host::config::config_root().to_string_lossy(),
        "registered_folders": folders,
        "extension_id": orchid.extension_id.clone().unwrap_or_default(),
    })
}

#[tauri::command]
fn set_registered_folders_cmd(folders: Vec<String>) -> Result<Vec<String>, String> {
    let mut orchid = lilac_host::config::load_orchid_config();
    orchid.registered_folders = folders.clone();
    lilac_host::config::save_orchid_config(&orchid)?;
    let mut cfg = lilac_host::config::load_config().0;
    cfg.registered_folders = folders.clone();
    lilac_host::config::save_config(&cfg)?;
    Ok(folders)
}

#[tauri::command]
fn set_records_root_cmd(path: String) -> Result<String, String> {
    let p = lilac_host::config::set_records_root(&path)?;
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
fn register_native_host(extension_id: String, host_binary_path: Option<String>) -> Result<String, String> {
    // Orchard is the single executable: the browser runs orchard.exe itself and
    // host mode is auto-detected (--parent-window / stdio pipes).
    let host_bin = match host_binary_path {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    if !host_bin.exists() {
        return Err(format!("host binary not found: {}", host_bin.display()));
    }
    let manifest_dir = dirs_local_app_data().join("Lilac");
    std::fs::create_dir_all(&manifest_dir).map_err(|e| e.to_string())?;
    let manifest_path = manifest_dir.join("lilac_host.json");

    let allowed = if extension_id.is_empty() {
        "[]".to_string()
    } else {
        format!("[\"chrome-extension://{}/\"]", extension_id)
    };
    let manifest = format!(
        r#"{{"name":"lilac_host","description":"Lilac record storage native host","path":"{}","type":"stdio","allowed_origins":{},"allowed_extensions":["lilac@orchid"]}}"#,
        host_bin.to_string_lossy().replace('\\', "\\\\"),
        allowed
    );
    std::fs::write(&manifest_path, manifest).map_err(|e| e.to_string())?;

    // Persist the extension id so the settings UI can prefill it.
    let mut orchid = lilac_host::config::load_orchid_config();
    orchid.extension_id = if extension_id.is_empty() { None } else { Some(extension_id) };
    lilac_host::config::save_orchid_config(&orchid)?;

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
fn get_blocks(state: tauri::State<Sessions>, id: String) -> Result<String, String> {
    let root = records_root();
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, report) = ensure_session(&mut map, &id, &path)?;
    // Pick up edits made by another program (or the browser host) while open.
    let external_changed = session.refresh_external().map_err(|e| e.to_string())?;
    Ok(session_view(session, report.as_ref(), external_changed).to_string())
}

#[tauri::command]
fn apply_block_edit_cmd(
    state: tauri::State<Sessions>,
    id: String,
    block_path: Vec<usize>,
    new_content: String,
    expected_content: Option<String>,
) -> Result<String, String> {
    let root = records_root();
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_session(&mut map, &id, &path)?;
    let mut external_changed = session.refresh_external().map_err(|e| e.to_string())?;
    // The block path is positional: if the file changed on disk, the same path
    // may now point at a different block. Refuse rather than overwrite it.
    if let Some(expected) = expected_content {
        let current = lilac_core::block_at_path(session.blocks(), &block_path)
            .map(|b| b.content.as_str());
        if current != Some(expected.as_str()) {
            return Err("this block changed on disk; reopen the record before editing".into());
        }
    }
    external_changed |= session
        .edit_at_path(&block_path, &new_content)
        .map_err(|e| e.to_string())?;
    Ok(session_view(session, None, external_changed).to_string())
}

#[tauri::command]
fn undo_block_edit_cmd(state: tauri::State<Sessions>, id: String) -> Result<String, String> {
    let root = records_root();
    let path = find_record_file(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_session(&mut map, &id, &path)?;
    session.undo().map_err(|e| e.to_string())?;
    Ok(session_view(session, None, false).to_string())
}

pub fn run() {
    tauri::Builder::default()
        .manage(Sessions::default())
        .invoke_handler(tauri::generate_handler![
            list_records,
            search,
            load_record,
            get_blocks,
            apply_block_edit_cmd,
            undo_block_edit_cmd,
            records_root_path,
            register_native_host,
            get_block_types,
            get_settings,
            open_folder,
            set_registered_folders_cmd,
            set_records_root_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running Orchard");
}