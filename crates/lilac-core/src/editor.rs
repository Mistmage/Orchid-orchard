//! Block-level editor: surgical edits, WAL logging, undo, external reconciliation.
//!
//! Rust is the sole authority: the editor owns the text and block tree; the
//! frontend only sends edits and renders the resulting view.

use crate::block::{Block, apply_block_edit, parse_blocks, reconcile_blocks};
use crate::wal::Wal;

pub struct BlockEditor {
    text: String,
    blocks: Vec<Block>,
    wal: Option<Wal>,
    history: Vec<String>,
    max_history: usize,
}

impl BlockEditor {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let blocks = parse_blocks(&text);
        Self {
            text,
            blocks,
            wal: None,
            history: Vec::new(),
            max_history: 64,
        }
    }

    pub fn with_wal(mut self, wal: Wal) -> Self {
        self.wal = Some(wal);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Apply a surgical edit to a single block. Logs to WAL if present.
    /// Pushes the previous text onto the undo stack.
    pub fn edit_block(&mut self, block_id: &str, new_content: &str) -> Result<(), String> {
        let new_text = apply_block_edit(&self.text, block_id, new_content)?;
        // snapshot for undo
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.text.clone());
        if let Some(wal) = &mut self.wal {
            let _ = wal.append(
                "block_edit",
                serde_json::json!({ "block_id": block_id, "new_content": new_content }),
            );
        }
        self.text = new_text;
        self.blocks = parse_blocks(&self.text);
        Ok(())
    }

    /// Undo the last edit. Returns true if undone.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.text = prev;
            self.blocks = parse_blocks(&self.text);
            true
        } else {
            false
        }
    }

    /// Reconcile after an external edit: reparse + preserve IDs.
    pub fn reconcile_external(&mut self, new_text: &str) {
        // keep for API compatibility; prefer reconcile_external_preserve
        self.text = new_text.to_string();
        self.blocks = parse_blocks(&self.text);
    }

    /// Reconcile preserving ids from a previous snapshot (call before overwriting text).
    pub fn reconcile_external_preserve(&mut self, new_text: &str) {
        let old_blocks = self.blocks.clone();
        let mut new_blocks = parse_blocks(new_text);
        reconcile_blocks(&old_blocks, &mut new_blocks);
        self.text = new_text.to_string();
        self.blocks = new_blocks;
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn edit_block_splices() {
        let mut ed = BlockEditor::new("# Hi\n\npara\n");
        let blocks = ed.blocks().to_vec();
        let id = blocks[0].id.clone().unwrap();
        ed.edit_block(&id, "# Hello\n").unwrap();
        assert_eq!(ed.text(), "# Hello\n\npara\n");
        assert_eq!(ed.history_len(), 1);
    }

    #[test]
    fn undo_restores() {
        let mut ed = BlockEditor::new("# Hi\n\npara\n");
        let id = ed.blocks()[0].id.clone().unwrap();
        ed.edit_block(&id, "# Hello\n").unwrap();
        assert!(ed.undo());
        assert_eq!(ed.text(), "# Hi\n\npara\n");
        assert!(!ed.undo());
    }

    #[test]
    fn edit_missing_block_errors() {
        let mut ed = BlockEditor::new("# Hi\n");
        assert!(ed.edit_block("nope", "x").is_err());
    }

    #[test]
    fn wal_logs_edit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("wal.log");
        let wal = Wal::new(&path);
        let mut ed = BlockEditor::new("# Hi\n").with_wal(wal);
        let id = ed.blocks()[0].id.clone().unwrap();
        ed.edit_block(&id, "# Hello\n").unwrap();
        let entries = Wal::replay(&path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].op, "block_edit");
    }

    #[test]
    fn reconcile_preserves_ids() {
        let mut ed = BlockEditor::new("# Title\n\npara\n");
        let new_text = "# Title\n\npara changed\n";
        ed.reconcile_external_preserve(new_text);
        assert_eq!(ed.text(), new_text);
        assert_eq!(ed.blocks()[0].id.as_deref(), Some("title"));
    }

    #[test]
    fn blocks_accessible() {
        let ed = BlockEditor::new("# Hi\n");
        assert_eq!(ed.blocks().len(), 1);
    }
}
