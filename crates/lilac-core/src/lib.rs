//! Lilac record storage core.

pub mod block;
pub mod block_types;
pub mod collection;
pub mod editor;
pub mod error;
pub mod naming;
pub mod record;
pub mod rules;
pub mod session;
pub mod storage;
pub mod wal;

pub use block::{
    Block, BlockKind, append_block, apply_block_edit, apply_block_edit_at_path, apply_block_edits,
    block_at_path, block_hash, delete_at_path, find_block, insert_after_path, parse_blocks,
    reconcile_blocks, reconcile_text, splice_block, subtree_end, validate_tree,
};
pub use block_types::{
    BLOCK_TYPES, BlockTypeDef, IdPolicy, MetaField, MetaKind, Nesting, Syntax, classify_line,
    describe as describe_block_types, registry as block_type_registry,
    validate_registry as validate_block_registry,
};
pub use collection::{CollectionItem, parse_collection_body, render_collection_body};
pub use editor::BlockEditor;
pub use error::Error;
pub use naming::{NamingConfig, NamingRule};
pub use record::Record;
pub use rules::{Action, Condition, Rule, RuleSet, Value, apply_rules, parse_rules};
pub use session::{OpenReport, RecordSession, fingerprint};
pub use storage::{
    Index, IndexEntry, delete_record, read_record, read_record_with_mtime, rebuild_index,
    record_filename, record_path, records_dir, search_records, slugify_title, write_record,
    write_record_if_unchanged,
};
pub use wal::{Wal, WalEntry};
