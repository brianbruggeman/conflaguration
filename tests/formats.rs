#![cfg(all(feature = "toml", feature = "json", feature = "yaml", feature = "xml", feature = "ron", feature = "json5"))]

use std::fmt::Debug;
use std::path::PathBuf;

use conflaguration::{ConfigBuilder, Error, Tree, from_file, from_tree, from_json5_str, from_json_str, from_ron_str, from_toml_str, from_xml_str, from_yaml_str};

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

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Deep {
    ports: Vec<u16>,
    limit: Option<u32>,
    weights: std::collections::BTreeMap<String, u32>,
    note: String,
    inner: Vec<Inner>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Holder {
    step: Step,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum Step {
    Wait { ms: u32 },
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Helped {
    step: HelpedStep,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum HelpedStep {
    Wait {
        #[serde(deserialize_with = "conflaguration::lenient::deserialize")]
        ms: u32,
        #[serde(default, deserialize_with = "conflaguration::lenient::option")]
        cap: Option<u64>,
    },
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

fn check_error<T>(result: conflaguration::Result<T>, needle: &str) -> Result<(), String> {
    match result {
        Err(Error::ValueDeserialize(error)) if error.to_string().contains(needle) => Ok(()),
        Err(error) => Err(format!("error does not match {needle:?}: {error}")),
        Ok(_) => Err("load succeeded where an error was expected".to_string()),
    }
}

fn base_fixture() -> Fixture {
    Fixture {
        name: "base".to_string(),
        port: 1,
        debug: false,
        ratio: 0.0,
        tags: vec![],
        single: vec![],
        inner: Inner { retries: 0, enabled: true },
        absent: None,
    }
}

fn base_deep() -> Deep {
    Deep { ports: vec![], limit: None, weights: std::collections::BTreeMap::new(), note: "x".to_string(), inner: vec![] }
}

fn named_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/formats").join(name)
}

fn check_layered(extension: &str) {
    let result = ConfigBuilder::<Fixture>::new().value(base_fixture()).file(fixture_path(extension)).build();
    check_fixture(result, &expected_fixture()).expect("layered fixture should equal the expected value");
}

fn check_overlay(extension: &str) {
    let result = ConfigBuilder::<Fixture>::new()
        .value(expected_fixture())
        .overlay_file(named_path(&format!("overlay.{extension}")))
        .build();
    let expected = Fixture { absent: Some("filled".to_string()), ..expected_fixture() };
    check_fixture(result, &expected).expect("overlay should fill only the absent field");
}

#[test]
fn format_layered_toml() {
    check_layered("toml");
}

#[test]
fn format_overlay_toml() {
    check_overlay("toml");
}

#[test]
fn format_layered_json() {
    check_layered("json");
}

#[test]
fn format_overlay_json() {
    check_overlay("json");
}

#[test]
fn format_layered_yaml() {
    check_layered("yaml");
}

#[test]
fn format_overlay_yaml() {
    check_overlay("yaml");
}

#[test]
fn format_layered_xml() {
    check_layered("xml");
}

#[test]
fn format_overlay_xml() {
    check_overlay("xml");
}

#[test]
fn format_layered_ron() {
    check_layered("ron");
}

#[test]
fn format_overlay_ron() {
    check_overlay("ron");
}

#[test]
fn format_layered_json5() {
    check_layered("json5");
}

#[test]
fn format_overlay_json5() {
    check_overlay("json5");
}

fn quoted_expected() -> Fixture {
    Fixture { port: 8080, debug: true, ratio: 0.5, single: vec!["only".to_string()], ..base_fixture() }
}

#[test]
fn format_lenient_quoted_scalars() {
    let result = ConfigBuilder::<Fixture>::new().value(base_fixture()).file(named_path("quoted.toml")).build();
    check_fixture(result, &quoted_expected()).expect("quoted scalars should be parsed");
}

#[test]
fn format_lenient_bad_number() {
    let result = ConfigBuilder::<Fixture>::new().value(base_fixture()).file(named_path("bad_port.toml")).build();
    check_error(result, "eighty").expect("an unparsable port should fail naming its text");
}

#[test]
fn format_lenient_typed_layer() {
    let result = ConfigBuilder::<Fixture>::new().value(base_fixture()).file(named_path("typed.toml")).build();
    check_fixture(result, &quoted_expected()).expect("typed values should be untouched");
}

#[test]
fn format_lenient_empty_elements() {
    let result = ConfigBuilder::<Fixture>::new().value(expected_fixture()).file(named_path("empties.xml")).build();
    let expected = Fixture { tags: vec![], absent: None, ..expected_fixture() };
    check_fixture(result, &expected).expect("empty elements should become an empty list and none");
}

fn check_deep(layer: &str, expected: Deep) {
    let result = ConfigBuilder::<Deep>::new().value(base_deep()).file(named_path(layer)).build();
    check_fixture(result, &expected).expect("deep layer should load into the expected value");
}

#[test]
fn format_lenient_deep_list() {
    let expected = Deep { ports: vec![1, 2], inner: vec![Inner { retries: 3, enabled: false }], ..base_deep() };
    check_deep("deep_list.xml", expected);
}

#[test]
fn format_lenient_deep_option() {
    check_deep("deep_option.xml", Deep { limit: Some(7), ..base_deep() });
}

#[test]
fn format_lenient_deep_map() {
    let weights = std::collections::BTreeMap::from([("a".to_string(), 1), ("b".to_string(), 2)]);
    check_deep("deep_map.xml", Deep { weights, ..base_deep() });
}

#[test]
fn format_lenient_deep_empty() {
    check_deep("deep_empty.xml", Deep { note: String::new(), ..base_deep() });
}

#[test]
fn ctl_formats_check_error_accepts_matching() {
    let failure: conflaguration::Result<Fixture> = Err(Error::ValueDeserialize(serde_value::DeserializerError::Custom("invalid value \"eighty\"".to_string())));
    check_error(failure, "eighty").expect("a matching value error should be accepted");
}

#[test]
fn ctl_formats_check_error_rejects_success() {
    let result = check_error(Ok(base_fixture()), "eighty");
    assert!(result.is_err(), "a successful load must be rejected");
}

fn check_tree(tree: Tree) {
    check_fixture(from_tree::<Fixture>(tree), &expected_fixture()).expect("tree should load into the expected value");
}

fn tree_of(extension: &str) -> Tree {
    let text = read_fixture(&format!("fixture.{extension}"));
    let parsed = match extension {
        "toml" => from_toml_str::<Tree>(&text),
        "json" => from_json_str::<Tree>(&text),
        "yaml" => from_yaml_str::<Tree>(&text),
        "xml" => from_xml_str::<Tree>(&text),
        "ron" => from_ron_str::<Tree>(&text),
        _ => from_json5_str::<Tree>(&text),
    };
    parsed.expect("fixture should parse as a tree")
}

#[test]
fn format_from_tree_toml() {
    check_tree(tree_of("toml"));
}

#[test]
fn format_from_tree_json() {
    check_tree(tree_of("json"));
}

#[test]
fn format_from_tree_yaml() {
    check_tree(tree_of("yaml"));
}

#[test]
fn format_from_tree_xml() {
    check_tree(tree_of("xml"));
}

#[test]
fn format_from_tree_ron() {
    check_tree(tree_of("ron"));
}

#[test]
fn format_from_tree_json5() {
    check_tree(tree_of("json5"));
}

#[test]
fn format_from_tree_json_value() {
    let parsed: serde_json::Value = serde_json::from_str(&read_fixture("fixture.json")).expect("fixture.json should be valid json");
    let tree = serde_json::from_value::<Tree>(parsed).expect("a json value should read as a tree");
    check_tree(tree);
}

#[test]
fn format_from_tree_bad_number() {
    let text = read_fixture("fixture.xml").replace("<port>8080</port>", "<port>eighty</port>");
    let tree = from_xml_str::<Tree>(&text).expect("the xml should parse as a tree");
    check_error(from_tree::<Fixture>(tree), "eighty").expect("an unparsable port should fail naming its text");
}

fn base_holder() -> Holder {
    Holder { step: Step::Wait { ms: 1 } }
}

fn base_helped() -> Helped {
    Helped { step: HelpedStep::Wait { ms: 1, cap: None } }
}

#[test]
fn format_limit_tagged_text() {
    let result = ConfigBuilder::<Holder>::new().value(base_holder()).file(named_path("tagged.xml")).build();
    check_error(result, "").expect("a tagged enum field should not be coerced from xml text");
}

fn helped_expected() -> Helped {
    Helped { step: HelpedStep::Wait { ms: 5, cap: Some(9) } }
}

#[test]
fn format_helper_xml() {
    let result = ConfigBuilder::<Helped>::new().value(base_helped()).file(named_path("helped.xml")).build();
    check_fixture(result, &helped_expected()).expect("the helpers should read xml text");
}

#[test]
fn format_helper_toml() {
    let result = ConfigBuilder::<Helped>::new().value(base_helped()).file(named_path("helped.toml")).build();
    check_fixture(result, &helped_expected()).expect("the helpers should read typed toml");
}

#[test]
fn format_helper_bad_text() {
    let result = ConfigBuilder::<Helped>::new().value(base_helped()).file(named_path("helped_bad.xml")).build();
    check_error(result, "five").expect("an unparsable ms should fail naming its text");
}
