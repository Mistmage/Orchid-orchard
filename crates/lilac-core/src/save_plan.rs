//! Save plans: the dry run that a commit writes.
//!
//! A plan is data. A preview renders it, a commit writes it, and both use the
//! same drafts, so the two cannot drift. Planning is also where rules and
//! placement run: a rule that moves a record decides the filename before the
//! user sees it, not after the file is written.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::collection::{CollectionEntry, GroupHint};
use crate::naming::NamingConfig;
use crate::record::Record;
use crate::rules::{Rule, apply_rules_reported};
use crate::storage::{
    generate_record_id, normalize_folder, now_timestamp, rebuild_index, record_filename,
    record_path, write_record,
};
use crate::workspace::WindowGeometry;

/// One tab a plan should capture. Everything is optional except the URL, since
/// a browser may not report a title, a group or tab state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanTab {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub group_color: Option<String>,
    #[serde(default)]
    pub group_collapsed: Option<bool>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub snapshot: Option<String>,
    #[serde(default)]
    pub snapshot_format: Option<String>,
    /// Open metadata from the browser, carried as-is (`window_hint`,
    /// `tab_group`). It is a hint and never a handle.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

/// The intent behind a save. `page` produces one page; every other source also
/// produces one set record (`collection`, or `workspace` for `workspace`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanRequest {
    pub source: String,
    #[serde(default)]
    pub tabs: Vec<PlanTab>,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub geometry: Option<WindowGeometry>,
}

/// One record the plan would write.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDraft {
    pub id: String,
    pub record: Record,
    pub body: String,
    /// Relative to the records root, exactly as the preview shows it.
    pub path: String,
    pub filename: String,
    /// The preview can keep a record out of the commit.
    #[serde(default)]
    pub excluded: bool,
    /// The page that already has this URL, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    /// Rule failures and other notes that belong to this record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl SaveDraft {
    /// What the commit would do with this draft.
    pub fn is_writable(&self) -> bool {
        !self.excluded
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePlan {
    pub source: String,
    pub records: Vec<SaveDraft>,
    /// Pages whose URL already exists (or repeats in this plan), mirrored at the
    /// top level so a preview can summarise them without scanning every draft.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicates: Vec<PlanDuplicate>,
    /// Notes that are about the save as a whole, not one record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDuplicate {
    pub id: String,
    pub url: String,
    pub duplicate_of: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WrittenDraft {
    pub id: String,
    pub path: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailedDraft {
    pub id: String,
    pub error: String,
}

/// Per-record outcome. A failing record does not stop the others.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CommitReport {
    pub written: Vec<WrittenDraft>,
    pub skipped: Vec<String>,
    pub failed: Vec<FailedDraft>,
}

impl CommitReport {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

fn fresh_id(records_root: &Path, taken: &mut HashSet<String>) -> String {
    for _ in 0..20 {
        let id = generate_record_id(records_root);
        if taken.insert(id.clone()) {
            return id;
        }
    }
    // The clock is stuck and disk ids keep colliding with this plan. A suffix
    // is ugly but keeps two drafts from writing the same file.
    let mut n = taken.len();
    loop {
        let id = format!("rec_plan_{n:x}");
        if taken.insert(id.clone()) {
            return id;
        }
        n += 1;
    }
}

/// Place a page in a folder, rejecting escapes instead of writing elsewhere.
/// A bad folder is a warning, not a failed save, because the user can fix it in
/// the preview.
fn place_folder(record: &mut Record, raw: Option<&str>, warnings: &mut Vec<String>) {
    let Some(raw) = raw else { return };
    if raw.trim().is_empty() {
        record.folder = None;
        return;
    }
    match normalize_folder(raw) {
        Ok(Some(folder)) if !folder.eq_ignore_ascii_case(&record.r#type) => {
            record.folder = Some(folder);
        }
        Ok(_) => record.folder = None,
        Err(e) => warnings.push(format!("folder was not used: {e}")),
    }
}

fn apply_rules(record: &mut Record, rules: &[Rule], warnings: &mut Vec<String>) {
    let (_, errors) = apply_rules_reported(record, rules);
    warnings.extend(errors);
    // A rule-owned folder is user input too: reject escapes rather than
    // letting the save fail or silently write outside Records/.
    if let Some(folder) = record.folder.clone() {
        match normalize_folder(&folder) {
            Ok(None) => record.folder = None,
            Ok(Some(f)) if f.eq_ignore_ascii_case(&record.r#type) => record.folder = None,
            Ok(Some(f)) => record.folder = Some(f),
            Err(e) => {
                record.folder = None;
                warnings.push(format!("folder was not used: {e}"));
            }
        }
    }
}

fn duplicates_of(drafts: &[SaveDraft]) -> Vec<PlanDuplicate> {
    drafts
        .iter()
        .filter_map(|draft| {
            Some(PlanDuplicate {
                id: draft.id.clone(),
                url: draft.record.url.clone()?,
                duplicate_of: draft.duplicate_of.clone()?,
            })
        })
        .collect()
}

fn path_entry(records_root: &Path, record: &Record) -> (String, String) {
    let path = record_path(records_root, record);
    let relative = path
        .strip_prefix(records_root)
        .ok()
        .and_then(|p| p.to_str())
        .map(|p| p.replace('\\', "/"))
        .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/"));
    (relative, record_filename(record))
}

/// Build the dry run. No file is written.
///
/// Rules run on every draft. Duplicates are reported, not dropped: whether a
/// duplicate should stop a save is T10's decision, and the preview needs to
/// show it either way.
pub fn plan_save(
    records_root: &Path,
    request: &PlanRequest,
    naming: &NamingConfig,
    rules: &[Rule],
) -> SavePlan {
    let stamp = now_timestamp();
    let index = rebuild_index(records_root).unwrap_or_default();
    let existing_pages: HashMap<String, String> = index
        .records
        .iter()
        .filter(|entry| entry.r#type.eq_ignore_ascii_case("page"))
        .filter_map(|entry| entry.url.clone().map(|url| (url, entry.id.clone())))
        .collect();
    let mut taken: HashSet<String> = index.records.iter().map(|e| e.id.clone()).collect();
    let mut plan_warnings = Vec::new();
    let mut drafts: Vec<SaveDraft> = Vec::new();
    let mut seen_urls: HashMap<String, String> = HashMap::new();

    let list: Vec<&PlanTab> = request
        .tabs
        .iter()
        .filter(|tab| !tab.url.trim().is_empty())
        .collect();
    if list.len() != request.tabs.len() {
        plan_warnings.push("a tab without a URL was left out of the plan".to_string());
    }
    if list.is_empty() {
        plan_warnings.push("there was nothing to save".to_string());
        return SavePlan {
            source: request.source.clone(),
            records: drafts,
            duplicates: Vec::new(),
            warnings: plan_warnings,
        };
    }

    for tab in &list {
        let mut warnings = Vec::new();
        let id = fresh_id(records_root, &mut taken);
        let mut record = Record::new(id.clone(), "page");
        record.url = Some(tab.url.clone());
        record.title = tab
            .title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .or_else(|| {
                Some(naming.derive_title(&record)).filter(|title| !title.trim().is_empty())
            });
        record.created = Some(stamp.clone());
        record.modified = Some(stamp.clone());
        record.snapshot = tab.snapshot.clone();
        record.snapshot_format = tab.snapshot_format.clone();
        for (key, value) in &tab.extra {
            record.extra.insert(key.clone(), value.clone());
        }
        place_folder(&mut record, request.folder.as_deref(), &mut warnings);
        apply_rules(&mut record, rules, &mut warnings);
        if record
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .is_none()
        {
            record.title = Some(naming.derive_title(&record)).filter(|t| !t.trim().is_empty());
        }

        let duplicate_of = record
            .url
            .as_deref()
            .and_then(|url| existing_pages.get(url).cloned())
            .or_else(|| {
                record
                    .url
                    .as_deref()
                    .and_then(|url| seen_urls.get(url).cloned())
            });
        if let Some(other) = duplicate_of.as_deref() {
            warnings.push(format!("another page already saves this URL: {other}"));
        }
        if let Some(url) = record.url.clone() {
            seen_urls.entry(url).or_insert_with(|| id.clone());
        }

        let (relative, filename) = path_entry(records_root, &record);
        drafts.push(SaveDraft {
            id,
            record,
            body: String::new(),
            path: relative,
            filename,
            excluded: false,
            duplicate_of,
            warnings,
        });
    }

    if request.source == "page" {
        return SavePlan {
            source: request.source.clone(),
            duplicates: duplicates_of(&drafts),
            records: drafts,
            warnings: plan_warnings,
        };
    }

    // A set mirrors the page drafts in captured order. The body is the same
    // list a person would write by hand, so the file stays readable.
    let entries: Vec<CollectionEntry> = drafts
        .iter()
        .map(|draft| {
            let tab = list
                .iter()
                .find(|tab| tab.url == draft.record.url.clone().unwrap_or_default());
            let group = tab
                .and_then(|tab| tab.group.clone())
                .filter(|g| !g.is_empty());
            CollectionEntry {
                id: Some(draft.id.clone()),
                url: draft.record.url.clone().unwrap_or_default(),
                title: draft.record.title.clone(),
                group: group.clone(),
                group_color: group
                    .as_ref()
                    .and_then(|_| tab.and_then(|tab| tab.group_color.clone()))
                    .filter(|c| !c.is_empty()),
                pinned: tab.map(|tab| tab.pinned).unwrap_or(false),
                active: tab.map(|tab| tab.active).unwrap_or(false),
                muted: tab.map(|tab| tab.muted).unwrap_or(false),
            }
        })
        .collect();

    let mut groups: Vec<GroupHint> = Vec::new();
    for tab in &list {
        let Some(name) = tab.group.clone().filter(|name| !name.is_empty()) else {
            continue;
        };
        if groups.iter().any(|group| group.name == name) {
            continue;
        }
        groups.push(GroupHint {
            name,
            color: tab.group_color.clone().unwrap_or_default(),
            collapsed: tab.group_collapsed.unwrap_or(false),
        });
    }

    let workspace = request.source == "workspace";
    let mut warnings = Vec::new();
    let id = fresh_id(records_root, &mut taken);
    let mut record = Record::new(
        id.clone(),
        if workspace { "workspace" } else { "collection" },
    );
    record.title = Some(
        request
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| {
                groups
                    .iter()
                    .find(|group| !group.name.is_empty())
                    .map(|group| {
                        if workspace {
                            format!("Workspace — {}", group.name)
                        } else {
                            format!("Saved tabs — {}", group.name)
                        }
                    })
                    .unwrap_or_else(|| {
                        let now = chrono::Local::now().format("%Y-%m-%d %H:%M");
                        if workspace {
                            format!("Workspace {now}")
                        } else {
                            format!("Saved tabs {now}")
                        }
                    })
            }),
    );
    record.created = Some(stamp.clone());
    record.modified = Some(stamp.clone());
    record.items = entries;
    record.tab_groups = groups;
    record.saved_from = Some(if workspace {
        "window".to_string()
    } else {
        request.source.clone()
    });
    if workspace {
        record.description = request
            .description
            .clone()
            .filter(|description| !description.trim().is_empty());
        record.geometry = request.geometry.clone();
    }
    // Rules may move a set too, but a chosen save folder deliberately applies
    // only to the pages: a collection keeps Records/collection.
    apply_rules(&mut record, rules, &mut warnings);

    let body = drafts
        .iter()
        .map(|draft| format!("- [[{}]]", draft.id))
        .collect::<Vec<_>>()
        .join("\n");
    let (relative, filename) = path_entry(records_root, &record);
    drafts.push(SaveDraft {
        id,
        record,
        body,
        path: relative,
        filename,
        excluded: false,
        duplicate_of: None,
        warnings,
    });

    SavePlan {
        source: request.source.clone(),
        duplicates: duplicates_of(&drafts),
        records: drafts,
        warnings: plan_warnings,
    }
}

/// Write every draft that was not excluded. Each record is independent: one
/// failure is reported and the rest still write.
pub fn commit_plan(records_root: &Path, plan: &SavePlan) -> CommitReport {
    let mut report = CommitReport::default();
    let excluded: HashSet<&str> = plan
        .records
        .iter()
        .filter(|draft| draft.excluded)
        .map(|draft| draft.id.as_str())
        .collect();
    // A corrected title or URL in the preview belongs to the page, but the set
    // that points at it should show the same value, not the old one.
    let corrected: HashMap<&str, &Record> = plan
        .records
        .iter()
        .filter(|draft| !draft.excluded)
        .map(|draft| (draft.id.as_str(), &draft.record))
        .collect();
    for draft in &plan.records {
        if !draft.is_writable() {
            report.skipped.push(draft.id.clone());
            continue;
        }
        // A set mirrors the pages that are actually written. If the preview
        // excluded one, its link leaves the list too, so a collection cannot
        // point at a record that was never saved. Corrected titles and URLs on
        // the page drafts are copied into the set's entries as well.
        let mut record = draft.record.clone();
        let mut body = draft.body.clone();
        if matches!(record.r#type.as_str(), "collection" | "workspace")
            && record.items.iter().any(|entry| entry.id.is_some())
        {
            let before = record.items.len();
            record.items.retain(|entry| {
                entry
                    .id
                    .as_deref()
                    .map(|id| !excluded.contains(id))
                    .unwrap_or(true)
            });
            let mut changed = record.items.len() != before;
            for entry in &mut record.items {
                let Some(id) = entry.id.as_deref() else {
                    continue;
                };
                if let Some(page) = corrected.get(id) {
                    let url = page.url.clone().unwrap_or_default();
                    if entry.url != url || entry.title != page.title {
                        entry.url = url;
                        entry.title = page.title.clone();
                        changed = true;
                    }
                }
            }
            if changed {
                body = record
                    .items
                    .iter()
                    .filter_map(|entry| entry.id.as_ref())
                    .map(|id| format!("- [[{id}]]"))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        let path = record_path(records_root, &record);
        match write_record(&path, &record, Some(&body)) {
            Ok(()) => report.written.push(WrittenDraft {
                id: draft.id.clone(),
                path: path
                    .strip_prefix(records_root)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(|p| p.replace('\\', "/"))
                    .unwrap_or_else(|| path.to_string_lossy().replace('\\', "/")),
                r#type: record.r#type.clone(),
            }),
            Err(error) => report.failed.push(FailedDraft {
                id: draft.id.clone(),
                error: error.to_string(),
            }),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{read_record, record_path as storage_path};
    use tempfile::TempDir;

    fn naming() -> NamingConfig {
        NamingConfig::default()
    }

    fn tab(url: &str, title: &str, group: Option<&str>) -> PlanTab {
        PlanTab {
            url: url.to_string(),
            title: Some(title.to_string()),
            group: group.map(str::to_string),
            group_color: group.map(|_| "blue".to_string()),
            ..PlanTab::default()
        }
    }

    fn request(source: &str, tabs: Vec<PlanTab>) -> PlanRequest {
        PlanRequest {
            source: source.to_string(),
            tabs,
            ..PlanRequest::default()
        }
    }

    #[test]
    fn a_set_plan_has_pages_plus_one_collection_in_order() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = plan_save(
            root,
            &request(
                "group",
                vec![
                    tab("https://a.example", "A", Some("Design")),
                    tab("https://b.example", "B", Some("Design")),
                ],
            ),
            &naming(),
            &[],
        );
        assert_eq!(plan.records.len(), 3);
        let collection = plan.records.last().unwrap();
        assert_eq!(collection.record.r#type, "collection");
        assert_eq!(collection.record.saved_from.as_deref(), Some("group"));
        assert_eq!(collection.record.items.len(), 2);
        assert_eq!(collection.record.tab_groups.len(), 1);
        assert_eq!(collection.record.tab_groups[0].name, "Design");
        assert!(
            collection
                .body
                .contains(&format!("[[{}]]", plan.records[0].id))
        );
        assert!(
            collection
                .body
                .contains(&format!("[[{}]]", plan.records[1].id))
        );
    }

    #[test]
    fn a_single_page_plan_has_no_set_record() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = plan_save(
            root,
            &request("page", vec![tab("https://a.example", "A", None)]),
            &naming(),
            &[],
        );
        assert_eq!(plan.records.len(), 1);
        assert_eq!(plan.records[0].record.r#type, "page");
        assert_eq!(plan.records[0].record.title.as_deref(), Some("A"));
    }

    #[test]
    fn commit_writes_exactly_the_planned_fields_and_path() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = plan_save(
            root,
            &request(
                "window",
                vec![
                    tab("https://a.example", "A", Some("Design")),
                    tab("https://b.example", "B", None),
                ],
            ),
            &naming(),
            &[],
        );
        let report = commit_plan(root, &plan);
        assert!(report.ok());
        assert_eq!(report.written.len(), 3);
        for draft in &plan.records {
            let path = record_path(root, &draft.record);
            let (written, body) = read_record(&path).unwrap();
            assert_eq!(written, draft.record);
            assert_eq!(body.unwrap_or_default(), draft.body);
            assert_eq!(
                draft.path,
                report
                    .written
                    .iter()
                    .find(|w| w.id == draft.id)
                    .unwrap()
                    .path
            );
        }
    }

    #[test]
    fn an_excluded_draft_is_not_written_and_is_reported() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut plan = plan_save(
            root,
            &request(
                "selection",
                vec![
                    tab("https://a.example", "A", None),
                    tab("https://b.example", "B", None),
                ],
            ),
            &naming(),
            &[],
        );
        plan.records[0].excluded = true;
        let report = commit_plan(root, &plan);
        assert_eq!(report.skipped, vec![plan.records[0].id.clone()]);
        assert_eq!(report.written.len(), 2);
        assert!(!record_path(root, &plan.records[0].record).exists());
    }

    #[test]
    fn an_excluded_page_leaves_the_set_it_belongs_to() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut plan = plan_save(
            root,
            &request(
                "group",
                vec![
                    tab("https://a.example", "A", None),
                    tab("https://b.example", "B", None),
                ],
            ),
            &naming(),
            &[],
        );
        let page_id = plan.records[0].id.clone();
        plan.records[0].excluded = true;
        commit_plan(root, &plan);
        let collection = plan.records.last().unwrap();
        let (written, body) = read_record(&record_path(root, &collection.record)).unwrap();
        assert_eq!(written.items.len(), 1);
        assert!(
            written
                .items
                .iter()
                .all(|entry| entry.id.as_deref() != Some(page_id.as_str()))
        );
        assert!(!body.unwrap_or_default().contains(&page_id));
    }

    #[test]
    fn a_corrected_page_title_reaches_the_set_entry() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut plan = plan_save(
            root,
            &request("group", vec![tab("https://a.example", "Old", None)]),
            &naming(),
            &[],
        );
        plan.records[0].record.title = Some("New".to_string());
        commit_plan(root, &plan);
        let collection = plan.records.last().unwrap();
        let (written, _) = read_record(&record_path(root, &collection.record)).unwrap();
        assert_eq!(written.items[0].title.as_deref(), Some("New"));
    }

    #[test]
    fn duplicates_are_reported_not_dropped() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let first = plan_save(
            root,
            &request("page", vec![tab("https://same.example", "One", None)]),
            &naming(),
            &[],
        );
        commit_plan(root, &first);
        let second = plan_save(
            root,
            &request("page", vec![tab("https://same.example", "Two", None)]),
            &naming(),
            &[],
        );
        assert_eq!(second.records.len(), 1);
        assert_eq!(
            second.records[0].duplicate_of.as_deref(),
            Some(first.records[0].id.as_str())
        );
        assert_eq!(second.duplicates.len(), 1);
        assert_eq!(second.duplicates[0].duplicate_of, first.records[0].id);
        assert!(
            second.records[0]
                .warnings
                .iter()
                .any(|warning| warning.contains("already saves"))
        );
    }

    #[test]
    fn a_bad_folder_is_a_warning_and_the_save_still_plans() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut req = request("page", vec![tab("https://a.example", "A", None)]);
        req.folder = Some("../escape".to_string());
        let plan = plan_save(root, &req, &naming(), &[]);
        assert_eq!(plan.records.len(), 1);
        assert!(plan.records[0].record.folder.is_none());
        assert!(
            plan.records[0]
                .warnings
                .iter()
                .any(|warning| warning.contains("folder"))
        );
    }

    #[test]
    fn a_workspace_plan_carries_geometry_and_description() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut req = request(
            "workspace",
            vec![tab("https://a.example", "A", Some("Design"))],
        );
        req.title = Some("Design desk".to_string());
        req.description = Some("Drawing".to_string());
        req.geometry = Some(WindowGeometry {
            left: Some(10.0),
            top: Some(20.0),
            width: Some(1200.0),
            height: Some(800.0),
            ..WindowGeometry::default()
        });
        let plan = plan_save(root, &req, &naming(), &[]);
        let workspace = plan.records.last().unwrap();
        assert_eq!(workspace.record.r#type, "workspace");
        assert_eq!(workspace.record.title.as_deref(), Some("Design desk"));
        assert_eq!(workspace.record.description.as_deref(), Some("Drawing"));
        assert_eq!(workspace.record.geometry, req.geometry);
    }

    #[test]
    fn a_missing_url_is_reported_and_skipped() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let plan = plan_save(
            root,
            &request("selection", vec![PlanTab::default()]),
            &naming(),
            &[],
        );
        assert!(plan.records.is_empty());
        assert!(plan.warnings.iter().any(|warning| warning.contains("URL")));
    }

    #[test]
    fn commit_paths_follow_an_edited_folder_and_title() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let mut plan = plan_save(
            root,
            &request("page", vec![tab("https://a.example", "A", None)]),
            &naming(),
            &[],
        );
        plan.records[0].record.title = Some("A better title".to_string());
        plan.records[0].record.folder = Some("inbox".to_string());
        let report = commit_plan(root, &plan);
        let path = storage_path(root, &plan.records[0].record);
        assert!(path.exists());
        let expected = format!("inbox/a-better-title--{}.md", plan.records[0].id);
        assert!(
            report.written[0].path.ends_with(&expected),
            "{} did not end with {}",
            report.written[0].path,
            expected
        );
    }
}
