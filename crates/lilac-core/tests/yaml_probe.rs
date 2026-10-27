use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct Rec {
    id: String,
    r#type: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_yaml::Value>,
}

#[test]
fn probe_bare_wikilink_status() {
    let yaml = "id: rec_1\ntype: page\nstatus: [[Reading]]\n";
    println!("YAML input:\n{}", yaml);
    match serde_yaml::from_str::<Rec>(yaml) {
        Ok(r) => println!("OK (bare status): {:?}", r),
        Err(e) => println!("ERR (bare status): {}", e),
    }
}

#[test]
fn probe_quoted_wikilink_status() {
    let yaml = "id: rec_1\ntype: page\nstatus: \"[[Reading]]\"\n";
    println!("YAML input:\n{}", yaml);
    match serde_yaml::from_str::<Rec>(yaml) {
        Ok(r) => println!("OK (quoted status): {:?}", r),
        Err(e) => println!("ERR (quoted status): {}", e),
    }
}

#[test]
fn probe_bare_wikilink_key() {
    // property name as wikilink key
    let yaml = "\"[[status]]\": \"[[Reading]]\"\nid: rec_1\ntype: page\n";
    println!("YAML input:\n{}", yaml);
    match serde_yaml::from_str::<Rec>(yaml) {
        Ok(r) => println!("OK (bracket key): {:?}", r),
        Err(e) => println!("ERR (bracket key): {}", e),
    }
}

#[test]
fn probe_tags_mixed() {
    let yaml = "id: rec_1\ntype: page\ntags:\n- rust\n- '[[Rust]]'\n";
    println!("YAML input:\n{}", yaml);
    match serde_yaml::from_str::<Rec>(yaml) {
        Ok(r) => println!("OK (tags): {:?}", r),
        Err(e) => println!("ERR (tags): {}", e),
    }
}

#[test]
fn probe_nested_extra() {
    let yaml = "id: rec_1\ntype: page\nmy_company:\n  project: Orchid\n  priority: high\nrating: 5\n";
    println!("YAML input:\n{}", yaml);
    match serde_yaml::from_str::<Rec>(yaml) {
        Ok(r) => println!("OK (extra): {:?}", r),
        Err(e) => println!("ERR (extra): {}", e),
    }
}
