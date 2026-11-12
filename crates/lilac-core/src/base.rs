//! Base records: a declarative query over other records.
//!
//! A base is the first *flower*: a terminal view with its own rendering
//! authority. It has two file forms. A `type: base` record lives under
//! `Records/`, is indexed, is backed up like everything else, and can carry
//! record metadata. A standalone `.base` file is the same YAML query without
//! frontmatter, so it can be dropped in from any editor. Both use the same
//! parser and renderer; neither needs a separate database table. The body is
//! the query document:
//!
//! ```yaml
//! source:
//!   type: book
//!   tag: reading
//! where: status == "Reading"
//! columns:
//!   - label: Title
//!     expr: title
//!   - status
//!   - label: Pages left
//!     expr: pages_total - pages_read
//! sort: "-modified"
//! limit: 100
//! ```
//!
//! Only the YAML body is the base; frontmatter stays ordinary record metadata.
//! The parser validates every expression once, and `render_base` evaluates it
//! against the current index. Evaluation is pure and shared with rules
//! (`crate::formula`), so a base cannot execute code, touch the network or
//! write files.
//!
//! Source filters use the index first (type, tag, folder, path), then read only
//! the candidates so formula fields that the index does not store (`url`,
//! `extra`) still work.

use crate::error::Error;
use crate::formula::{Expr, Value as FormulaValue, eval_formula, parse_formula};
use crate::storage::{
    generate_record_id, read_record, rebuild_index, slugify_title, walk_extension, write_text,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

pub const BASE_TYPE: &str = "base";

/// Extension of a standalone base flower: a YAML query without record
/// frontmatter. Both `file.base` and `type: base` records are supported so a
/// user can choose between a view with record metadata and a plain query file.
pub const BASE_EXTENSION: &str = "base";

/// Which records a base looks at. All filters combine with AND; an absent
/// filter matches everything.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BaseSource {
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Folder under `Records/`; matches the folder itself and its subfolders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Case-insensitive substring of the path stored by the index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseColumn {
    pub label: String,
    pub expr: String,
    #[serde(skip)]
    pub parsed: Option<Expr>,
}

/// One column as written in YAML: a bare expression or a labelled one.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawColumn {
    Simple(String),
    Detailed {
        #[serde(default)]
        label: Option<String>,
        expr: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseSpec {
    #[serde(default)]
    pub source: BaseSource,
    /// Optional formula; a record is shown only when it is truthy.
    #[serde(default, rename = "where", skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(default, deserialize_with = "deserialize_columns")]
    pub columns: Vec<BaseColumn>,
    /// Expression to sort by. A leading `-` sorts descending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    // Parsed forms are not part of the document.
    #[serde(skip)]
    filter_expr: Option<Expr>,
    #[serde(skip)]
    sort_expr: Option<Expr>,
    #[serde(skip)]
    sort_descending: bool,
}

impl Default for BaseSpec {
    fn default() -> Self {
        Self {
            source: BaseSource::default(),
            filter: None,
            columns: default_columns(),
            sort: None,
            limit: None,
            filter_expr: None,
            sort_expr: None,
            sort_descending: false,
        }
    }
}

fn default_columns() -> Vec<BaseColumn> {
    ["file.name", "status", "file.folder"]
        .into_iter()
        .map(|expr| BaseColumn {
            label: expr.to_string(),
            expr: expr.to_string(),
            parsed: None,
        })
        .collect()
}

fn deserialize_columns<'de, D>(deserializer: D) -> Result<Vec<BaseColumn>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Vec::<RawColumn>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .map(|c| match c {
            RawColumn::Simple(expr) => BaseColumn {
                label: expr.clone(),
                expr,
                parsed: None,
            },
            RawColumn::Detailed { label, expr } => BaseColumn {
                label: label.unwrap_or_else(|| expr.clone()),
                expr,
                parsed: None,
            },
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BaseRow {
    pub id: String,
    pub title: Option<String>,
    pub path: String,
    /// One display string per column, in declaration order.
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BaseView {
    pub labels: Vec<String>,
    pub rows: Vec<BaseRow>,
    /// Records that passed the source filter and `where`, before `limit`.
    pub matched: usize,
    /// Records considered by the source filter.
    pub scanned: usize,
    /// Non-fatal problems: unreadable files, a failed `where` or cell.
    pub warnings: Vec<String>,
}

/// Parse and validate a base document. An empty document is a valid base that
/// lists every record with the default columns.
pub fn parse_base(text: &str) -> Result<BaseSpec, String> {
    let trimmed = text.trim();
    let mut spec: BaseSpec = if trimmed.is_empty() {
        BaseSpec::default()
    } else {
        // `serde_yaml` reports the line; prefixing keeps the UI message useful
        // without pretending it is a Rust panic.
        serde_yaml::from_str(trimmed).map_err(|e| format!("base YAML: {}", e))?
    };
    if spec.columns.is_empty() {
        spec.columns = default_columns();
    }
    for column in &mut spec.columns {
        let expr =
            parse_formula(&column.expr).map_err(|e| format!("column {:?}: {}", column.label, e))?;
        column.parsed = Some(expr);
    }
    if let Some(filter) = spec.filter.clone() {
        spec.filter_expr = Some(parse_formula(&filter).map_err(|e| format!("where: {}", e))?);
    }
    if let Some(raw) = spec.sort.clone() {
        let (text, descending) = match raw.trim().strip_prefix('-') {
            Some(rest) => (rest.trim().to_string(), true),
            None => (raw.trim().to_string(), false),
        };
        if text.is_empty() {
            return Err("sort: expected an expression".into());
        }
        spec.sort_expr = Some(parse_formula(&text).map_err(|e| format!("sort: {}", e))?);
        spec.sort_descending = descending;
    }
    if spec.limit == Some(0) {
        return Err("limit must be greater than zero".into());
    }
    Ok(spec)
}

/// Render a base against the files currently under `records_root`.
pub fn render_base(records_root: &Path, spec: &BaseSpec) -> Result<BaseView, Error> {
    let index = rebuild_index(records_root)?;
    let mut scanned = 0usize;
    let mut warnings = Vec::new();
    let mut rows: Vec<(BaseRow, Option<FormulaValue>)> = Vec::new();

    for entry in &index.records {
        if !source_matches(&spec.source, entry) {
            continue;
        }
        scanned += 1;
        let path = records_root.join(&entry.path);
        let record = match read_record(&path) {
            Ok((record, _body)) => record,
            Err(e) => {
                warnings.push(format!("{}: {}", entry.path, e));
                continue;
            }
        };
        if let Some(filter) = &spec.filter_expr {
            match eval_formula(filter, &record) {
                Ok(value) if value.truthy() => {}
                Ok(_) => continue,
                Err(e) => {
                    warnings.push(format!("{}: where: {}", entry.id, e));
                    continue;
                }
            }
        }
        let mut cells = Vec::with_capacity(spec.columns.len());
        for column in &spec.columns {
            let expr = column
                .parsed
                .as_ref()
                .expect("parse_base stores parsed expressions");
            match eval_formula(expr, &record) {
                Ok(value) => cells.push(value.display()),
                Err(e) => {
                    warnings.push(format!("{}: column {:?}: {}", entry.id, column.label, e));
                    cells.push(String::new());
                }
            }
        }
        let sort_value = match &spec.sort_expr {
            Some(expr) => match eval_formula(expr, &record) {
                Ok(value) => Some(value),
                Err(e) => {
                    warnings.push(format!("{}: sort: {}", entry.id, e));
                    None
                }
            },
            None => None,
        };
        rows.push((
            BaseRow {
                id: record.id.clone(),
                title: record.title.clone(),
                path: entry.path.clone(),
                cells,
            },
            sort_value,
        ));
    }

    if spec.sort_expr.is_some() {
        rows.sort_by(|a, b| {
            let ord = compare_values(a.1.as_ref(), b.1.as_ref());
            if spec.sort_descending {
                ord.reverse()
            } else {
                ord
            }
        });
    }
    let matched = rows.len();
    if let Some(limit) = spec.limit {
        rows.truncate(limit);
    }

    Ok(BaseView {
        labels: spec.columns.iter().map(|c| c.label.clone()).collect(),
        rows: rows.into_iter().map(|(row, _)| row).collect(),
        matched,
        scanned,
        warnings,
    })
}

fn source_matches(source: &BaseSource, entry: &crate::storage::IndexEntry) -> bool {
    if let Some(want) = &source.r#type
        && !entry.r#type.eq_ignore_ascii_case(want.trim())
    {
        return false;
    }
    if let Some(tag) = &source.tag {
        let tag = tag.trim();
        if !entry.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            return false;
        }
    }
    if let Some(folder) = &source.folder {
        let folder = folder.trim().trim_matches('/');
        let actual = entry.folder.as_deref().unwrap_or("");
        if !(actual.eq_ignore_ascii_case(folder)
            || actual
                .to_ascii_lowercase()
                .starts_with(&format!("{}/", folder.to_ascii_lowercase())))
        {
            return false;
        }
    }
    if let Some(path) = &source.path
        && !entry
            .path
            .to_ascii_lowercase()
            .contains(&path.trim().to_ascii_lowercase())
    {
        return false;
    }
    true
}

/// Deterministic ordering across formula types. Nulls sort last, which keeps
/// an unfinished record at the bottom instead of silently first.
fn compare_values(a: Option<&FormulaValue>, b: Option<&FormulaValue>) -> Ordering {
    fn rank(v: Option<&FormulaValue>) -> u8 {
        match v {
            None | Some(FormulaValue::Null) => 4,
            Some(FormulaValue::Bool(_)) => 0,
            Some(FormulaValue::Num(_)) => 1,
            Some(FormulaValue::Str(_)) => 2,
            Some(FormulaValue::List(_)) => 3,
        }
    }
    match (a, b) {
        (Some(FormulaValue::Bool(x)), Some(FormulaValue::Bool(y))) => x.cmp(y),
        (Some(FormulaValue::Num(x)), Some(FormulaValue::Num(y))) => {
            x.partial_cmp(y).unwrap_or(Ordering::Equal)
        }
        (Some(FormulaValue::Str(x)), Some(FormulaValue::Str(y))) => x.cmp(y),
        (Some(FormulaValue::List(x)), Some(FormulaValue::List(y))) => {
            let xs = x.iter().map(|v| v.display()).collect::<Vec<_>>().join(", ");
            let ys = y.iter().map(|v| v.display()).collect::<Vec<_>>().join(", ");
            xs.cmp(&ys)
        }
        _ => rank(a).cmp(&rank(b)),
    }
}

/// A starter document for a newly created base.
pub fn starter_document(title: &str) -> String {
    let safe = title.replace('"', "");
    format!(
        "# {}\n\
         # This is a base: a saved query over your records.\n\
         # Edit the YAML below; Orchard renders it as a table.\n\
         source:\n  type: note\n\
         # where: status == \"Reading\"\n\
         columns:\n  - label: Title\n    expr: file.name\n  - status\n  - tags\n\
         sort: \"-modified\"\n\
         limit: 100\n",
        if safe.trim().is_empty() {
            "New base"
        } else {
            safe.trim()
        }
    )
}

// ---------------------------------------------------------------------------
// File-backed bases (`.base`)
// ---------------------------------------------------------------------------

/// A standalone `.base` flower on disk. It has no record frontmatter; `id` and
/// `title` are optional YAML keys at the top of the query document.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BaseFileEntry {
    pub id: String,
    pub title: String,
    /// Path relative to the records root, always with forward slashes, e.g.
    /// `Records/base/books.base`.
    pub path: String,
    /// Why the query cannot be parsed, if it cannot. A broken file is still
    /// listed so the user can find and fix it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Every `.base` file under `Records/`, sorted by title.
///
/// Scanning is independent of the record index: a base is a view, not a record,
/// and must remain usable when the record index is empty or being rebuilt.
pub fn scan_base_files(records_root: &Path) -> Result<Vec<BaseFileEntry>, Error> {
    let mut files = Vec::new();
    walk_extension(&records_root.join("Records"), BASE_EXTENSION, &mut |path| {
        if let Ok(source) = std::fs::read_to_string(path) {
            files.push(entry_from_source(
                relative_flower_path(records_root, path),
                &source,
            ));
        }
    })?;
    files.sort_by(|a, b| {
        a.title
            .to_lowercase()
            .cmp(&b.title.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    Ok(files)
}

/// Read the raw YAML of a `.base` file, for the definition editor.
pub fn read_base_file(records_root: &Path, relative: &str) -> Result<String, Error> {
    let path = resolve_base_file(records_root, relative)?;
    std::fs::read_to_string(path).map_err(Error::from)
}

/// Validate and render a `.base` file without changing it.
pub fn render_base_file(records_root: &Path, relative: &str) -> Result<BaseView, Error> {
    let source = read_base_file(records_root, relative)?;
    let spec = parse_base(&source).map_err(Error::Invalid)?;
    render_base(records_root, &spec)
}

/// Validate and atomically save a `.base` file. An invalid query is never
/// written; when `expected_source` is given, a file changed by another program
/// is refused rather than overwritten.
pub fn save_base_file(
    records_root: &Path,
    relative: &str,
    text: &str,
    expected_source: Option<&str>,
) -> Result<BaseView, Error> {
    let path = resolve_base_file(records_root, relative)?;
    if let Some(expected) = expected_source {
        let current = std::fs::read_to_string(&path)?;
        if current != expected {
            return Err(Error::Stale(relative.to_string()));
        }
    }
    let spec = parse_base(text).map_err(Error::Invalid)?;
    write_text(&path, text)?;
    render_base(records_root, &spec)
}

/// Create a new `.base` file under `Records/base/` and return its entry.
pub fn create_base_file(records_root: &Path, title: &str) -> Result<BaseFileEntry, Error> {
    let title = title.trim();
    let title = if title.is_empty() { "New base" } else { title };
    for _ in 0..10 {
        let id = generate_record_id(records_root);
        let slug = slugify_title(Some(title), None);
        let relative = format!("Records/{}/{slug}--{id}.{BASE_EXTENSION}", BASE_TYPE);
        let path = records_root.join(&relative);
        if path.exists() {
            continue;
        }
        let source = starter_file_document(title, &id);
        write_text(&path, &source)?;
        return Ok(entry_from_source(relative, &source));
    }
    Err(Error::Invalid(
        "could not allocate a unique base filename".into(),
    ))
}

fn starter_file_document(title: &str, id: &str) -> String {
    // JSON strings are valid YAML, which keeps a title with a quote or colon
    // from producing a file that does not parse.
    let title_yaml = serde_json::to_string(title).unwrap_or_else(|_| "\"New base\"".into());
    let id_yaml = serde_json::to_string(id).unwrap_or_else(|_| "\"base\"".into());
    format!(
        "id: {id_yaml}\ntitle: {title_yaml}\n{}",
        starter_document(title)
    )
}

fn relative_flower_path(records_root: &Path, path: &Path) -> String {
    path.strip_prefix(records_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn entry_from_source(path: String, source: &str) -> BaseFileEntry {
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(source).ok();
    let id = yaml
        .as_ref()
        .and_then(|doc| yaml_field(doc, "id"))
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("base:{path}"));
    let title = yaml
        .as_ref()
        .and_then(|doc| yaml_field(doc, "title"))
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("base")
                .to_string()
        });
    BaseFileEntry {
        id,
        title,
        path,
        error: parse_base(source).err(),
    }
}

fn yaml_field<'a>(doc: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
    doc.as_mapping()?
        .get(serde_yaml::Value::String(key.to_string()))?
        .as_str()
}

/// Resolve a path received from the UI. Only `.base` files inside `Records/`
/// are accepted, so a command can never read or overwrite an arbitrary file.
fn resolve_base_file(records_root: &Path, relative: &str) -> Result<PathBuf, Error> {
    let cleaned = relative.trim().replace('\\', "/");
    if cleaned.is_empty() {
        return Err(Error::Invalid("base path is empty".into()));
    }
    if cleaned.starts_with('/') {
        return Err(Error::Invalid(format!(
            "base path must be relative to the records root: {relative}"
        )));
    }
    let mut segments = Vec::new();
    for segment in cleaned.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(Error::Invalid(format!(
                "base path may not contain empty, . or .. segments: {relative}"
            )));
        }
        if segment.contains(':') || segment.chars().any(char::is_control) {
            return Err(Error::Invalid(format!(
                "base path contains an invalid segment: {relative}"
            )));
        }
        segments.push(segment);
    }
    if segments.first().copied() != Some("Records") {
        return Err(Error::Invalid(format!(
            "base path must start with Records/: {relative}"
        )));
    }
    if !cleaned
        .to_ascii_lowercase()
        .ends_with(&format!(".{BASE_EXTENSION}"))
    {
        return Err(Error::Invalid(format!(
            "base path must end with .{BASE_EXTENSION}: {relative}"
        )));
    }
    let mut path = records_root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use crate::storage::{record_path, write_record, write_text};
    use serde_yaml::Value as Yaml;
    use tempfile::TempDir;

    fn seed(root: &Path, id: &str, r#type: &str, title: &str, extra: &[(&str, Yaml)]) -> Record {
        let mut record = Record::new(id, r#type);
        record.title = Some(title.to_string());
        record.folder = None;
        for (k, v) in extra {
            record.extra.insert(k.to_string(), v.clone());
        }
        let path = record_path(root, &record);
        write_record(&path, &record, Some("body")).unwrap();
        record
    }

    fn num(n: f64) -> Yaml {
        Yaml::Number(serde_yaml::Number::from(n as i64))
    }

    #[test]
    fn empty_document_is_a_valid_base_with_defaults() {
        let spec = parse_base("").unwrap();
        assert_eq!(
            spec.labels_for_test(),
            vec!["file.name", "status", "file.folder"]
        );
        assert!(spec.filter_expr.is_none());
    }

    #[test]
    fn yaml_columns_accept_shorthand_and_labels() {
        let spec = parse_base("columns:\n  - title\n  - label: State\n    expr: status\n").unwrap();
        assert_eq!(spec.columns[0].label, "title");
        assert_eq!(spec.columns[1].label, "State");
        assert_eq!(spec.columns[1].expr, "status");
    }

    #[test]
    fn invalid_expressions_fail_at_parse_time() {
        let err = parse_base("columns:\n  - label: Bad\n    expr: 1 +\n").unwrap_err();
        assert!(err.contains("Bad"), "{err}");
        let err = parse_base("where: 1 +\n").unwrap_err();
        assert!(err.contains("where"), "{err}");
        let err = parse_base("sort: \"-\"\n").unwrap_err();
        assert!(err.contains("sort"), "{err}");
        assert!(parse_base("limit: 0\n").is_err());
    }

    #[test]
    fn filters_source_by_type_and_tag() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut book = seed(root, "rec_b", "book", "Dune", &[("pages_read", num(10.0))]);
        book.tags = vec!["reading".into()];
        let path = record_path(root, &book);
        write_record(&path, &book, Some("body")).unwrap();
        seed(root, "rec_n", "note", "A thought", &[]);

        let spec = parse_base("source:\n  type: book\n  tag: reading\n").unwrap();
        let view = render_base(root, &spec).unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].title.as_deref(), Some("Dune"));
        assert_eq!(view.scanned, 1);
        assert_eq!(view.matched, 1);
        assert!(view.warnings.is_empty(), "{:?}", view.warnings);
    }

    #[test]
    fn where_and_computed_columns_run_formulas() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(
            root,
            "rec_a",
            "book",
            "Dune",
            &[("pages_total", num(400.0)), ("pages_read", num(100.0))],
        );
        seed(
            root,
            "rec_b",
            "book",
            "Emma",
            &[("pages_total", num(300.0)), ("pages_read", num(300.0))],
        );

        let spec = parse_base(
            "source:\n  type: book\n\
             where: pages_total > pages_read\n\
             columns:\n  - file.name\n  - label: Left\n    expr: pages_total - pages_read\n",
        )
        .unwrap();
        let view = render_base(root, &spec).unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(
            view.labels,
            vec!["file.name".to_string(), "Left".to_string()]
        );
        assert_eq!(
            view.rows[0].cells,
            vec!["Dune".to_string(), "300".to_string()]
        );
        assert_eq!(view.matched, 1);
        assert_eq!(view.scanned, 2);
    }

    #[test]
    fn sorts_numbers_numerically_and_limit_cuts_rows() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(root, "rec_1", "book", "Ten", &[("score", num(10.0))]);
        seed(root, "rec_2", "book", "Nine", &[("score", num(9.0))]);
        seed(root, "rec_3", "book", "Hundred", &[("score", num(100.0))]);

        let spec =
            parse_base("source:\n  type: book\ncolumns:\n  - file.name\nsort: score\nlimit: 2\n")
                .unwrap();
        let view = render_base(root, &spec).unwrap();
        assert_eq!(
            view.rows
                .iter()
                .map(|r| r.cells[0].as_str())
                .collect::<Vec<_>>(),
            vec!["Nine", "Ten"]
        );
        assert_eq!(view.matched, 3);
        assert_eq!(view.rows.len(), 2);
    }

    #[test]
    fn descending_sort_uses_a_leading_minus() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(root, "rec_1", "book", "Ten", &[("score", num(10.0))]);
        seed(root, "rec_2", "book", "Nine", &[("score", num(9.0))]);

        let spec = parse_base("source:\n  type: book\ncolumns:\n  - file.name\nsort: \"-score\"\n")
            .unwrap();
        let view = render_base(root, &spec).unwrap();
        assert_eq!(view.rows[0].cells[0], "Ten");
        assert_eq!(view.rows[1].cells[0], "Nine");
    }

    #[test]
    fn a_failing_cell_is_reported_but_does_not_hide_the_row() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(
            root,
            "rec_1",
            "note",
            "A",
            &[("x", Yaml::String("nope".into()))],
        );

        let spec =
            parse_base("columns:\n  - file.name\n  - label: Bad\n    expr: x / 2\n").unwrap();
        let view = render_base(root, &spec).unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].cells[0], "A");
        assert_eq!(view.rows[0].cells[1], "");
        assert_eq!(view.warnings.len(), 1);
        assert!(view.warnings[0].contains("Bad"));
    }

    #[test]
    fn starter_document_parses() {
        let spec = parse_base(&starter_document("My base")).unwrap();
        assert!(!spec.columns.is_empty());
    }

    #[test]
    fn scans_only_base_files_and_reads_optional_identity() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(root, "rec_note", "note", "A note", &[]);
        let base_dir = root.join("Records").join("base");
        std::fs::create_dir_all(&base_dir).unwrap();
        write_text(
            &base_dir.join("books.base"),
            "id: base_books\ntitle: Books\nsource:\n  type: book\n",
        )
        .unwrap();

        let files = scan_base_files(root).unwrap();
        assert_eq!(files.len(), 1, "{files:?}");
        assert_eq!(files[0].id, "base_books");
        assert_eq!(files[0].title, "Books");
        assert_eq!(files[0].path, "Records/base/books.base");
        assert!(files[0].error.is_none());
    }

    #[test]
    fn a_broken_base_file_is_listed_with_its_error() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let base_dir = root.join("Records").join("base");
        std::fs::create_dir_all(&base_dir).unwrap();
        write_text(&base_dir.join("broken.base"), "columns: [unclosed\n").unwrap();

        let files = scan_base_files(root).unwrap();
        assert_eq!(files.len(), 1);
        // No id field: the path is the stable fallback identity.
        assert_eq!(files[0].id, "base:Records/base/broken.base");
        assert_eq!(files[0].title, "broken");
        assert!(
            files[0]
                .error
                .as_deref()
                .unwrap_or("")
                .contains("base YAML")
        );
    }

    #[test]
    fn file_backed_bases_render_and_refuse_a_stale_save() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed(root, "rec_book", "book", "Dune", &[]);
        let base_dir = root.join("Records").join("base");
        std::fs::create_dir_all(&base_dir).unwrap();
        let path = base_dir.join("books.base");
        let source = "source:\n  type: book\ncolumns:\n  - file.name\n";
        write_text(&path, source).unwrap();

        let view = render_base_file(root, "Records/base/books.base").unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].cells[0], "Dune");

        let stale = save_base_file(
            root,
            "Records/base/books.base",
            "source:\n  type: note\n",
            Some("something else"),
        );
        assert!(matches!(stale, Err(Error::Stale(_))));

        save_base_file(root, "Records/base/books.base", source, Some(source)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    }

    #[test]
    fn base_file_paths_are_confined_to_records() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        for bad in [
            "../evil.base",
            "/absolute.base",
            "notes/book.base",
            "Records/../book.base",
            "Records/book.md",
        ] {
            assert!(
                read_base_file(root, bad).is_err(),
                "accepted unsafe path: {bad}"
            );
        }
    }

    #[test]
    fn create_base_file_writes_a_parseable_flower() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let entry = create_base_file(root, "Books").unwrap();
        assert_eq!(entry.title, "Books");
        assert!(!entry.id.is_empty());
        assert!(entry.path.starts_with("Records/base/"));
        assert!(entry.path.ends_with(".base"));
        assert!(entry.error.is_none());
        let source = read_base_file(root, &entry.path).unwrap();
        assert!(source.contains("title: \"Books\""));
        parse_base(&source).unwrap();
    }

    impl BaseSpec {
        fn labels_for_test(&self) -> Vec<&str> {
            self.columns.iter().map(|c| c.label.as_str()).collect()
        }
    }
}
