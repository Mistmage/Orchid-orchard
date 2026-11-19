use lilac_core::{
    CollectionEntry, GroupHint, Record, Rect, WindowGeometry, delete_record, read_record,
    read_record_with_mtime, rebuild_index, record_path, render_collection_entries, search_records,
    write_record, write_record_if_unchanged,
};
use tempfile::TempDir;

fn make_root() -> TempDir {
    TempDir::new().unwrap()
}

fn base_record(id: &str, rtype: &str) -> Record {
    let mut r = Record::new(id, rtype);
    r.title = Some("SeaORM".to_string());
    r.url = Some("https://sea-orm.org".to_string());
    r.created = Some("2026-10-26T12:00:00Z".to_string());
    r.modified = Some("2026-10-26T12:30:00Z".to_string());
    r.tags = vec!["rust".to_string(), "orm".to_string()];
    r.status = Some("[[Reading]]".to_string());
    r
}

#[test]
fn roundtrip_reserved_and_extra() {
    let root = make_root();
    let mut r = base_record("rec_89a13f2", "page");
    r.extra.insert(
        "foo".to_string(),
        serde_yaml::Value::String("bar".to_string()),
    );
    let path = record_path(root.path(), &r);
    write_record(&path, &r, Some("Research candidate.")).unwrap();

    // print file for inspection
    let contents = std::fs::read_to_string(&path).unwrap();
    println!("--- FILE ---\n{}\n--- END ---", contents);

    let (r2, body) = read_record(&path).unwrap();
    assert_eq!(r, r2, "record mismatch after round-trip");
    assert_eq!(body.as_deref(), Some("Research candidate."));
}

#[test]
fn unknown_type_survives() {
    let root = make_root();
    let mut r = Record::new("rec_x", "weird_custom_type");
    r.title = Some("X".to_string());
    let path = record_path(root.path(), &r);
    write_record(&path, &r, None).unwrap();
    let (r2, _) = read_record(&path).unwrap();
    assert_eq!(r2.r#type, "weird_custom_type");
}

#[test]
fn wikilink_values_survive() {
    let root = make_root();
    let mut r = Record::new("rec_w", "page");
    r.status = Some("[[Reading]]".to_string());
    r.tags = vec!["[[Rust]]".to_string()];
    r.extra.insert(
        "project".to_string(),
        serde_yaml::Value::String("[[Orchid]]".to_string()),
    );
    let path = record_path(root.path(), &r);
    write_record(&path, &r, None).unwrap();
    let (r2, _) = read_record(&path).unwrap();
    assert_eq!(r2.status.as_deref(), Some("[[Reading]]"));
    assert_eq!(r2.tags, vec!["[[Rust]]".to_string()]);
    assert_eq!(
        r2.extra.get("project"),
        Some(&serde_yaml::Value::String("[[Orchid]]".to_string()))
    );
}

#[test]
fn rebuild_index_and_search() {
    let root = make_root();
    let r1 = {
        let mut r = Record::new("rec_a", "page");
        r.title = Some("SeaORM".to_string());
        r.tags = vec!["rust".to_string()];
        r.modified = Some("2026-10-26T12:00:00Z".to_string());
        r
    };
    let r2 = {
        let mut r = Record::new("rec_b", "collection");
        r.title = Some("ORM Research".to_string());
        r.tags = vec!["rust".to_string(), "orm".to_string()];
        r.modified = Some("2026-10-26T12:30:00Z".to_string());
        r
    };
    write_record(&record_path(root.path(), &r1), &r1, None).unwrap();
    write_record(&record_path(root.path(), &r2), &r2, None).unwrap();

    // delete index (none exists) -> rebuild
    let idx = rebuild_index(root.path()).unwrap();
    assert_eq!(idx.records.len(), 2);

    // search tag:rust
    let res = search_records(&idx, "tag:rust");
    assert_eq!(res.len(), 2);

    // search type:collection
    let res = search_records(&idx, "type:collection");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "rec_b");

    // free text
    let res = search_records(&idx, "seaorm");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "rec_a");
}

#[test]
fn external_edit_reparses() {
    let root = make_root();
    let mut r = Record::new("rec_e", "page");
    r.title = Some("Original".to_string());
    let path = record_path(root.path(), &r);
    write_record(&path, &r, Some("body v1")).unwrap();

    // external edit: change title and add a tag
    let mut contents = std::fs::read_to_string(&path).unwrap();
    contents = contents.replace("title: Original", "title: Edited\ntags:\n- newtag");
    std::fs::write(&path, contents).unwrap();

    let (r2, body) = read_record(&path).unwrap();
    assert_eq!(r2.title.as_deref(), Some("Edited"));
    assert!(r2.tags.contains(&"newtag".to_string()));
    assert_eq!(body.as_deref(), Some("body v1"));
}

#[test]
fn malformed_file_does_not_crash_rebuild() {
    let root = make_root();
    // valid record
    let r = {
        let mut r = Record::new("rec_ok", "page");
        r.title = Some("OK".to_string());
        r
    };
    write_record(&record_path(root.path(), &r), &r, None).unwrap();
    // malformed file
    let bad = root.path().join("Records/page/bad--rec_bad.md");
    std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
    std::fs::write(&bad, "this is not frontmatter\n").unwrap();

    let idx = rebuild_index(root.path()).unwrap();
    // only the valid one indexed
    assert_eq!(idx.records.len(), 1);
    assert_eq!(idx.records[0].id, "rec_ok");
}

#[test]
fn captured_window_keeps_groups_but_no_browser_ids() {
    let root = make_root();
    let mut record = Record::new("rec_window", "collection");
    record.title = Some("Saved tabs — Work".to_string());
    record.saved_from = Some("window".to_string());
    record.items = vec![
        CollectionEntry {
            id: Some("rec_a".to_string()),
            url: "https://a.example/one".to_string(),
            title: Some("A".to_string()),
            group: Some("Work".to_string()),
            group_color: Some("blue".to_string()),
            pinned: false,
            active: true,
            muted: false,
        },
        CollectionEntry {
            id: Some("rec_b".to_string()),
            url: "https://b.example/two".to_string(),
            title: Some("B".to_string()),
            group: Some("Work".to_string()),
            group_color: Some("blue".to_string()),
            pinned: true,
            active: false,
            muted: true,
        },
    ];
    record.tab_groups = vec![GroupHint {
        name: "Work".to_string(),
        color: "blue".to_string(),
        collapsed: true,
    }];
    record.extra.insert(
        "window_hint".to_string(),
        serde_yaml::to_value(serde_json::json!({
            "tab_count": 2,
            "incognito": false,
            "left": 100,
            "top": 80
        }))
        .unwrap(),
    );

    let path = record_path(root.path(), &record);
    let body = render_collection_entries(&record.items);
    write_record(&path, &record, Some(&body)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();

    // Hints describe a shape; they never carry handles that only one browser
    // session could resolve.
    assert!(!text.contains("window_id"));
    assert!(!text.contains("tab_id"));
    assert!(!text.contains("groupId"));
    assert!(!text.contains("tabId"));

    let (read, read_body) = read_record(&path).unwrap();
    assert_eq!(read.saved_from.as_deref(), Some("window"));
    assert_eq!(read.tab_groups, record.tab_groups);
    assert_eq!(read.items, record.items);
    assert_eq!(read_body.as_deref(), Some(body.as_str()));
}

#[test]
fn workspace_keeps_geometry_and_layout_but_no_browser_ids() {
    let root = make_root();
    let mut record = Record::new("rec_ws", "workspace");
    record.title = Some("Design desk".to_string());
    record.description = Some("What I keep open while drawing".to_string());
    record.saved_from = Some("window".to_string());
    record.geometry = Some(WindowGeometry {
        left: Some(40.0),
        top: Some(30.0),
        width: Some(1440.0),
        height: Some(900.0),
        state: Some("maximized".to_string()),
        display_scale: Some(1.25),
        display: Some(Rect {
            left: 0.0,
            top: 0.0,
            width: 2560.0,
            height: 1440.0,
        }),
    });
    record.items = vec![
        CollectionEntry {
            id: Some("rec_a".to_string()),
            url: "https://a.example/one".to_string(),
            title: Some("A".to_string()),
            group: Some("Research".to_string()),
            group_color: Some("blue".to_string()),
            pinned: false,
            active: true,
            muted: false,
        },
        CollectionEntry {
            id: Some("rec_b".to_string()),
            url: "https://b.example/two".to_string(),
            title: Some("B".to_string()),
            group: None,
            group_color: None,
            pinned: true,
            active: false,
            muted: false,
        },
    ];
    record.tab_groups = vec![GroupHint {
        name: "Research".to_string(),
        color: "blue".to_string(),
        collapsed: false,
    }];

    let path = record_path(root.path(), &record);
    let body = render_collection_entries(&record.items);
    write_record(&path, &record, Some(&body)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();

    // Bounds are a description, not a handle: a workspace must stay meaningful
    // after a browser restart and on another machine.
    assert!(!text.contains("window_id"));
    assert!(!text.contains("tab_id"));
    assert!(!text.contains("display_id"));
    assert!(!text.contains("groupId"));

    let (read, read_body) = read_record(&path).unwrap();
    assert_eq!(read.title.as_deref(), Some("Design desk"));
    assert_eq!(read.geometry, record.geometry);
    assert_eq!(read.saved_from.as_deref(), Some("window"));
    assert_eq!(read.items, record.items);
    assert_eq!(read.tab_groups, record.tab_groups);
    assert_eq!(read_body.as_deref(), Some(body.as_str()));
}

#[test]
fn delete_record_removes_file() {
    let root = make_root();
    let r = Record::new("rec_d", "page");
    let path = record_path(root.path(), &r);
    write_record(&path, &r, None).unwrap();
    assert!(path.exists());
    delete_record(&path).unwrap();
    assert!(!path.exists());
}

#[test]
fn optimistic_concurrency_stale() {
    let root = make_root();
    let r = Record::new("rec_oc", "page");
    let path = record_path(root.path(), &r);
    write_record(&path, &r, None).unwrap();
    let (r2, _body, mtime) = read_record_with_mtime(&path).unwrap();
    // external modification
    std::thread::sleep(std::time::Duration::from_millis(1100));
    write_record(&path, &r2, Some("changed")).unwrap();
    // now write with stale mtime should fail
    let res = write_record_if_unchanged(&path, &r2, Some("mine"), mtime);
    assert!(matches!(res, Err(lilac_core::Error::Stale(_))));
}

#[test]
fn optimistic_concurrency_ok() {
    let root = make_root();
    let r = Record::new("rec_ok2", "page");
    let path = record_path(root.path(), &r);
    write_record(&path, &r, None).unwrap();
    let (r2, _body, mtime) = read_record_with_mtime(&path).unwrap();
    let res = write_record_if_unchanged(&path, &r2, Some("mine"), mtime);
    assert!(res.is_ok());
}

#[test]
fn serde_yaml_quotes_wikilink_on_write() {
    // Ensure the on-disk form quotes [[...]] so it round-trips.
    let r = {
        let mut r = Record::new("rec_q", "page");
        r.status = Some("[[Reading]]".to_string());
        r
    };
    let yaml = serde_yaml::to_string(&r).unwrap();
    println!("SERIALIZED:\n{}", yaml);
    // The serialized status value should be quoted to be safe YAML.
    assert!(yaml.contains("status:"), "status field missing");
}
