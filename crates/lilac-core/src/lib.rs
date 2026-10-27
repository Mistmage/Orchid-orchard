//! Lilac record storage core.

pub mod error;
pub mod record;
pub mod storage;

pub use error::Error;
pub use record::Record;
pub use storage::{
    delete_record, read_record, write_record, rebuild_index, search_records, record_path, records_dir,
    record_filename, slugify_title, Index, IndexEntry,
};
