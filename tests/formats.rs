#![cfg(all(feature = "toml", feature = "json", feature = "yaml", feature = "xml", feature = "ron", feature = "json5"))]

use std::fmt::Debug;
use std::path::PathBuf;

use conflaguration::{ConfigBuilder, Error, Tree, from_file, from_json_str, from_json5_str, from_ron_str, from_toml_str, from_tree, from_xml_str, from_yaml_str};

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
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/formats")
        .join(name);
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
    Deep {
        ports: vec![],
        limit: None,
        weights: std::collections::BTreeMap::new(),
        note: "x".to_string(),
        inner: vec![],
    }
}

fn named_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/formats")
        .join(name)
}

fn check_layered(extension: &str) {
    let result = ConfigBuilder::<Fixture>::new()
        .value(base_fixture())
        .file(fixture_path(extension))
        .build();
    check_fixture(result, &expected_fixture()).expect("layered fixture should equal the expected value");
}

fn check_overlay(extension: &str) {
    let result = ConfigBuilder::<Fixture>::new()
        .value(expected_fixture())
        .overlay_file(named_path(&format!("overlay.{extension}")))
        .build();
    let expected = Fixture {
        absent: Some("filled".to_string()),
        ..expected_fixture()
    };
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
    Fixture {
        port: 8080,
        debug: true,
        ratio: 0.5,
        single: vec!["only".to_string()],
        ..base_fixture()
    }
}

#[test]
fn format_lenient_quoted_scalars() {
    let result = ConfigBuilder::<Fixture>::new()
        .value(base_fixture())
        .file(named_path("quoted.toml"))
        .build();
    check_fixture(result, &quoted_expected()).expect("quoted scalars should be parsed");
}

#[test]
fn format_lenient_bad_number() {
    let result = ConfigBuilder::<Fixture>::new()
        .value(base_fixture())
        .file(named_path("bad_port.toml"))
        .build();
    check_error(result, "eighty").expect("an unparsable port should fail naming its text");
}

#[test]
fn format_lenient_typed_layer() {
    let result = ConfigBuilder::<Fixture>::new()
        .value(base_fixture())
        .file(named_path("typed.toml"))
        .build();
    check_fixture(result, &quoted_expected()).expect("typed values should be untouched");
}

#[test]
fn format_lenient_empty_elements() {
    let result = ConfigBuilder::<Fixture>::new()
        .value(expected_fixture())
        .file(named_path("empties.xml"))
        .build();
    let expected = Fixture {
        tags: vec![],
        absent: None,
        ..expected_fixture()
    };
    check_fixture(result, &expected).expect("empty elements should become an empty list and none");
}

fn check_deep(layer: &str, expected: Deep) {
    let result = ConfigBuilder::<Deep>::new().value(base_deep()).file(named_path(layer)).build();
    check_fixture(result, &expected).expect("deep layer should load into the expected value");
}

#[test]
fn format_lenient_deep_list() {
    let expected = Deep {
        ports: vec![1, 2],
        inner: vec![Inner { retries: 3, enabled: false }],
        ..base_deep()
    };
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
    Helped {
        step: HelpedStep::Wait { ms: 1, cap: None },
    }
}

#[test]
fn format_limit_tagged_text() {
    let result = ConfigBuilder::<Holder>::new()
        .value(base_holder())
        .file(named_path("tagged.xml"))
        .build();
    check_error(result, "").expect("a tagged enum field should not be coerced from xml text");
}

fn helped_expected() -> Helped {
    Helped {
        step: HelpedStep::Wait { ms: 5, cap: Some(9) },
    }
}

#[test]
fn format_helper_xml() {
    let result = ConfigBuilder::<Helped>::new()
        .value(base_helped())
        .file(named_path("helped.xml"))
        .build();
    check_fixture(result, &helped_expected()).expect("the helpers should read xml text");
}

#[test]
fn format_helper_toml() {
    let result = ConfigBuilder::<Helped>::new()
        .value(base_helped())
        .file(named_path("helped.toml"))
        .build();
    check_fixture(result, &helped_expected()).expect("the helpers should read typed toml");
}

#[test]
fn format_helper_bad_text() {
    let result = ConfigBuilder::<Helped>::new()
        .value(base_helped())
        .file(named_path("helped_bad.xml"))
        .build();
    check_error(result, "five").expect("an unparsable ms should fail naming its text");
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct Def {
    data_path: String,
    n: u32,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Req {
    data_path: String,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptDef {
    capture: Option<Def>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptReq {
    capture: Option<Req>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptVal {
    capture: Option<serde_json::Value>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fields {
    s: String,
    v: Vec<String>,
    d: Def,
    m: std::collections::BTreeMap<String, u32>,
}

const EMPTY_FORMS: [&str; 3] = ["<config><capture/></config>", "<config><capture></capture></config>", "<config><capture>  </capture></config>"];

fn xml_tree(text: &str) -> conflaguration::Result<Tree> {
    conflaguration::tree_from_xml_str(text)
}

fn check_empty_form(text: &str) -> Result<(), String> {
    let tree = xml_tree(text).map_err(|error| error.to_string())?;
    check_json(serde_json::Value::from(tree.clone()), &serde_json::json!({"capture": null}))?;
    check_fixture(from_tree::<OptDef>(tree.clone()), &OptDef { capture: None })?;
    check_fixture(from_tree::<OptReq>(tree.clone()), &OptReq { capture: None })?;
    check_fixture(from_tree::<OptVal>(tree), &OptVal { capture: None })
}

#[test]
fn format_empty_xml_selfclosing() {
    check_empty_form(EMPTY_FORMS[0]).expect("a self-closing element should read as unit");
}

#[test]
fn format_empty_xml_open_close() {
    check_empty_form(EMPTY_FORMS[1]).expect("an open-close pair should read as unit");
}

#[test]
fn format_empty_xml_whitespace() {
    check_empty_form(EMPTY_FORMS[2]).expect("a whitespace-only element should read as unit");
}

#[test]
fn format_empty_xml_plain_fields() {
    let tree = xml_tree("<config><s/><v/><d/><m/></config>").expect("empty elements should parse");
    let expected = Fields {
        s: String::new(),
        v: vec![],
        d: Def::default(),
        m: std::collections::BTreeMap::new(),
    };
    check_fixture(from_tree::<Fields>(tree), &expected).expect("unit should read as empty for each plain type");
}

#[test]
fn format_empty_xml_root() {
    let tree = xml_tree("<config/>").expect("an empty root should parse");
    check_fixture(from_tree::<Def>(tree), &Def::default()).expect("an empty root should read as the default struct");
}

#[test]
fn format_empty_xml_roundtrip() {
    let tree = xml_tree(EMPTY_FORMS[0]).expect("an empty element should parse");
    let json = serde_json::Value::from(tree);
    let again = serde_json::from_value::<Tree>(json).expect("the json should re-enter as a tree");
    check_json(serde_json::Value::from(again.clone()), &serde_json::json!({"capture": null})).expect("unit should survive the round trip");
    check_fixture(from_tree::<OptDef>(again), &OptDef { capture: None }).expect("the round-tripped tree should read as none");
}

#[test]
fn format_empty_xml_repeated() {
    let actual = tree_json(xml_tree("<config><a/><a/></config>"));
    check_json(actual, &serde_json::json!({"a": [null, null]})).expect("repeated empty siblings should be a list of unit");
}

#[test]
fn format_empty_xml_from_str_keeps_map() {
    let actual = tree_json(from_xml_str::<Tree>("<config><capture/></config>"));
    check_json(actual, &serde_json::json!({"capture": {}})).expect("from_xml_str should keep the empty map");
}

#[test]
fn format_empty_layer_xml_option() {
    let base = OptDef { capture: None };
    let result = ConfigBuilder::<OptDef>::new()
        .value(base)
        .file(named_path("empty_capture.xml"))
        .build();
    check_fixture(result, &OptDef { capture: None }).expect("an empty element layer should leave the option none");
}

fn check_keeps_base(layer: &str) {
    let override_result = ConfigBuilder::<Fixture>::new()
        .value(expected_fixture())
        .file(named_path(layer))
        .build();
    check_fixture(override_result, &expected_fixture()).expect("an override layer of unit should keep the base");
    let overlay_result = ConfigBuilder::<Fixture>::new()
        .value(expected_fixture())
        .overlay_file(named_path(layer))
        .build();
    check_fixture(overlay_result, &expected_fixture()).expect("an overlay layer of unit should keep the base");
}

#[test]
fn format_empty_layer_xml_keeps_base() {
    check_keeps_base("empty_inner.xml");
}

#[test]
fn format_empty_layer_json_null_keeps_base() {
    check_keeps_base("empty_inner_null.json");
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct PlainDef {
    capture: Def,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct PlainReq {
    capture: Req,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S1 {
    capture: String,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S2 {
    capture: Vec<String>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S3 {
    capture: Vec<Def>,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct NullFields {
    s: String,
    v: Vec<String>,
    d: Def,
}

fn serde_read<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String> {
    serde_json::from_value::<T>(json.clone()).map_err(|error| error.to_string())
}

fn tree_read<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String> {
    let tree = serde_json::from_value::<Tree>(json.clone()).map_err(|error| error.to_string())?;
    from_tree::<T>(tree).map_err(|error| error.to_string())
}

fn agree<T: PartialEq + Debug>(oracle: Result<T, String>, got: Result<T, String>) -> Result<(), String> {
    match (oracle, got) {
        (Ok(want), Ok(have)) if want == have => Ok(()),
        (Ok(want), Ok(have)) => Err(format!("value differs: {have:?} != {want:?}")),
        (Err(_), Err(_)) => Ok(()),
        (oracle, got) => Err(format!("disagree: oracle {oracle:?}, got {got:?}")),
    }
}

fn same_as_serde<T: serde::de::DeserializeOwned + PartialEq + Debug>(json: &serde_json::Value) -> Result<(), String> {
    agree(serde_read::<T>(json), tree_read::<T>(json))
}

fn empty_capture() -> serde_json::Value {
    serde_json::json!({"capture": {}})
}

fn check_err_has<T: Debug>(result: Result<T, String>, needle: &str) -> Result<(), String> {
    match result {
        Err(text) if text.contains(needle) => Ok(()),
        Err(text) => Err(format!("error lacks {needle:?}: {text}")),
        Ok(value) => Err(format!("expected an error containing {needle:?}, got {value:?}")),
    }
}

#[test]
fn format_empty_json_option_default() {
    same_as_serde::<OptDef>(&empty_capture()).expect("an empty map should read as serde_json reads it");
    let expected = OptDef { capture: Some(Def::default()) };
    check_read(tree_read::<OptDef>(&empty_capture()), &expected).expect("an empty map should be some default");
}

#[test]
fn format_empty_json_option_required() {
    same_as_serde::<OptReq>(&empty_capture()).expect("both reads should fail");
    check_err_has(serde_read::<OptReq>(&empty_capture()), "missing field `data_path`").expect("serde_json names the field");
    check_err_has(tree_read::<OptReq>(&empty_capture()), "data_path").expect("the tree read names the field");
}

#[test]
fn format_empty_json_option_value() {
    same_as_serde::<OptVal>(&empty_capture()).expect("an empty map should read as serde_json reads it");
    let expected = OptVal { capture: Some(serde_json::json!({})) };
    check_read(tree_read::<OptVal>(&empty_capture()), &expected).expect("an empty map should be some empty object");
}

#[test]
fn format_empty_json_plain_default() {
    same_as_serde::<PlainDef>(&empty_capture()).expect("an empty map should read as serde_json reads it");
    let expected = PlainDef { capture: Def::default() };
    check_read(tree_read::<PlainDef>(&empty_capture()), &expected).expect("an empty map should be the default struct");
}

#[test]
fn format_empty_json_plain_required() {
    same_as_serde::<PlainReq>(&empty_capture()).expect("both reads should fail");
    check_err_has(serde_read::<PlainReq>(&empty_capture()), "data_path").expect("serde_json names the field");
    check_err_has(tree_read::<PlainReq>(&empty_capture()), "data_path").expect("the tree read names the field");
}

#[test]
fn format_empty_json_string() {
    same_as_serde::<S1>(&empty_capture()).expect("both reads should fail");
    check_err_has(serde_read::<S1>(&empty_capture()), "invalid type: map").expect("serde_json rejects a map for a string");
    assert!(tree_read::<S1>(&empty_capture()).is_err(), "the tree read should reject a map for a string");
}

#[test]
fn format_empty_json_vec_string() {
    same_as_serde::<S2>(&empty_capture()).expect("both reads should fail");
    check_err_has(serde_read::<S2>(&empty_capture()), "invalid type: map").expect("serde_json rejects a map for a list");
    assert!(tree_read::<S2>(&empty_capture()).is_err(), "the tree read should reject a map for a list of strings");
}

#[test]
fn format_empty_json_vec_struct() {
    let expected = S3 { capture: vec![Def::default()] };
    check_read(tree_read::<S3>(&empty_capture()), &expected).expect("a lone map should be a one-element list");
}

#[test]
fn format_empty_null_non_option() {
    let json = serde_json::json!({"s": null, "v": null, "d": null});
    let expected = NullFields {
        s: String::new(),
        v: vec![],
        d: Def::default(),
    };
    check_read(tree_read::<NullFields>(&json), &expected).expect("null should read as empty for each plain type");
    check_err_has(serde_read::<NullFields>(&json), "invalid type: null").expect("serde_json rejects the nulls");
}

#[test]
fn format_empty_layer_toml_default() {
    let base = OptDef { capture: None };
    let result = ConfigBuilder::<OptDef>::new()
        .value(base)
        .file(named_path("empty_capture.toml"))
        .build();
    check_fixture(result, &OptDef { capture: Some(Def::default()) }).expect("an empty table layer should be some default");
}

#[test]
fn format_empty_layer_toml_required() {
    let base = OptReq { capture: None };
    let result = ConfigBuilder::<OptReq>::new()
        .value(base)
        .file(named_path("empty_capture.toml"))
        .build();
    check_error(result, "data_path").expect("an empty table layer should miss the required field");
}

fn check_nonxml_empty(result: conflaguration::Result<Tree>) -> Result<(), String> {
    check_json(tree_json(result), &serde_json::json!({"capture": {}}))
}

#[test]
fn format_empty_nonxml_toml() {
    check_nonxml_empty(from_toml_str::<Tree>("capture = {}")).expect("a toml empty table should stay a map");
}

#[test]
fn format_empty_nonxml_yaml() {
    check_nonxml_empty(from_yaml_str::<Tree>("capture: {}")).expect("a yaml empty map should stay a map");
}

#[test]
fn format_empty_nonxml_json5() {
    check_nonxml_empty(from_json5_str::<Tree>("{capture: {}}")).expect("a json5 empty object should stay a map");
}

#[test]
fn format_empty_nonxml_ron() {
    check_nonxml_empty(from_ron_str::<Tree>("(capture: {})")).expect("a ron empty map should stay a map");
}

#[test]
fn ctl_empty_serde_json_option_default() {
    let expected = OptDef { capture: Some(Def::default()) };
    check_read(serde_read::<OptDef>(&empty_capture()), &expected).expect("serde_json reads an empty map as some default");
}

#[test]
fn ctl_empty_serde_json_option_required() {
    check_err_has(serde_read::<OptReq>(&empty_capture()), "missing field `data_path`").expect("serde_json names the missing field");
}

#[test]
fn ctl_empty_serde_json_option_value() {
    let expected = OptVal { capture: Some(serde_json::json!({})) };
    check_read(serde_read::<OptVal>(&empty_capture()), &expected).expect("serde_json reads an empty map as some empty object");
}

#[test]
fn ctl_empty_serde_json_plain_default() {
    let expected = PlainDef { capture: Def::default() };
    check_read(serde_read::<PlainDef>(&empty_capture()), &expected).expect("serde_json reads an empty map as the default struct");
}

#[test]
fn ctl_empty_serde_json_plain_required() {
    check_err_has(serde_read::<PlainReq>(&empty_capture()), "data_path").expect("serde_json names the missing field");
}

#[test]
fn ctl_empty_serde_json_string() {
    check_err_has(serde_read::<S1>(&empty_capture()), "invalid type: map").expect("serde_json rejects a map for a string");
}

#[test]
fn ctl_empty_serde_json_vec_string() {
    check_err_has(serde_read::<S2>(&empty_capture()), "invalid type: map").expect("serde_json rejects a map for a list of strings");
}

#[test]
fn ctl_empty_serde_json_vec_struct() {
    check_err_has(serde_read::<S3>(&empty_capture()), "invalid type: map").expect("serde_json rejects a map for a list of structs");
}

#[test]
fn ctl_empty_serde_json_null_fields() {
    let json = serde_json::json!({"s": null, "v": null, "d": null});
    check_err_has(serde_read::<NullFields>(&json), "invalid type: null").expect("serde_json rejects nulls for plain types");
}

#[test]
fn ctl_empty_tree_json_null() {
    let tree = serde_json::from_value::<Tree>(serde_json::json!({"capture": null})).expect("a null literal should build a tree");
    check_json(serde_json::Value::from(tree), &serde_json::json!({"capture": null})).expect("a hand-built unit tree should convert to null");
}

#[test]
fn ctl_empty_literal_rejects() {
    let verdict = agree(Ok(Some(Def::default())), Ok(None::<Def>));
    assert!(verdict.is_err(), "agree should reject a some against a none");
}

fn check_read<T: PartialEq + Debug>(actual: Result<T, String>, expected: &T) -> Result<(), String> {
    match actual {
        Ok(value) if &value == expected => Ok(()),
        Ok(value) => Err(format!("value differs: {value:?} != {expected:?}")),
        Err(error) => Err(format!("read failed: {error}")),
    }
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum Shape {
    Text {
        #[serde(default)]
        pattern: Option<String>,
    },
    Count {
        #[serde(default)]
        min: Option<i64>,
    },
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(untagged)]
enum ShapeRef {
    Inline(Shape),
    Named(String),
}

#[derive(Debug, Default, PartialEq, serde::Deserialize)]
struct EmptyInner {
    #[serde(default)]
    a: Option<String>,
    #[serde(default)]
    n: u32,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(untagged)]
enum InnerOrText {
    Struct(EmptyInner),
    Text(String),
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct OptInner {
    #[serde(default)]
    inner: Option<EmptyInner>,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(untagged)]
enum OptOrText {
    Opt(OptInner),
    Text(String),
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct ShapeCfg {
    items: Shape,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct ShapeRefCfg {
    items: ShapeRef,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct InnerCfg {
    inner: InnerOrText,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct OptCfg {
    opt: OptOrText,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct ValueCfg {
    x: serde_json::Value,
}

fn read_xml_unit_path<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let tree = xml_tree(text).map_err(|error| error.to_string())?;
    from_tree::<T>(tree).map_err(|error| error.to_string())
}

fn read_xml_map_path<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let tree = from_xml_str::<Tree>(text).map_err(|error| error.to_string())?;
    from_tree::<T>(tree).map_err(|error| error.to_string())
}

macro_rules! empty_enum_case {
    ($format_name:ident, $ctl_name:ident, $type:ty, $xml:expr, $expected:expr) => {
        #[test]
        fn $format_name() {
            check_read(read_xml_unit_path::<$type>($xml), &$expected).expect("an empty element should read as an empty map through deserialize_any");
        }

        #[test]
        fn $ctl_name() {
            check_read(read_xml_map_path::<$type>($xml), &$expected).expect("2.1.0's empty map path is the oracle for the same literal");
        }
    };
}

empty_enum_case!(
    format_empty_enum_adjacent_selfclosing,
    ctl_empty_enum_adjacent_selfclosing,
    ShapeCfg,
    "<cfg><items><type>text</type><value/></items></cfg>",
    ShapeCfg { items: Shape::Text { pattern: None } }
);

empty_enum_case!(
    format_empty_enum_adjacent_open_close,
    ctl_empty_enum_adjacent_open_close,
    ShapeCfg,
    "<cfg><items><type>count</type><value></value></items></cfg>",
    ShapeCfg { items: Shape::Count { min: None } }
);

empty_enum_case!(
    format_empty_enum_adjacent_content_first,
    ctl_empty_enum_adjacent_content_first,
    ShapeCfg,
    "<cfg><items><value/><type>text</type></items></cfg>",
    ShapeCfg { items: Shape::Text { pattern: None } }
);

empty_enum_case!(
    format_empty_enum_untagged_adjacent_selfclosing,
    ctl_empty_enum_untagged_adjacent_selfclosing,
    ShapeRefCfg,
    "<cfg><items><type>text</type><value/></items></cfg>",
    ShapeRefCfg {
        items: ShapeRef::Inline(Shape::Text { pattern: None })
    }
);

empty_enum_case!(
    format_empty_enum_untagged_adjacent_open_close,
    ctl_empty_enum_untagged_adjacent_open_close,
    ShapeRefCfg,
    "<cfg><items><type>text</type><value></value></items></cfg>",
    ShapeRefCfg {
        items: ShapeRef::Inline(Shape::Text { pattern: None })
    }
);

empty_enum_case!(
    format_empty_enum_untagged_struct,
    ctl_empty_enum_untagged_struct,
    InnerCfg,
    "<cfg><inner/></cfg>",
    InnerCfg {
        inner: InnerOrText::Struct(EmptyInner::default())
    }
);

empty_enum_case!(
    format_empty_enum_untagged_option,
    ctl_empty_enum_untagged_option,
    OptCfg,
    "<cfg><opt><inner/></opt></cfg>",
    OptCfg {
        opt: OptOrText::Opt(OptInner { inner: Some(EmptyInner::default()) })
    }
);

empty_enum_case!(
    format_empty_enum_value_target,
    ctl_empty_enum_value_target,
    ValueCfg,
    "<cfg><x/></cfg>",
    ValueCfg { x: serde_json::json!({}) }
);

fn read_json_through_tree<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let tree = from_json_str::<Tree>(text).map_err(|error| error.to_string())?;
    from_tree::<T>(tree).map_err(|error| error.to_string())
}

fn read_json_with_serde<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    serde_json::from_str::<T>(text).map_err(|error| error.to_string())
}

#[test]
fn format_empty_enum_json_null_value_target() {
    let expected = ValueCfg { x: serde_json::json!({}) };
    check_read(read_json_through_tree::<ValueCfg>(r#"{"x": null}"#), &expected).expect("a json null reads as an empty element through the tree, R3's stated cost");
}

#[test]
fn ctl_empty_enum_json_null_value_target() {
    let expected = ValueCfg { x: serde_json::Value::Null };
    check_read(read_json_with_serde::<ValueCfg>(r#"{"x": null}"#), &expected).expect("serde_json reads a json null into a Value as Null");
}

#[test]
fn format_empty_enum_json_null_untagged_option() {
    let expected = OptCfg {
        opt: OptOrText::Opt(OptInner { inner: Some(EmptyInner::default()) }),
    };
    check_read(read_json_through_tree::<OptCfg>(r#"{"opt": {"inner": null}}"#), &expected).expect("a json null buffers as an empty element, so the untagged option reads Some, R3's stated cost");
}

#[test]
fn ctl_empty_enum_json_null_untagged_option() {
    let expected = OptCfg {
        opt: OptOrText::Opt(OptInner { inner: None }),
    };
    check_read(read_json_with_serde::<OptCfg>(r#"{"opt": {"inner": null}}"#), &expected).expect("serde_json reads a json null into an untagged option as None");
}
