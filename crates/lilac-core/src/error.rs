use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("record not found: {0}")]
    NotFound(String),

    #[error("invalid record: {0}")]
    Invalid(String),

    #[error("frontmatter missing or malformed")]
    NoFrontmatter,

    #[error("file contains conflict markers: {0}")]
    Conflict(String),
}
