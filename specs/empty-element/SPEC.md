# empty-element -- an XML empty element is `Unit`, a real empty map stays a map (2.2.0)

status: draft (amended 2026-11-20 after spec-auditor REFUSE round 1: R1 post-pass replaces the thread-local flag, R5 folded into R3, AC6 and the counts re-derived; amended again after REFUSE round 2: R7 and AC8, R1 untested clauses pinned by 6 new tests (AC1, AC10, AC11), AC6 runnable with plain `cargo val`, AC7(b) pathspec widened, AC9 discharges R2 and R3, J4 quotes the sibling spec, every count recomputed; amended again after REFUSE round 3: AC11 (d)-(f) prove the xml gate, AC12 proves the formatting commit `0366c5a`, AC6 braces for zsh and a copy check, AC7/AC8 literal ranges, AC7 pathspec widened to `crates/`, RON grounding corrected, policy.py cited by absolute path); amendment CONFLAG-9 (2026-11-21, after 2.2.0 landed on `main` and before its publish): R3 also covers the `deserialize_any` path (adjacently tagged and untagged enums, buffered content, `serde_json::Value`), AC13, slice 4
owner: brian
created: 2026-11-20

Census: `/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/CONFLAG-8/evidence/census.md`
(12-row table, each field with its proxima `file:line`). Every "census" below means that file.

Why now: proxima's census for PROXIMA-50 found 12 config fields whose meaning changed once configs
went through conflaguration 2.1.0, among them a TLS `tls_client = {}` (on, with defaults) reading as
off and a retry `budget = {}` reading as off. Measured by a probe on 2026-11-20 against 2.1.0, log
`~/.cache/proxima50/probe-empty.log`. Every `file:line` below is in this repo
(`slot-0/conflaguration-publish`) at `5bb4b91` (main, "chore: release 2.1.0"), read 2026-11-20, with one exception: `/Users/brianbruggeman/repos/slot-0/tools/main-only-worktrees/policy.py:185-188`, outside the repo, cited by absolute path (AC preamble, AC6).
Sibling spec, the admitted style: `specs/xml-lenient/`.

## problem

The lenient reader treats a non-XML empty map as an XML empty element: `Tree` represents `<capture/>`,
`<capture></capture>` and whitespace-only content as `Map({})`, identical to the JSON `{"capture": {}}`
(probe cases 1, 2, 2b, 5), and `Lenient` then reads any empty map as absent for `Option`
(`src/lenient_de.rs:124-127`), `""` for `String` (`:109-114`) and `[]` for `Vec` (`:150-153`), so
`from_tree` and the builder layers change the meaning of `{}` for `Option`, `String` and `Vec` fields,
measured by the probe's cases 5, 6, 7 and 8 against `serde_json`: for `Option<S>` with `S` all-defaulted
`from_tree` gives `None` where serde_json gives `Some(S::default())`; with `S` requiring a field
`Ok(None)` where serde_json gives `Err(missing field)`; `Option<serde_json::Value>` gives `None` where
serde_json gives `Some(Object {})`.

## refutation condition

If, after the fix, any of the probe's XML empty forms (`<capture/>`, `<capture></capture>`,
whitespace-only) for an `Option` field reads as anything but `None`, or any of its non-XML `{}` cases
for an `Option` field reads differently from `serde_json::from_value`, the "empty element is `Unit`,
a map is a map" frame is wrong and this spec is struck. It is also struck if proxima's 12 census
fields (the 12-row table in `.../22.1 - Tickets/CONFLAG-8/evidence/census.md`, each with its proxima `file:line`)
cannot be made to read as serde_json reads them without a per-field annotation.

## requirements

| id | requirement | testable in isolation |
|---|---|---|
| R1 | a new `pub fn tree_from_xml_str(text: &str) -> Result<Tree>` (in `src/file.rs` beside `from_xml_str`, `#[cfg(feature = "xml")]`, re-exported in `src/lib.rs` next to `from_xml_str`) parses with quick_xml into `Tree`, then runs an XML-only post-pass turning EVERY empty `Map` in the tree into `Value::Unit`, recursively through maps and sequences. Safe because in a tree parsed from XML an empty map can only come from an empty element (`<x/>`, `<x></x>`, whitespace only); an element with an attribute or a child is non-empty. The `.xml` arm of `file_to_value` (`src/file.rs:134`, the builder layer path) calls it. ~~`from_xml_str::<Tree>`, a `Tree` nested in a `T` given to `from_xml_str::<T>`, and the `.xml` arm all converted via a thread-local flag~~ (struck: hidden per-thread state, and it leaks into a JSON / TOML parse made inside an XML parse, audit round 1). `from_xml_str::<T>` is unchanged: for `T = Tree` it still returns `Map({})` for an empty element, documented on `from_xml_str`, which points callers who want a `Tree` at `tree_from_xml_str`; a `Tree` nested inside a `T` parsed by `from_xml_str::<T>` is out of scope (still `Map({})`). No non-XML tree is touched: `serde_json::from_value::<Tree>`, `from_toml_str::<Tree>` and the rest keep `Map({})` for `{}`. It survives `Tree`'s JSON round trip: `to_json` maps `Unit` to `null` (`src/tree.rs:172`) and `null` re-enters through `visit_unit` (`:94`). Each clause has a test: repeated empty siblings (the `Seq` arm of the post-pass) by E2, `from_xml_str::<Tree>` unchanged by E3 (both AC1), no non-XML tree touched by P1 (AC10), the doc pointer to `tree_from_xml_str` by AC11 | yes |
| R2 | `Lenient::deserialize_option` no longer maps an empty map to `None`: a non-XML `{}` reaches `visit_some` and `S` is read from the empty map, as serde_json reads it. `Value::Unit` (an XML empty element, or a JSON `null`) gives `None` (the existing `Unit` arm, `:130`) | yes |
| R3 | wherever `Lenient` turned an empty map into a value, `Unit` gets the same treatment. One loosening, stated once: a `Unit`, whether an XML empty element or a JSON / YAML / builder-layer TOML `null`, for a non-`Option` field reads as empty: `String` -> `""`, `Vec` -> `[]`, a struct -> its empty-map read (defaults, or `missing field`), a map type (`BTreeMap`, `HashMap`) -> empty. Both the XML and the `null` input were errors in 2.1.0 (and are under serde_json) or loaded with this same meaning; nothing that loads today changes meaning, so this only loosens. The three empty-map branches (`is_empty_map` at `:110`, `:125`, `:151`) are removed, not kept: `String` and `Vec` read a non-XML `{}` as serde_json does (an error; see the `Vec` row of decisions for the one-element rule). Amendment CONFLAG-9: the same holds where serde reads the value through `deserialize_any`, the path an adjacently tagged or untagged enum's content, serde's buffered content and a `serde_json::Value` target take: a `Unit` there reads as an empty map, so a struct or a struct variant gets its empty-map read; only `deserialize_option` (`None`), `deserialize_unit` and `deserialize_unit_struct` (`()`) keep reading `Unit` as unit. At `210bdb1` `deserialize_any` visits `Unit` as unit (`src/lenient_de.rs:80`), so `<value/>` as an adjacently tagged struct variant's content failed (`invalid type: unit value, expected struct variant`) and, behind an untagged enum, `data did not match any variant of untagged enum` (PROXIMA-50 row 24: `config_format_twin_validate_config`, `<tickets>/PROXIMA-50/evidence/slice24-lib-format.log`), where 2.1.0's empty map read it. Accepted cost, the same as 2.1.0's for XML: an `Option` buffered inside an untagged enum reads `<x/>` as `Some(<empty-map read>)`, and a `serde_json::Value` target reads it as `{}`; a typed `Option` field still reads `None`. The cost is NOT only XML's, and it is the one place this spec changes the meaning of input that loads today: a JSON / YAML `null` is the same `Unit` (R1: `null` re-enters through `visit_unit`), and a consumer such as proxima turns every format into `serde_json::Value` before the typed read, so an XML empty element and a JSON `null` cannot be told apart there. Through `deserialize_any` a `null` therefore reads as `{}` for a `serde_json::Value` field (2.1.0 and serde_json: `null`) and as `Some(<empty-map read>)` for an `Option` buffered in an untagged enum (2.1.0 and serde_json: `None`). The alternative, reading `Unit` as unit in `deserialize_any`, keeps `null` exact but leaves every XML empty element read as an enum's content broken, as at `210bdb1`; the cost is pinned on both sides by AC13's two JSON-null rows, and the owner decides it with the 2.2.0 publish | yes |
| R4 | incumbents: the full suite passes with its 2.1.0 count (358 passed, 0 failed, 2 ignored, `specs/xml-lenient` AC6) plus this spec's new tests (~~20~~ 26 `format_empty_` + 11 `ctl_empty_`; struck: 6 tests added for R1's untested clauses; amendment CONFLAG-9 adds 10 `format_empty_enum_` + 10 `ctl_empty_enum_`, AC13). No existing test's expected value moves: the two tests that exercise an XML empty element (`tests/formats.rs:358`, `:387`) pass unchanged, and are listed so a change in their path is visible. The post-slice-2 formatting commit `0366c5a` is formatting only, proven by AC12. Any test that does move is listed by name with its old and new value, never rewritten silently | yes |
| ~~R5~~ | ~~the cost: a JSON (or YAML, or builder-layer TOML) `null` for a `String`, `Vec` or struct field now reads leniently...~~ struck: folded into R3 (audit G2: R5 cannot fail apart from R3; one loosening, one requirement). Its test, AC and risk row moved to R3 | n/a |
| R6 | a builder layer's `Unit` over a map base keeps the base: `merge_value` (`src/file.rs:173-189`) given `(Value::Map, Value::Unit)` leaves the base map as it is, in Override and Overlay. `Unit` is an XML empty element (the empty `Map({})` it replaces merged to a no-op in 2.1.0) or a JSON / YAML `null` (an error in 2.1.0: Override replaced the map with `Unit`, which a struct read rejects). `<inner/>` and `{"inner": null}` over a populated `inner` keep `inner` | yes |
| R7 | the release is 2.2.0, a minor bump. It adds the public `tree_from_xml_str` and changes lenient reads that were errors or wrong values. No public signature changes | yes |

## literals

Test-only fixtures and helpers in `tests/formats.rs` (which starts with the six-feature `#![cfg]`):

```rust
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct Def { data_path: String, n: u32 }                 // all-defaulted inner type
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Req { data_path: String }                         // required-field inner type
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptDef { capture: Option<Def> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptReq { capture: Option<Req> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct OptVal { capture: Option<serde_json::Value> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct PlainDef { capture: Def }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct PlainReq { capture: Req }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S1 { capture: String }                            // J2
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S2 { capture: Vec<String> }                       // J3
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct S3 { capture: Vec<Def> }                          // J4
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fields { s: String, v: Vec<String>, d: Def, m: std::collections::BTreeMap<String, u32> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct NullFields { s: String, v: Vec<String>, d: Def } // N1: no `m`, no field defaults, so serde_json's Err comes from the nulls alone

fn xml_tree(text: &str) -> conflaguration::Result<Tree> { conflaguration::tree_from_xml_str(text) }
fn serde_read<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String>;   // serde_json::from_value, error as text
fn tree_read<T: serde::de::DeserializeOwned>(json: &serde_json::Value) -> Result<T, String>;    // from_tree(serde_json::from_value::<Tree>(json))
fn agree<T: PartialEq + Debug>(oracle: Result<T, String>, got: Result<T, String>) -> Result<(), String>; // Ok if both Ok and equal, or both Err; Err(diff) otherwise
fn same_as_serde<T: serde::de::DeserializeOwned + PartialEq + Debug>(json: &serde_json::Value) -> Result<(), String>; // agree(serde_read::<T>(json), tree_read::<T>(json))
```

`same_as_serde` is the one comparison helper. The `format_empty_json_` tests (J1 x5, J2, J3) call it and then
assert the `Err` content separately (field name for the missing-field cases; `is_err` for the type-mismatch cases,
whose tree-side wording is serde-value's). The `ctl_empty_` controls call `serde_read` (the same oracle half
`same_as_serde` is built from) and assert the literal; `ctl_empty_literal_rejects` calls `agree` directly.

- E1, the three XML empty forms of the probe: `<config><capture/></config>`,
  `<config><capture></capture></config>`, `<config><capture>  </capture></config>` (probe cases 1, 2, 2b),
  each through `xml_tree` (`tree_from_xml_str`). Each, as `Tree` converted to JSON, is exactly
  `{"capture":null}` (2.1.0: `{"capture":{}}`). Each through `from_tree` gives `Ok(OptDef { capture: None })`,
  `Ok(OptReq { capture: None })`, `Ok(OptVal { capture: None })`.
- E2, repeated empty siblings (the `Seq` arm of the post-pass; `src/tree.rs:128-135` builds `Seq[Map({}), Map({})]`):
  `xml_tree("<config><a/><a/></config>")` as Tree JSON is exactly `{"a":[null,null]}` (2.1.0: `{"a":[{},{}]}`).
- E3, `from_xml_str` is unchanged: `from_xml_str::<Tree>("<config><capture/></config>")` as Tree JSON is exactly
  `{"capture":{}}`, on the fixed tree as on 2.1.0 (the documented behaviour, R1).
- P1, no non-XML tree is touched: as Tree JSON each of `from_toml_str::<Tree>("capture = {}")`,
  `from_yaml_str::<Tree>("capture: {}")`, `from_json5_str::<Tree>("{capture: {}}")` and
  `from_ron_str::<Tree>("(capture: {})")` is exactly `{"capture":{}}`, on the fixed tree as on 2.1.0. The RON text is a
  struct-form top level holding an empty map. Grounded in `ron` 0.8.1 (`Cargo.toml:26`), `src/de/mod.rs:249` (`(` gives
  `handle_any_struct`) and `:251` (`{` gives `deserialize_map`), through `deserialize_any`. ~~the form `tests/fixtures/formats/payload.ron` already uses (a `(` document with a `{}` map inside it)~~ struck: false, `payload.ron` uses only the `(` struct form and holds no `{}` map (audit round 3).
- J1, the JSON `{"capture":{}}`, read two ways by `same_as_serde::<T>`:
  - `OptDef`: `Ok(OptDef { capture: Some(Def::default()) })` both ways (2.1.0 tree: `None`)
  - `OptReq`: serde_json `Err` containing "missing field `data_path`"; `from_tree` an `Err` whose text contains
    `data_path` (serde-value's wording is `Missing field data_path`, probe case 7 tree rows; so the test compares
    `is_err` plus the field name, not the string). 2.1.0 tree: `Ok(OptReq { capture: None })`
  - `OptVal`: `Ok(OptVal { capture: Some(serde_json::json!({})) })` both ways (2.1.0 tree: `None`)
  - `PlainDef`: `Ok(PlainDef { capture: Def::default() })` both ways (unchanged from 2.1.0)
  - `PlainReq`: an `Err` containing `data_path` both ways (unchanged from 2.1.0)
- J2, `{"capture":{}}` into `S1`: serde_json `Err` containing `invalid type: map`; `from_tree` an `Err` (2.1.0 tree: `Ok(S1 { capture: "" })`).
  J3, the same into `S2`: both `Err` (serde_json contains `invalid type: map`; 2.1.0 tree: `Ok(S2 { capture: [] })`).
  J4, `{"capture":{}}` into `S3`: `from_tree` is `Ok(S3 { capture: vec![Def::default()] })`. Its oracle is the sibling
  spec's one-element rule (`specs/xml-lenient/SPEC.md` R4, "a single value where `T` wants a list becomes a one-element list"; ~~"single -> one-element seq"~~ struck: not R4's words), not serde_json, which
  errors (`invalid type: map`, pinned by `ctl_empty_serde_json_vec_struct`); a documented loosening, in risks
  (2.1.0 tree: `[]`).
- X1, `<config><s/><v/><d/><m/></config>` through `xml_tree` then `from_tree::<Fields>`:
  `Ok(Fields { s: "".into(), v: vec![], d: Def::default(), m: BTreeMap::new() })`. X2, root `<config/>` through
  `xml_tree` then `from_tree::<Def>`: `Ok(Def::default())`.
- N1, the JSON `{"s":null,"v":null,"d":null}` (R3's null case): `from_tree::<NullFields>` is
  `Ok(NullFields { s: "".into(), v: vec![], d: Def::default() })`; `serde_json::from_value::<NullFields>` is an
  `Err` whose text contains `invalid type: null`.
- T1, round trip: `serde_json::from_value::<Tree>(serde_json::Value::from(xml_tree(E1[0])?))` converts to JSON
  `{"capture":null}` and through `from_tree` gives `Ok(OptDef { capture: None })`.
- C1, the hand-literal Tree control: `serde_json::Value::from(serde_json::from_value::<Tree>(serde_json::json!({"capture":null}))?)`
  equals the literal `serde_json::json!({"capture":null})`. It builds the Tree from a literal, not from XML, so it
  passes on 2.1.0 and shows the failures of E1 and T1 there come from the parser, not from `to_json`.
- B0, the base: `OptDef { capture: None }`. Layers over B0 through `ConfigBuilder::<OptDef>::new().value(B0).file(..)`:
  `tests/fixtures/formats/empty_capture.xml` `<config><capture/></config>` -> `Ok(OptDef { capture: None })`;
  `tests/fixtures/formats/empty_capture.toml` `capture = {}` -> `Ok(OptDef { capture: Some(Def::default()) })`; the same toml
  layer through `ConfigBuilder::<OptReq>` over base `OptReq { capture: None }` -> `Err` naming `data_path`.
- K0, the keep-base case: base `expected_fixture()` (from `tests/formats.rs`), layer
  `tests/fixtures/formats/empty_inner.xml` `<config><inner/></config>`, in Override (`.file`) and as an
  `.overlay_file`: both give `Ok(expected_fixture())` unchanged (`inner: Inner { retries: 3, enabled: false }`).
- K1, the builder-layer null case (R3 / R6): base `expected_fixture()`, layer
  `tests/fixtures/formats/empty_inner_null.json` `{"inner": null}`, in Override and as an `.overlay_file`: both give
  `Ok(expected_fixture())` unchanged (2.1.0: Override gives an `Err`, the map replaced by `Unit`).

## architecture

~~Why the post-pass R1's brief asked for cannot sit in `from_xml_str`: it is generic over `T`... only XML parsing sets a scope flag.~~ struck: hidden per-thread state, and it leaks into a JSON / TOML parse made inside an XML parse (audit round 1).

~~`thread_local! { static XML_SCOPE }`, `pub(crate) struct XmlScope(bool)` RAII guard, the `visit_map` check `entries.is_empty() && XML_SCOPE.get()`, the `XmlScope::enter()` lines in `from_xml_str` and `file_to_value`~~ struck, same reason. `TreeVisitor` is not touched.

```rust
// src/file.rs (new, #[cfg(feature = "xml")]); re-exported in src/lib.rs beside from_xml_str
pub fn tree_from_xml_str(text: &str) -> Result<Tree> {
    let tree: Tree = quick_xml::de::from_str(text)?;
    Ok(Tree(empties_to_unit(tree.into_value())))
}
// src/tree.rs (new, #[cfg(feature = "xml")], pub(crate)): the XML-only post-pass
fn empties_to_unit(value: Value) -> Value {
    match value {
        Value::Map(entries) if entries.is_empty() => Value::Unit,
        Value::Map(entries) => Value::Map(entries.into_iter().map(|(key, inner)| (key, empties_to_unit(inner))).collect()),
        Value::Seq(items) => Value::Seq(items.into_iter().map(empties_to_unit).collect()),
        other => other,
    }
}

// src/file.rs file_to_value xml arm (:134)
"xml" => Ok(tree_from_xml_str(&contents)?.into_value()),
// from_xml_str::<T> is unchanged. Its doc comment gains: for T = Tree an empty element is Map({}); use tree_from_xml_str.

// src/file.rs merge_value (:173): one added arm before the catch-all
(Value::Map(_), Value::Unit) => {}               // R6: an empty element or a null adds no keys

// src/lenient_de.rs
// is_empty_map (:34-36) is deleted; match Value::Unit inline.
// deserialize_string: Value::Unit -> visit_string(String::new()); other -> deserialize_any
// deserialize_option: Value::Unit -> visit_unit (None); Option(_) -> deserialize_any; else visit_some(self)
// deserialize_seq: Value::Unit -> empty SeqDeserializer; Seq|Bytes -> deserialize_any; single -> one-element seq
// deserialize_map, deserialize_struct: Value::Unit -> visit_map over an empty MapDeserializer
```

Why the post-pass is safe: in a tree parsed from XML, an empty map can only come from an empty element; an element
with an attribute or a child is non-empty, and the pass touches only maps, so no non-XML tree reaches it.

Scope: `tree_from_xml_str` and the builder's `.xml` layer give `Unit`. `from_xml_str::<Tree>` and a `Tree` nested in a
`T` under `from_xml_str::<T>` keep `Map({})` as in 2.1.0 (documented on `from_xml_str` and `Tree`); a user's own
`quick_xml::de::from_str::<Tree>` does too.

### decisions

| decision | chosen | why not the alternative |
|---|---|---|
| where the XML-only conversion runs | a new `tree_from_xml_str` whose post-pass turns every empty `Map` into `Unit`; the builder's `.xml` arm uses it | ~~a thread-local scope entered in `from_xml_str` and `file_to_value`, read in `TreeVisitor::visit_map`~~ struck: hidden per-thread state that leaks into a JSON / TOML parse made inside an XML parse (audit round 1). `from_xml_str::<T>` keeps its signature (a `TypeId` check needs `T: 'static`, a breaking bound); a `Tree` nested in `T` stays out of scope |
| the `String` and `Vec` empty-map branches (R3) | removed; `Unit` only | parity: a non-XML config loads as serde_json loads it, and serde_json errors on `{}` for `String` and `Vec`. Inventory of incumbents that rely on the branches: `format_lenient_empty_elements` (`tests/formats.rs:358`, fixture `empties.xml`, `<tags/>` on a `Vec`) and `format_lenient_deep_empty` (`:387`, `deep_empty.xml`, `<note/>` on a `String`); both are XML, so both arrive as `Unit` after R1 (they go through `file_to_value`, which uses `tree_from_xml_str`; if either reads through `from_xml_str::<Tree>` instead, slice 1 repoints it to `tree_from_xml_str` and lists it by name with old and new value) and pass unchanged. No test, fixture or doc example in the repo writes a non-XML `{}` for a `String` or `Vec` (grep of `tests/`, `docs/`, `README.md`, `src/` for `{}`, `Map({})`, `empty map`, 2026-11-20). Keeping the branches would keep the defect for the two types R2 does not cover |
| `Vec` and a non-XML `{}` | falls to the existing single-value rule (`deserialize_seq`, `src/lenient_de.rs:154-157`, ~~the arm `single => one-element seq`~~ struck round 4: not the code's text; the `single =>` arm, which wraps the value in a one-element seq (`src/lenient_de.rs:156`); the rule is `specs/xml-lenient` R4: "a single value where `T` wants a list becomes a one-element list"): `Vec<String>` from `{}` errors (the `String` element rejects `{}`), `Vec<Def>` from `{}` is `[Def::default()]` | a special case refusing a map for a `Vec` would add a second rule beside `specs/xml-lenient` R4's "a single value where `T` wants a list becomes a one-element list" (~~"single -> one-element seq"~~ struck: not R4's words); the 2.1.0 value `[]` for `Vec<Def>` moves to `[Def::default()]`, listed in risks and pinned by J4 |
| `Unit` for a struct | `deserialize_struct` and `deserialize_map` read `Unit` as an empty map | an XML `<inner/>` for a non-`Option` struct field loaded in 2.1.0 (`Map({})` reached `visit_map`); without this it would regress to a `visit_unit` type error |
| a `Unit` layer over a map | no-op (R6) | `Unit` through the catch-all arm would replace the base map under Override; 2.1.0's `Map({})` merged key by key to nothing, and a JSON `null` replaced the map and then errored |

## acceptance criteria

Every cargo command runs as `timeout -k 30s 25m git wt cargo ...`, `CARGO_BUILD_JOBS=2`, except AC6 and AC12, which run in scratch worktrees outside `.wt/` and so use plain cargo (`git wt cargo` refuses there, `/Users/brianbruggeman/repos/slot-0/tools/main-only-worktrees/policy.py:185-188` (outside this repo)). `F6` is
`toml,json,yaml,xml,ron,json5`. All new tests are in `tests/formats.rs`, prefixes `format_empty_` and
`ctl_empty_` (neither contains another existing prefix: `format_lenient_`, `format_layered_`, ... differ
after `format_`). The ~~20~~ 26 `format_empty_` tests (struck: 6 added for R1's untested clauses), by prefix:
`format_empty_xml_` (~~6~~ 8: `selfclosing`, `open_close`, `whitespace` = E1 x3; `plain_fields` = X1; `root` = X2;
`roundtrip` = T1; `repeated` = E2; `from_str_keeps_map` = E3), `format_empty_json_` (8: `option_default`, `option_required`, `option_value`,
`plain_default`, `plain_required` = J1; `string` = J2; `vec_string` = J3; `vec_struct` = J4),
`format_empty_null_` (1: `non_option` = N1), `format_empty_layer_` (5: `xml_option`, `xml_keeps_base` = K0 Override
and overlay in one test, `json_null_keeps_base` = K1 Override and overlay in one test, `toml_default`, `toml_required`),
`format_empty_nonxml_` (4, P1: `toml`, `yaml`, `json5`, `ron`; one per format, each with its exact literal). 8 + 8 + 1 + 5 + 4 = 26.
The 11 `ctl_empty_` tests, each PASS on 2.1.0 (the oracle is serde_json or a hand literal, which 2.1.0 does not touch):
`ctl_empty_serde_json_option_default` (`serde_read::<OptDef>(J1)` is exactly `Ok(OptDef { capture: Some(Def::default()) })`),
`ctl_empty_serde_json_option_required` (`Err` containing "missing field `data_path`"),
`ctl_empty_serde_json_option_value` (`Ok(OptVal { capture: Some(serde_json::json!({})) })`),
`ctl_empty_serde_json_plain_default` (`Ok(PlainDef { capture: Def::default() })`),
`ctl_empty_serde_json_plain_required` (`Err` containing `data_path`),
`ctl_empty_serde_json_string` (J2: `Err` containing `invalid type: map`),
`ctl_empty_serde_json_vec_string` (J3: `Err` containing `invalid type: map`),
`ctl_empty_serde_json_vec_struct` (J4: `Err` containing `invalid type: map`, so the loosening is pinned),
`ctl_empty_serde_json_null_fields` (N1: `serde_read::<NullFields>` is `Err` containing `invalid type: null`),
`ctl_empty_tree_json_null` (C1: the hand-literal Tree gives JSON equal to the literal `{"capture":null}`),
`ctl_empty_literal_rejects` (`agree(Ok(Some(Def::default())), Ok(None::<Def>))` returns `Err`, so the tests cannot pass vacuously).

| id | discharges | command | expected |
|---|---|---|---|
| AC1 | R1, R3 (XML side) | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_xml_)'` | ~~6~~ 8 passed (struck: E2 and E3 added): E1 x3 give Tree JSON `{"capture":null}` and `Ok(OptDef { capture: None })`, `Ok(OptReq { capture: None })`, `Ok(OptVal { capture: None })`; X1 gives `Ok(Fields { s: "", v: [], d: Def::default(), m: {} })`; X2 gives `Ok(Def::default())`; T1 round trip gives `{"capture":null}` and `Ok(OptDef { capture: None })`; E2 `<config><a/><a/></config>` through `xml_tree` gives Tree JSON `{"a":[null,null]}`; E3 `from_xml_str::<Tree>("<config><capture/></config>")` gives Tree JSON `{"capture":{}}` |
| AC2 | R2 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_json_option_)'` | 3 passed: J1 for `OptDef` is `Ok(OptDef { capture: Some(Def::default()) })`, for `OptReq` an `Err` containing `data_path` (as `serde_json` is), for `OptVal` `Ok(OptVal { capture: Some(serde_json::json!({})) })`; each compared with `serde_json::from_value` through `same_as_serde` |
| AC3 | R3 | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_json_plain_) or test(format_empty_json_string) or test(format_empty_json_vec_) or test(format_empty_null_)'` | 6 passed (2 + 1 + 2 + 1): `Ok(PlainDef { capture: Def::default() })` and `PlainReq` `Err` containing `data_path` (both unchanged from 2.1.0); J2 `Err` (serde_json: `invalid type: map`); J3 `Err` (the same); J4 `Ok(S3 { capture: vec![Def::default()] })`; N1 `Ok(NullFields { s: "", v: [], d: Def::default() })` through `from_tree` while `serde_json::from_value::<NullFields>` is an `Err` containing `invalid type: null` |
| ~~AC4~~ | ~~R5~~ | ~~`... test(format_empty_null_)`, 1 passed~~ | struck: R5 folded into R3; N1 is in AC3 and K1 in AC5 |
| AC5 | R1, R2, R3 (null), R6 (builder layers) | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_layer_)'` | 5 passed: `.xml` layer `<capture/>` over B0 -> `Ok(OptDef { capture: None })`; `<inner/>` over `expected_fixture()` gives `Ok(expected_fixture())` in Override and overlay; JSON `{"inner": null}` over `expected_fixture()` gives `Ok(expected_fixture())` in Override and overlay; `.toml` layer `capture = {}` over B0 -> `Ok(OptDef { capture: Some(Def::default()) })`; the same over the `OptReq` base -> `Err` containing `data_path` |
| AC6 | the defect, shown (R1, R2, R3) | in a scratch worktree of `5bb4b91`, run from the repo root with `S2` the slice-2 commit, bound literally (slice 2 is the commit that adds the last of the `format_empty_` / `ctl_empty_` tests and `empty_capture.toml`; it landed as `b194701`; ~~`S2=$(git rev-parse HEAD)` right after it lands~~ struck: not reproducible after later commits): `S2=b194701`; `D=/Users/brianbruggeman/repos/slot-0/conflaguration-ctl-2-1-0`; `git worktree add --detach $D 5bb4b91`; `git show "${S2}:tests/formats.rs" > "$D/tests/formats.rs"`; `for f in empty_capture.xml empty_capture.toml empty_inner.xml empty_inner_null.json; do git show "${S2}:tests/fixtures/formats/${f}" > "$D/tests/fixtures/formats/${f}"; done` (every revision:path is written with braces and quotes: in zsh, `$S2:tests` is read as `$S2` plus the `:t` history modifier, which corrupts the path); then the pre-run copy check, `grep -c 'fn format_empty_' "$D/tests/formats.rs"` prints `26` and `grep -c 'fn ctl_empty_' "$D/tests/formats.rs"` prints `11`; then, because `tree_from_xml_str` does not exist at `5bb4b91`, point the one `xml_tree` helper at the 2.1.0 entry point: `sed -i '' 's/conflaguration::tree_from_xml_str(text)/conflaguration::from_xml_str::<Tree>(text)/' $D/tests/formats.rs`; then (a) `cd $D && CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=$D/target timeout -k 30s 25m cargo val --no-fail-fast --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_)'` and (b) the same command with `-E 'test(ctl_empty_)'` (~~`git wt cargo val`~~ struck: `git wt cargo` refuses a worktree outside `.wt/`, `/Users/brianbruggeman/repos/slot-0/tools/main-only-worktrees/policy.py:185-188` (outside this repo), so plain `cargo val` with an explicit `CARGO_TARGET_DIR`; `--no-fail-fast` added because `cargo val` defaults to fail-fast, `--max-fail 1` (the flags `--no-fail-fast` and `--max-fail <N>` and the line "default: fail fast, --max-fail 1" are in the `cargo-val` binary's help text, read with `strings` because `cargo val --help` was refused by the load guard on 2026-11-20), which would stop at the first failure and make the 15/11 split non-deterministic); afterwards `git worktree remove --force $D` | (a) ~~14 failed, 6 passed~~ 15 failed, 11 passed (26 tests; struck: 6 added, 1 fails and 5 pass). Failed: `xml_selfclosing`, `xml_open_close`, `xml_whitespace` (Tree JSON is `{"capture":{}}`), `xml_repeated` (the sed points `xml_tree` at `from_xml_str::<Tree>`, which gives `{"a":[{},{}]}`, not `{"a":[null,null]}`), `xml_roundtrip`, `json_option_default`, `json_option_required`, `json_option_value`, `json_string`, `json_vec_string`, `json_vec_struct`, `null_non_option` (`s` from `null` reaches `visit_unit`, a type error), `layer_json_null_keeps_base` (Override replaces `inner` with `Unit`, the struct read errors), `layer_toml_default`, `layer_toml_required`. Passed: `xml_plain_fields`, `xml_root` (the 2.1.0 empty-map branches read `Map({})` as `""`, `[]`, defaults), `xml_from_str_keeps_map` (`from_xml_str::<Tree>` is the unchanged 2.1.0 call, `{"capture":{}}`), `nonxml_toml`, `nonxml_yaml`, `nonxml_json5`, `nonxml_ron` (no non-XML tree is touched by 2.1.0 either), `json_plain_default`, `json_plain_required`, `layer_xml_option`, `layer_xml_keeps_base`. (b) 11 passed (the oracle is serde_json or a hand literal, which 2.1.0 does not touch) |
| AC7 | R4 | (a) `cargo val --workspace --all-features --output plain` (all targets plus doc-tests, as the 358 baseline ran); (b) `git diff 5bb4b91 b194701 -- tests src/lib.rs src/template.rs crates/conflaguration-derive/src/lib.rs crates/conflaguration-core/tests \| grep -c -E '^-([^-]\|$)'`; (c) `git diff --numstat 5bb4b91 b194701 -- tests src/lib.rs src/template.rs crates/conflaguration-derive/src/lib.rs crates/conflaguration-core/tests \| awk '{n+=$2} END{print n+0}'` (c counts removed lines whose text starts with `-`, such as a YAML `- item` fixture line, which `^-([^-]\|$)` cannot see). (d) doc-tests: `git diff 5bb4b91 b194701 -- src crates \| grep -c -E '^-[[:space:]]*//[/!]'`. Why (b)/(c) do not simply add `src/file.rs` and `src/tree.rs`: the slices legitimately change code lines there, while (d) checks exactly the doc lines. No doc-comment line is removed or changed, so none of the doc-tests in the 358 (`src/file.rs:31-43,246-263`, `src/tree.rs:20-32`, `src/builder.rs:31-48,175-209`, and any in `crates/`) can have its expected value moved. (e) the markdown doc-tests (added after audit round 5): `README.md` and `docs/guide.md` are doc-tests through `src/lib.rs:736,741` (`#[doc = include_str!(...)]`), 17 passed and 1 ignored of the 358 (`slice0-baseline-conflaguration.log`); `git diff 5bb4b91 39a69ff -- README.md docs \| wc -l` prints `0` (measured 2026-11-21): neither file is touched anywhere on the branch, so none of their expected values moves. (f) the commits after the format commit (added after audit round 5): slice 3 (`39a69ff`, `chore: release 2.2.0`) is in no (b)-(d) range, so `git diff --numstat 0366c5a 39a69ff` prints exactly two lines, `1	1	Cargo.lock` and `1	1	Cargo.toml` (measured 2026-11-21): one version line in each, no test source and no `pub` line. ~~`git diff 5bb4b91 -- tests/formats.rs tests/fixtures \| grep -c '^-[^-]'`~~ struck: it missed every other test source, and `[^-]` did not count removed blank lines. ~~pathspec `tests src/lib.rs src/template.rs`~~ struck round 3: no `crates/` test sources; the pathspec now adds `crates/conflaguration-derive/src/lib.rs` (its `#[cfg(test)]` module, `:924`) and `crates/conflaguration-core/tests`, which hold 21 of the 358. The range is literal, `5bb4b91 b194701` (slice 2 ends at `b194701`): the later `chore` commit `0366c5a` is nightly rustfmt output, which re-wraps existing lines, and is discharged by AC12, not by (b) and (c) | (a) ~~389~~ 395 passed, 0 failed, 2 ignored (358 + 26 `format_empty_` + 11 `ctl_empty_` = 395; struck: 389 counted 20 `format_empty_`); `format_lenient_empty_elements` and `format_lenient_deep_empty` among the passes with their expected values unchanged; (b) prints `0` (grep exits 1 on a zero count) and (c) prints `0`, and (d) prints `0` (measured by the spec-auditor, round 4), measured 2026-11-20 on `5bb4b91 b194701` with the widened pathspec: no existing line is removed or changed in `tests/` (every test file and fixture), in the in-source test modules (`src/lib.rs:757`, `src/template.rs:207`, `crates/conflaguration-derive/src/lib.rs:924`) or in `crates/conflaguration-core/tests`. `git diff --stat 5bb4b91 b194701 -- crates` prints nothing: the slices did not touch `crates/`. The fix touches `src/lib.rs` only to ADD the `tree_from_xml_str` re-export (no removed line) and does not touch `src/template.rs`. R6's `merge_value` change is in `src/file.rs`, which is not a test source and so is not in the pathspec |
| AC8 | ~~R4 (version)~~ R7 (struck: the version is R7, not R4) | (a) `grep -c '^version = "2.2.0"$' Cargo.toml`; (b) `grep -A1 '^name = "conflaguration"$' Cargo.lock \| grep -c '^version = "2.2.0"$'`; (c) `git diff 5bb4b91 b194701 -- src \| grep -c -E '^-[[:space:]]*pub '` (literal range, matching the prose; ~~`git diff 5bb4b91 -- src`~~ struck: open-ended) | (a) prints `1`; (b) prints `1` (both after slice 3, the bump); (c) prints `0` (measured 2026-11-20 on `5bb4b91 b194701`): no `pub` line in `src/` is removed or changed, so no public signature changes (R7's last sentence; `tree_from_xml_str` is an added line) |
| AC9 | ~~the controls, on the fixed tree~~ R2, R3 (struck: AC9 discharged nothing; the controls are the serde_json oracles R2 and R3 compare against), the controls on the fixed tree | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(ctl_empty_)'` | 11 passed: the serde_json oracles and the hand literal agree with the literals above after the fix as before it |
| AC10 | R1 (no non-XML tree is touched) | `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_nonxml_)'` | 4 passed (P1): `from_toml_str::<Tree>("capture = {}")`, `from_yaml_str::<Tree>("capture: {}")`, `from_json5_str::<Tree>("{capture: {}}")` and `from_ron_str::<Tree>("(capture: {})")` each give Tree JSON exactly `{"capture":{}}` |
| AC11 | R1 (the doc pointer, the export, the `xml` gate) | (a) `grep -c 'tree_from_xml_str' src/file.rs`; (b) `grep -B20 '^pub fn from_xml_str' src/file.rs \| grep -c '^/// .*tree_from_xml_str'`; (c) `grep -c '^pub use file::tree_from_xml_str;' src/lib.rs`; (d) `git show b194701:src/file.rs \| grep -B3 'pub fn tree_from_xml_str' \| grep -c '#\[cfg(feature = "xml")\]'`; (e) `git show b194701:src/lib.rs \| grep -B3 'pub use file::tree_from_xml_str' \| grep -c 'feature = "xml"'`; (f) with xml OFF: ~~`timeout -k 30s 25m git wt cargo check -p conflaguration --no-default-features --features std,toml,json > "$LOG" 2>&1`, then `grep -c '^error' "$LOG"`~~ struck round 4: `$LOG` was unbound and a refused or killed run also prints `0` errors; now `LOG="/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/CONFLAG-8/evidence/ac11f-xml-off.log"`; `cd /Users/brianbruggeman/repos/slot-0/conflaguration-publish/.wt/card/conflag-8 && timeout -k 30s 25m git wt cargo check -p conflaguration --no-default-features --features std,toml,json > "$LOG" 2>&1; echo "exit=$?"`; then `grep -c '^error' "$LOG"` and `grep -c 'Finished' "$LOG"` | (a) prints `3`: the `pub fn tree_from_xml_str` line, the doc mention on `from_xml_str`, the `.xml` arm of `file_to_value` (the new fn's own doc does not name itself); (b) prints `1`: the doc comment of `from_xml_str` names `tree_from_xml_str`; (c) prints `1`; (d) prints `1` (measured 2026-11-20 on `b194701`): the exact `#[cfg(feature = "xml")]` sits within 3 lines above the `pub fn`; (e) prints `1` (measured 2026-11-20 on `b194701`): the re-export carries `#[cfg(all(any(feature = "default", feature = "std"), feature = "xml"))]`, the sibling gate plus `xml`, so the exact literal of (d) prints `0` there and (e) matches `feature = "xml"` instead; (f) prints three values: `exit=0`; `0` for the `^error` count; `1` for the `Finished` count (NOT measured by this spec edit, which ran no cargo; to be measured in slice 1 validation): the `Finished` count is what proves the crate compiled, since a refused or killed run prints no `Finished`; the gate does not break a build without XML |
| AC12 | R4 (the formatting commit `0366c5a` is formatting only) | in a scratch worktree outside `.wt/`, plain cargo (`git wt cargo` refuses there, `/Users/brianbruggeman/repos/slot-0/tools/main-only-worktrees/policy.py:185-188`, outside this repo): ~~`D=/Users/brianbruggeman/repos/slot-0/conflaguration-fmt-check`; `git worktree add --detach "$D" b194701`; `cd "$D" && cargo +nightly fmt --all`~~ struck round 4: zsh's `cd ""` stays put, so an unbound `D` would format the wrong checkout; bound in one command: `D=/Users/brianbruggeman/repos/slot-0/conflaguration-fmt-check; cd /Users/brianbruggeman/repos/slot-0/conflaguration-publish && git worktree add --detach "$D" b194701 && cd "$D" && cargo +nightly fmt --all` (the repo's `.rustfmt.toml`: edition 2018, `max_width = 200`, `chain_width = 80`, nightly `reorder_imports`); (a) `git diff --stat 0366c5a`; (b) `git diff 0366c5a \| wc -l`; afterwards `cd /Users/brianbruggeman/repos/slot-0/conflaguration-publish && git worktree remove --force "$D"` | (a) prints nothing and (b) prints `0`: the tree after rustfmt on slice 2's tree (`b194701`) equals `0366c5a` exactly, so `0366c5a` is rustfmt's output and nothing else: formatting only, no semantic edit. It reorders three `pub use file::...` lines in `src/lib.rs` (nightly `reorder_imports`) and re-wraps existing lines, which is why AC7 (b) and (c) stop at `b194701`. Not measured by this spec edit (no cargo run); measured when slice 2b is validated |
| AC13 | R3 (amendment CONFLAG-9: the `deserialize_any` path) | (after) `cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_empty_enum_) or test(ctl_empty_enum_)'`; (before) the same with `src/lenient_de.rs` at `210bdb1` (`git show 210bdb1:src/lenient_de.rs > src/lenient_de.rs`, restored after), `--no-fail-fast`; each run to `<tickets>/CONFLAG-9/evidence/ac13-{after,before}.log` | after: 20 passed. Before: the 10 `ctl_empty_enum_` pass and the 10 `format_empty_enum_` fail, each failure recorded (~~16, 8 and 8~~ struck: the two JSON-null rows added to pin R3's stated cost). The AC13 table below gives each test's input and literal; each `format_empty_enum_<case>` reads its XML through `tree_from_xml_str` then `from_tree`, and its `ctl_empty_enum_<case>` reads the same XML through 2.1.0's path, `from_xml_str::<Tree>` then `from_tree` (an empty element stays `Map({})` there, R1), asserting the same literal: 2.1.0's reading is the oracle |

AC13's cases (amendment CONFLAG-9; the types live in `tests/formats.rs`: `Shape` = `#[serde(tag = "type", content = "value", rename_all = "snake_case")] enum Shape { Text { #[serde(default)] pattern: Option<String> }, Count { #[serde(default)] min: Option<i64> } }`; `ShapeRef` = `#[serde(untagged)] enum ShapeRef { Inline(Shape), Named(String) }`; `EmptyInner` = `#[derive(Default)] struct EmptyInner { #[serde(default)] a: Option<String>, #[serde(default)] n: u32 }`; `InnerOrText` = `#[serde(untagged)] enum InnerOrText { Struct(EmptyInner), Text(String) }`; `OptInner` = `struct OptInner { #[serde(default)] inner: Option<EmptyInner> }`; `OptOrText` = `#[serde(untagged)] enum OptOrText { Opt(OptInner), Text(String) }`; each wrapped in a one-field struct read from `<cfg>...</cfg>`; `EmptyInner`, not `Inner`, because `tests/formats.rs:21` already declares an `Inner`):

| case | XML inside `<cfg>` | literal |
|---|---|---|
| `adjacent_selfclosing` | `<items><type>text</type><value/></items>`, field `items: Shape` | `Shape::Text { pattern: None }` |
| `adjacent_open_close` | `<items><type>count</type><value></value></items>`, field `items: Shape` | `Shape::Count { min: None }` |
| `adjacent_content_first` | `<items><value/><type>text</type></items>`, field `items: Shape` | `Shape::Text { pattern: None }` |
| `untagged_adjacent_selfclosing` | `<items><type>text</type><value/></items>`, field `items: ShapeRef` | `ShapeRef::Inline(Shape::Text { pattern: None })` |
| `untagged_adjacent_open_close` | `<items><type>text</type><value></value></items>`, field `items: ShapeRef` | the same |
| `untagged_struct` | `<inner/>`, field `inner: InnerOrText` | `InnerOrText::Struct(EmptyInner::default())` |
| `untagged_option` | `<opt><inner/></opt>`, field `opt: OptOrText` | `OptOrText::Opt(OptInner { inner: Some(EmptyInner::default()) })` (R3's accepted cost: buffered, so `Some`, as 2.1.0 read it) |
| `value_target` | `<x/>`, field `x: serde_json::Value` | `serde_json::json!({})` (as 2.1.0) |
| `json_null_value_target` | not XML: the JSON `{"x": null}` through `from_json_str::<Tree>` then `from_tree`, field `x: serde_json::Value` | `serde_json::json!({})`, R3's stated cost; its `ctl_empty_enum_json_null_value_target` reads the same JSON with `serde_json::from_str` and asserts `serde_json::Value::Null`, so the two readings differ visibly |
| `json_null_untagged_option` | not XML: the JSON `{"opt": {"inner": null}}` the same way, field `opt: OptOrText` | `OptOrText::Opt(OptInner { inner: Some(EmptyInner::default()) })`, R3's stated cost; its ctl reads the JSON with `serde_json::from_str` and asserts `OptOrText::Opt(OptInner { inner: None })` |

## out of scope

- attributes, namespaces, mixed content in XML (as in `specs/xml-lenient`)
- XML has no other way to write "absent", so `<x/>` for an `Option` stays `None`
- an empty element for a numeric or bool field: `Unit` there is a type error, as `Map({})` was in 2.1.0
- ~~a `Tree` built by a caller's own `quick_xml::de::from_str::<Tree>` outside `from_xml_str` and `from_file`~~ widened: a `Tree` from `from_xml_str::<Tree>`, a `Tree` nested inside a `T` parsed by `from_xml_str::<T>`, and a caller's own `quick_xml::de::from_str::<Tree>` all keep `Map({})`; only `tree_from_xml_str` and the builder's `.xml` layer give `Unit`
- the 12 census fields in proxima (census: `.../22.1 - Tickets/CONFLAG-8/evidence/census.md`): proxima's follow-up once 2.2.0 is on crates.io
- publishing to crates.io, and pushing main: the owner's go (not a slice)

## risks

| risk | likelihood | what it costs | what we do about it |
|---|---|---|---|
| a JSON, YAML or builder-layer TOML `null` for a `String`, `Vec` or struct field reads as `""`, `[]` or the empty read (R3), where 2.1.0 and serde_json error: `Unit` is both XML's empty element and a null. The builder-layer case: JSON `{"inner": null}` over `expected_fixture()` keeps the base (R6; 2.1.0 errored under Override) | certain, for such input | a config that serde_json rejects loads; a null layer no longer wipes a map | pinned by N1 / AC3 and K1 / AC5; only loosens (nothing that loads today changes meaning); named in the 2.2.0 release note; the alternative, a distinct `Value` for XML-empty, needs a variant `serde_value::Value` lacks and breaks `Tree`'s JSON round trip, which R1 requires |
| `Vec<Def>` from a non-XML `{}` moves from `[]` (2.1.0) to `[Def::default()]` (the one-element rule); serde_json errors on it, so this is a documented loosening, not parity | low | a layer that wrote `{}` for a struct list gets one default element | J4 pins it, `ctl_empty_serde_json_vec_struct` pins serde_json's error; no incumbent test or doc writes it |
| ~~the thread-local flag leaks into a non-XML parse on the same thread~~ | struck | struck | struck with the thread-local design: no flag exists (audit round 1) |
| a `Tree` from `from_xml_str::<Tree>`, nested in a `T`, or from the caller's own quick-xml call keeps the 2.1.0 `Map({})` | low | their `<x/>` reads as an empty map, so `Option` gives `Some(default)` | documented on `from_xml_str` and `Tree`, pointing at `tree_from_xml_str`; the builder's `.xml` layer is the supported config path |
| the shared machine's load guard refuses runs | high | delay | wrap every run; one cargo job at a time, `-j 2` |

## context

- `src/lenient_de.rs` (the three empty-map branches `:109-114,124-133,150-158`, `is_empty_map` `:34-36`), `src/tree.rs` (`TreeVisitor` `:49-147`, `to_json` `:156-189`), `src/file.rs` (`from_xml_str` `:45`, `file_to_value` xml arm `:134`, `merge_value` `:173-189`, `from_tree` `:264`), `src/lib.rs` (`Tree`, `from_tree` re-exports `:1233-1243`)
- `~/.cache/proxima50/probe-empty.log` (probe cases 1-8), `specs/xml-lenient/` (the 2.1.0 spec: R4, R5, AC6 baseline 358)
- census: `/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/CONFLAG-8/evidence/census.md` (12 rows, proxima `file:line`); proxima PROXIMA-50, ticket record `.../CONFLAG-8/CONFLAG-8.md`
