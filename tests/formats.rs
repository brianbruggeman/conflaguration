#![cfg(all(feature = "toml", feature = "json", feature = "yaml", feature = "xml", feature = "ron", feature = "json5"))]

use std::fmt::Debug;
use std::path::PathBuf;

use conflaguration::from_file;

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
