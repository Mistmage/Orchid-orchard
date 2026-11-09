//! Record editing sessions: surgical block edits, WAL recovery, external-edit
//! reconciliation and optimistic writes.
//!
//! The frontend addresses blocks by position path (`[0, 2]`), not by id: every
//! block, including paragraphs and code, is editable without inventing ids.
//! Ids stay for stable references (headings/frontmatter).
//!
//! A session owns the record text while the editor is open. Before each write
//! it verifies that the file on disk is still the version it started from. If
//! another editor changed the file, the new text is reconciled first (ids are
//! preserved where content matches) and the caller is told that an external
//! change was incorporated.

use std::fs;
use std::path::{Path, PathBuf};

use crate::block::{
    Block, append_block, apply_block_edit_at_path, delete_at_path, insert_after_path, parse_blocks,
    reconcile_blocks,
};
use crate::error::Error;
use crate::record::Record;
use crate::storage::{read_record, write_record};
use crate::wal::Wal;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OpenReport {
    /// Number of WAL edits replayed on open.
    pub recovered_edits: usize,
    /// Set when a WAL belonged to a different file version and was moved aside.
    pub orphaned_wal: Option<PathBuf>,
    /// The file changed since the session last saw it.
    pub external_changed: bool,
}

pub struct RecordSession {
    path: PathBuf,
    record: Record,
    body: String,
    blocks: Vec<Block>,
    wal: Wal,
    base_fingerprint: u64,
    history: Vec<String>,
    max_history: usize,
    recovered_edits: usize,
    orphaned_wal: Option<PathBuf>,
}

impl RecordSession {
    pub fn open(path: impl Into<PathBuf>, wal: Wal) -> Result<(Self, OpenReport), Error> {
        let path = path.into();
        let raw = fs::read_to_string(&path)?;
        let (record, body) = read_record(&path)?;
        let body = body.unwrap_or_default();
        let mut session = Self {
            path,
            record,
            body,
            blocks: Vec::new(),
            wal,
            base_fingerprint: fingerprint(raw.as_bytes()),
            history: Vec::new(),
            max_history: 64,
            recovered_edits: 0,
            orphaned_wal: None,
        };
        session.blocks = parse_blocks(&session.body);
        session.recover()?;
        let report = OpenReport {
            recovered_edits: session.recovered_edits,
            orphaned_wal: session.orphaned_wal.clone(),
            external_changed: false,
        };
        Ok((session, report))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn body(&self) -> &str {
        &self.body
    }
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
    pub fn record(&self) -> &Record {
        &self.record
    }
    pub fn recovered_edits(&self) -> usize {
        self.recovered_edits
    }

    /// Replay WAL edits when they belong to the current file version.
    /// A WAL from another version is never applied: it is moved aside so the
    /// user can inspect it, and the file is left untouched.
    fn recover(&mut self) -> Result<(), Error> {
        let entries = Wal::replay(&self.wal.path)?;
        if entries.is_empty() {
            return Ok(());
        }
        let first_base = entries
            .iter()
            .find(|e| e.op == "begin")
            .and_then(|e| e.payload.get("base"))
            .and_then(|v| v.as_u64());
        if first_base != Some(self.base_fingerprint) {
            let orphan = self
                .wal
                .path
                .with_extension(format!("orphan-{}.log", timestamp()));
            let _ = fs::rename(&self.wal.path, &orphan);
            self.orphaned_wal = Some(orphan);
            let _ = self.wal.truncate();
            return Ok(());
        }

        let original = self.body.clone();
        let mut recovered = 0usize;
        for entry in &entries {
            let content = entry
                .payload
                .get("new_content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let next = match entry.op.as_str() {
                "edit" => match path_from_payload(&entry.payload) {
                    Some(path) if !path.is_empty() => {
                        apply_block_edit_at_path(&self.body, &path, content)
                    }
                    _ => Err("edit without path".to_string()),
                },
                "insert" => match path_from_payload(&entry.payload) {
                    Some(path) if !path.is_empty() => insert_after_path(&self.body, &path, content),
                    _ => append_block(&self.body, content),
                },
                "delete" => match path_from_payload(&entry.payload) {
                    Some(path) if !path.is_empty() => delete_at_path(&self.body, &path),
                    _ => Err("delete without path".to_string()),
                },
                _ => continue,
            };
            match next {
                Ok(text) => {
                    self.body = text;
                    recovered += 1;
                }
                Err(_) => {
                    self.body = original;
                    recovered = 0;
                    break;
                }
            }
        }

        if recovered > 0 {
            self.record.modified = Some(chrono::Utc::now().to_rfc3339());
            write_record(&self.path, &self.record, Some(&self.body))?;
            self.base_fingerprint = fingerprint(fs::read(&self.path)?.as_slice());
            self.blocks = parse_blocks(&self.body);
            self.recovered_edits = recovered;
        }
        let _ = self.wal.truncate();
        Ok(())
    }

    /// Detect a file changed by another process. Reconciles ids when possible.
    pub fn refresh_external(&mut self) -> Result<bool, Error> {
        let current = self.current_fingerprint()?;
        if current == self.base_fingerprint {
            return Ok(false);
        }
        let (record, body) = read_record(&self.path)?;
        let body = body.unwrap_or_default();
        let old_blocks = self.blocks.clone();
        let mut new_blocks = parse_blocks(&body);
        reconcile_blocks(&old_blocks, &mut new_blocks);
        self.record = record;
        self.body = body;
        self.blocks = new_blocks;
        self.base_fingerprint = current;
        self.history.clear();
        Ok(true)
    }

    /// Apply a surgical edit addressed by position path. WAL is written before
    /// the file, then truncated after a successful write, so a crash replays
    /// the edit on the next open.
    pub fn edit_at_path(&mut self, path: &[usize], new_content: &str) -> Result<bool, Error> {
        let external_changed = self.refresh_external()?;
        let before = self.body.clone();
        let new_body =
            apply_block_edit_at_path(&self.body, path, new_content).map_err(Error::Invalid)?;

        self.check_disk_unchanged()?;
        self.wal.append(
            "begin",
            serde_json::json!({ "base": self.base_fingerprint, "path": path }),
        )?;
        self.wal.append(
            "edit",
            serde_json::json!({ "path": path, "new_content": new_content }),
        )?;
        self.commit(before, new_body, external_changed)
    }

    /// Insert Markdown after the block at `after`, or at the end when `after` is
    /// `None`. A section inserts after its whole subtree, so the new block is a
    /// sibling of the section rather than a new child.
    pub fn insert_block(&mut self, after: Option<&[usize]>, content: &str) -> Result<bool, Error> {
        let external_changed = self.refresh_external()?;
        let before = self.body.clone();
        let new_body = match after {
            Some(path) if !path.is_empty() => {
                insert_after_path(&self.body, path, content).map_err(Error::Invalid)?
            }
            _ => append_block(&self.body, content).map_err(Error::Invalid)?,
        };

        self.check_disk_unchanged()?;
        self.wal.append(
            "begin",
            serde_json::json!({ "base": self.base_fingerprint, "path": after }),
        )?;
        self.wal.append(
            "insert",
            serde_json::json!({ "path": after, "new_content": content }),
        )?;
        self.commit(before, new_body, external_changed)
    }

    /// Delete the block at `path` and any children it owns.
    pub fn delete_block(&mut self, path: &[usize]) -> Result<bool, Error> {
        let external_changed = self.refresh_external()?;
        let before = self.body.clone();
        let new_body = delete_at_path(&self.body, path).map_err(Error::Invalid)?;

        self.check_disk_unchanged()?;
        self.wal.append(
            "begin",
            serde_json::json!({ "base": self.base_fingerprint, "path": path }),
        )?;
        self.wal
            .append("delete", serde_json::json!({ "path": path }))?;
        self.commit(before, new_body, external_changed)
    }

    fn check_disk_unchanged(&self) -> Result<(), Error> {
        if self.current_fingerprint()? != self.base_fingerprint {
            return Err(Error::Stale(self.path.to_string_lossy().to_string()));
        }
        Ok(())
    }

    /// Write a body that has already been prepared and journalled. Restores the
    /// previous body if the write fails, so the session never claims an edit
    /// that is not on disk.
    fn commit(
        &mut self,
        before: String,
        new_body: String,
        external_changed: bool,
    ) -> Result<bool, Error> {
        self.body = new_body;
        self.blocks = parse_blocks(&self.body);
        self.record.modified = Some(chrono::Utc::now().to_rfc3339());
        if let Err(e) = write_record(&self.path, &self.record, Some(&self.body)) {
            self.body = before;
            self.blocks = parse_blocks(&self.body);
            let _ = self.wal.truncate();
            return Err(e);
        }
        self.base_fingerprint = self.current_fingerprint()?;
        let _ = self.wal.truncate();
        self.push_history(before);
        Ok(external_changed)
    }

    pub fn undo(&mut self) -> Result<Option<String>, Error> {
        let Some(previous) = self.history.pop() else {
            return Ok(None);
        };
        if self.current_fingerprint()? != self.base_fingerprint {
            return Err(Error::Stale(self.path.to_string_lossy().to_string()));
        }
        self.body = previous;
        self.blocks = parse_blocks(&self.body);
        self.record.modified = Some(chrono::Utc::now().to_rfc3339());
        write_record(&self.path, &self.record, Some(&self.body))?;
        self.base_fingerprint = self.current_fingerprint()?;
        Ok(Some(self.body.clone()))
    }

    fn current_fingerprint(&self) -> Result<u64, Error> {
        Ok(fingerprint(fs::read(&self.path)?.as_slice()))
    }

    fn push_history(&mut self, text: String) {
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(text);
    }
}

fn path_from_payload(payload: &serde_json::Value) -> Option<Vec<usize>> {
    payload
        .get("path")?
        .as_array()?
        .iter()
        .map(|v| v.as_u64().map(|n| n as usize))
        .collect()
}

fn timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Stable, dependency-free content fingerprint (FNV-1a 64).
pub fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::write_record;
    use tempfile::TempDir;

    fn record_file(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("note--rec_1.md");
        let mut record = Record::new("rec_1", "note");
        record.title = Some("Test".into());
        write_record(&path, &record, Some("# Hi\n\npara\n")).unwrap();
        path
    }

    #[test]
    fn edit_persists_and_truncates_wal() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (mut session, report) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        assert_eq!(report.recovered_edits, 0);
        session.edit_at_path(&[0], "# Hello\n").unwrap();
        assert!(session.body().contains("# Hello"));
        assert!(Wal::replay(&wal_path).unwrap().is_empty());
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("# Hello"));
    }

    #[test]
    fn external_change_is_reconciled() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (mut session, _) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        // Another process rewrites the file.
        let mut record = Record::new("rec_1", "note");
        record.title = Some("Test".into());
        write_record(&path, &record, Some("# Changed outside\n\npara\n")).unwrap();
        session.edit_at_path(&[0, 0], "edited\n").unwrap();
        assert!(session.body().contains("# Changed outside"));
        assert!(session.body().contains("edited"));
    }

    #[test]
    fn wal_is_replayed_after_crash() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (session, _) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        let base = session.base_fingerprint;
        let mut wal = Wal::new(&wal_path);
        wal.append("begin", serde_json::json!({ "base": base, "path": [0] }))
            .unwrap();
        wal.append(
            "edit",
            serde_json::json!({ "path": [0], "new_content": "# Recovered\n" }),
        )
        .unwrap();
        drop(session);

        let (session, report) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        assert_eq!(report.recovered_edits, 1);
        assert!(session.body().contains("# Recovered"));
        assert!(fs::read_to_string(&path).unwrap().contains("# Recovered"));
    }

    #[test]
    fn orphaned_wal_is_moved_aside() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let mut wal = Wal::new(&wal_path);
        wal.append("begin", serde_json::json!({ "base": 12345, "path": [0] }))
            .unwrap();
        wal.append(
            "edit",
            serde_json::json!({ "path": [0], "new_content": "# Nope\n" }),
        )
        .unwrap();
        let (session, report) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        assert_eq!(report.recovered_edits, 0);
        assert!(report.orphaned_wal.is_some());
        assert!(!session.body().contains("# Nope"));
    }

    #[test]
    fn undo_restores_previous_body() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (mut session, _) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        let original = session.body().to_string();
        session.edit_at_path(&[0], "# Changed\n").unwrap();
        assert!(session.undo().unwrap().is_some());
        assert_eq!(session.body(), original);
    }

    #[test]
    fn insert_and_delete_persist_and_undo() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (mut session, _) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        let original = session.body().to_string();

        session.insert_block(None, "appended").unwrap();
        assert!(session.body().contains("appended"));
        assert!(fs::read_to_string(&path).unwrap().contains("appended"));

        let blocks = session.blocks().to_vec();
        let appended = blocks[0]
            .children
            .iter()
            .position(|b| b.content.contains("appended"))
            .expect("inserted block");
        session.delete_block(&[0, appended]).unwrap();
        assert!(!session.body().contains("appended"));

        session.undo().unwrap();
        assert!(session.body().contains("appended"));
        session.undo().unwrap();
        assert_eq!(session.body(), original);
    }

    #[test]
    fn wal_replays_insert_and_delete() {
        let dir = TempDir::new().unwrap();
        let path = record_file(&dir);
        let wal_path = dir.path().join("wal.log");
        let (session, _) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        let base = session.base_fingerprint;
        drop(session);

        let mut wal = Wal::new(&wal_path);
        wal.append("begin", serde_json::json!({ "base": base, "path": null }))
            .unwrap();
        wal.append(
            "insert",
            serde_json::json!({ "path": null, "new_content": "recovered" }),
        )
        .unwrap();
        drop(wal);

        let (session, report) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        assert_eq!(report.recovered_edits, 1);
        assert!(session.body().contains("recovered"));
        assert!(fs::read_to_string(&path).unwrap().contains("recovered"));

        // Now a delete journal.
        let base = session.base_fingerprint;
        drop(session);
        let mut wal = Wal::new(&wal_path);
        wal.append("begin", serde_json::json!({ "base": base, "path": [0] }))
            .unwrap();
        wal.append("delete", serde_json::json!({ "path": [0] }))
            .unwrap();
        drop(wal);

        let (session, report) = RecordSession::open(&path, Wal::new(&wal_path)).unwrap();
        assert_eq!(report.recovered_edits, 1);
        assert!(!session.body().contains("# Hi"));
        assert!(!fs::read_to_string(&path).unwrap().contains("# Hi"));
    }
}
