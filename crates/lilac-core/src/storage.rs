use crate::error::Error;
use crate::record::Record;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Frontmatter split
// ---------------------------------------------------------------------------

pub fn split_frontmatter(contents: &str) -> Result<(&str, Option<&str>), Error> {
    let trimmed = contents.trim_start_matches('\u{feff}');
    let first_line_end = trimmed.find('\n').map(|i| i + 1).unwrap_or(trimmed.len());
    if trimmed[..first_line_end].trim() != "---" {
        return Err(Error::NoFrontmatter);
    }
    let after_first = &trimmed[first_line_end..];
    let closing = after_first.find("\n---").ok_or(Error::NoFrontmatter)?;
    let after_marker = closing + 4; // "\n---"
    let next = after_first.get(after_marker..).unwrap_or("");
    if !next.is_empty() && !next.starts_with('\n') && !next.starts_with('\r') {
        return Err(Error::NoFrontmatter);
    }
    let yaml = &after_first[..closing];
    let rest = &after_first[after_marker..];
    let body = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .unwrap_or(rest);
    let body = if body.trim().is_empty() {
        None
    } else {
        Some(body)
    };
    Ok((yaml, body))
}

// ---------------------------------------------------------------------------
// Read / write
// ---------------------------------------------------------------------------

pub fn read_record(path: &Path) -> Result<(Record, Option<String>), Error> {
    let contents = fs::read_to_string(path)?;
    if contents.contains("<<<<<<<") && contents.contains(">>>>>>>") {
        return Err(Error::Conflict(path.to_string_lossy().to_string()));
    }
    let (yaml, body) = split_frontmatter(&contents)?;
    let record: Record = serde_yaml::from_str(yaml)?;
    let body = body
        .map(|b| b.trim_end_matches('\n').to_string())
        .filter(|b| !b.is_empty());
    Ok((record, body))
}

pub fn write_record(path: &Path, record: &Record, body: Option<&str>) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(record)?;
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(yaml.trim_end_matches('\n'));
    out.push_str("\n---\n");
    if let Some(b) = body {
        let b = b.trim();
        if !b.is_empty() {
            out.push_str(b);
            out.push('\n');
        }
    }
    atomic_write(path, out.as_bytes())
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("record")
    ));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Read a record and return its last-modified time (seconds since epoch) for
/// optimistic-concurrency checks.
pub fn read_record_with_mtime(path: &Path) -> Result<(Record, Option<String>, u64), Error> {
    let meta = fs::metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (rec, body) = read_record(path)?;
    Ok((rec, body, mtime))
}

/// Write a record only if the file's mtime matches `expected_mtime`.
/// Returns `Error::Stale` if the file changed since the read.
pub fn write_record_if_unchanged(
    path: &Path,
    record: &Record,
    body: Option<&str>,
    expected_mtime: u64,
) -> Result<(), Error> {
    if path.exists() {
        let meta = fs::metadata(path)?;
        let cur = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if cur != expected_mtime {
            return Err(Error::Stale(path.to_string_lossy().to_string()));
        }
    }
    write_record(path, record, body)
}

pub fn delete_record(path: &Path) -> Result<(), Error> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Naming
// ---------------------------------------------------------------------------

pub fn slugify_title(title: Option<&str>, url: Option<&str>) -> String {
    let source = title
        .filter(|s| !s.trim().is_empty())
        .or_else(|| url.and_then(host_of))
        .unwrap_or("record");
    let s = slug::slugify(source);
    if s.is_empty() {
        "record".to_string()
    } else if s.len() > 40 {
        s[..40].trim_end_matches('-').to_string()
    } else {
        s
    }
}

fn host_of(url: &str) -> Option<&str> {
    let after = url.split("://").nth(1).unwrap_or(url);
    let host = after.split('/').next().unwrap_or(after);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() { None } else { Some(host) }
}

pub fn record_filename(record: &Record) -> String {
    let slug = slugify_title(record.title.as_deref(), record.url.as_deref());
    format!("{}--{}.md", slug, record.id)
}

pub fn records_dir(records_root: &Path, rtype: &str) -> PathBuf {
    records_root.join("Records").join(rtype)
}

/// Normalise a user-supplied folder to a path relative to `Records/`.
///
/// `Ok(None)` means "the type's default folder". Absolute paths and `.`/`..`
/// segments are rejected rather than silently stripped, so a typo cannot place
/// a record somewhere the user did not choose. Backslashes are accepted as
/// separators because Windows users paste them.
pub fn normalize_folder(raw: &str) -> Result<Option<String>, Error> {
    let trimmed = raw.trim().replace('\\', "/");
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.starts_with('/') {
        return Err(Error::Invalid(format!(
            "folder must be relative to Records/: {}",
            raw
        )));
    }
    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err(Error::Invalid(format!(
                "folder may not contain . or .. segments: {}",
                raw
            )));
        }
        if segment.contains(':') {
            return Err(Error::Invalid(format!(
                "folder may not contain a drive or stream separator: {}",
                raw
            )));
        }
        if segment.chars().any(char::is_control) {
            return Err(Error::Invalid("folder contains control characters".into()));
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        Ok(None)
    } else {
        Ok(Some(segments.join("/")))
    }
}

/// Move a record into `folder` (relative to `Records/`, empty = the type's
/// default folder) and update its `folder` field. The destination is written
/// before the source is removed, so a failed move never loses the record.
pub fn move_record_to_folder(
    records_root: &Path,
    path: &Path,
    folder: &str,
) -> Result<PathBuf, Error> {
    let target = normalize_folder(folder)?;
    let (mut record, body) = read_record(path)?;
    let original = record.folder.clone();
    // An explicit folder equal to the type folder is the default; omit it so
    // moving a page to `page/` and back leaves no redundant field behind.
    record.folder = match target {
        Some(f) if f == record.r#type => None,
        other => other,
    };
    let destination = record_path(records_root, &record);
    if destination == path {
        if record.folder != original {
            write_record(path, &record, body.as_deref())?;
        }
        return Ok(path.to_path_buf());
    }
    if destination.exists() {
        return Err(Error::Invalid(format!(
            "destination already exists: {}",
            destination.display()
        )));
    }
    write_record(&destination, &record, body.as_deref())?;
    fs::remove_file(path)?;
    Ok(destination)
}

/// Find a record file by id anywhere under `Records/`.
pub fn find_record_by_id(records_root: &Path, id: &str) -> Option<PathBuf> {
    let mut found = None;
    let records_subdir = records_root.join("Records");
    let _ = walk_md(&records_subdir, &mut |path| {
        if found.is_some() {
            return;
        }
        match read_record(path) {
            Ok((record, _)) if record.id == id => found = Some(path.to_path_buf()),
            _ => {}
        }
    });
    found
}

pub fn record_path(records_root: &Path, record: &Record) -> PathBuf {
    let base = records_root.join("Records");
    let dir = match &record.folder {
        Some(f) if !f.trim().is_empty() => {
            // sanitize: no traversal
            let clean: String = f
                .replace('\\', "/")
                .split('/')
                .filter(|seg| !seg.is_empty() && *seg != ".." && *seg != ".")
                .collect::<Vec<_>>()
                .join("/");
            if clean.is_empty() {
                base.join(&record.r#type)
            } else {
                base.join(clean)
            }
        }
        _ => base.join(&record.r#type),
    };
    dir.join(record_filename(record))
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Index {
    pub records: Vec<IndexEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub r#type: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    pub path: String,
    pub modified: Option<String>,
}

pub fn rebuild_index(records_root: &Path) -> Result<Index, Error> {
    let mut records = Vec::new();
    let records_subdir = records_root.join("Records");
    if !records_subdir.exists() {
        return Ok(Index { records });
    }
    walk_md(&records_subdir, &mut |path| {
        if let Ok((rec, _body)) = read_record(path) {
            let rel = path
                .strip_prefix(records_root)
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            records.push(IndexEntry {
                id: rec.id,
                r#type: rec.r#type,
                title: rec.title,
                tags: rec.tags,
                status: rec.status,
                folder: rec.folder,
                path: rel,
                modified: rec.modified,
            });
        }
    })?;
    records.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.id.cmp(&b.id)));
    Ok(Index { records })
}

fn walk_md(dir: &Path, cb: &mut dyn FnMut(&Path)) -> Result<(), Error> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_md(&path, cb)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            cb(&path);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

pub fn search_records(index: &Index, query: &str) -> Vec<IndexEntry> {
    let terms: Vec<&str> = query.split_whitespace().collect();
    index
        .records
        .iter()
        .filter(|e| {
            terms.iter().all(|t| {
                if let Some((k, v)) = t.split_once(':') {
                    match k {
                        "tag" => e.tags.iter().any(|x| x.eq_ignore_ascii_case(v)),
                        "type" => e.r#type.eq_ignore_ascii_case(v),
                        "status" => e
                            .status
                            .as_deref()
                            .map(|s| s.eq_ignore_ascii_case(v))
                            .unwrap_or(false),
                        _ => true,
                    }
                } else {
                    let needle = t.to_lowercase();
                    e.title
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&needle)
                        || e.tags.iter().any(|x| x.to_lowercase().contains(&needle))
                        || e.id.to_lowercase().contains(&needle)
                }
            })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn seed(root: &Path, id: &str, rtype: &str, folder: Option<&str>) -> PathBuf {
        let mut record = Record::new(id, rtype);
        record.title = Some(format!("Test {id}"));
        record.folder = folder.map(str::to_string);
        record.extra.insert(
            "custom".to_string(),
            serde_yaml::Value::String("keep".to_string()),
        );
        let path = record_path(root, &record);
        write_record(&path, &record, Some("# Body\n\ntext")).unwrap();
        path
    }

    #[test]
    fn normalize_folder_accepts_nested_and_rejects_escape() {
        assert_eq!(normalize_folder("").unwrap(), None);
        assert_eq!(normalize_folder("   ").unwrap(), None);
        assert_eq!(
            normalize_folder("Archive\\2026").unwrap(),
            Some("Archive/2026".to_string())
        );
        assert_eq!(normalize_folder("a//b/").unwrap(), Some("a/b".to_string()));
        assert!(normalize_folder("../outside").is_err());
        assert!(normalize_folder("a/./b").is_err());
        assert!(normalize_folder("/absolute").is_err());
        assert!(normalize_folder("C:/drive").is_err());
    }

    #[test]
    fn move_writes_destination_preserves_metadata_and_removes_source() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = seed(root, "rec_move", "page", None);

        let destination = move_record_to_folder(root, &source, "Archive/2026").unwrap();

        assert!(!source.exists());
        assert!(destination.exists());
        assert_eq!(
            destination,
            root.join("Records")
                .join("Archive")
                .join("2026")
                .join(record_filename(&read_record(&destination).unwrap().0))
        );
        let (record, body) = read_record(&destination).unwrap();
        assert_eq!(record.folder.as_deref(), Some("Archive/2026"));
        assert_eq!(
            record.extra.get("custom"),
            Some(&serde_yaml::Value::String("keep".to_string()))
        );
        assert_eq!(body.as_deref(), Some("# Body\n\ntext"));
        assert_eq!(
            find_record_by_id(root, "rec_move"),
            Some(destination.clone())
        );

        let index = rebuild_index(root).unwrap();
        let entry = index
            .records
            .iter()
            .find(|e| e.id == "rec_move")
            .expect("indexed");
        assert_eq!(entry.folder.as_deref(), Some("Archive/2026"));
        assert!(entry.path.replace('\\', "/").contains("Archive/2026"));
    }

    #[test]
    fn move_to_type_folder_clears_redundant_field() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = seed(root, "rec_home", "note", Some("Archive"));

        let destination = move_record_to_folder(root, &source, "note").unwrap();

        assert_eq!(
            destination,
            root.join("Records")
                .join("note")
                .join(record_filename(&read_record(&destination).unwrap().0))
        );
        let (record, _) = read_record(&destination).unwrap();
        assert_eq!(record.folder, None);
    }

    #[test]
    fn move_refuses_to_overwrite_an_existing_file() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let source = seed(root, "rec_dup", "page", None);
        // A second file with the same id already sits at the destination.
        seed(root, "rec_dup", "page", Some("Archive"));

        let error = move_record_to_folder(root, &source, "Archive").unwrap_err();
        assert!(error.to_string().contains("destination already exists"));
        assert!(source.exists());
    }

    #[test]
    fn moving_within_the_same_folder_only_updates_the_field() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        // Hand-written folder spelling with a trailing separator normalises.
        let source = seed(root, "rec_same", "note", Some("note/"));
        let before = fs::read_to_string(&source).unwrap();

        let destination = move_record_to_folder(root, &source, "").unwrap();

        assert_eq!(destination, source);
        assert!(
            !fs::read_to_string(&destination)
                .unwrap()
                .contains("folder:")
        );
        assert_ne!(before, fs::read_to_string(&destination).unwrap());
        let (record, _) = read_record(&destination).unwrap();
        assert_eq!(record.folder, None);
    }
}
