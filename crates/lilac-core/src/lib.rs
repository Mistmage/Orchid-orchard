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
pub mod save_plan;
pub mod session;
pub mod storage;
pub mod template;
pub mod wal;
pub mod workspace;

pub use base::{
    BASE_EXTENSION, BASE_TYPE, BaseColumn, BaseFileEntry, BaseRow, BaseSource, BaseSpec, BaseView,
    BaseViewSpec, create_base_file, parse_base, read_base_file, render_base, render_base_file,
    render_base_file_views, render_base_views, save_base_file, scan_base_files,
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
pub use collection::{
    CollectionEntry, CollectionItem, GroupHint, collection_entries, dedupe_entries,
    parse_collection_body, parse_collection_entries, render_collection_body,
    render_collection_entries, set_collection_entries,
};
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
pub use save_plan::{
    CommitReport, FailedDraft, PlanDuplicate, PlanRequest, PlanTab, SaveDraft, SavePlan,
    WrittenDraft, commit_plan, plan_save,
};
pub use session::{OpenReport, RecordSession, fingerprint};
pub use storage::{
    BackfillReport, Index, IndexEntry, backfill_lifecycle, delete_record, find_record_by_id,
    generate_record_id, host_of, move_record_to_folder, normalize_folder, now_timestamp,
    read_record, read_record_with_mtime, rebuild_index, record_filename, record_path, records_dir,
    search_records, slugify_title, touch_opened, write_record, write_record_if_unchanged,
};
pub use template::{
    TEMPLATE_TYPE, TemplateInfo, create_from_template, create_from_template_reported, is_template,
    list_templates, save_as_template, substitute as substitute_template,
};
pub use wal::{Wal, WalEntry};
pub use workspace::{
    DisplayInfo, Placement, Rect, WindowGeometry, describe_display, display_for, place_window,
};
