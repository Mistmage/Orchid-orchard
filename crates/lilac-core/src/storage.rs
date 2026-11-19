use crate::error::Error;
use crate::record::Record;
use std::cmp::Ordering;
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

/// The lifecycle timestamp format: RFC 3339 in UTC. One helper so `created`,
/// `modified` and `opened` cannot drift into different shapes.
pub fn now_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn trimmed_body(body: Option<&str>) -> &str {
    body.map(str::trim).unwrap_or("")
}

/// Compare two records and bodies as *content*: timestamps are deliberately
/// ignored, because a write that only refreshes `modified` is not itself a
/// modification. `opened` is also ignored so opening a record never makes it
/// look edited.
fn same_payload(
    left: &Record,
    left_body: Option<&str>,
    right: &Record,
    right_body: Option<&str>,
) -> bool {
    let mut a = left.clone();
    let mut b = right.clone();
    for record in [&mut a, &mut b] {
        record.created = None;
        record.modified = None;
        record.opened = None;
        record.last_viewed = None;
    }
    a == b && trimmed_body(left_body) == trimmed_body(right_body)
}

fn record_text(record: &Record, body: Option<&str>) -> Result<String, Error> {
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
    Ok(out)
}

/// Write a record, filling lifecycle timestamps on the way through.
///
/// `created` is set once: an existing file's value wins, then an explicit value
/// on the record, then now. `modified` moves only when the content or metadata
/// actually changed, so an unchanged save does not churn the file. A brand-new
/// record keeps an explicitly supplied `modified` (imports, fixtures) and
/// otherwise gets now.
pub fn write_record(path: &Path, record: &Record, body: Option<&str>) -> Result<(), Error> {
    let mut stamped = record.clone();
    let now = now_timestamp();
    let previous = read_record(path).ok();
    match &previous {
        Some((old, old_body)) => {
            stamped.created = old.created.clone().or(stamped.created.clone());
            if same_payload(old, old_body.as_deref(), &stamped, body) {
                stamped.modified = old.modified.clone().or(stamped.modified.clone());
            } else {
                stamped.modified = Some(now);
            }
        }
        None => {
            if stamped.created.is_none() {
                stamped.created = Some(now.clone());
            }
            if stamped.modified.is_none() {
                stamped.modified = Some(now);
            }
        }
    }
    if stamped.created.is_none() {
        stamped.created = Some(now_timestamp());
    }
    write_record_raw(path, &stamped, body)
}

/// Write exactly the record given, with no timestamp policy. Only lifecycle
/// helpers should call this; ordinary saves go through `write_record`.
fn write_record_raw(path: &Path, record: &Record, body: Option<&str>) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = record_text(record, body)?;
    if fs::read_to_string(path).ok().as_deref() == Some(text.as_str()) {
        return Ok(());
    }
    atomic_write(path, text.as_bytes())
}

/// Update only `opened`: the record was opened as a tab from Lilac. This is a
/// separate write because it must not touch `modified`; `write_record` already
/// ignores `opened` when deciding whether content changed.
pub fn touch_opened(path: &Path) -> Result<String, Error> {
    let (mut record, body) = read_record(path)?;
    let now = now_timestamp();
    record.opened = Some(now.clone());
    write_record(path, &record, body.as_deref())?;
    Ok(now)
}

#[derive(Debug, Default, serde::Serialize)]
pub struct BackfillReport {
    pub scanned: usize,
    pub updated: usize,
    pub errors: Vec<String>,
}

fn file_time(value: std::io::Result<std::time::SystemTime>) -> Option<String> {
    value
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}

/// Give records written before lifecycle timestamps existed the values their
/// file already implies: `created` from the filesystem's creation time and
/// `modified` from its modification time. It never deletes or rewrites content,
/// and the second run updates nothing.
pub fn backfill_lifecycle(records_root: &Path) -> Result<BackfillReport, Error> {
    let mut report = BackfillReport::default();
    let records_subdir = records_root.join("Records");
    if !records_subdir.exists() {
        return Ok(report);
    }
    walk_md(&records_subdir, &mut |path| {
        report.scanned += 1;
        let (record, body) = match read_record(path) {
            Ok(found) => found,
            Err(e) => {
                report.errors.push(format!("{}: {}", path.display(), e));
                return;
            }
        };
        if record.created.is_some() && record.modified.is_some() {
            return;
        }
        let meta = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(e) => {
                report.errors.push(format!("{}: {}", path.display(), e));
                return;
            }
        };
        let modified = file_time(meta.modified()).unwrap_or_else(now_timestamp);
        let created = file_time(meta.created()).unwrap_or_else(|| modified.clone());
        let mut stamped = record;
        if stamped.created.is_none() {
            stamped.created = Some(created);
        }
        if stamped.modified.is_none() {
            stamped.modified = Some(modified);
        }
        match write_record_raw(path, &stamped, body.as_deref()) {
            Ok(()) => report.updated += 1,
            Err(e) => report.errors.push(format!("{}: {}", path.display(), e)),
        }
    })?;
    Ok(report)
}

/// Write text atomically, creating parent directories. Used for non-record files
/// such as `.base` flowers, which must never be left half-written.
pub fn write_text(path: &Path, text: &str) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(path, text.as_bytes())
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

pub fn host_of(url: &str) -> Option<&str> {
    let after = url.split("://").nth(1).unwrap_or(url);
    let host = after.split('/').next().unwrap_or(after);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() { None } else { Some(host) }
}

pub fn record_filename(record: &Record) -> String {
    let slug = slugify_title(record.title.as_deref(), record.url.as_deref());
    format!("{}--{}.md", slug, record.id)
}

/// Generate a short id that no record under `Records/` uses yet. Nanoseconds
/// keep ids unique in practice; the scan makes the guarantee, not the clock.
///
/// Only the *low* 28 bits of the timestamp are kept. Taking the leading hex
/// digits instead would freeze the id for tens of seconds at a time, because
/// the high bits of a nanosecond epoch change slowly.
pub fn generate_record_id(records_root: &Path) -> String {
    for _ in 0..10 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let candidate = format!("rec_{:07x}", (nanos as u64) & 0x0fff_ffff);
        if find_record_by_id(records_root, &candidate).is_none() {
            return candidate;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    // The clock is stuck or the ids were taken. Mix in a process-wide counter
    // so repeated fallbacks cannot return the same id.
    use std::sync::atomic::{AtomicU64, Ordering};
    static FALLBACK: AtomicU64 = AtomicU64::new(0);
    let n = FALLBACK.fetch_add(1, Ordering::Relaxed);
    format!("rec_{:x}{:x}", std::process::id(), n)
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    pub modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<String>,
    /// Number of entries in a collection; zero for every other type, so lists
    /// can show the size without reading the file.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub item_count: usize,
    /// Where a saved set came from (`selection`, `group`, `window`). A page
    /// record leaves it empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_from: Option<String>,
    /// Captured tab group name and colour, for list badges. Hints, not truth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_color: Option<String>,
    /// Captured window bounds for a workspace, so lists can describe it without
    /// reading the file. Hints, not handles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<crate::workspace::WindowGeometry>,
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

/// `tab_group` is open metadata, so the index reads it defensively instead of
/// trusting its shape. `tab_groups` is typed and is the fallback for a set
/// captured with several groups.
fn tab_group_hint(record: &Record) -> (Option<String>, Option<String>) {
    let from_value = record.extra.get("tab_group").and_then(|value| match value {
        serde_yaml::Value::Mapping(map) => {
            let read = |key: &str| {
                map.get(serde_yaml::Value::String(key.to_string()))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            };
            let name = read("title").filter(|s| !s.is_empty());
            let color = read("color").filter(|s| !s.is_empty());
            if name.is_some() || color.is_some() {
                Some((name, color))
            } else {
                None
            }
        }
        _ => None,
    });
    from_value.unwrap_or_else(|| {
        record
            .tab_groups
            .first()
            .map(|group| {
                (
                    Some(group.name.clone()).filter(|s| !s.is_empty()),
                    Some(group.color.clone()).filter(|s| !s.is_empty()),
                )
            })
            .unwrap_or((None, None))
    })
}

pub fn rebuild_index(records_root: &Path) -> Result<Index, Error> {
    let mut records = Vec::new();
    let records_subdir = records_root.join("Records");
    if !records_subdir.exists() {
        return Ok(Index { records });
    }
    walk_md(&records_subdir, &mut |path| {
        if let Ok((rec, body)) = read_record(path) {
            let rel = path
                .strip_prefix(records_root)
                .ok()
                .and_then(|p| p.to_str().map(|s| s.to_string()))
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            // A legacy collection has entries only in its body; count them here
            // so list views do not have to read the file to show a size.
            let item_count =
                if rec.r#type.eq_ignore_ascii_case("collection") && rec.items.is_empty() {
                    body.as_deref()
                        .map(|text| crate::collection::parse_collection_body(text).len())
                        .unwrap_or(0)
                } else {
                    rec.items.len()
                };
            let (group_name, group_color) = tab_group_hint(&rec);
            records.push(IndexEntry {
                id: rec.id,
                r#type: rec.r#type,
                title: rec.title,
                tags: rec.tags,
                status: rec.status,
                folder: rec.folder,
                path: rel,
                created: rec.created,
                modified: rec.modified,
                opened: rec.opened,
                item_count,
                saved_from: rec.saved_from,
                group_name,
                group_color,
                geometry: rec.geometry,
            });
        }
    })?;
    records.sort_by(|a, b| {
        // Compare instants, not strings: mixed offsets still sort by real time.
        let left = a.modified.as_deref().and_then(parse_timestamp);
        let right = b.modified.as_deref().and_then(parse_timestamp);
        match (left, right) {
            (Some(left), Some(right)) => right.cmp(&left).then(a.id.cmp(&b.id)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.id.cmp(&b.id),
        }
    });
    Ok(Index { records })
}

fn parse_timestamp(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value).ok()
}

pub(crate) fn walk_md(dir: &Path, cb: &mut dyn FnMut(&Path)) -> Result<(), Error> {
    walk_extension(dir, "md", cb)
}

/// Walk every file with `extension` under `dir`, recursively. Used by the index
/// (`.md`) and by base files (`.base`), so both agree on what "inside Records"
/// means, including hidden directories and Windows separators.
pub(crate) fn walk_extension(
    dir: &Path,
    extension: &str,
    cb: &mut dyn FnMut(&Path),
) -> Result<(), Error> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_extension(&path, extension, cb)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some(extension) {
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
    fn new_records_are_stamped_and_opened_does_not_touch_modified() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("record.md");
        let record = Record::new("rec_life", "note");
        write_record(&path, &record, Some("body")).unwrap();
        let (first, _) = read_record(&path).unwrap();
        let created = first.created.clone().expect("created is automatic");
        let modified = first.modified.clone().expect("modified is automatic");
        assert_eq!(created, modified);

        let before = std::fs::read_to_string(&path).unwrap();
        write_record(&path, &first, Some("body")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);

        let opened = touch_opened(&path).unwrap();
        let (second, _) = read_record(&path).unwrap();
        assert_eq!(second.created, first.created);
        assert_eq!(second.modified, first.modified);
        assert_eq!(second.opened.as_deref(), Some(opened.as_str()));

        let mut edited = second.clone();
        edited.title = Some("Changed".to_string());
        write_record(&path, &edited, Some("body")).unwrap();
        let (third, _) = read_record(&path).unwrap();
        assert_eq!(third.created, first.created);
        assert_ne!(third.modified, first.modified);
        assert_eq!(third.opened, second.opened);
    }

    #[test]
    fn explicit_new_timestamps_are_preserved() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("imported.md");
        let mut record = Record::new("rec_import", "note");
        record.created = Some("2026-01-01T00:00:00Z".to_string());
        record.modified = Some("2026-01-02T00:00:00Z".to_string());
        write_record(&path, &record, None).unwrap();
        let (read, _) = read_record(&path).unwrap();
        assert_eq!(read.created.as_deref(), Some("2026-01-01T00:00:00Z"));
        assert_eq!(read.modified.as_deref(), Some("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn backfill_gives_old_files_timestamps_once() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let target = root.join("Records/note/legacy.md");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(
            &target,
            "---\nid: rec_legacy\ntype: note\ntitle: Legacy\ncustom: keep\n---\n\nbody\n",
        )
        .unwrap();

        let report = backfill_lifecycle(root).unwrap();
        assert_eq!(report.scanned, 1);
        assert_eq!(report.updated, 1);
        assert!(report.errors.is_empty());
        let (record, body) = read_record(&target).unwrap();
        assert!(record.created.is_some());
        assert!(record.modified.is_some());
        assert_eq!(body.as_deref(), Some("body"));
        assert_eq!(
            record.extra.get("custom"),
            Some(&serde_yaml::Value::String("keep".to_string()))
        );

        let second = backfill_lifecycle(root).unwrap();
        assert_eq!(second.scanned, 1);
        assert_eq!(second.updated, 0);
    }

    #[test]
    fn legacy_collection_body_is_counted_in_the_index() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let path = root.join("Records/collection/legacy.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "---\nid: rec_collegacy\ntype: collection\ntitle: Old set\n---\n\n- [[rec_a]]\n- https://example.com\n",
        )
        .unwrap();
        let index = rebuild_index(root).unwrap();
        let entry = index
            .records
            .iter()
            .find(|e| e.id == "rec_collegacy")
            .expect("indexed");
        assert_eq!(entry.item_count, 2);
    }

    #[test]
    fn index_sorts_by_instant_across_offsets() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut early = Record::new("rec_early", "note");
        early.modified = Some("2024-01-01T10:00:00+02:00".to_string());
        let mut late = Record::new("rec_late", "note");
        late.modified = Some("2024-01-01T09:00:00Z".to_string());
        write_record(&record_path(root, &early), &early, None).unwrap();
        write_record(&record_path(root, &late), &late, None).unwrap();

        let index = rebuild_index(root).unwrap();
        let ids: Vec<&str> = index.records.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["rec_late", "rec_early"]);
    }

    #[test]
    fn index_carries_lifecycle_timestamps() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let path = seed(root, "rec_index", "note", None);
        let opened = touch_opened(&path).unwrap();
        let index = rebuild_index(root).unwrap();
        let entry = index
            .records
            .iter()
            .find(|e| e.id == "rec_index")
            .expect("indexed");
        assert!(entry.created.is_some());
        assert!(entry.modified.is_some());
        assert_eq!(entry.opened.as_deref(), Some(opened.as_str()));
    }

    #[test]
    fn generated_ids_are_short_and_unique() {
        let dir = TempDir::new().unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            let id = generate_record_id(dir.path());
            assert!(id.starts_with("rec_"), "unexpected id: {id}");
            assert!(seen.insert(id.clone()), "duplicate id: {id}");
        }
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
