# xml-lenient -- XML (and RON, JSON5) for conflaguration, typed

status: draft, round 2 (round 1 REFUSE, 13 reasons, each amended)
owner: brian
created: 2026-11-20

Why now: the owner wants every proxima pipeline step configurable in any format, through
conflaguration ("conflaguration should let us do anything: json, toml, yaml, xml, etc.";
"conflaguration is nested config using Setting"), recorded in
`/Users/brianbruggeman/repos/slot-0/pm/topics/agentic-mcp.md`, section "any format, every step".
Every `file:line` below is in this repo (`slot-0/conflaguration-publish`) at `beded1a` (main after
CONFLAG-2), read 2026-11-20. Ticket evidence lives under
`/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/`
(`<tickets>` below). quick-xml citations are to 0.41.0, the version proxima locks.

## problem

conflaguration loads TOML, JSON and YAML only (`src/file.rs:28-53`, features `Cargo.toml:62-64`),
so a config written in XML, RON or JSON5 cannot be loaded at all; and its builder merges file
layers through an untyped `serde_value::Value` (`src/file.rs:57-82,131-139`), which cannot carry
XML's text-only scalars (`"5"` where a `u16` is wanted; quick-xml even wraps element text as
`{"$text": "5"}` when no type guides it, `de/map.rs:677-683`) or its repeated sibling elements (a
map keeps one) -- measured by one fixture config, written once in each of the six formats, loading
into the same typed struct with the same values, both through `from_file` and layered over a base
value through the builder.

## refutation condition

If the XML fixture cannot reach the same typed value as its TOML twin without the caller adding
per-field annotations (`#[serde(deserialize_with = ...)]`) to its config structs, the "any format,
same struct" frame is wrong for XML and this spec is struck.

## requirements

| id | requirement | testable in isolation |
|---|---|---|
| R1 | feature `xml` (quick-xml with its `serialize` feature, which is what provides `quick_xml::de`, quick-xml `src/lib.rs:57-58`): `from_xml_str::<T>(&str) -> Result<T>` deserializes straight into `T`; `from_file` maps `.xml` to it. Element text becomes the typed field; repeated sibling elements fill a `Vec` field, including with exactly one; an absent element leaves an `Option` field `None` | yes |
| R2 | features `ron` and `json5`: `from_ron_str` / `from_json5_str` and the `.ron` / `.json5` extensions in `from_file`, wired as `toml` / `json` / `yaml` are (`src/file.rs:9-53`) | yes |
| R3 | builder layers after the first source (`merge_file`, `src/file.rs:131-139`, reached by `.file()` on a non-empty chain, `src/builder.rs:219-225`, and by `.overlay_file()`) accept `.xml`, `.ron`, `.json5`: `file_to_value` parses them into the merge tree. For XML the tree is built by `Tree`'s visitor (R5), so element text is a scalar, never a `{"$text": ..}` map, and repeated siblings are one list in document order | yes |
| R4 | the merged tree deserializes into `T` leniently, the way conflaguration already treats env values (`FromStr`, `crates/conflaguration-core/src/lib.rs:22-27`): a string where `T` wants a bool, an integer or a float is parsed; a single value where `T` wants a list becomes a one-element list. A string that does not parse fails with `Error::ValueDeserialize` whose text contains that string. Values that already have the wanted type are untouched | yes |
| R5 | `conflaguration::Tree`: a free-form value for fields whose shape is not known in advance (a request payload). Its `Deserialize` visitor keeps maps, lists and scalars, groups repeated map keys into a list (never last-one-wins), and turns a map whose only key is `$text` into that scalar. It deserializes from all six formats; with feature `json`, `serde_json::Value::from(Tree)` converts it | yes |
| R6 | incumbents unchanged: the full suite passes with its post-CONFLAG-2 count plus the new tests; no existing `pub` item is removed or changes signature; `default = ["std"]` is unchanged | yes |

## literals

The fixture structs (in `tests/formats.rs`, test-only):

```rust
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fixture { name: String, port: u16, debug: bool, ratio: f64,
                 tags: Vec<String>, single: Vec<String>, inner: Inner, absent: Option<String> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Inner { retries: u32, enabled: bool }
```

- F1, the expected value: `Fixture { name: "review", port: 8080, debug: true, ratio: 0.5, tags:
  ["a", "b", "c"], single: ["only"], inner: Inner { retries: 3, enabled: false }, absent: None }`.
- F2, the XML source `tests/fixtures/formats/fixture.xml`:
  `<config><name>review</name><port>8080</port><debug>true</debug><ratio>0.5</ratio><tags>a</tags><tags>b</tags><tags>c</tags><single>only</single><inner><retries>3</retries><enabled>false</enabled></inner></config>`.
  The TOML, JSON, YAML, RON and JSON5 twins are F1 written natively in each format, in
  `tests/fixtures/formats/fixture.{toml,json,yaml,ron,json5}`.
- B0, the base value layers go over: `Fixture { name: "base", port: 1, debug: false, ratio: 0.0,
  tags: [], single: [], inner: Inner { retries: 0, enabled: true }, absent: None }`.
- F5, an overlay layer `tests/fixtures/formats/overlay.{xml,ron,json5}` setting only `absent` to
  `"filled"` and `port` to `9` (XML: `<config><absent>filled</absent><port>9</port></config>`).
- F3, a free-form payload, `tests/fixtures/formats/payload.{xml,toml,json,yaml,ron,json5}`; XML:
  `<payload><method>tools/call</method><id>1</id><args><id>A</id><id>B</id></args></payload>`; the
  other five write `method = "tools/call"`, `id = "1"`, `args.id = ["A", "B"]` natively. As `Tree`
  converted to JSON, every one equals exactly `{"args":{"id":["A","B"]},"id":"1","method":"tools/call"}`
  (keys sorted: `serde_json` without `preserve_order`).
- F4, a TOML layer over B0 with quoted scalars: `port = "8080"`, `debug = "true"`, `ratio = "0.5"`,
  `single = "only"`; and F4x: `port = "eighty"`.

## architecture

```rust
// src/file.rs, beside from_toml_str / from_json_str / from_yaml_str (:9-24)
#[cfg(feature = "xml")]   pub fn from_xml_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
#[cfg(feature = "ron")]   pub fn from_ron_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
#[cfg(feature = "json5")] pub fn from_json5_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
// from_file and file_to_value gain the three extension arms (:32-52, :61-81); file_to_value's
// XML arm deserializes a Tree (R5) and converts it into serde_value::Value
// merge_file / merge_mapping deserialize through a lenient Deserializer over serde_value::Value:
// deserialize_bool / _u* / _i* / _f* parse a Value::String with FromStr, deserialize_seq wraps a
// non-Seq in a one-element Seq, everything else forwards to serde_value's own

// src/tree.rs (new), re-exported from lib.rs
pub struct Tree(/* serde_value::Value */);   // Deserialize: group repeated keys; collapse a lone `$text`
#[cfg(feature = "json")] impl From<Tree> for serde_json::Value;
```

Cargo features: `xml = ["dep:serde", "dep:quick-xml", "dep:serde-value"]` with
`cargo add --optional quick-xml --features serialize`; `ron = [...,"dep:ron"]`, `json5 = [...,"dep:json5"]`
via `cargo add --optional`.

### decisions

| decision | chosen | why not the alternative |
|---|---|---|
| XML into a typed target | `quick_xml::de::from_str::<T>` direct | an untyped tree first loses what the target type knows (numbers, single-element lists); `from_file` already deserializes direct for the other formats (`src/file.rs:28-53`) |
| XML in builder layers | `Tree` visitor builds the merge tree, then lenient deserialize | layers are partial, so they cannot be parsed straight into `T`; `deserialize_any` alone yields `$text` maps and drops repeated keys |
| leniency for every layer | yes | the merged tree does not record each value's source; a typed value is never touched, so the only new behaviour for TOML / JSON / YAML is that a quoted bool, number, or single value where a list is wanted now loads (F4) |
| a free-form type | `Tree` in conflaguration | a payload typed `serde_json::Value` takes XML through `deserialize_any`: `$text` maps and last-one-wins (`serde_json-1.0.151/src/value/de.rs:134-142`, measured in PROXIMA-47) |
| RON and JSON5 | in scope | proxima's registry reads them today (`proxima/proxima-config/src/lib.rs:250-257`) and the owner chose to delegate it to conflaguration |

## acceptance criteria

Every cargo command runs as `timeout -k 30s 25m git wt cargo ...`, `CARGO_BUILD_JOBS=2`. `F6` below is
the feature set `toml,json,yaml,xml,ron,json5`. Helpers in `tests/formats.rs`: `check_fixture(actual,
expected) -> Result<(), String>` (equality), `check_json(tree, expected_json) -> Result<(), String>`
(converts and compares), `check_error(result, needle) -> Result<(), String>` (an
`Error::ValueDeserialize` whose text contains `needle`). Prefixes: `format_from_file_`,
`format_layered_`, `format_overlay_`, `format_lenient_`, `format_tree_`, `ctl_formats_`; no prefix
contains another.

| id | discharges | command | expected |
|---|---|---|---|
| AC1 | R1, R2 | `cargo val --features F6 --test formats -E 'test(format_from_file_)'` | 6 passed: `from_file` on each of `fixture.{toml,json,yaml,xml,ron,json5}`, `check_fixture(_, F1)` -> Ok |
| AC2 | R3, R4 | `cargo val --features F6 --test formats -E 'test(format_layered_)'` | 6 passed: `builder::<Fixture>().value(B0).file(<twin>).build()` for each twin (a non-empty chain, so `merge_file`), `check_fixture(_, F1)` -> Ok |
| AC8 | R3 | `cargo val --features F6 --test formats -E 'test(format_overlay_)'` | 3 passed: `builder::<Fixture>().value(F1).overlay_file(overlay.{xml,ron,json5}).build()`, `check_fixture(_, F1 with absent: Some("filled"))` -> Ok (port stays 8080: overlay never replaces a present value) |
| AC3 | R4 | `cargo val --features toml --test formats -E 'test(format_lenient_)'` | 2 passed: F4 over B0 gives `port 8080`, `debug true`, `ratio 0.5`, `single ["only"]` (`check_fixture` against B0 with those four fields); F4x over B0, `check_error(_, "eighty")` -> Ok |
| AC4 | R5 | `cargo val --features F6 --test formats -E 'test(format_tree_)'` | 6 passed: each `payload.*` deserialized as `Tree`, `check_json(_, F3's JSON)` -> Ok |
| AC5 | R1-R5 controls | `cargo val --features F6 --test formats -E 'test(ctl_formats_)'` | 7 passed: `check_fixture(F1, F1)` -> Ok; `check_fixture(F1 with single: [], F1)` -> Err; `check_json(Tree of payload.json, F3)` -> Ok; `check_json(Tree of payload.json, F3 with "id":["A"])` -> Err; `check_error(Err(ValueDeserialize("eighty ...")), "eighty")` -> Ok; `check_error(Ok(B0), "eighty")` -> Err; `quick_xml::de::from_str::<serde_json::Value>` on F2 gives `tags` as one `{"$text":"c"}` (the defect R3/R5 remove; measured by the round-1 audit) |
| AC6 | R6 | `cargo val --workspace --all-features --output plain` (all targets plus doc-tests, as the baseline ran) | 330 passed, 0 failed (298 after CONFLAG-2, `<tickets>/CONFLAG-2/evidence/full.log`, plus the 32 new tests) |
| AC7 | R6 | (a) `grep -c '^default = \["std"\]$' Cargo.toml`; (b) `git diff beded1a -- src/ crates/ \| grep -c -E '^-[[:space:]]*pub (fn\|struct\|enum\|trait\|type\|use\|mod) '` | (a) 1; (b) 0 (no `pub` item line removed or rewritten). Control for (b): the same pipeline over a scratch diff that renames one `pub fn` prints 1 |

Counts: AC1 6 + AC2 6 + AC8 3 + AC3 2 + AC4 6 + AC5 7 = 30 new tests in `tests/formats.rs`,
plus 2 doc-tests on `from_xml_str` and `Tree` = 32.

## out of scope

- attributes, namespaces and mixed content in XML config (element-only configs)
- writing configs out in any format (serializers)
- proxima delegating `proxima-config` to conflaguration: proxima's spec, after this release
- publishing the release: owner's go

## risks

| risk | likelihood | what it costs | what we do about it |
|---|---|---|---|
| lenient coercion changes a test that relied on a type error | low | AC6 below 330 | AC6 is the measurement; any such test is reported, not rewritten |
| quick-xml behaviour differs from the 0.41.0 trace on the version `cargo add` picks | low | AC1 / AC4 XML cases fail | pin the version proxima locks (0.41.0) if a newer one differs |
| the shared machine's load guard refuses runs | high | delay | wrap every run; coordinate quiet windows with peer sessions |

## context

- `src/file.rs` (formats, merge), `src/builder.rs` (layer methods: first source vs merge at `:219-225`), `src/lib.rs:280-295` (re-exports)
- `docs/guide.md` sections 2-3 (nesting, builder sources)
- `<tickets>/PROXIMA-47/` (format inventory, XML probes), `<tickets>/PROXIMA-48/` (proxima coverage), `<tickets>/CONFLAG-2/` (green baseline)
