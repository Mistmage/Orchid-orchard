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
    let (yaml, body) = split_frontmatter(&contents)?;
    let record: Record = serde_yaml::from_str(yaml)?;
    let body = body.map(|b| b.trim_end_matches('\n').to_string()).filter(|b| !b.is_empty());
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
        path.file_name().and_then(|s| s.to_str()).unwrap_or("record")
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
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

pub fn record_filename(record: &Record) -> String {
    let slug = slugify_title(record.title.as_deref(), record.url.as_deref());
    format!("{}--{}.md", slug, record.id)
}

pub fn records_dir(records_root: &Path, rtype: &str) -> PathBuf {
    records_root.join("Records").join(rtype)
}

pub fn record_path(records_root: &Path, record: &Record) -> PathBuf {
    records_dir(records_root, &record.r#type).join(record_filename(record))
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Index {
    pub records: Vec<IndexEntry>,
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub id: String,
    pub r#type: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
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
                        _ => true,
                    }
                } else {
                    let needle = t.to_lowercase();
                    e.title.as_deref().unwrap_or("").to_lowercase().contains(&needle)
                        || e.tags.iter().any(|x| x.to_lowercase().contains(&needle))
                        || e.id.to_lowercase().contains(&needle)
                }
            })
        })
        .cloned()
        .collect()
}
