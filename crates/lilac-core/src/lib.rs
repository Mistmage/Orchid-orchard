//! Lilac record storage core.

pub mod block;
pub mod collection;
pub mod error;
pub mod naming;
pub mod record;
pub mod rules;
pub mod storage;
pub mod wal;

pub use block::{
    Block, BlockKind, block_hash, find_block, parse_blocks, reconcile_blocks, reconcile_text,
    splice_block,
};
pub use collection::{CollectionItem, parse_collection_body, render_collection_body};
pub use error::Error;
pub use naming::{NamingConfig, NamingRule};
pub use record::Record;
pub use rules::{Action, Condition, Rule, RuleSet, Value, apply_rules, parse_rules};
pub use storage::{
    Index, IndexEntry, delete_record, read_record, rebuild_index, record_filename, record_path,
    records_dir, search_records, slugify_title, write_record,
};
pub use wal::{Wal, WalEntry};
