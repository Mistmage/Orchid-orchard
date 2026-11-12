use lilac_core::session::{OpenReport, RecordSession};
use lilac_core::{
    Record, Wal, BASE_TYPE, apply_rules_reported as apply_rules_core, base_starter_document,
    create_from_template_reported, find_record_by_id, generate_record_id, list_templates,
    move_record_to_folder, normalize_folder, parse_base, parse_rules, read_record, record_path,
    rebuild_index, render_base, save_as_template, search_records, write_record,
};
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

/// Look the record up *after* the session lock is held. A move command holds
/// that same lock while it renames files, so a command that was waiting cannot
/// keep using the path it saw before the move.
fn ensure_open<'a>(
    map: &'a mut HashMap<String, RecordSession>,
    id: &str,
) -> Result<(&'a mut RecordSession, Option<OpenReport>), String> {
    let path = find_record_by_id(&records_root(), id).ok_or_else(|| format!("not found: {}", id))?;
    ensure_session(map, id, &path)
}

fn session_view(
    session: &RecordSession,
    report: Option<&OpenReport>,
    external_changed: bool,
) -> serde_json::Value {
    serde_json::json!({
        "blocks": session.blocks(),
        "body": session.body(),
        "record": session.record(),
        // A 64-bit fingerprint exceeds JavaScript's safe integer range, so it
        // travels as a decimal string and is parsed again in Rust.
        "fingerprint": session.base_fingerprint().to_string(),
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
    folder: Option<String>,
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
            folder: r.folder,
        }
    }
}

#[derive(serde::Deserialize)]
struct RecordFields {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

fn clean_field(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if !tag.is_empty() && !out.iter().any(|t| t == tag) {
            out.push(tag.to_string());
        }
    }
    out
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
        folder: e.folder,
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
        folder: e.folder,
    }).collect())
}

#[tauri::command]
fn load_record(id: String) -> Result<String, String> {
    let root = records_root();
    // find file
    let path = find_record_by_id(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (rec, body) = read_record(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&serde_json::json!({"record": rec, "body": body})).unwrap_or_default())
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
        "naming": cfg.naming,
        "auto_apply_rules": cfg.rules.auto_apply,
        "rules_path": lilac_host::config::rules_path().to_string_lossy(),
    })
}

#[tauri::command]
fn set_naming_cmd(naming: lilac_core::NamingConfig) -> Result<serde_json::Value, String> {
    let mut cfg = lilac_host::config::load_config().0;
    cfg.naming = naming;
    lilac_host::config::save_config(&cfg)?;
    serde_json::to_value(&cfg.naming).map_err(|e| e.to_string())
}

/// Preview a naming config against a title/URL without saving a record.
#[tauri::command]
fn preview_naming_cmd(naming: lilac_core::NamingConfig, title: String, url: String) -> String {
    let mut record = Record::new("preview", "page");
    record.title = if title.trim().is_empty() {
        None
    } else {
        Some(title)
    };
    record.url = if url.trim().is_empty() {
        None
    } else {
        Some(url)
    };
    naming.derive_title(&record)
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
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, report) = ensure_open(&mut map, &id)?;
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
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
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
fn insert_block_cmd(
    state: tauri::State<Sessions>,
    id: String,
    after_path: Option<Vec<usize>>,
    content: String,
) -> Result<String, String> {
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
    let external_changed = session
        .insert_block(after_path.as_deref(), &content)
        .map_err(|e| e.to_string())?;
    Ok(session_view(session, None, external_changed).to_string())
}

#[tauri::command]
fn delete_block_cmd(
    state: tauri::State<Sessions>,
    id: String,
    block_path: Vec<usize>,
    expected_content: Option<String>,
) -> Result<String, String> {
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
    let external_changed = session.refresh_external().map_err(|e| e.to_string())?;
    // Same positional-path guard as edits: never delete a block that changed.
    if let Some(expected) = expected_content {
        let current =
            lilac_core::block_at_path(session.blocks(), &block_path).map(|b| b.content.as_str());
        if current != Some(expected.as_str()) {
            return Err("this block changed on disk; reopen the record before deleting".into());
        }
    }
    let external_changed = external_changed
        | session
            .delete_block(&block_path)
            .map_err(|e| e.to_string())?;
    Ok(session_view(session, None, external_changed).to_string())
}

#[tauri::command]
fn update_record_fields_cmd(
    state: tauri::State<Sessions>,
    id: String,
    fields: RecordFields,
    expected_fingerprint: Option<String>,
) -> Result<String, String> {
    let expected = match expected_fingerprint {
        Some(value) if !value.is_empty() => Some(
            value
                .parse::<u64>()
                .map_err(|_| format!("invalid fingerprint: {}", value))?,
        ),
        _ => None,
    };
    let mut map = state
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
    let external_changed = session
        .update_record(expected, |record| {
            record.title = clean_field(fields.title);
            record.tags = clean_tags(fields.tags);
            record.status = clean_field(fields.status);
            record.url = clean_field(fields.url);
            record.description = clean_field(fields.description);
        })
        .map_err(|e| e.to_string())?;
    Ok(session_view(session, None, external_changed).to_string())
}

#[tauri::command]
fn list_templates_cmd() -> Result<String, String> {
    let templates = list_templates(&records_root()).map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&templates).unwrap_or_else(|_| "[]".to_string()))
}

#[tauri::command]
fn create_from_template_cmd(
    template_id: String,
    title: Option<String>,
    url: Option<String>,
) -> Result<String, String> {
    let root = records_root();
    let (record, path, warnings) =
        create_from_template_reported(&root, &template_id, title.as_deref(), url.as_deref())
            .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "record": record,
        "path": path.to_string_lossy(),
        "warnings": warnings,
    })
    .to_string())
}

#[tauri::command]
fn save_as_template_cmd(source_id: String, title: Option<String>) -> Result<String, String> {
    let root = records_root();
    let (record, path) = save_as_template(&root, &source_id, title.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "record": record,
        "path": path.to_string_lossy(),
    })
    .to_string())
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// Read `rules.txt`, its parse state and the current scheduling switch. The
/// text is the source of truth; a parse error is reported instead of hidden.
#[tauri::command]
fn get_rules_cmd() -> serde_json::Value {
    let path = lilac_host::config::rules_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (count, error) = match parse_rules(&text) {
        Ok(set) => (set.rules.len(), serde_json::Value::Null),
        Err(e) => (0, serde_json::json!(e)),
    };
    let auto_apply = lilac_host::config::load_config().0.rules.auto_apply;
    serde_json::json!({
        "path": path.to_string_lossy(),
        "text": text,
        "rules": count,
        "error": error,
        "auto_apply": auto_apply,
    })
}

/// Replace `rules.txt` after validating it. An invalid file is never written,
/// so a typo in the editor cannot disable rules on the next browser save.
#[tauri::command]
fn set_rules_cmd(text: String) -> Result<serde_json::Value, String> {
    let set = parse_rules(&text)?;
    let path = lilac_host::config::rules_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "path": path.to_string_lossy(),
        "rules": set.rules.len(),
    }))
}

#[tauri::command]
fn set_auto_apply_rules_cmd(enabled: bool) -> Result<bool, String> {
    let mut cfg = lilac_host::config::load_config().0;
    cfg.rules.auto_apply = enabled;
    lilac_host::config::save_config(&cfg)?;
    Ok(enabled)
}

/// Apply the current rules to one stored record, writing metadata and moving
/// the file when a `move` action fires. The open session (if any) is dropped so
/// the next view reads the new file instead of a stale path.
#[tauri::command]
fn apply_rules_cmd(state: tauri::State<Sessions>, id: String) -> Result<String, String> {
    let root = records_root();
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    let path = find_record_by_id(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (mut record, body) = read_record(&path).map_err(|e| e.to_string())?;
    let folder_before = record.folder.clone();
    let text = std::fs::read_to_string(lilac_host::config::rules_path()).unwrap_or_default();
    let ruleset = parse_rules(&text)?;
    let (fired, rule_errors) = apply_rules_core(&mut record, &ruleset.rules);
    let fired: Vec<String> = fired.iter().map(|a| format!("{:?}", a)).collect();
    if fired.is_empty() {
        return Ok(serde_json::json!({
            "record": record,
            "path": path.to_string_lossy(),
            "fired": Vec::<String>::new(),
            "errors": rule_errors,
            "moved": false,
        })
        .to_string());
    }
    if let Some(folder) = record.folder.clone() {
        match normalize_folder(&folder) {
            Ok(None) => record.folder = None,
            Ok(Some(f)) if f == record.r#type => record.folder = None,
            Ok(f) => record.folder = f,
            Err(e) => return Err(e.to_string()),
        }
    }
    record.modified = Some(chrono::Utc::now().to_rfc3339());
    // Only placement changes should rename or relocate a file. Recomputing the
    // slug after a metadata-only rule would silently rename an imported or
    // hand-named record.
    let destination = if record.folder == folder_before {
        path.clone()
    } else {
        record_path(&root, &record)
    };
    if destination != path {
        write_record(&destination, &record, body.as_deref()).map_err(|e| e.to_string())?;
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    } else {
        write_record(&path, &record, body.as_deref()).map_err(|e| e.to_string())?;
    }
    sessions.remove(&id);
    Ok(serde_json::json!({
        "record": record,
        "path": destination.to_string_lossy(),
        "fired": fired,
        "errors": rule_errors,
        "moved": destination != path,
    })
    .to_string())
}

#[tauri::command]
fn render_base_cmd(id: String) -> Result<String, String> {
    let root = records_root();
    let path = find_record_by_id(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (record, body) = read_record(&path).map_err(|e| e.to_string())?;
    if !record.r#type.eq_ignore_ascii_case(BASE_TYPE) {
        return Err(format!("{} is not a base (type {})", id, record.r#type));
    }
    let source = body.unwrap_or_default();
    let spec = parse_base(&source)?;
    let view = render_base(&root, &spec).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "source": source, "view": view }).to_string())
}

#[tauri::command]
fn save_base_source_cmd(
    id: String,
    text: String,
    expected_source: Option<String>,
) -> Result<String, String> {
    let root = records_root();
    let path = find_record_by_id(&root, &id).ok_or_else(|| format!("not found: {}", id))?;
    let (mut record, body) = read_record(&path).map_err(|e| e.to_string())?;
    if !record.r#type.eq_ignore_ascii_case(BASE_TYPE) {
        return Err(format!("{} is not a base (type {})", id, record.r#type));
    }
    // A base is still a file another program may have changed. Refuse to
    // overwrite a definition the user has not seen.
    let current = body.unwrap_or_default();
    if let Some(expected) = expected_source {
        if current != expected {
            return Err("base changed since it was loaded; reopen it before saving".into());
        }
    }
    let spec = parse_base(&text)?;
    record.modified = Some(chrono::Utc::now().to_rfc3339());
    write_record(&path, &record, Some(&text)).map_err(|e| e.to_string())?;
    let view = render_base(&root, &spec).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "source": text, "view": view }).to_string())
}

#[tauri::command]
fn create_base_cmd(title: String) -> Result<String, String> {
    let root = records_root();
    let id = generate_record_id(&root);
    let mut record = Record::new(id.clone(), BASE_TYPE);
    let title = title.trim();
    record.title = Some(if title.is_empty() {
        "New base".to_string()
    } else {
        title.to_string()
    });
    let now = chrono::Utc::now().to_rfc3339();
    record.created = Some(now.clone());
    record.modified = Some(now);
    let path = record_path(&root, &record);
    let body = base_starter_document(record.title.as_deref().unwrap_or("New base"));
    write_record(&path, &record, Some(&body)).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
fn move_records_cmd(
    state: tauri::State<Sessions>,
    ids: Vec<String>,
    folder: String,
) -> Result<String, String> {
    let root = records_root();
    // Validate once so an invalid target fails the batch before any file moves.
    let target = normalize_folder(&folder)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let mut moved = 0usize;
    let mut errors: Vec<String> = Vec::new();
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    for id in ids {
        let Some(path) = find_record_by_id(&root, &id) else {
            errors.push(format!("{}: not found", id));
            continue;
        };
        match move_record_to_folder(&root, &path, &target) {
            Ok(_) => {
                // The session's path is now stale; the next open uses the new
                // file and the same WAL name, so dropping it is safe.
                sessions.remove(&id);
                moved += 1;
            }
            Err(e) => errors.push(format!("{}: {}", id, e)),
        }
    }
    Ok(serde_json::json!({ "moved": moved, "errors": errors }).to_string())
}

#[tauri::command]
fn undo_block_edit_cmd(state: tauri::State<Sessions>, id: String) -> Result<String, String> {
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
    session.undo().map_err(|e| e.to_string())?;
    Ok(session_view(session, None, false).to_string())
}

#[tauri::command]
fn redo_block_edit_cmd(state: tauri::State<Sessions>, id: String) -> Result<String, String> {
    let mut map = state.0.lock().map_err(|_| "session lock poisoned".to_string())?;
    let (session, _report) = ensure_open(&mut map, &id)?;
    session.redo().map_err(|e| e.to_string())?;
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
            insert_block_cmd,
            delete_block_cmd,
            move_records_cmd,
            update_record_fields_cmd,
            list_templates_cmd,
            create_from_template_cmd,
            save_as_template_cmd,
            get_rules_cmd,
            set_rules_cmd,
            set_auto_apply_rules_cmd,
            apply_rules_cmd,
            render_base_cmd,
            save_base_source_cmd,
            create_base_cmd,
            undo_block_edit_cmd,
            redo_block_edit_cmd,
            records_root_path,
            register_native_host,
            get_block_types,
            get_settings,
            set_naming_cmd,
            preview_naming_cmd,
            open_folder,
            set_registered_folders_cmd,
            set_records_root_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running Orchard");
}