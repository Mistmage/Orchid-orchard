//! Base records: a declarative query over other records.
//!
//! A base is the first *flower*: a terminal view with its own rendering
//! authority. It is still an ordinary record (`type: base`), so it lives under
//! `Records/`, is indexed, is backed up like everything else, and never needs a
//! separate database table. Its body is the query document:
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
use crate::storage::{read_record, rebuild_index};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;

pub const BASE_TYPE: &str = "base";

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Record;
    use crate::storage::{record_path, write_record};
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

    impl BaseSpec {
        fn labels_for_test(&self) -> Vec<&str> {
            self.columns.iter().map(|c| c.label.as_str()).collect()
        }
    }
}
