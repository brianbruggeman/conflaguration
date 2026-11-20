#![cfg(any(feature = "xml", feature = "ron", feature = "json5"))]
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

#[cfg(any(feature = "xml", feature = "ron", feature = "json5"))]
fn load(extension: &str) -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/formats")
        .join(format!("fixture.{extension}"));
    from_file::<Fixture>(&path).expect("fixture should load")
}

#[cfg(feature = "xml")]
#[test]
fn format_single_xml() {
    assert_eq!(load("xml"), expected_fixture());
}

#[cfg(feature = "ron")]
#[test]
fn format_single_ron() {
    assert_eq!(load("ron"), expected_fixture());
}

#[cfg(feature = "json5")]
#[test]
fn format_single_json5() {
    assert_eq!(load("json5"), expected_fixture());
}
