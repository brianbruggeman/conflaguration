#![cfg(all(feature = "toml", feature = "json", feature = "yaml", feature = "xml", feature = "ron", feature = "json5"))]

use std::fmt::Debug;
use std::path::PathBuf;

use conflaguration::{Tree, from_file, from_json5_str, from_json_str, from_ron_str, from_toml_str, from_xml_str, from_yaml_str};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fixture {
    name: String,
    port: u16,
    debug: bool,
    ratio: f64,
    tags: Vec<String>,
    single: Vec<String>,
    inner: Inner,
    absent: Option<String>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Inner {
    retries: u32,
    enabled: bool,
}

fn expected_fixture() -> Fixture {
    Fixture {
        name: "review".to_string(),
        port: 8080,
        debug: true,
        ratio: 0.5,
        tags: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        single: vec!["only".to_string()],
        inner: Inner { retries: 3, enabled: false },
        absent: None,
    }
}

fn fixture_path(extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/formats")
        .join(format!("fixture.{extension}"))
}

fn check_fixture<T: PartialEq + Debug>(actual: conflaguration::Result<T>, expected: &T) -> Result<(), String> {
    match actual {
        Ok(value) if &value == expected => Ok(()),
        Ok(value) => Err(format!("value differs: {value:?} != {expected:?}")),
        Err(error) => Err(format!("load failed: {error}")),
    }
}

fn check_from_file(extension: &str) {
    let result = check_fixture(from_file::<Fixture>(&fixture_path(extension)), &expected_fixture());
    result.expect("fixture should load into the expected value");
}

#[test]
fn format_from_file_toml() {
    check_from_file("toml");
}

#[test]
fn format_from_file_json() {
    check_from_file("json");
}

#[test]
fn format_from_file_yaml() {
    check_from_file("yaml");
}

#[test]
fn format_from_file_xml() {
    check_from_file("xml");
}

#[test]
fn format_from_file_ron() {
    check_from_file("ron");
}

#[test]
fn format_from_file_json5() {
    check_from_file("json5");
}

#[test]
fn ctl_formats_check_fixture_accepts_equal() {
    let result = check_fixture(Ok(expected_fixture()), &expected_fixture());
    result.expect("an equal value should be accepted");
}

#[test]
fn ctl_formats_check_fixture_rejects_single_empty() {
    let altered = Fixture { single: vec![], ..expected_fixture() };
    let result = check_fixture(Ok(altered), &expected_fixture());
    assert!(result.is_err(), "a value with an empty single must be rejected");
}

const PAYLOAD_TYPED: &str = r#"{"args":{"id":["A","B"]},"id":1,"method":"tools/call","ok":true,"ratio":0.5}"#;
const PAYLOAD_XML_TEXT: &str = r#"{"args":{"id":["A","B"]},"id":"1","method":"tools/call","ok":"true","ratio":"0.5"}"#;

fn check_json(actual: serde_json::Value, expected: &serde_json::Value) -> Result<(), String> {
    if &actual == expected { Ok(()) } else { Err(format!("json differs: {actual} != {expected}")) }
}

fn json_of(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("expected literal should be valid json")
}

fn read_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/formats").join(name);
    std::fs::read_to_string(&path).expect("fixture file should be readable")
}

fn tree_json(result: conflaguration::Result<Tree>) -> serde_json::Value {
    serde_json::Value::from(result.expect("fixture should load as a tree"))
}

#[test]
fn format_tree_toml() {
    let actual = tree_json(from_toml_str::<Tree>(&read_fixture("payload.toml")));
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("toml payload should match");
}

#[test]
fn format_tree_json() {
    let actual = tree_json(from_json_str::<Tree>(&read_fixture("payload.json")));
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("json payload should match");
}

#[test]
fn format_tree_yaml() {
    let actual = tree_json(from_yaml_str::<Tree>(&read_fixture("payload.yaml")));
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("yaml payload should match");
}

#[test]
fn format_tree_xml() {
    let actual = tree_json(from_xml_str::<Tree>(&read_fixture("payload.xml")));
    check_json(actual, &json_of(PAYLOAD_XML_TEXT)).expect("xml payload should match");
}

#[test]
fn format_tree_ron() {
    let actual = tree_json(from_ron_str::<Tree>(&read_fixture("payload.ron")));
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("ron payload should match");
}

#[test]
fn format_tree_json5() {
    let actual = tree_json(from_json5_str::<Tree>(&read_fixture("payload.json5")));
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("json5 payload should match");
}

#[test]
fn format_tree_datetime() {
    let actual = tree_json(from_toml_str::<Tree>(&read_fixture("datetime.toml")));
    check_json(actual, &json_of(r#"{"when":"1979-05-27T07:32:00Z"}"#)).expect("datetime should collapse to its string");
}

#[test]
fn format_tree_infinity() {
    let actual = tree_json(from_toml_str::<Tree>(&read_fixture("inf.toml")));
    check_json(actual, &json_of(r#"{"x":null}"#)).expect("infinity should become null");
}

#[test]
fn ctl_formats_check_json_accepts_incumbent_path() {
    let actual = serde_json::from_str::<serde_json::Value>(&read_fixture("payload.json")).expect("payload.json should parse");
    check_json(actual, &json_of(PAYLOAD_TYPED)).expect("the incumbent path should match the typed literal");
}

#[test]
fn ctl_formats_check_json_rejects_altered_id() {
    let actual = tree_json(from_json_str::<Tree>(&read_fixture("payload.json")));
    let altered = json_of(r#"{"args":{"id":["A","B"]},"id":2,"method":"tools/call","ok":true,"ratio":0.5}"#);
    assert!(check_json(actual, &altered).is_err(), "a different id must be rejected");
}

#[test]
fn ctl_formats_quick_xml_value_keeps_text_wrapper() {
    let actual = quick_xml::de::from_str::<serde_json::Value>(&read_fixture("fixture.xml")).expect("fixture.xml should parse as json value");
    assert_eq!(actual["tags"], json_of(r#"{"$text":"c"}"#), "incumbent path keeps only the last repeated element");
}

#[test]
fn ctl_formats_tree_differs_from_quick_xml_value() {
    let text = read_fixture("payload.xml");
    let incumbent = quick_xml::de::from_str::<serde_json::Value>(&text).expect("payload.xml should parse as json value");
    let grouped = tree_json(from_xml_str::<Tree>(&text));
    assert_ne!(grouped, incumbent, "the tree must differ from the incumbent reading");
}
