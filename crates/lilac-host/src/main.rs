mod config;

use base64::Engine;
use config::{load_config, logs_dir, set_records_root};
use lilac_core::{
    NamingConfig, NamingRule, Record, apply_rules as apply_rules_core, delete_record, parse_rules,
    read_record, rebuild_index, record_path, search_records, write_record,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::path::PathBuf;

fn main() {
    // logging to file
    let _ = std::fs::create_dir_all(logs_dir());
    let log_path = logs_dir().join("host.log");
    // simple: append via eprintln is not used; we write to file best-effort.
    // main loop: re-read config each request so set_records_root takes effect
    // without restarting the host.
    loop {
        match read_message() {
            Ok(Some(msg)) => {
                let (_cfg, records_root) = load_config();
                let resp = handle(&records_root, msg);
                if let Err(e) = write_message(&resp) {
                    log(&log_path, &format!("write error: {}", e));
                    break;
                }
            }
            Ok(None) => {
                // EOF
                break;
            }
            Err(e) => {
                log(&log_path, &format!("read error: {}", e));
                break;
            }
        }
    }
}

fn log(path: &std::path::Path, msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{} {}", chrono::Utc::now().to_rfc3339(), msg);
    }
}

fn read_message() -> Result<Option<Value>, String> {
    let mut stdin = io::stdin().lock();
    let mut len_buf = [0u8; 4];
    match stdin.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.to_string()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > 16 * 1024 * 1024 {
        return Err("invalid message length".into());
    }
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;
    Ok(Some(v))
}

fn write_message(v: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    stdout.write_all(&bytes).map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn handle(records_root: &std::path::Path, msg: Value) -> Value {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));

    let result = dispatch(records_root, method, params);
    match result {
        Ok(r) => json!({ "id": id, "ok": true, "result": r }),
        Err(e) => json!({ "id": id, "ok": false, "error": { "code": "ERROR", "message": e } }),
    }
}

fn dispatch(records_root: &std::path::Path, method: &str, params: Value) -> Result<Value, String> {
    match method {
        "get_config" => {
            let rr = records_root.to_string_lossy().to_string();
            Ok(
                json!({ "records_root": rr, "config_root": config::config_root().to_string_lossy() }),
            )
        }
        "set_records_root" => {
            let new = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("missing path")?;
            let p = set_records_root(new)?;
            Ok(json!({ "records_root": p.to_string_lossy() }))
        }
        "save_record" => {
            let record_val = params.get("record").cloned().ok_or("missing record")?;
            let mut record: Record =
                serde_json::from_value(record_val).map_err(|e| e.to_string())?;
            if record.id.trim().is_empty() {
                record.id = generate_id(records_root);
            }
            let now = chrono::Utc::now().to_rfc3339();
            if record.created.is_none() {
                record.created = Some(now.clone());
            }
            if record.modified.is_none() {
                record.modified = Some(now);
            }
            let body = params.get("body").and_then(|v| v.as_str());
            let path = record_path(records_root, &record);
            write_record(&path, &record, body).map_err(|e| e.to_string())?;
            Ok(json!({ "id": record.id, "path": path.to_string_lossy() }))
        }
        "update_record" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing id")?;
            // find existing file by scanning
            let path =
                find_record_path(records_root, id).ok_or_else(|| format!("not found: {}", id))?;
            let (mut rec, body) = read_record(&path).map_err(|e| e.to_string())?;
            // apply patch: merge top-level fields from params.patch
            if let Some(patch) = params.get("patch") {
                apply_patch(&mut rec, patch);
            }
            let new_body = params
                .get("body")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(body);
            write_record(&path, &rec, new_body.as_deref()).map_err(|e| e.to_string())?;
            Ok(json!({ "id": rec.id, "path": path.to_string_lossy() }))
        }
        "delete_record" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing id")?;
            let path =
                find_record_path(records_root, id).ok_or_else(|| format!("not found: {}", id))?;
            delete_record(&path).map_err(|e| e.to_string())?;
            Ok(json!({}))
        }
        "load_record" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing id")?;
            let path =
                find_record_path(records_root, id).ok_or_else(|| format!("not found: {}", id))?;
            let (rec, body) = read_record(&path).map_err(|e| e.to_string())?;
            Ok(json!({ "record": rec, "body": body, "path": path.to_string_lossy() }))
        }
        "search_records" => {
            let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let idx = rebuild_index(records_root).map_err(|e| e.to_string())?;
            let res = search_records(&idx, query);
            Ok(json!({ "results": res }))
        }
        "rebuild_index" => {
            let idx = rebuild_index(records_root).map_err(|e| e.to_string())?;
            // write index.json to config root
            let index_path = config::config_root().join("index.json");
            let _ = std::fs::create_dir_all(config::config_root());
            let _ = std::fs::write(
                &index_path,
                serde_json::to_string_pretty(&idx_records_to_json(&idx)).unwrap_or_default(),
            );
            Ok(json!({ "count": idx.records.len() }))
        }
        "open_folder" => {
            let target = params
                .get("path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| records_root.to_path_buf());
            let target = match target.to_string_lossy().as_ref() {
                "records" => records_root.to_path_buf(),
                "config" => config::config_root(),
                "" => records_root.to_path_buf(),
                _ => target,
            };
            std::fs::create_dir_all(&target).ok();
            open_in_file_manager(&target)?;
            Ok(json!({ "path": target.to_string_lossy() }))
        }
        "save_asset" => {
            let data_b64 = params
                .get("data")
                .and_then(|v| v.as_str())
                .ok_or("missing data")?;
            let subdir = params
                .get("subdir")
                .and_then(|v| v.as_str())
                .unwrap_or("Assets");
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .map_err(|e| e.to_string())?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = format!("{:x}", hasher.finalize());
            let ext = params.get("ext").and_then(|v| v.as_str()).unwrap_or("bin");
            let filename = format!("{}.{}", hash, ext);
            let dir = records_root.join(subdir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(&filename);
            if !path.exists() {
                std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
            }
            Ok(json!({ "path": format!("{}/{}", subdir, filename), "hash": hash }))
        }
        "list_conflicts" => {
            let mut out = Vec::new();
            let subdir = records_root.join("Records");
            collect_conflicts(&subdir, records_root, &mut out);
            Ok(json!({ "conflicts": out }))
        }
        "derive_title" => {
            let record_val = params.get("record").cloned().ok_or("missing record")?;
            let record: Record = serde_json::from_value(record_val).map_err(|e| e.to_string())?;
            let rule: NamingRule = params
                .get("rule")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e: serde_json::Error| e.to_string())?
                .unwrap_or_default();
            let cfg = NamingConfig { rules: vec![rule] };
            Ok(json!({ "title": cfg.derive_title(&record) }))
        }
        "apply_rules" => {
            let record_val = params.get("record").cloned().ok_or("missing record")?;
            let mut record: Record =
                serde_json::from_value(record_val).map_err(|e| e.to_string())?;
            let rules_text = if let Some(t) = params.get("rules").and_then(|v| v.as_str()) {
                t.to_string()
            } else {
                std::fs::read_to_string(config::rules_path()).unwrap_or_default()
            };
            let ruleset = parse_rules(&rules_text).map_err(|e| e.to_string())?;
            let fired = apply_rules_core(&mut record, &ruleset.rules);
            let fired_json: Vec<Value> = fired
                .iter()
                .map(|a| json!({ "action": format!("{:?}", a) }))
                .collect();
            Ok(json!({ "record": record, "fired": fired_json }))
        }
        _ => Err(format!("unknown method: {}", method)),
    }
}

fn generate_id(records_root: &std::path::Path) -> String {
    for _ in 0..10 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let cand = format!("rec_{:x}", nanos as u64);
        // shorten to 7 hex
        let short = &cand[..std::cmp::min(cand.len(), "rec_".len() + 7)];
        let candidate = short.to_string();
        if find_record_path(records_root, &candidate).is_none() {
            return candidate;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    format!("rec_{}", chrono::Utc::now().timestamp_millis())
}

fn open_in_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(clippy::collapsible_if)]
fn collect_conflicts(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<Value>) {
    if !dir.is_dir() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_conflicts(&path, root, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if contents.contains("<<<<<<<") && contents.contains(">>>>>>>") {
                        let id = read_record(&path).map(|r| r.0.id).unwrap_or_default();
                        let rel = path
                            .strip_prefix(root)
                            .ok()
                            .and_then(|p| p.to_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| path.to_string_lossy().to_string());
                        out.push(json!({ "id": id, "path": rel }));
                    }
                }
            }
        }
    }
}

fn find_record_path(records_root: &std::path::Path, id: &str) -> Option<PathBuf> {
    // scan records root
    let subdir = records_root.join("Records");
    find_by_id(&subdir, id)
}

fn find_by_id(dir: &std::path::Path, id: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(p) = find_by_id(&path, id) {
                    return Some(p);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("md")
                && let Ok((rec, _)) = read_record(&path)
                && rec.id == id
            {
                return Some(path);
            }
        }
    }
    None
}

fn apply_patch(rec: &mut Record, patch: &Value) {
    // merge a few well-known fields
    if let Some(v) = patch.get("title").and_then(|x| x.as_str()) {
        rec.title = Some(v.to_string());
    }
    if let Some(v) = patch.get("status").and_then(|x| x.as_str()) {
        rec.status = Some(v.to_string());
    }
    if let Some(v) = patch.get("url").and_then(|x| x.as_str()) {
        rec.url = Some(v.to_string());
    }
    if let Some(arr) = patch.get("tags").and_then(|x| x.as_array()) {
        rec.tags = arr
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = patch.get("description").and_then(|x| x.as_str()) {
        rec.description = Some(v.to_string());
    }
    // open metadata: merge remaining keys into extra
    if let Some(obj) = patch.as_object() {
        for (k, v) in obj {
            if !matches!(
                k.as_str(),
                "title" | "status" | "url" | "tags" | "description"
            ) {
                // try to convert to yaml value via json->yaml roundtrip
                if let Ok(yv) = serde_yaml::to_value(v) {
                    // serde_yaml::to_value on a serde_json::Value may not work directly; fallback store as string
                    rec.extra.insert(k.clone(), yv);
                }
            }
        }
    }
    // bump modified
    rec.modified = Some(chrono::Utc::now().to_rfc3339());
}

fn idx_records_to_json(idx: &lilac_core::Index) -> Value {
    // reuse IndexEntry serialization; build array
    let arr: Vec<Value> = idx
        .records
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "type": e.r#type,
                "title": e.title,
                "tags": e.tags,
                "path": e.path,
                "modified": e.modified,
            })
        })
        .collect();
    json!({ "records": arr })
}
