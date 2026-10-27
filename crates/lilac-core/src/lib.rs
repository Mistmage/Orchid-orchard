//! Lilac record storage core.

pub mod error;
pub mod record;
pub mod storage;

pub use error::Error;
pub use record::Record;
pub use storage::{
    Index, IndexEntry, delete_record, read_record, rebuild_index, record_filename, record_path,
    records_dir, search_records, slugify_title, write_record,
};
