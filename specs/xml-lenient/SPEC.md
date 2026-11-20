# xml-lenient -- XML (and RON, JSON5) for conflaguration, typed

status: draft, round 5 (rounds 1-4 REFUSE 13, 7, 3, 1 reasons; each amended. Round 5 also adds
R5's TOML-datetime and non-finite-float rules and R8, both from proxima's census for PROXIMA-50)
owner: brian
created: 2026-11-20

Why now: the owner wants every proxima pipeline step configurable in any format, through
conflaguration ("conflaguration should let us do anything: json, toml, yaml, xml, etc.";
"conflaguration is nested config using Setting"), recorded in
`/Users/brianbruggeman/repos/slot-0/pm/topics/agentic-mcp.md`, section "any format, every step".
Every `file:line` below is in this repo (`slot-0/conflaguration-publish`) at `beded1a` (main after
CONFLAG-2; this branch adds only `specs/` over it), read 2026-11-20. Ticket evidence lives under
`/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/`
(`<tickets>` below). quick-xml citations are to 0.41.0 (the version proxima locks), serde-value to 0.7.0.

## problem

conflaguration loads TOML, JSON and YAML only (`src/file.rs:28-53`, features `Cargo.toml:62-64`),
so a config written in XML, RON or JSON5 cannot be loaded at all; its builder merges file
layers through an untyped `serde_value::Value` (`src/file.rs:57-82,131-139`), which cannot carry
XML's text-only scalars (`"5"` where a `u16` is wanted; quick-xml wraps element text as
`{"$text": "5"}` when no type guides it, `de/map.rs:677-683`) or its repeated sibling elements (a
map keeps one); and a caller that has already parsed a document (proxima parses a whole config
file, then hands each pipe's subtree to that pipe's factory) has no typed, lenient way to read a
subtree -- measured by one fixture config, written once in each of the six formats, loading into
the same typed struct with the same values through `from_file`, through the builder over a base
value, and through a parsed tree.

## refutation condition

If the XML fixture -- plain structs, lists, maps and options, none of serde's buffered paths --
cannot reach the same typed value as its TOML twin without the caller adding per-field annotations
(`#[serde(deserialize_with = ...)]`) to its config structs, the "any format, same struct" frame is
wrong for XML and this spec is struck. (R4 states and AC11 pins the one place where annotations
remain the caller's: fields behind serde's buffer.)

## requirements

| id | requirement | testable in isolation |
|---|---|---|
| R1 | feature `xml` (quick-xml with its `serialize` feature, which provides `quick_xml::de`, quick-xml `src/lib.rs:57-58`): `from_xml_str::<T>(&str) -> Result<T>` deserializes straight into `T`; `from_file` maps `.xml` to it. Element text becomes the typed field; repeated sibling elements fill a `Vec` field, including with exactly one; an absent element leaves an `Option` field `None`. Every gate in `src/` that reads `any(feature = "toml", feature = "json", feature = "yaml")` (the 8 lines in L13 below that contain it) widens to the six format features, so the `file` module (`src/lib.rs:274`), `from_file`, the builder's file methods (`src/builder.rs:149`) and the merge errors (`src/lib.rs:402,407`) exist under `xml`, `ron` or `json5` alone | yes |
| R2 | features `ron` and `json5`: `from_ron_str` / `from_json5_str` and the `.ron` / `.json5` extensions in `from_file`, each usable with that feature alone, wired as `toml` / `json` / `yaml` are (`src/file.rs:9-53`) | yes |
| R3 | builder layers after the first source (`merge_file`, `src/file.rs:131-139`, reached by `.file()` on a non-empty chain, `src/builder.rs:157-158,219-225`, and by `.overlay_file()`) accept `.xml`, `.ron`, `.json5`: `file_to_value` parses them into the merge tree; the XML tree is built by `Tree`'s visitor (R5), so element text is a scalar, never a `{"$text": ..}` map, and repeated siblings are one list in document order | yes |
| R4 | the merged tree deserializes into `T` through a lenient deserializer over `serde_value::Value`, implemented in full so every child of a struct, map, list or option is deserialized by the same lenient deserializer (serde-value's own struct/seq access rebuilds children with its strict `ValueDeserializer`, `serde-value de.rs:329-354`): a string where `T` wants a bool, an integer or a float is parsed (`FromStr`, as env values are, `crates/conflaguration-core/src/lib.rs:22-27`); a single value where `T` wants a list becomes a one-element list; an empty map (an empty XML element) where `T` wants a string, a list or an option becomes `""`, `[]` or `None`. A string that does not parse fails with `Error::ValueDeserialize` whose text contains that string. Values that already have the wanted type are untouched. Limit, stated and pinned: fields serde reaches only through its own buffer (internally tagged and untagged enums, `#[serde(flatten)]`) are deserialized by serde's strict buffered deserializer, not by this one, so an XML string there is not coerced and the load fails with `Error::ValueDeserialize` | yes |
| R5 | `conflaguration::Tree`: a free-form value for fields whose shape is not known in advance (a request payload). Its `Deserialize` visitor keeps maps, lists and scalars (bools, integers, floats and strings keep their kinds), groups repeated map keys into a list (never last-one-wins), and turns a map whose only key is `$text` into that scalar. It deserializes from all six formats and from any self-describing deserializer (a `serde_json::Value` included); with feature `json`, `serde_json::Value::from(Tree)` converts it. A TOML datetime (which the `toml` crate hands a visitor as a map whose only key is `$__toml_private_datetime`) becomes its RFC 3339 string, as `$text` does; a non-finite float (TOML `inf`, `nan`) converts to JSON `null`, which `serde_json` cannot otherwise hold (proxima's own TOML converter does both today, `proxima/proxima-config/src/lib.rs:402-418`) | yes |
| R6 | incumbents unchanged: (a) the full suite passes with its post-CONFLAG-2 count plus the new tests; (b) the only existing lines of `src/` and `crates/` that are removed or rewritten are the 13 in L13 (8 widened gates, 3 doc lines that list the file extensions, and the 2 strict `T::deserialize(base_value)` calls in `merge_file` / `merge_mapping`); every other change is an added line, so no existing item's signature, bound or body changes beyond those 13; (c) `default = ["std"]` is unchanged | yes |
| R7 | `from_tree::<T: DeserializeOwned>(tree: Tree) -> Result<T>` deserializes a `Tree` into `T` through R4's lenient deserializer, so a typed config is read out of a document parsed once, without knowing `T` at parse time | yes |
| R8 | for the fields R4's limit leaves out, `conflaguration::lenient::deserialize` (for `T: Deserialize + FromStr`) and `conflaguration::lenient::option` (for `Option<T>`) are `deserialize_with` functions that accept the typed value or its text (`FromStr`), so one attribute makes such a field load from XML; a text that does not parse fails with an error whose text contains it. Exported under any of the six format features | yes |

## literals

The fixture structs (in `tests/formats.rs`, test-only; `Fixture` and `Inner` also in
`tests/formats_single.rs`):

```rust
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fixture { name: String, port: u16, debug: bool, ratio: f64,
                 tags: Vec<String>, single: Vec<String>, inner: Inner, absent: Option<String> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Inner { retries: u32, enabled: bool }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Deep { ports: Vec<u16>, limit: Option<u32>, weights: std::collections::BTreeMap<String, u32>,
              note: String, inner: Vec<Inner> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Holder { step: Step }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum Step { Wait { ms: u32 } }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Helped { step: HelpedStep }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
enum HelpedStep { Wait {
    #[serde(deserialize_with = "conflaguration::lenient::deserialize")] ms: u32,
    #[serde(default, deserialize_with = "conflaguration::lenient::option")] cap: Option<u64> } }
```

- F1, the expected value: `Fixture { name: "review", port: 8080, debug: true, ratio: 0.5, tags:
  ["a", "b", "c"], single: ["only"], inner: Inner { retries: 3, enabled: false }, absent: None }`.
- F2, the XML source `tests/fixtures/formats/fixture.xml`:
  `<config><name>review</name><port>8080</port><debug>true</debug><ratio>0.5</ratio><tags>a</tags><tags>b</tags><tags>c</tags><single>only</single><inner><retries>3</retries><enabled>false</enabled></inner></config>`.
  The TOML, JSON, YAML, RON and JSON5 twins are F1 written natively, `tests/fixtures/formats/fixture.{toml,json,yaml,ron,json5}`.
  F2x is F2 with `<port>eighty</port>` in place of `<port>8080</port>` (an inline string in the test).
- B0, the base value layers go over: `Fixture { name: "base", port: 1, debug: false, ratio: 0.0,
  tags: [], single: [], inner: Inner { retries: 0, enabled: true }, absent: None }`.
- F5, an overlay layer `tests/fixtures/formats/overlay.{toml,json,yaml,xml,ron,json5}` setting
  only `absent` to `"filled"` and `port` to `9` (XML: `<config><absent>filled</absent><port>9</port></config>`).
- F3, a free-form payload `tests/fixtures/formats/payload.{xml,toml,json,yaml,ron,json5}`: `method`
  `"tools/call"`, `id` `1`, `ratio` `0.5`, `ok` `true`, `args.id` `["A", "B"]`, each written natively
  (XML: `<payload><method>tools/call</method><id>1</id><ratio>0.5</ratio><ok>true</ok><args><id>A</id><id>B</id></args></payload>`).
  As `Tree` converted to JSON (keys sorted: `serde_json` without `preserve_order`), the five typed
  formats give exactly F3J `{"args":{"id":["A","B"]},"id":1,"method":"tools/call","ok":true,"ratio":0.5}`;
  XML gives exactly F3X `{"args":{"id":["A","B"]},"id":"1","method":"tools/call","ok":"true","ratio":"0.5"}`
  (XML has no scalar types; a free-form `Tree` has no target type to coerce to).
- F4, a TOML layer over B0 with quoted scalars: `port = "8080"`, `debug = "true"`, `ratio = "0.5"`,
  `single = "only"`; F4T, the same four fields typed: `port = 8080`, `debug = true`, `ratio = 0.5`,
  `single = ["only"]`; F4x: `port = "eighty"`; F4E, an XML layer over F1:
  `<config><tags/><absent/></config>`.
- D0, the base value for `Deep`: `Deep { ports: [], limit: None, weights: {}, note: "x", inner: [] }`.
  Four XML layers over D0, one per child kind of R4, in `tests/fixtures/formats/`:
  - DL `deep_list.xml` `<deep><ports>1</ports><ports>2</ports><inner><retries>3</retries><enabled>false</enabled></inner></deep>`,
    expected D0 with `ports: [1, 2]`, `inner: [Inner { retries: 3, enabled: false }]` (typed list items; a single struct where a list is wanted)
  - DO `deep_option.xml` `<deep><limit>7</limit></deep>`, expected D0 with `limit: Some(7)` (a typed option child)
  - DM `deep_map.xml` `<deep><weights><a>1</a><b>2</b></weights></deep>`, expected D0 with `weights: {"a": 1, "b": 2}` (typed map values)
  - DE `deep_empty.xml` `<deep><note/></deep>`, expected D0 with `note: ""` (an empty element on a string field)
- H0, the base value for `Holder`: `Holder { step: Step::Wait { ms: 1 } }`; HX, the XML layer
  `tests/fixtures/formats/tagged.xml` `<holder><step><kind>Wait</kind><ms>5</ms></step></holder>`.
- G0, the base value for `Helped`: `Helped { step: HelpedStep::Wait { ms: 1, cap: None } }`; three
  layers over G0 in `tests/fixtures/formats/`: GX `helped.xml`
  `<helped><step><kind>Wait</kind><ms>5</ms><cap>9</cap></step></helped>`; GT `helped.toml`
  `[step]` `kind = "Wait"` `ms = 5` `cap = 9`; GB `helped_bad.xml`
  `<helped><step><kind>Wait</kind><ms>five</ms></step></helped>`. GX and GT both give
  `Helped { step: HelpedStep::Wait { ms: 5, cap: Some(9) } }`.
- TD, `tests/fixtures/formats/datetime.toml`: `when = 1979-05-27T07:32:00Z`; as `Tree` converted to
  JSON it gives exactly `{"when":"1979-05-27T07:32:00Z"}`. TI, `tests/fixtures/formats/inf.toml`:
  `x = inf`; as `Tree` converted to JSON it gives exactly `{"x":null}`.
- L13, the lines of `beded1a` that R6(b) allows to be removed or rewritten, in `git diff` order
  (each prefixed with `-` as `git diff` prints a removed line):

```
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-    /// Format detected by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`).
-/// Load config from a file, detecting format by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`).
-    Ok(T::deserialize(base_value)?)
-    Ok(T::deserialize(base_value)?)
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-/// extension (`.toml`, `.json`, `.yaml`, `.yml`). Other extensions are rejected.
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
-    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
-    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
```

  Sources, in order: `src/builder.rs:149,155`; `src/file.rs:26,138,150`; `src/lib.rs:271,274,278,279,282,285,402,407`.

## architecture

```rust
// src/file.rs, beside from_toml_str / from_json_str / from_yaml_str (:9-24)
#[cfg(feature = "xml")]   pub fn from_xml_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
#[cfg(feature = "ron")]   pub fn from_ron_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
#[cfg(feature = "json5")] pub fn from_json5_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
pub fn from_tree<T: DeserializeOwned>(tree: Tree) -> Result<T>;   // Lenient over the tree
// from_file / file_to_value gain the three extension arms (added lines before `other =>`);
// file_to_value's XML arm deserializes a Tree (R5) and converts it into serde_value::Value.
// merge_file / merge_mapping: `Ok(T::deserialize(base_value)?)` (:138, :150) becomes
// `Ok(T::deserialize(Lenient(base_value))?)`. Lenient(serde_value::Value) is a full Deserializer
// whose MapAccess / SeqAccess / EnumAccess / option hand every child to Lenient again; scalar
// methods parse a Value::String with FromStr; deserialize_seq wraps a non-Seq in a one-element Seq;
// an empty Map becomes "" / [] / None for deserialize_string / _seq / _option; deserialize_any
// visits the value as it is (serde's buffered paths enter here, hence R4's stated limit).

// src/tree.rs (new), re-exported from lib.rs with from_tree and the new from_*_str
pub struct Tree(/* serde_value::Value */);   // Deserialize: group repeated keys; collapse a lone `$text`
#[cfg(feature = "json")] impl From<Tree> for serde_json::Value;

// src/lenient.rs (new), `pub mod lenient` in lib.rs under the six-feature gate
pub fn deserialize<'de, D: Deserializer<'de>, T: Deserialize<'de> + FromStr>(d: D) -> Result<T, D::Error>
where T::Err: Display;
pub fn option<'de, D: Deserializer<'de>, T: Deserialize<'de> + FromStr>(d: D) -> Result<Option<T>, D::Error>
where T::Err: Display;
// both read one value through deserialize_any: a string goes through FromStr, any other scalar is
// handed to T::deserialize; `option` maps unit / none to None
```

Cargo features: `xml = ["dep:serde", "dep:quick-xml", "dep:serde-value"]` with
`cargo add --optional quick-xml --features serialize`; `ron = [...,"dep:ron"]`, `json5 = [...,"dep:json5"]`
via `cargo add --optional`. New error variants (`Xml`, `Ron`, `Json5`) are added lines in
`src/lib.rs`'s `Error`.

### decisions

| decision | chosen | why not the alternative |
|---|---|---|
| XML into a typed target | `quick_xml::de::from_str::<T>` direct | an untyped tree first loses what the target type knows; `from_file` already deserializes direct for the other formats (`src/file.rs:28-53`) |
| XML in builder layers | `Tree` visitor builds the merge tree, then a full lenient deserializer | layers are partial, so they cannot be parsed straight into `T`; forwarding to serde-value's struct access would lose leniency one level down |
| leniency for every layer | yes | the merged tree does not record each value's source; a typed value is never touched (AC3 case 3 checks a typed TOML layer through the same check), so the only new behaviour for TOML / JSON / YAML is that a quoted scalar or a single value where a list is wanted now loads |
| a free-form type | `Tree` in conflaguration | a payload typed `serde_json::Value` takes XML through `deserialize_any`: `$text` maps and last-one-wins (`serde_json-1.0.151/src/value/de.rs:134-142`, measured in PROXIMA-47) |
| typed read of a parsed subtree | `from_tree` | the builder reads files and `Serialize` mappings, not a subtree of a document already parsed; a caller holding a `serde_json::Value` would otherwise re-serialize it to text and parse again, losing XML's grouping |
| serde's buffered paths | stated limit, pinned by AC11, with R8's per-field helper as the remedy (AC12) | serde deserializes tagged / untagged enums and flatten from its private strict buffer after one `deserialize_any`; guessing types inside `deserialize_any` would turn a numeric-looking string field (a token `"12345"`) into a number and break it instead |
| RON and JSON5 | in scope | proxima's registry reads them today (`proxima/proxima-config/src/lib.rs:250-257`) and the owner chose to delegate it to conflaguration |
| incumbent check | the removed-line list L13, plus the full suite | a function-pointer typing of each public function misses a new bound on a generic function once its type arguments are fixed (round 3); a removed or rewritten line is what any signature, bound or body change produces |

## acceptance criteria

Every cargo command runs as `timeout -k 30s 25m git wt cargo ...`, `CARGO_BUILD_JOBS=2`. `F6` is the
feature set `toml,json,yaml,xml,ron,json5`. `tests/formats.rs` starts with
`#![cfg(all(feature = "toml", feature = "json", feature = "yaml", feature = "xml", feature = "ron", feature = "json5"))]`,
so its tests exist only under F6 (or `--all-features`). `tests/formats_single.rs` holds three tests,
`format_single_xml` under `#[cfg(feature = "xml")]`, `format_single_ron` under
`#[cfg(feature = "ron")]`, `format_single_json5` under `#[cfg(feature = "json5")]`, each
`from_file::<Fixture>` on its `fixture.<fmt>` compared to F1 with `assert_eq!`; that file uses
nothing else from the crate. Helpers in `tests/formats.rs`: `check_fixture<T: PartialEq +
Debug>(actual: Result<T>, expected: &T) -> Result<(), String>` (Ok iff `actual` is `Ok` and equal),
`check_json(json, expected_json) -> Result<(), String>` (compares two `serde_json::Value`s; the
caller converts), `check_error<T>(result: Result<T>, needle) -> Result<(), String>` (Ok iff an
`Error::ValueDeserialize` whose text contains `needle`). Prefixes: `format_from_file_`,
`format_layered_`, `format_overlay_`, `format_lenient_`, `format_tree_`, `format_from_tree_`,
`format_limit_`, `format_helper_`, `ctl_formats_` (in `tests/formats.rs`) and `format_single_` (in
`tests/formats_single.rs`); no prefix is a substring of another (`format_from_tree_` holds
`tree_` after `from_`, not `format_tree_`; `ctl_formats_` holds `formats_`, not `format_`).

| id | discharges | command | expected |
|---|---|---|---|
| AC1 | R1, R2 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_from_file_)'` | 6 passed: `from_file::<Fixture>` on each of `fixture.{toml,json,yaml,xml,ron,json5}`, `check_fixture(_, &F1)` -> Ok |
| AC9 | R1, R2 (each feature alone, with the widened gates) | (a) `cargo val --features xml --test formats_single -E 'test(format_single_)'`; (b) `cargo val --features ron --test formats_single -E 'test(format_single_)'`; (c) `cargo val --features json5 --test formats_single -E 'test(format_single_)'` | (a) 1 passed (`format_single_xml`); (b) 1 passed (`format_single_ron`); (c) 1 passed (`format_single_json5`) |
| AC2 | R3, R4 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_layered_)'` | 6 passed: `builder::<Fixture>().value(B0).file(<twin>).build()` per twin (a non-empty chain, so `merge_file`), `check_fixture(_, &F1)` -> Ok |
| AC8 | R3 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_overlay_)'` | 6 passed: `builder::<Fixture>().value(F1).overlay_file(overlay.<fmt>).build()` for all six formats, `check_fixture(_, &F1 with absent: Some("filled"))` -> Ok (port stays 8080: an overlay never replaces a present value; the toml/json/yaml cases run today's overlay path through the same check) |
| AC3 | R4 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_lenient_)'` | 8 passed: (1) F4 over B0, `check_fixture(_, &B0 with port 8080, debug true, ratio 0.5, single ["only"])` -> Ok; (2) F4x over B0, `check_error(_, "eighty")` -> Ok; (3) F4T over B0 (typed, today's path), the same `check_fixture` as (1) -> Ok; (4) F4E over F1, `check_fixture(_, &F1 with tags [] and absent None)` -> Ok; (5)-(8) `builder::<Deep>().value(D0).file(<layer>).build()` for DL, DO, DM, DE, each `check_fixture` against its expected value -> Ok |
| AC4 | R5 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_tree_)'` | 8 passed: (1)-(6) each `payload.<fmt>` as `Tree` converted to JSON, `check_json(_, F3J)` -> Ok for toml/json/yaml/ron/json5 and `check_json(_, F3X)` -> Ok for xml; (7) TD as `Tree` converted to JSON, `check_json(_, {"when":"1979-05-27T07:32:00Z"})` -> Ok; (8) TI the same way, `check_json(_, {"x":null})` -> Ok |
| AC10 | R7, R5 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_from_tree_)'` | 8 passed: (1)-(6) `from_<fmt>_str::<Tree>` on each `fixture.<fmt>`, then `from_tree::<Fixture>`, `check_fixture(_, &F1)` -> Ok; (7) `serde_json::from_value::<Tree>(serde_json::from_str::<serde_json::Value>(fixture.json))`, then `from_tree::<Fixture>`, `check_fixture(_, &F1)` -> Ok; (8) `from_tree::<Fixture>(from_xml_str::<Tree>(F2x))`, `check_error(_, "eighty")` -> Ok |
| AC11 | R4 (the stated limit) | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_limit_)'` | 1 passed: `builder::<Holder>().value(H0).file(tagged.xml).build()`, `check_error(_, "")` -> Ok (an `Error::ValueDeserialize`: `ms` reaches serde's buffer as the string `"5"`) |
| AC12 | R8 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_helper_)'` | 3 passed: (1) `builder::<Helped>().value(G0).file(helped.xml).build()`, `check_fixture(_, &Helped { step: HelpedStep::Wait { ms: 5, cap: Some(9) } })` -> Ok (the same tagged shape AC11 refuses, loading once the two fields carry R8's helpers); (2) the same over GT (typed TOML through the helpers) -> Ok; (3) GB over G0, `check_error(_, "five")` -> Ok |
| AC5 | R1-R5 controls | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(ctl_formats_)'` | 8 passed: `check_fixture(Ok(F1), &F1)` -> Ok; `check_fixture(Ok(F1 with single: []), &F1)` -> Err; `check_json(serde_json::from_str::<serde_json::Value>(payload.json), F3J)` -> Ok (the incumbent free-form path, without `Tree`); `check_json(Tree of payload.json, F3J with "id":2)` -> Err; `check_error(Err(ValueDeserialize("eighty ...")), "eighty")` -> Ok; `check_error(Ok(B0), "eighty")` -> Err; `quick_xml::de::from_str::<serde_json::Value>` on F2 gives `tags` as one `{"$text":"c"}` (the defect R3/R5 remove); `serde_json::Value::from(Tree of payload.xml)` differs from `quick_xml::de::from_str::<serde_json::Value>` on the same text |
| AC6 | R6(a) | `cargo val --workspace --all-features --output plain` (all targets plus doc-tests, as the baseline ran) | 358 passed, 0 failed (298 after CONFLAG-2, `<tickets>/CONFLAG-2/evidence/full.log`, plus 60 new: 54 in `tests/formats.rs`, 3 in `tests/formats_single.rs`, 3 doc-tests on `from_xml_str`, `Tree` and `from_tree`) |
| AC7 | R6(b), R6(c) | (a) `grep -c '^default = \["std"\]$' Cargo.toml`; (b) `git diff -U0 beded1a HEAD -- src crates \| grep '^-' \| grep -v '^--- '` | (a) 1; (b) exactly the 13 lines of L13, in that order, and nothing else |

Counts: `tests/formats.rs` AC1 6 + AC2 6 + AC8 6 + AC3 8 + AC4 8 + AC10 8 + AC11 1 + AC12 3 + AC5 8 = 54;
`tests/formats_single.rs` 3; doc-tests 3; 60 new. AC9 runs one test of `formats_single` per feature.

## out of scope

- attributes, namespaces and mixed content (text beside child elements) in XML config
- writing configs out in any format (serializers)
- coercing XML text inside serde's buffered paths (R4's limit): a caller whose config uses a tagged
  or untagged enum or `flatten` around a non-string field marks that field with R8's helper
- proxima delegating `proxima-config` to conflaguration: proxima's spec, after this release
- publishing the release: owner's go

## risks

| risk | likelihood | what it costs | what we do about it |
|---|---|---|---|
| lenient coercion changes a test that relied on a type error | low | AC6 below 358 | AC6 is the measurement; any such test is reported, not rewritten |
| quick-xml behaviour differs from the 0.41.0 trace on the version `cargo add` picks | low | AC1 / AC4 XML cases fail | pin 0.41.0 if a newer version differs |
| the shared machine's load guard refuses runs | high | delay | wrap every run; coordinate quiet windows with peer sessions |

## context

- `src/file.rs` (formats, merge), `src/builder.rs` (first source vs merge, `:149-168,219-238`), `src/lib.rs:271-295,385-415` (gates, re-exports, errors)
- `docs/guide.md` sections 2-3 (nesting, builder sources)
- `<tickets>/PROXIMA-47/` (format inventory, XML probes), `<tickets>/PROXIMA-48/` (proxima coverage), `<tickets>/CONFLAG-2/` (green baseline)
