pub mod config;

use base64::Engine;
use config::{load_config, logs_dir, set_records_root};
use lilac_core::{
    NamingConfig, NamingRule, Record, Wal, apply_block_edit,
    apply_rules_reported as apply_rules_core, delete_record, generate_record_id, normalize_folder,
    parse_rules, read_record, rebuild_index, record_filename, record_path, search_records,
    touch_opened, write_record,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub fn run_host() {
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

fn plan_with_rules(
    records_root: &std::path::Path,
    request: &lilac_core::PlanRequest,
    cfg: &config::Config,
    rules_text: &str,
) -> lilac_core::SavePlan {
    // Rules only run when the user has turned auto-apply on. A preview must
    // agree with the commit, so both go through this one decision.
    let (rules, warning) = if cfg.rules.auto_apply {
        parse_save_rules(rules_text)
    } else {
        (Vec::new(), None)
    };
    let mut plan = lilac_core::plan_save(records_root, request, &cfg.naming, &rules);
    if let Some(warning) = warning {
        plan.warnings.push(warning);
    }
    plan
}

fn parse_save_rules(text: &str) -> (Vec<lilac_core::Rule>, Option<String>) {
    if text.trim().is_empty() {
        return (Vec::new(), None);
    }
    match parse_rules(text) {
        Ok(ruleset) => (ruleset.rules, None),
        Err(e) => {
            // A broken rules file never blocks a save. It is logged and the
            // preview says so, so the file is not silently ignored.
            let mut wal = Wal::new(config::config_root().join("wal.log"));
            let _ = wal.append("rules_parse_error", json!({ "error": e }));
            (
                Vec::new(),
                Some(format!(
                    "rules could not be read, so none were applied: {e}"
                )),
            )
        }
    }
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
            Ok({
                let (cfg, _) = config::load_config();
                let orchid = config::load_orchid_config();
                let mut folders = cfg.registered_folders.clone();
                if folders.is_empty() {
                    folders = orchid.registered_folders.clone();
                }
                json!({
                    "records_root": rr,
                    "config_root": config::config_root().to_string_lossy(),
                    "registered_folders": folders,
                })
            })
        }
        "set_registered_folders" => {
            let folders: Vec<String> = params
                .get("folders")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let mut oc = config::load_orchid_config();
            oc.registered_folders = folders.clone();
            config::save_orchid_config(&oc)?;
            let mut lc = config::load_config().0;
            lc.registered_folders = folders.clone();
            config::save_config(&lc)?;
            Ok(json!({ "registered_folders": folders }))
        }
        "set_records_root" => {
            let new = params
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("missing path")?;
            let p = set_records_root(new)?;
            Ok(json!({ "records_root": p.to_string_lossy() }))
        }
        "plan_save" => {
            // A preview is the dry run of the commit, not a second
            // implementation: both use the same drafts and the same rules.
            let request: lilac_core::PlanRequest =
                serde_json::from_value(params).map_err(|e| e.to_string())?;
            let (cfg, _) = load_config();
            let rules_text = std::fs::read_to_string(config::rules_path()).unwrap_or_default();
            let plan = plan_with_rules(records_root, &request, &cfg, &rules_text);
            serde_json::to_value(plan).map_err(|e| e.to_string())
        }
        "commit_save" => {
            let plan: lilac_core::SavePlan =
                serde_json::from_value(params.get("plan").cloned().ok_or("missing plan")?)
                    .map_err(|e| e.to_string())?;
            let report = lilac_core::commit_plan(records_root, &plan);
            serde_json::to_value(report).map_err(|e| e.to_string())
        }
        "plan_path" => {
            // The preview recomputes the filename after the user edits a title
            // or folder, so the shown path stays the path that will be written.
            let record: Record =
                serde_json::from_value(params.get("record").cloned().ok_or("missing record")?)
                    .map_err(|e| e.to_string())?;
            let path = record_path(records_root, &record);
            let relative = path
                .strip_prefix(records_root)
                .ok()
                .and_then(|p| p.to_str())
                .map(|p| p.replace('\\', "/"))
                .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"));
            Ok(json!({ "path": relative, "filename": record_filename(&record) }))
        }
        "save_record" => {
            let record_val = params.get("record").cloned().ok_or("missing record")?;
            let mut record: Record =
                serde_json::from_value(record_val).map_err(|e| e.to_string())?;
            if record.id.trim().is_empty() {
                record.id = generate_record_id(records_root);
            }
            let now = chrono::Utc::now().to_rfc3339();
            if record.created.is_none() {
                record.created = Some(now.clone());
            }
            if record.modified.is_none() {
                record.modified = Some(now);
            }
            // Rules run on the new record before its path is chosen, so a
            // `move` action decides where the file lands. A broken rules file
            // never blocks a save: it is logged and skipped.
            if load_config().0.rules.auto_apply {
                let text = std::fs::read_to_string(config::rules_path()).unwrap_or_default();
                if !text.trim().is_empty() {
                    match parse_rules(&text) {
                        Ok(ruleset) => {
                            let (_, errors) = apply_rules_core(&mut record, &ruleset.rules);
                            if !errors.is_empty() {
                                let mut wal = Wal::new(config::config_root().join("wal.log"));
                                let _ = wal.append(
                                    "rules_action_error",
                                    serde_json::json!({ "record": record.id, "errors": errors }),
                                );
                            }
                        }
                        Err(e) => {
                            let mut wal = Wal::new(config::config_root().join("wal.log"));
                            let _ = wal.append(
                                "rules_parse_error",
                                serde_json::json!({ "record": record.id, "error": e }),
                            );
                        }
                    }
                }
                // A rule-owned folder is user input too: reject escapes rather
                // than letting the save fail or silently write outside.
                if let Some(folder) = record.folder.clone() {
                    match normalize_folder(&folder) {
                        Ok(None) => record.folder = None,
                        Ok(Some(f)) if f == record.r#type => record.folder = None,
                        Ok(f) => record.folder = f,
                        Err(_) => record.folder = None,
                    }
                }
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
            let mut new_body = params
                .get("body")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or(body);
            // surgical block edit: splice a single block, preserving the rest
            if let Some(be) = params.get("block_edit") {
                let bid = be
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or("block_edit missing id")?;
                let content = be
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("block_edit missing content")?;
                let cur = new_body.clone().unwrap_or_default();
                let spliced = apply_block_edit(&cur, bid, content).map_err(|e| e.to_string())?;
                new_body = Some(spliced);
                let mut wal = Wal::new(config::config_root().join("wal.log"));
                let _ = wal.append(
                    "block_edit",
                    serde_json::json!({ "record": id, "block": bid }),
                );
            }
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
        "touch_opened" => {
            // Lilac opened this record as a tab. Only `opened` moves; the record
            // did not become edited just because it was viewed.
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing id")?;
            let path =
                find_record_path(records_root, id).ok_or_else(|| format!("not found: {}", id))?;
            let opened = touch_opened(&path).map_err(|e| e.to_string())?;
            Ok(json!({ "id": id, "opened": opened }))
        }
        "describe_display" => {
            // Capture is a browser concern; deciding which display a window
            // was on is geometry logic, so it lives with the other geometry
            // rules in the core.
            let geometry: lilac_core::WindowGeometry =
                serde_json::from_value(params.get("geometry").cloned().ok_or("missing geometry")?)
                    .map_err(|e| e.to_string())?;
            let displays: Vec<lilac_core::DisplayInfo> =
                serde_json::from_value(params.get("displays").cloned().unwrap_or(json!([])))
                    .map_err(|e| e.to_string())?;
            let described = lilac_core::describe_display(&geometry, &displays);
            Ok(json!({ "geometry": described }))
        }
        "place_window" => {
            // Bounds are hints: the caller still opens the window if this
            // returns nothing, and reports that geometry was not applied.
            let geometry: lilac_core::WindowGeometry =
                serde_json::from_value(params.get("geometry").cloned().ok_or("missing geometry")?)
                    .map_err(|e| e.to_string())?;
            let displays: Vec<lilac_core::DisplayInfo> =
                serde_json::from_value(params.get("displays").cloned().unwrap_or(json!([])))
                    .map_err(|e| e.to_string())?;
            match lilac_core::place_window(&geometry, &displays) {
                Some(placement) => Ok(json!({ "placement": placement })),
                None => Ok(json!({ "placement": null })),
            }
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
            // An explicit rule (old extension clients) wins. Otherwise use the
            // naming config edited in Orchard Settings, which is authoritative.
            let explicit: Option<NamingRule> = params
                .get("rule")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e: serde_json::Error| e.to_string())?;
            let cfg = match explicit {
                Some(rule) => NamingConfig {
                    rules: vec![rule],
                    domains: Vec::new(),
                },
                None => config::load_config().0.naming,
            };
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
            let (fired, errors) = apply_rules_core(&mut record, &ruleset.rules);
            let fired_json: Vec<Value> = fired
                .iter()
                .map(|a| json!({ "action": format!("{:?}", a) }))
                .collect();
            Ok(json!({ "record": record, "fired": fired_json, "errors": errors }))
        }
        "move_record" => {
            let id = params
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("missing id")?;
            let folder = params.get("folder").and_then(|v| v.as_str()).unwrap_or("");
            let path =
                find_record_path(records_root, id).ok_or_else(|| format!("not found: {}", id))?;
            let (mut rec, body) = read_record(&path).map_err(|e| e.to_string())?;
            if folder.trim().is_empty() {
                rec.folder = None;
            } else {
                rec.folder = Some(folder.trim().to_string());
            }
            let new_path = record_path(records_root, &rec);
            if new_path != path {
                if let Some(parent) = new_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                // write to new path first, then remove old
                write_record(&new_path, &rec, body.as_deref()).map_err(|e| e.to_string())?;
                let _ = std::fs::remove_file(&path);
            } else {
                write_record(&path, &rec, body.as_deref()).map_err(|e| e.to_string())?;
            }
            Ok(json!({ "id": rec.id, "path": new_path.to_string_lossy() }))
        }
        "move_file" => {
            let src = params
                .get("src")
                .and_then(|v| v.as_str())
                .ok_or("missing src")?;
            let dst = params
                .get("dst")
                .and_then(|v| v.as_str())
                .ok_or("missing dst")?;
            let src_p = PathBuf::from(src);
            let dst_p = PathBuf::from(dst);
            if let Some(parent) = dst_p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::rename(&src_p, &dst_p).map_err(|e| e.to_string())?;
            Ok(json!({ "src": src, "dst": dst }))
        }
        _ => Err(format!("unknown method: {}", method)),
    }
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
    if let Some(v) = patch.get("description") {
        // An explicit null or empty string clears the description; a missing
        // key leaves it alone, like every other field.
        if v.is_null() {
            rec.description = None;
        } else if let Some(text) = v.as_str() {
            rec.description = if text.trim().is_empty() {
                None
            } else {
                Some(text.to_string())
            };
        }
    }
    if let Some(v) = patch.get("folder").and_then(|x| x.as_str()) {
        if v.trim().is_empty() {
            rec.folder = None;
        } else {
            rec.folder = Some(v.trim().to_string());
        }
    }
    if let Some(v) = patch.get("saved_from") {
        rec.saved_from = v
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
    }
    // Typed list fields are replaced as a whole; a malformed patch leaves the
    // record untouched rather than silently emptying it.
    if let Some(v) = patch.get("items") {
        if v.is_null() {
            rec.items.clear();
        } else if let Ok(items) = serde_json::from_value(v.clone()) {
            rec.items = items;
        }
    }
    if let Some(v) = patch.get("tab_groups") {
        if v.is_null() {
            rec.tab_groups.clear();
        } else if let Ok(groups) = serde_json::from_value(v.clone()) {
            rec.tab_groups = groups;
        }
    }
    if let Some(v) = patch.get("geometry") {
        rec.geometry = if v.is_null() {
            None
        } else {
            serde_json::from_value(v.clone()).ok()
        };
    }
    // open metadata: merge remaining keys into extra
    if let Some(obj) = patch.as_object() {
        for (k, v) in obj {
            if !matches!(
                k.as_str(),
                "title"
                    | "status"
                    | "url"
                    | "tags"
                    | "description"
                    | "folder"
                    | "saved_from"
                    | "items"
                    | "tab_groups"
                    | "geometry"
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
                "url": e.url,
                "path": e.path,
                "created": e.created,
                "modified": e.modified,
                "opened": e.opened,
                "item_count": e.item_count,
                "saved_from": e.saved_from,
                "group_name": e.group_name,
                "group_color": e.group_color,
                "geometry": e.geometry,
            })
        })
        .collect();
    json!({ "records": arr })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::OnceLock;
    use tempfile::TempDir;

    /// One config root for every test in this module: `LILAC_CONFIG_ROOT` is
    /// process-wide, so two tests pointing it at different directories would
    /// race. The root only holds logs and the index cache.
    fn test_config() -> &'static TempDir {
        static CONFIG: OnceLock<TempDir> = OnceLock::new();
        CONFIG.get_or_init(|| {
            let dir = TempDir::new().unwrap();
            // SAFETY: set once, before any other test runs, and never changed.
            unsafe { std::env::set_var("LILAC_CONFIG_ROOT", dir.path()) };
            dir
        })
    }

    fn call(root: &Path, method: &str, params: Value) -> Value {
        handle(root, json!({ "id": 1, "method": method, "params": params }))
    }

    fn record_path_of(root: &Path, id: &str) -> std::path::PathBuf {
        find_record_path(root, id).expect("record on disk")
    }

    #[test]
    fn saves_are_stamped_and_opening_only_touches_opened() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let saved = call(
            root,
            "save_record",
            json!({ "record": { "id": "rec_life", "type": "page", "title": "Life" }, "body": "text" }),
        );
        assert_eq!(saved.get("ok").and_then(|v| v.as_bool()), Some(true));
        let path = record_path_of(root, "rec_life");
        let (first, body) = read_record(&path).unwrap();
        assert_eq!(body.as_deref(), Some("text"));
        let created = first.created.clone().expect("created is automatic");
        let modified = first.modified.clone().expect("modified is automatic");
        assert_eq!(created, modified);
        assert!(first.opened.is_none());

        let touched = call(root, "touch_opened", json!({ "id": "rec_life" }));
        assert_eq!(touched.get("ok").and_then(|v| v.as_bool()), Some(true));
        let (second, _) = read_record(&path).unwrap();
        assert_eq!(second.created, first.created);
        assert_eq!(second.modified, first.modified);
        assert!(second.opened.is_some());

        let updated = call(
            root,
            "update_record",
            json!({ "id": "rec_life", "patch": { "title": "Life, edited" } }),
        );
        assert_eq!(updated.get("ok").and_then(|v| v.as_bool()), Some(true));
        let (third, _) = read_record(&path).unwrap();
        assert_eq!(third.created, first.created);
        assert_ne!(third.modified, first.modified);
        assert_eq!(third.opened, second.opened);
    }

    #[test]
    fn search_results_carry_collection_metadata_without_browser_ids() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let saved = call(
            root,
            "save_record",
            json!({
                "record": {
                    "id": "rec_win",
                    "type": "collection",
                    "title": "Saved tabs — Work",
                    "saved_from": "window",
                    "tab_group": { "title": "Work", "color": "blue" },
                    "tab_groups": [
                        { "name": "Work", "color": "blue", "collapsed": true }
                    ],
                    "items": [
                        { "id": "rec_a", "url": "https://a.example", "title": "A" },
                        { "id": "rec_b", "url": "https://b.example", "title": "B", "pinned": true }
                    ]
                },
                "body": "- [[rec_a]]\n- [[rec_b]]"
            }),
        );
        assert_eq!(saved.get("ok").and_then(|v| v.as_bool()), Some(true));

        // The popup's collection list is built from search results, so every
        // value it renders has to be in the index, not only in the file.
        let found = call(root, "search_records", json!({ "query": "" }));
        let results = found
            .get("result")
            .and_then(|r| r.get("results"))
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let entry = results
            .iter()
            .find(|e| e.get("id").and_then(|v| v.as_str()) == Some("rec_win"))
            .expect("collection is in the results");
        assert_eq!(entry.get("item_count").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            entry.get("saved_from").and_then(|v| v.as_str()),
            Some("window")
        );
        assert_eq!(
            entry.get("group_name").and_then(|v| v.as_str()),
            Some("Work")
        );
        assert_eq!(
            entry.get("group_color").and_then(|v| v.as_str()),
            Some("blue")
        );
        let text = serde_json::to_string(entry).unwrap();
        assert!(!text.contains("window_id"));
        assert!(!text.contains("tab_id"));
        assert!(!text.contains("groupId"));
    }

    #[test]
    fn workspace_geometry_is_described_and_clamped_by_the_host() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let displays = json!([
            {
                "bounds": { "left": 0.0, "top": 0.0, "width": 1920.0, "height": 1080.0 },
                "work_area": { "left": 0.0, "top": 0.0, "width": 1920.0, "height": 1040.0 },
                "scale": 1.0,
                "primary": true
            },
            {
                "bounds": { "left": 1920.0, "top": 0.0, "width": 2560.0, "height": 1440.0 },
                "scale": 1.5,
                "primary": false
            }
        ]);

        // Capture asks the host which display the window was on, so the answer
        // does not depend on the extension knowing about monitors.
        let described = call(
            root,
            "describe_display",
            json!({
                "geometry": { "left": 2000.0, "top": 100.0, "width": 1200.0, "height": 800.0 },
                "displays": displays
            }),
        );
        let geometry = &described["result"]["geometry"];
        assert_eq!(geometry["display"]["left"].as_f64(), Some(1920.0));
        assert_eq!(geometry["display_scale"].as_f64(), Some(1.5));

        // Reopening on a smaller set of displays clamps an off-screen window.
        let placed = call(
            root,
            "place_window",
            json!({
                "geometry": { "left": 5000.0, "top": 5000.0, "width": 800.0, "height": 600.0 },
                "displays": [{
                    "bounds": { "left": 0.0, "top": 0.0, "width": 1280.0, "height": 720.0 },
                    "primary": true
                }]
            }),
        );
        let placement = &placed["result"]["placement"];
        assert_eq!(placement["clamped"].as_bool(), Some(true));
        assert_eq!(placement["left"].as_f64(), Some(240.0));
        assert_eq!(placement["top"].as_f64(), Some(60.0));

        // Partial bounds have nothing to place; the caller still opens the set.
        let partial = call(
            root,
            "place_window",
            json!({
                "geometry": { "left": 10.0, "top": 20.0 },
                "displays": displays
            }),
        );
        assert!(partial["result"]["placement"].is_null());
    }

    fn plan_params(source: &str) -> Value {
        json!({
            "source": source,
            "tabs": [
                { "url": "https://a.example/one", "title": "Alpha", "group": "Design" },
                { "url": "https://b.example/two", "title": "Beta", "group": "Design" }
            ]
        })
    }

    #[test]
    fn preview_and_commit_write_exactly_the_planned_drafts() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let planned = call(root, "plan_save", plan_params("group"));
        assert_eq!(planned.get("ok").and_then(|v| v.as_bool()), Some(true));
        let plan = planned["result"].clone();
        let drafts = plan["records"].as_array().unwrap();
        assert_eq!(drafts.len(), 3);
        assert_eq!(drafts[2]["record"]["type"].as_str(), Some("collection"));
        assert_eq!(drafts[2]["record"]["saved_from"].as_str(), Some("group"));
        // Nothing is written by planning.
        assert!(!root.join("Records").exists());

        let committed = call(root, "commit_save", json!({ "plan": plan }));
        let report = &committed["result"];
        assert_eq!(report["written"].as_array().unwrap().len(), 3);
        assert!(report["failed"].as_array().unwrap().is_empty());

        // Every draft's written path is the path the preview showed.
        for draft in drafts {
            let expected = draft["path"].as_str().unwrap();
            let found = report["written"]
                .as_array()
                .unwrap()
                .iter()
                .any(|written| written["path"].as_str() == Some(expected));
            assert!(found, "preview path {expected} was not written");
            let full = root.join(expected);
            let (written, body) = read_record(&full).unwrap();
            assert_eq!(written.id, draft["id"].as_str().unwrap());
            assert_eq!(body.unwrap_or_default(), draft["body"].as_str().unwrap());
        }
    }

    #[test]
    fn an_excluded_draft_is_skipped_not_written() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut plan = call(root, "plan_save", plan_params("selection"))["result"].clone();
        plan["records"][0]["excluded"] = json!(true);
        let excluded_id = plan["records"][0]["id"].as_str().unwrap().to_string();
        let excluded_path = plan["records"][0]["path"].as_str().unwrap().to_string();

        let report = call(root, "commit_save", json!({ "plan": plan }));
        let skipped = report["result"]["skipped"].as_array().unwrap();
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].as_str(), Some(excluded_id.as_str()));
        assert_eq!(report["result"]["written"].as_array().unwrap().len(), 2);
        assert!(!root.join(&excluded_path).exists());
    }

    #[test]
    fn a_duplicate_page_is_reported_in_the_preview() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let one = json!({
            "source": "page",
            "tabs": [{ "url": "https://same.example/one", "title": "One" }]
        });
        let first = call(root, "plan_save", one.clone())["result"].clone();
        call(root, "commit_save", json!({ "plan": first }));

        let second = call(root, "plan_save", one)["result"].clone();
        let draft = &second["records"][0];
        assert!(draft["duplicate_of"].is_string());
        assert!(
            draft["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w.as_str().unwrap_or("").contains("already saves"))
        );
    }

    #[test]
    fn a_broken_rules_file_becomes_a_plan_warning() {
        // The parser is what decides; the host turns its failure into a
        // warning instead of blocking the save.
        let (rules, warning) = parse_save_rules("this is not a rule");
        assert!(rules.is_empty());
        assert!(warning.unwrap().contains("could not be read"));

        let (rules, warning) = parse_save_rules("");
        assert!(rules.is_empty());
        assert!(warning.is_none());
    }

    #[test]
    fn a_broken_rules_file_does_not_stop_the_plan() {
        test_config();
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let request: lilac_core::PlanRequest = serde_json::from_value(plan_params("page")).unwrap();
        let cfg = config::Config {
            rules: config::RuleRuntimeConfig { auto_apply: true },
            ..config::Config::default()
        };
        let plan = plan_with_rules(root, &request, &cfg, "this is not a rule");
        assert_eq!(plan.records.len(), 2);
        assert!(
            plan.warnings
                .iter()
                .any(|w| w.contains("could not be read"))
        );
    }
}
