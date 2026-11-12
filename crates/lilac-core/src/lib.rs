//! Lilac record storage core.

pub mod base;
pub mod block;
pub mod block_types;
pub mod collection;
pub mod editor;
pub mod error;
pub mod formula;
pub mod naming;
pub mod record;
pub mod rules;
pub mod session;
pub mod storage;
pub mod template;
pub mod wal;

pub use base::{
    BASE_TYPE, BaseColumn, BaseRow, BaseSource, BaseSpec, BaseView, parse_base, render_base,
    starter_document as base_starter_document,
};
pub use block::{
    Block, BlockKind, append_block, apply_block_edit, apply_block_edit_at_path, apply_block_edits,
    block_at_path, block_hash, delete_at_path, find_block, insert_after_path, parse_blocks,
    reconcile_blocks, reconcile_text, splice_block, subtree_end, validate_tree,
};
pub use block_types::{
    BLOCK_TYPES, BlockTypeDef, IdPolicy, MetaField, MetaKind, MetaValue, Nesting, Syntax,
    classify_line, describe as describe_block_types, registry as block_type_registry,
    validate_registry as validate_block_registry,
};
pub use collection::{CollectionItem, parse_collection_body, render_collection_body};
pub use editor::BlockEditor;
pub use error::Error;
pub use formula::{
    BinOp, Expr, UnOp, Value as FormulaValue, eval_formula, evaluate as evaluate_formula,
    parse_formula,
};
pub use naming::{NamingConfig, NamingRule};
pub use record::Record;
pub use rules::{
    Action, Condition, Rule, RulePreview, RuleSet, Value, apply_rules, apply_rules_reported,
    parse_rules, preview_rules,
};
pub use session::{OpenReport, RecordSession, fingerprint};
pub use storage::{
    Index, IndexEntry, delete_record, find_record_by_id, generate_record_id, host_of,
    move_record_to_folder, normalize_folder, read_record, read_record_with_mtime, rebuild_index,
    record_filename, record_path, records_dir, search_records, slugify_title, write_record,
    write_record_if_unchanged,
};
pub use template::{
    TEMPLATE_TYPE, TemplateInfo, create_from_template, create_from_template_reported, is_template,
    list_templates, save_as_template, substitute as substitute_template,
};
pub use wal::{Wal, WalEntry};
