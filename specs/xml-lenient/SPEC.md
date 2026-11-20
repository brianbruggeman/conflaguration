# xml-lenient -- XML (and RON, JSON5) for conflaguration, typed

status: draft (audit pending)
owner: brian
created: 2026-11-20

Why now: the owner wants every proxima pipeline step configurable in any format, through
conflaguration ("conflaguration should let us do anything: json, toml, yaml, xml, etc.";
"conflaguration is nested config using Setting"), recorded in
`/Users/brianbruggeman/repos/slot-0/pm/topics/agentic-mcp.md`, section "any format, every step".
Every `file:line` below is in this repo (`slot-0/conflaguration-publish`) at `68fd339`, read
2026-11-20. Ticket evidence lives under
`/Users/brianbruggeman/repos/obsidian/slot-0/slot-0/20 - Collaboration and Records/22 - Investigations/22.1 - Tickets/`
(`<tickets>` below).

## problem

conflaguration loads TOML, JSON and YAML only (`src/file.rs:28-53`, features `Cargo.toml:62-64`),
so a config written in XML, RON or JSON5 cannot be loaded at all; and its builder merges file
layers through an untyped `serde_value::Value` (`src/file.rs:57-82,131-139`), which cannot carry
XML's text-only scalars (`"5"` where a `u16` is wanted) or its repeated sibling elements (a map
keeps one) -- measured by one fixture config, written once in each of the six formats, loading into
the same typed struct with the same values, both through `from_file` and through `builder().file()`.

## refutation condition

If the XML fixture cannot reach the same typed value as its TOML twin without the caller adding
per-field annotations (`#[serde(deserialize_with = ...)]`) to its config structs, the "any format,
same struct" frame is wrong for XML and this spec is struck.

## requirements

| id | requirement | testable in isolation |
|---|---|---|
| R1 | feature `xml`: `from_xml_str::<T>(&str) -> Result<T>` deserializes straight into `T` with `quick-xml`; `from_file` maps the `.xml` extension to it. Element text becomes the typed field (number, bool, string); repeated sibling elements fill a `Vec` field, including when there is exactly one; an absent element leaves an `Option` field `None` | yes |
| R2 | features `ron` and `json5`: `from_ron_str` / `from_json5_str` and the `.ron` / `.json5` extensions in `from_file`, the same way `toml` / `json` / `yaml` are wired today (`src/file.rs:9-53`) | yes |
| R3 | `builder().file(path)` / `.overlay_file(path)` accept `.xml`, `.ron`, `.json5`: `file_to_value` parses them into the merge tree; for XML, repeated sibling elements become one list in document order (never last-one-wins) | yes |
| R4 | the merged tree deserializes into `T` with the coercion conflaguration already applies to env values (`FromStr`, `crates/conflaguration-core/src/lib.rs:22-27`): a string where `T` wants a bool, integer or float is parsed; a single value where `T` wants a list becomes a one-element list. A string that does not parse fails with `Error::ValueDeserialize` whose text contains the offending string. Values that already have the wanted type are untouched, so TOML / JSON / YAML layers behave exactly as today | yes |
| R5 | `conflaguration::Tree`: a format-neutral free-form value for fields whose shape is not known in advance (a request payload). Deserializing it from any supported format keeps maps, lists and scalars, and groups repeated map keys into a list instead of keeping the last one; with feature `json`, `serde_json::Value::from(Tree)` converts it | yes |
| R6 | incumbents unchanged: the full suite passes with its post-CONFLAG-2 count; no existing public signature changes; default features unchanged (`default = ["std"]`) | yes |

## literals

The fixture struct (in `tests/formats.rs`, test-only):

```rust
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Fixture { name: String, port: u16, debug: bool, ratio: f64,
                 tags: Vec<String>, single: Vec<String>, inner: Inner, absent: Option<String> }
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
struct Inner { retries: u32, enabled: bool }
```

F1, the expected value, for every format: `Fixture { name: "review", port: 8080, debug: true,
ratio: 0.5, tags: ["a", "b", "c"], single: ["only"], inner: Inner { retries: 3, enabled: false },
absent: None }`.

F2, the XML source (`tests/fixtures/formats/fixture.xml`):

```xml
<config>
  <name>review</name><port>8080</port><debug>true</debug><ratio>0.5</ratio>
  <tags>a</tags><tags>b</tags><tags>c</tags>
  <single>only</single>
  <inner><retries>3</retries><enabled>false</enabled></inner>
</config>
```

The TOML, JSON, YAML, RON and JSON5 twins are F1 written natively in each format, in
`tests/fixtures/formats/fixture.{toml,json,yaml,ron,json5}`.

F3, a free-form payload in XML: `<payload><method>tools/call</method><id>1</id><args><id>A</id><id>B</id></args></payload>`
deserialized as `Tree` and converted to JSON equals `{"method":"tools/call","id":"1","args":{"id":["A","B"]}}`
(XML scalars stay strings in a free-form `Tree`: there is no target type to coerce to).

F4, a TOML layer that today fails: `port = "8080"` over a base with `port = 1` -- under R4 it
deserializes to `8080` (the env-style coercion now covers file layers); `port = "eighty"` fails with
an error containing `eighty`.

## architecture

```rust
// src/file.rs, beside from_toml_str / from_json_str / from_yaml_str (:9-24)
#[cfg(feature = "xml")]   pub fn from_xml_str<T: DeserializeOwned>(contents: &str) -> Result<T>;   // quick_xml::de::from_str
#[cfg(feature = "ron")]   pub fn from_ron_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
#[cfg(feature = "json5")] pub fn from_json5_str<T: DeserializeOwned>(contents: &str) -> Result<T>;
// from_file / file_to_value gain the three extension arms (:32-52, :61-81)
// XML into the merge tree: a Deserialize visitor that, on a repeated map key, turns the slot into
// a Seq and appends (shared with Tree's visitor)
// merge_file / merge_mapping deserialize through a lenient Deserializer over serde_value::Value
// (deserialize_bool / _u* / _i* / _f* parse a Value::String with FromStr; deserialize_seq wraps a
// non-Seq in a one-element Seq; everything else forwards)

// src/tree.rs (new), re-exported from lib.rs
pub struct Tree(/* serde_value::Value with repeated keys grouped */);
#[cfg(feature = "json")] impl From<Tree> for serde_json::Value;
```

Cargo features: `xml = ["dep:serde", "dep:quick-xml", "dep:serde-value"]`, `ron = [... "dep:ron" ...]`,
`json5 = [... "dep:json5" ...]`, dependencies added with `cargo add --optional`.

### decisions

| decision | chosen | why not the alternative |
|---|---|---|
| XML into a typed target | `quick_xml::de::from_str::<T>` direct | going through an untyped tree first loses exactly what the target type knows (numbers, single-element lists); `from_file` already deserializes direct for the other formats (`src/file.rs:28-53`) |
| XML in builder layers | grouped tree + lenient deserialize | layers are partial, so they cannot be parsed straight into `T`; coercion against `T` at the end is the same idea as env coercion |
| leniency for every layer, not only XML | yes | the merged tree does not record which layer a value came from; a typed value is never touched, so the only new behaviour for TOML / JSON / YAML is that a quoted number or bool now loads (F4) instead of failing |
| a free-form type | `Tree` in conflaguration | a payload field typed `serde_json::Value` takes XML through `deserialize_any`, which keeps the last of repeated keys (`serde_json-1.0.151/src/value/de.rs:134-142`, measured in PROXIMA-47); the grouping belongs in the format layer, not in each consumer |
| RON and JSON5 | in scope | proxima's own registry reads them today (`proxima/proxima-config/src/lib.rs:250-257`) and the owner chose to delegate it to conflaguration; dropping them would remove formats proxima users have |

## acceptance criteria

Runner `cargo val` / `cargo test` through `git wt cargo`, wrapped `timeout -k 30s 25m`,
`CARGO_BUILD_JOBS=2`.

| id | discharges | command | expected |
|---|---|---|---|
| AC1 | R1, R2 | `git wt cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_from_file_)'` | 6 passed: `from_file` on each of `fixture.{toml,json,yaml,xml,ron,json5}` returns exactly F1, through helper `check_fixture(value, F1)` |
| AC2 | R3, R4 | `git wt cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(format_layered_)'` | 6 passed: `builder::<Fixture>().file(<twin>)` for each twin returns exactly F1 (XML through the grouped tree + lenient deserialize) |
| AC3 | R4 | `git wt cargo val --features toml --test formats -E 'test(format_lenient_)'` | 2 passed: F4 -- `port = "8080"` layered over `port = 1` gives 8080; `port = "eighty"` gives `Error::ValueDeserialize` whose text contains `eighty` |
| AC4 | R5 | `git wt cargo val --features xml,json --test formats -E 'test(format_tree_)'` | 2 passed: F3's XML as `Tree` converts to exactly F3's JSON; the same payload written in TOML (`args.id = ["A","B"]`, `id = "1"`) converts to the same JSON |
| AC5 | controls | `git wt cargo val --features toml,json,yaml,xml,ron,json5 --test formats -E 'test(ctl_formats_)'` | 4 passed: `check_fixture` on F1 -> Ok; on F1 with `single: []` -> Err; `quick_xml::de::from_str::<serde_json::Value>` on F2 keeps one `tags` value, not three (the incumbent defect R3 removes, measured); `serde_json::from_str::<serde_json::Value>` on F3's XML-to-JSON with `args.id` as a single string compared to F3's JSON -> not equal |
| AC6 | R6 | `git wt cargo val --workspace --all-features --output plain` | the post-CONFLAG-2 count plus the 20 new tests, 0 failed |
| AC7 | R6 | `git diff 68fd339 -- Cargo.toml \| grep -E '^[-+]default'` | no output (default features unchanged) |

## out of scope

- schema-driven XML (attributes, namespaces, mixed content); config XML here is element-only
- writing configs out in any format (serializers)
- proxima delegating `proxima-config` to conflaguration: proxima's spec, after this release
- publishing the release: owner's go

## risks

| risk | likelihood | what it costs | what we do about it |
|---|---|---|---|
| quick-xml's typed deserializer differs from the spec for `Option` absent / single-element `Vec` | medium | AC1 XML fails | AC1 is the measurement; quick-xml documents both, the test decides |
| lenient coercion changes a test that relied on a type error | low | AC6 drops below the count | AC6 compares to the measured count; any such test is reported, not rewritten |
| the shared machine's load guard refuses runs | high | delay | wrap every run; coordinate quiet windows with peer sessions |

## context

- `src/file.rs` (formats, merge), `src/builder.rs` (layer methods), `src/lib.rs:280-295` (re-exports)
- `docs/guide.md` sections 2-3 (nesting, builder sources)
- `<tickets>/PROXIMA-47/` (format inventory, XML probes), `<tickets>/PROXIMA-48/` (proxima coverage)
