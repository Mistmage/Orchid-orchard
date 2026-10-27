//! Write-ahead log (WAL) foundation for crash-safe, incremental writes.
//!
//! Entries are appended as JSON lines. Replay returns them in order.
//! Truncate clears the log after a successful compaction.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WalEntry {
    pub seq: u64,
    pub op: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Default)]
pub struct Wal {
    pub path: std::path::PathBuf,
    pub next_seq: u64,
}

impl Wal {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let next_seq = Self::replay(&path).map(|e| e.len() as u64 + 1).unwrap_or(1);
        Self { path, next_seq }
    }

    pub fn append(&mut self, op: &str, payload: serde_json::Value) -> Result<u64, std::io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let seq = self.next_seq;
        let entry = WalEntry {
            seq,
            op: op.to_string(),
            payload,
        };
        let line = serde_json::to_string(&entry).map_err(std::io::Error::other)?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{}", line)?;
        f.sync_all()?;
        self.next_seq += 1;
        Ok(seq)
    }

    pub fn replay(path: &Path) -> Result<Vec<WalEntry>, std::io::Error> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let f = fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(e) = serde_json::from_str::<WalEntry>(&line) {
                out.push(e);
            }
        }
        Ok(out)
    }

    pub fn truncate(&self) -> Result<(), std::io::Error> {
        if self.path.exists() {
            fs::write(&self.path, "")?;
        }
        Ok(())
    }

    pub fn entries(&self) -> Vec<WalEntry> {
        Self::replay(&self.path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn append_and_replay() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.log");
        let mut wal = Wal::new(&path);
        let s1 = wal.append("edit", serde_json::json!({"id":"a"})).unwrap();
        let s2 = wal.append("delete", serde_json::json!({"id":"b"})).unwrap();
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        let entries = Wal::replay(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].op, "edit");
        assert_eq!(entries[1].op, "delete");
    }

    #[test]
    fn truncate_clears() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.log");
        let mut wal = Wal::new(&path);
        wal.append("edit", serde_json::json!({})).unwrap();
        wal.truncate().unwrap();
        assert!(Wal::replay(&path).unwrap().is_empty());
    }

    #[test]
    fn empty_replay_ok() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nope.log");
        assert!(Wal::replay(&path).unwrap().is_empty());
    }

    #[test]
    fn survives_reopen() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.log");
        {
            let mut wal = Wal::new(&path);
            wal.append("x", serde_json::json!({})).unwrap();
        }
        let wal = Wal::new(&path);
        assert_eq!(wal.next_seq, 2);
    }
}
