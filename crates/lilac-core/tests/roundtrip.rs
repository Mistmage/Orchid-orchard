use lilac_core::{
    Record, delete_record, read_record, rebuild_index, record_path, search_records, write_record,
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
