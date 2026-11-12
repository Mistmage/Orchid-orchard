//! Template records: a record that declares the type it produces and seeds a
//! new record with its body and metadata.
//!
//! A template is an ordinary record with `type: template` and a required
//! `target_type`. Keeping it a record means templates are files like everything
//! else: editable, searchable, movable, and covered by the same round-trip
//! rules. Nothing about applying a template lives in the frontend.
//!
//! Placeholders are text substitutions, not a programming language:
//!
//! | Placeholder | Value |
//! |---|---|
//! | `{{id}}` | the new record id |
//! | `{{title}}` | the requested title, else the template's title |
//! | `{{url}}` | the requested URL, else the template's URL |
//! | `{{host}}` | host of the URL |
//! | `{{slug}}` | filename slug of title/URL |
//! | `{{date}}` | local date, `YYYY-MM-DD` |
//! | `{{now}}` | UTC timestamp, RFC 3339 |
//! | `{{record:<id>.<expression>}}` | a formula evaluated against another record |
//!
//! A reference reads another record by id and evaluates a formula against it:
//! `{{record:rec_books.pages_total - record.pages_read}}` or the simpler
//! `{{record:rec_author.file.name}}`. The referenced record is never modified.
//! An unknown record or a failed expression leaves the placeholder as written
//! and is reported as a warning by `create_from_template_reported`.
//!
//! Unknown placeholders stay untouched. Substitution applies to the body and to
//! string values in the copied metadata (including unknown fields); identity,
//! timestamps and placement are not template data.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::formula::{eval_formula, parse_formula};
use crate::record::Record;
use crate::storage::{
    find_record_by_id, generate_record_id, host_of, read_record, record_path, slugify_title,
    walk_md, write_record,
};

pub const TEMPLATE_TYPE: &str = "template";

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct TemplateInfo {
    pub id: String,
    pub title: Option<String>,
    pub target_type: String,
    pub description: Option<String>,
    pub path: String,
}

/// List every template under `Records/`, sorted by title then id. A template
/// without `target_type` is still listed, with an empty target, so the UI can
/// report it instead of hiding a broken declaration.
pub fn list_templates(records_root: &Path) -> Result<Vec<TemplateInfo>, Error> {
    let mut templates = Vec::new();
    let records_subdir = records_root.join("Records");
    walk_md(&records_subdir, &mut |path| {
        let Ok((record, _body)) = read_record(path) else {
            return;
        };
        if !is_template(&record) {
            return;
        }
        let rel = path
            .strip_prefix(records_root)
            .ok()
            .and_then(|p| p.to_str().map(str::to_string))
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        templates.push(TemplateInfo {
            id: record.id,
            title: record.title,
            target_type: record.target_type.unwrap_or_default(),
            description: record.description,
            path: rel,
        });
    })?;
    templates.sort_by(|a, b| {
        // A template that cannot produce a record goes last, so the first
        // selectable entry in the UI is usable.
        a.target_type
            .is_empty()
            .cmp(&b.target_type.is_empty())
            .then(
                a.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b.title.as_deref().unwrap_or("").to_lowercase()),
            )
            .then(a.id.cmp(&b.id))
    });
    Ok(templates)
}

pub fn is_template(record: &Record) -> bool {
    record.r#type.eq_ignore_ascii_case(TEMPLATE_TYPE)
}

/// Build the placeholder values for one application of a template.
pub fn placeholder_values(id: &str, title: &str, url: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    vars.insert("id".to_string(), id.to_string());
    vars.insert("title".to_string(), title.to_string());
    vars.insert("url".to_string(), url.to_string());
    vars.insert("host".to_string(), host_of(url).unwrap_or("").to_string());
    vars.insert("slug".to_string(), slugify_title(Some(title), Some(url)));
    vars.insert(
        "date".to_string(),
        chrono::Local::now().format("%Y-%m-%d").to_string(),
    );
    vars.insert("now".to_string(), chrono::Utc::now().to_rfc3339());
    vars
}

/// Replace `{{name}}` (spaces allowed inside the braces) for known names.
/// A lone `{{` or an unknown name is preserved as written.
pub fn substitute(text: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let key = after[..end].trim();
                match vars.get(key) {
                    Some(value) => {
                        out.push_str(value);
                        rest = &after[end + 2..];
                    }
                    None => {
                        out.push_str("{{");
                        rest = after;
                    }
                }
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Lazy id → record cache for `{{record:...}}` references.
///
/// A template may mention the same record many times; each file is read at most
/// once per application. A missing record is cached as `None` so a bad id is
/// only searched for once.
pub struct ReferenceCache<'a> {
    root: &'a Path,
    records: BTreeMap<String, Option<Record>>,
}

impl<'a> ReferenceCache<'a> {
    pub fn new(records_root: &'a Path) -> Self {
        Self {
            root: records_root,
            records: BTreeMap::new(),
        }
    }

    fn get(&mut self, id: &str) -> Option<&Record> {
        if !self.records.contains_key(id) {
            let found = find_record_by_id(self.root, id)
                .and_then(|path| read_record(&path).ok())
                .map(|(record, _body)| record);
            self.records.insert(id.to_string(), found);
        }
        self.records.get(id).and_then(|entry| entry.as_ref())
    }
}

/// Resolve one `{{record:<id>.<expression>}}` body.
///
/// The first `.` separates the id from the formula, so `rec_a.file.name` reads
/// the `file.name` property of `rec_a`. The referenced record is never modified.
pub fn resolve_reference(
    spec: &str,
    cache: &mut ReferenceCache<'_>,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let (id, expression) = match spec.split_once('.') {
        Some((id, expression)) => (id.trim(), expression.trim()),
        None => {
            warnings.push(format!(
                "reference {:?}: expected {{record:<id>.<expression>}}",
                spec
            ));
            return None;
        }
    };
    if id.is_empty() || expression.is_empty() {
        warnings.push(format!(
            "reference {:?}: expected an id and an expression",
            spec
        ));
        return None;
    }
    let record = match cache.get(id) {
        Some(record) => record,
        None => {
            warnings.push(format!("reference {}: record not found", id));
            return None;
        }
    };
    let expr = match parse_formula(expression) {
        Ok(expr) => expr,
        Err(e) => {
            warnings.push(format!("reference {}: {}", id, e));
            return None;
        }
    };
    match eval_formula(&expr, record) {
        Ok(value) => Some(value.display()),
        Err(e) => {
            warnings.push(format!("reference {}: {}: {}", id, expression, e));
            None
        }
    }
}

/// Replace placeholders, including references that read other records.
///
/// Ordinary names come from `vars`; a `record:` key is resolved through
/// `cache`. An unresolved reference stays in the text exactly as written and
/// adds one warning, so a template still produces a record.
pub fn substitute_references(
    text: &str,
    vars: &BTreeMap<String, String>,
    cache: &mut ReferenceCache<'_>,
    warnings: &mut Vec<String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                let key = after[..end].trim();
                let replacement = match vars.get(key) {
                    Some(value) => Some(value.clone()),
                    None => match key.strip_prefix("record:") {
                        Some(spec) => resolve_reference(spec.trim(), cache, warnings),
                        None => None,
                    },
                };
                match replacement {
                    Some(value) => {
                        out.push_str(&value);
                        rest = &after[end + 2..];
                    }
                    None => {
                        out.push_str("{{");
                        rest = after;
                    }
                }
            }
            None => {
                out.push_str("{{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Create a new record from `template_id`. The template's body and metadata are
/// copied, identity and timestamps are fresh, and the new record's type is the
/// template's declared `target_type`.
pub fn create_from_template(
    records_root: &Path,
    template_id: &str,
    title: Option<&str>,
    url: Option<&str>,
) -> Result<(Record, PathBuf), Error> {
    create_from_template_reported(records_root, template_id, title, url)
        .map(|(record, path, _warnings)| (record, path))
}

/// Same as [`create_from_template`], but returns warnings for `{{record:...}}`
/// references that could not be resolved. The record is still created: one
/// broken reference must not make a template unusable.
pub fn create_from_template_reported(
    records_root: &Path,
    template_id: &str,
    title: Option<&str>,
    url: Option<&str>,
) -> Result<(Record, PathBuf, Vec<String>), Error> {
    let path = find_record_by_id(records_root, template_id)
        .ok_or_else(|| Error::NotFound(template_id.to_string()))?;
    let (template, body) = read_record(&path)?;
    if !is_template(&template) {
        return Err(Error::Invalid(format!(
            "{} is not a template (type: {})",
            template_id, template.r#type
        )));
    }
    let target = template
        .target_type
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            Error::Invalid(format!(
                "template {} does not declare target_type",
                template_id
            ))
        })?
        .to_string();

    let new_id = generate_record_id(records_root);
    let chosen_title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .or_else(|| template.title.clone())
        .unwrap_or_default();
    let chosen_url = url
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string)
        .or_else(|| template.url.clone())
        .unwrap_or_default();
    let vars = placeholder_values(&new_id, &chosen_title, &chosen_url);
    let mut cache = ReferenceCache::new(records_root);
    let mut warnings = Vec::new();

    let mut record = template;
    record.id = new_id;
    record.r#type = target;
    record.target_type = None;
    record.title = Some(chosen_title);
    record.url = if chosen_url.is_empty() {
        None
    } else {
        Some(chosen_url)
    };
    record.created = Some(chrono::Utc::now().to_rfc3339());
    record.modified = record.created.clone();
    record.opened = None;
    record.last_viewed = None;
    // Placement is a property of the instance, not the template.
    record.folder = None;
    substitute_record(&mut record, &vars, &mut cache, &mut warnings);
    if record
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        record.title = None;
    }

    let body = body.map(|b| substitute_references(&b, &vars, &mut cache, &mut warnings));
    let destination = record_path(records_root, &record);
    write_record(&destination, &record, body.as_deref())?;
    Ok((record, destination, warnings))
}

/// Copy an existing record into a template for its own type. The source file is
/// left untouched. Refuses to template a template, which would make the target
/// type ambiguous (the template would point at `template`).
pub fn save_as_template(
    records_root: &Path,
    source_id: &str,
    title: Option<&str>,
) -> Result<(Record, PathBuf), Error> {
    let path = find_record_by_id(records_root, source_id)
        .ok_or_else(|| Error::NotFound(source_id.to_string()))?;
    let (source, body) = read_record(&path)?;
    if is_template(&source) {
        return Err(Error::Invalid(format!(
            "{} is already a template",
            source_id
        )));
    }
    let target_type = source.r#type.clone();
    let new_id = generate_record_id(records_root);
    let chosen_title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| match source.title.as_deref() {
            Some(name) if !name.trim().is_empty() => format!("{} template", name),
            _ => format!("{} template", target_type),
        });

    let mut record = source;
    record.id = new_id;
    record.r#type = TEMPLATE_TYPE.to_string();
    record.target_type = Some(target_type);
    record.title = Some(chosen_title);
    record.created = Some(chrono::Utc::now().to_rfc3339());
    record.modified = record.created.clone();
    record.opened = None;
    record.last_viewed = None;
    record.folder = None;
    record.snapshot = None;
    record.snapshot_format = None;

    let destination = record_path(records_root, &record);
    write_record(&destination, &record, body.as_deref())?;
    Ok((record, destination))
}

fn substitute_record(
    record: &mut Record,
    vars: &BTreeMap<String, String>,
    cache: &mut ReferenceCache<'_>,
    warnings: &mut Vec<String>,
) {
    for value in [
        &mut record.title,
        &mut record.icon,
        &mut record.color,
        &mut record.description,
        &mut record.status,
        &mut record.url,
        &mut record.parent,
    ]
    .into_iter()
    .flatten()
    {
        *value = substitute_references(value, vars, cache, warnings);
    }
    for list in [
        &mut record.tags,
        &mut record.categories,
        &mut record.related,
        &mut record.links,
    ] {
        for value in list.iter_mut() {
            *value = substitute_references(value, vars, cache, warnings);
        }
    }
    for value in record.extra.values_mut() {
        substitute_value(value, vars, cache, warnings);
    }
}

fn substitute_value(
    value: &mut serde_yaml::Value,
    vars: &BTreeMap<String, String>,
    cache: &mut ReferenceCache<'_>,
    warnings: &mut Vec<String>,
) {
    match value {
        serde_yaml::Value::String(text) => {
            *text = substitute_references(text, vars, cache, warnings)
        }
        serde_yaml::Value::Sequence(items) => {
            for item in items {
                substitute_value(item, vars, cache, warnings);
            }
        }
        serde_yaml::Value::Mapping(map) => {
            for value in map.values_mut() {
                substitute_value(value, vars, cache, warnings);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn seed_template(root: &Path, id: &str, target: Option<&str>) -> PathBuf {
        let mut record = Record::new(id, TEMPLATE_TYPE);
        record.title = Some("Meeting notes".into());
        record.description = Some("Template for {{host}}".into());
        record.target_type = target.map(str::to_string);
        record.tags = vec!["meeting".into(), "{{host}}".into()];
        record.extra.insert(
            "custom".to_string(),
            serde_yaml::Value::String("source: {{title}}".into()),
        );
        let path = record_path(root, &record);
        write_record(
            &path,
            &record,
            Some("# {{title}}\n\nSource: {{url}}\n\nDate: {{date}}\n"),
        )
        .unwrap();
        path
    }

    #[test]
    fn substitution_replaces_known_placeholders_and_keeps_unknown() {
        let mut vars = BTreeMap::new();
        vars.insert("title".to_string(), "Hello".to_string());
        assert_eq!(substitute("{{title}}!", &vars), "Hello!");
        assert_eq!(substitute("{{ title }}!", &vars), "Hello!");
        assert_eq!(substitute("{{missing}}", &vars), "{{missing}}");
        assert_eq!(substitute("no braces", &vars), "no braces");
        assert_eq!(substitute("{{unclosed", &vars), "{{unclosed");
    }

    #[test]
    fn create_copies_body_and_metadata_with_fresh_identity() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_template(root, "rec_tpl", Some("note"));

        let (record, path) = create_from_template(
            root,
            "rec_tpl",
            Some("Standup"),
            Some("https://example.com/x"),
        )
        .unwrap();

        assert_eq!(record.r#type, "note");
        assert_eq!(record.target_type, None);
        assert_ne!(record.id, "rec_tpl");
        assert!(record.id.starts_with("rec_"));
        assert_eq!(record.title.as_deref(), Some("Standup"));
        assert_eq!(record.url.as_deref(), Some("https://example.com/x"));
        assert_eq!(record.folder, None);
        assert_eq!(record.tags, vec!["meeting", "example.com"]);
        assert_eq!(
            record.extra.get("custom"),
            Some(&serde_yaml::Value::String("source: Standup".into()))
        );
        assert!(path.starts_with(root.join("Records").join("note")));
        let (saved, body) = read_record(&path).unwrap();
        assert_eq!(saved.id, record.id);
        assert_eq!(saved.created, saved.modified);
        assert!(saved.created.is_some());
        let body = body.unwrap();
        assert!(body.contains("# Standup"));
        assert!(body.contains("https://example.com/x"));
        assert!(body.contains("Date: "));
        assert!(!body.contains("{{title}}"));
    }

    #[test]
    fn create_uses_the_template_title_when_none_is_given() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_template(root, "rec_tpl", Some("note"));

        let (record, _) = create_from_template(root, "rec_tpl", None, None).unwrap();
        assert_eq!(record.title.as_deref(), Some("Meeting notes"));
    }

    #[test]
    fn template_without_target_type_is_refused() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_template(root, "rec_broken", None);

        let error = create_from_template(root, "rec_broken", Some("X"), None).unwrap_err();
        assert!(error.to_string().contains("target_type"));
    }

    #[test]
    fn a_non_template_cannot_be_applied() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut note = Record::new("rec_note", "note");
        note.title = Some("Note".into());
        let path = record_path(root, &note);
        write_record(&path, &note, Some("body")).unwrap();

        let error = create_from_template(root, "rec_note", Some("X"), None).unwrap_err();
        assert!(error.to_string().contains("not a template"));
    }

    #[test]
    fn save_as_template_clones_the_source_and_declares_its_type() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut note = Record::new("rec_note", "note");
        note.title = Some("Daily log".into());
        note.tags = vec!["journal".into()];
        note.folder = Some("Journal".into());
        let path = record_path(root, &note);
        write_record(&path, &note, Some("# {{date}}\n")).unwrap();

        let (template, template_path) = save_as_template(root, "rec_note", None).unwrap();

        assert_eq!(template.r#type, TEMPLATE_TYPE);
        assert_eq!(template.target_type.as_deref(), Some("note"));
        assert_eq!(template.title.as_deref(), Some("Daily log template"));
        assert_eq!(template.folder, None);
        assert_eq!(template.tags, vec!["journal"]);
        assert!(template_path.starts_with(root.join("Records").join(TEMPLATE_TYPE)));
        // The source keeps its identity, type and folder.
        let (source, _) = read_record(&path).unwrap();
        assert_eq!(source.id, "rec_note");
        assert_eq!(source.r#type, "note");
        assert_eq!(source.folder.as_deref(), Some("Journal"));

        // Applying the new template produces the source type.
        let (created, _) = create_from_template(root, &template.id, None, None).unwrap();
        assert_eq!(created.r#type, "note");
        assert_eq!(created.title.as_deref(), Some("Daily log template"));
    }

    #[test]
    fn template_cannot_be_templated_again() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_template(root, "rec_tpl", Some("note"));

        let error = save_as_template(root, "rec_tpl", None).unwrap_err();
        assert!(error.to_string().contains("already a template"));
    }

    #[test]
    fn list_templates_returns_declared_targets() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_template(root, "rec_b", Some("page"));
        seed_template(root, "rec_a", Some("note"));
        let mut note = Record::new("rec_note", "note");
        note.title = Some("Not a template".into());
        let path = record_path(root, &note);
        write_record(&path, &note, None).unwrap();

        let list = list_templates(root).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].target_type, "note");
        assert_eq!(list[1].target_type, "page");
    }

    /// A book record used as the target of `{{record:...}}` in the tests.
    fn seed_book(root: &Path) -> PathBuf {
        let mut book = Record::new("rec_book", "book");
        book.title = Some("Dune".into());
        book.extra.insert(
            "pages_total".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(400)),
        );
        book.extra.insert(
            "pages_read".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(100)),
        );
        let path = record_path(root, &book);
        write_record(&path, &book, Some("source text")).unwrap();
        path
    }

    fn seed_referencing_template(root: &Path) {
        let mut template = Record::new("rec_ref_tpl", TEMPLATE_TYPE);
        template.title = Some("Reading note".into());
        template.target_type = Some("note".into());
        template.description = Some("About {{record:rec_book.title}}".into());
        let body = "# {{title}}\n\nBook: {{record:rec_book.title}}\n\
                    File: {{record:rec_book.file.name}}\n\
                    Left: {{record:rec_book.pages_total - record.pages_read}}\n";
        let path = record_path(root, &template);
        write_record(&path, &template, Some(body)).unwrap();
    }

    #[test]
    fn references_read_body_metadata_and_nested_properties() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let book_path = seed_book(root);
        seed_referencing_template(root);

        let (record, path, warnings) =
            create_from_template_reported(root, "rec_ref_tpl", Some("Dune notes"), None).unwrap();
        assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
        assert_eq!(record.description.as_deref(), Some("About Dune"));
        let (_saved, body) = read_record(&path).unwrap();
        let body = body.unwrap();
        assert!(body.contains("Book: Dune"), "body: {body}");
        assert!(body.contains("File: Dune"), "body: {body}");
        assert!(body.contains("Left: 300"), "body: {body}");
        assert!(!body.contains("{{record:"), "body: {body}");

        // Referenced records are only read, never written.
        let (book, book_body) = read_record(&book_path).unwrap();
        assert_eq!(book.title.as_deref(), Some("Dune"));
        assert_eq!(book_body.as_deref(), Some("source text"));
    }

    #[test]
    fn an_unknown_reference_is_preserved_and_reported() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut template = Record::new("rec_bad_ref", TEMPLATE_TYPE);
        template.target_type = Some("note".into());
        let path = record_path(root, &template);
        write_record(
            &path,
            &template,
            Some("Who: {{record:rec_missing.title}}\n"),
        )
        .unwrap();

        let (record, path, warnings) =
            create_from_template_reported(root, "rec_bad_ref", Some("X"), None).unwrap();
        assert_eq!(record.title.as_deref(), Some("X"));
        let (_saved, body) = read_record(&path).unwrap();
        assert!(body.unwrap().contains("{{record:rec_missing.title}}"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("rec_missing"), "{:?}", warnings);
    }

    #[test]
    fn a_broken_reference_expression_is_preserved_and_reported() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        seed_book(root);
        let mut template = Record::new("rec_bad_expr", TEMPLATE_TYPE);
        template.target_type = Some("note".into());
        let path = record_path(root, &template);
        write_record(
            &path,
            &template,
            Some("Broken: {{record:rec_book.1 +}}\nNo dot: {{record:rec_book}}\n"),
        )
        .unwrap();

        let (_record, path, warnings) =
            create_from_template_reported(root, "rec_bad_expr", Some("X"), None).unwrap();
        let (_saved, body) = read_record(&path).unwrap();
        let body = body.unwrap();
        assert!(body.contains("{{record:rec_book.1 +}}"), "body: {body}");
        assert!(body.contains("{{record:rec_book}}"), "body: {body}");
        assert_eq!(warnings.len(), 2, "{:?}", warnings);
        assert!(
            warnings.iter().any(|w| w.contains("expected")),
            "{:?}",
            warnings
        );
    }
}
