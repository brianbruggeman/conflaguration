//! Divan benches for conflaguration's five suspect hot spots (see
//! `docs/discipline.md`): derive/`from_env` field-count scaling, file load,
//! builder merge layering, template expansion, and `join_key` in isolation.
//!
//! Run: `cargo bench --bench bench_conflaguration --features derive,toml,json`

// bench fixtures panic loudly on setup failure rather than silently mis-measuring.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::io::Write;

use conflaguration::template::{EnvResolver, render};
use conflaguration::{ConfigBuilder, Settings, Validate, from_file};
use divan::AllocProfiler;
use figment::Figment;
use figment::providers::{Format, Toml};
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

// ---------------------------------------------------------------------
// 1. derive / from_env — flat 5-field, flat 20-field, 3-level nested
// ---------------------------------------------------------------------

#[derive(Settings)]
#[settings(prefix = "BENCH_FLAT5")]
struct Flat5Config {
    #[setting(default = 8080)]
    field_a: u16,
    #[setting(default = "localhost")]
    field_b: String,
    #[setting(default = false)]
    field_c: bool,
    #[setting(default = 30)]
    field_d: u64,
    #[setting(default = 1.5)]
    field_e: f64,
}

#[derive(Settings)]
#[settings(prefix = "BENCH_FLAT20")]
struct Flat20Config {
    #[setting(default = 1)]
    field_01: u32,
    #[setting(default = 2)]
    field_02: u32,
    #[setting(default = 3)]
    field_03: u32,
    #[setting(default = 4)]
    field_04: u32,
    #[setting(default = 5)]
    field_05: u32,
    #[setting(default = 6)]
    field_06: u32,
    #[setting(default = 7)]
    field_07: u32,
    #[setting(default = 8)]
    field_08: u32,
    #[setting(default = 9)]
    field_09: u32,
    #[setting(default = 10)]
    field_10: u32,
    #[setting(default = "a")]
    field_11: String,
    #[setting(default = "b")]
    field_12: String,
    #[setting(default = "c")]
    field_13: String,
    #[setting(default = "d")]
    field_14: String,
    #[setting(default = "e")]
    field_15: String,
    #[setting(default = false)]
    field_16: bool,
    #[setting(default = true)]
    field_17: bool,
    #[setting(default = 1.1)]
    field_18: f64,
    #[setting(default = 2.2)]
    field_19: f64,
    #[setting(default = 3.3)]
    field_20: f64,
}

#[derive(Settings)]
#[settings(prefix = "BENCH_NEST_LEAF")]
struct NestedLeaf {
    #[setting(default = "leaf")]
    value: String,
    #[setting(default = 1)]
    count: u32,
}

#[derive(Settings)]
struct NestedMiddle {
    #[setting(nested)]
    leaf: NestedLeaf,
    #[setting(default = "middle")]
    label: String,
}

#[derive(Settings)]
#[settings(prefix = "BENCH_NEST_ROOT")]
struct Nested3Config {
    #[setting(nested)]
    middle: NestedMiddle,
    #[setting(default = "root")]
    label: String,
}

#[divan::bench]
fn from_env_flat5(bencher: divan::Bencher) {
    bencher.bench(|| Flat5Config::from_env().expect("flat5 from_env"));
}

#[divan::bench]
fn from_env_flat20(bencher: divan::Bencher) {
    bencher.bench(|| Flat20Config::from_env().expect("flat20 from_env"));
}

#[divan::bench]
fn from_env_nested3(bencher: divan::Bencher) {
    bencher.bench(|| Nested3Config::from_env().expect("nested3 from_env"));
}

// ---------------------------------------------------------------------
// 1b. env into struct — home-turf incumbent arm (gates 6/13, docs/discipline.md).
//
// envy is the identical design point to conflaguration::from_env(): one call,
// serde struct, env prefix. Both sides here read the EXACT SAME env vars —
// conflaguration's join_key(prefix, FIELD_NAME) produces
// "BENCH_INCUMBENTxx_FIELD_NN", and envy::prefixed("BENCH_INCUMBENTxx_")
// strips that identical literal prefix before a case-insensitive field-name
// match — so neither side is handicapped by naming convention. A fresh
// "INCUMBENT" prefix (distinct from BENCH_FLAT5/BENCH_FLAT20 above) keeps
// this from mutating the process env the existing default-fallback arms
// depend on.
// ---------------------------------------------------------------------

#[derive(Settings)]
#[settings(prefix = "BENCH_INCUMBENT20")]
struct IncumbentFlat20Config {
    #[setting(default = 1)]
    field_01: u32,
    #[setting(default = 2)]
    field_02: u32,
    #[setting(default = 3)]
    field_03: u32,
    #[setting(default = 4)]
    field_04: u32,
    #[setting(default = 5)]
    field_05: u32,
    #[setting(default = 6)]
    field_06: u32,
    #[setting(default = 7)]
    field_07: u32,
    #[setting(default = 8)]
    field_08: u32,
    #[setting(default = 9)]
    field_09: u32,
    #[setting(default = 10)]
    field_10: u32,
    #[setting(default = "a")]
    field_11: String,
    #[setting(default = "b")]
    field_12: String,
    #[setting(default = "c")]
    field_13: String,
    #[setting(default = "d")]
    field_14: String,
    #[setting(default = "e")]
    field_15: String,
    #[setting(default = false)]
    field_16: bool,
    #[setting(default = true)]
    field_17: bool,
    #[setting(default = 1.1)]
    field_18: f64,
    #[setting(default = 2.2)]
    field_19: f64,
    #[setting(default = 3.3)]
    field_20: f64,
}

// fields exist only to receive envy's deserialize; the bench measures
// construction cost, never reads a field back out.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EnvyFlat20Config {
    field_01: u32,
    field_02: u32,
    field_03: u32,
    field_04: u32,
    field_05: u32,
    field_06: u32,
    field_07: u32,
    field_08: u32,
    field_09: u32,
    field_10: u32,
    field_11: String,
    field_12: String,
    field_13: String,
    field_14: String,
    field_15: String,
    field_16: bool,
    field_17: bool,
    field_18: f64,
    field_19: f64,
    field_20: f64,
}

#[derive(Settings)]
#[settings(prefix = "BENCH_INCUMBENT5")]
struct IncumbentFlat5Config {
    #[setting(default = 8080)]
    field_a: u16,
    #[setting(default = "localhost")]
    field_b: String,
    #[setting(default = false)]
    field_c: bool,
    #[setting(default = 30)]
    field_d: u64,
    #[setting(default = 1.5)]
    field_e: f64,
}

// fields exist only to receive envy's deserialize; the bench measures
// construction cost, never reads a field back out.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EnvyFlat5Config {
    field_a: u16,
    field_b: String,
    field_c: bool,
    field_d: u64,
    field_e: f64,
}

fn set_incumbent20_env() {
    // SAFETY: single-threaded bench binary; set once before each timed loop,
    // on a prefix no other bench arm reads.
    unsafe {
        for index in 1..=10u32 {
            std::env::set_var(format!("BENCH_INCUMBENT20_FIELD_{index:02}"), index.to_string());
        }
        for (offset, letter) in ('a'..='e').enumerate() {
            let index = offset + 11;
            std::env::set_var(format!("BENCH_INCUMBENT20_FIELD_{index:02}"), letter.to_string());
        }
        std::env::set_var("BENCH_INCUMBENT20_FIELD_16", "false");
        std::env::set_var("BENCH_INCUMBENT20_FIELD_17", "true");
        std::env::set_var("BENCH_INCUMBENT20_FIELD_18", "1.1");
        std::env::set_var("BENCH_INCUMBENT20_FIELD_19", "2.2");
        std::env::set_var("BENCH_INCUMBENT20_FIELD_20", "3.3");
    }
}

fn set_incumbent5_env() {
    // SAFETY: single-threaded bench binary; set once before each timed loop.
    unsafe {
        std::env::set_var("BENCH_INCUMBENT5_FIELD_A", "9090");
        std::env::set_var("BENCH_INCUMBENT5_FIELD_B", "example.internal");
        std::env::set_var("BENCH_INCUMBENT5_FIELD_C", "true");
        std::env::set_var("BENCH_INCUMBENT5_FIELD_D", "60");
        std::env::set_var("BENCH_INCUMBENT5_FIELD_E", "2.5");
    }
}

#[divan::bench]
fn from_env_flat20_incumbent_conflaguration(bencher: divan::Bencher) {
    set_incumbent20_env();
    bencher.bench(|| IncumbentFlat20Config::from_env().expect("incumbent flat20 from_env"));
}

// design-favors: incumbent — the load-bearing home-turf arm (gate 13). Same
// 20 env vars as the conflaguration arm above, read through
// envy::prefixed("BENCH_INCUMBENT20_").from_env::<T>(), envy's single-call
// serde env->struct entry point — the identical design point to
// conflaguration::from_env().
#[divan::bench]
fn from_env_flat20_incumbent_envy(bencher: divan::Bencher) {
    set_incumbent20_env();
    bencher.bench(|| {
        envy::prefixed("BENCH_INCUMBENT20_")
            .from_env::<EnvyFlat20Config>()
            .expect("envy flat20 from_env")
    });
}

#[divan::bench]
fn from_env_flat5_incumbent_conflaguration(bencher: divan::Bencher) {
    set_incumbent5_env();
    bencher.bench(|| IncumbentFlat5Config::from_env().expect("incumbent flat5 from_env"));
}

// design-favors: incumbent — second input size for the envy head-to-head
// (variance discipline: two sizes per arm pair).
#[divan::bench]
fn from_env_flat5_incumbent_envy(bencher: divan::Bencher) {
    set_incumbent5_env();
    bencher.bench(|| {
        envy::prefixed("BENCH_INCUMBENT5_")
            .from_env::<EnvyFlat5Config>()
            .expect("envy flat5 from_env")
    });
}

// ---------------------------------------------------------------------
// 2. file load — from_file() for toml/json, small vs ~200-key
// ---------------------------------------------------------------------

fn small_toml() -> String {
    "field_a = \"one\"\nfield_b = \"two\"\nfield_c = \"three\"\n".to_owned()
}

fn small_json() -> String {
    r#"{"field_a": "one", "field_b": "two", "field_c": "three"}"#.to_owned()
}

fn wide_toml(key_count: usize) -> String {
    let mut buffer = String::with_capacity(key_count * 24);
    for index in 0..key_count {
        buffer.push_str(&format!("key_{index} = \"value_{index}\"\n"));
    }
    buffer
}

fn wide_json(key_count: usize) -> String {
    let mut buffer = String::with_capacity(key_count * 24);
    buffer.push('{');
    for index in 0..key_count {
        if index > 0 {
            buffer.push(',');
        }
        buffer.push_str(&format!("\"key_{index}\":\"value_{index}\""));
    }
    buffer.push('}');
    buffer
}

fn write_temp(extension: &str, contents: &str) -> tempfile::TempPath {
    let mut file = tempfile::Builder::new().suffix(extension).tempfile().expect("tempfile");
    file.write_all(contents.as_bytes()).expect("write tempfile");
    file.flush().expect("flush tempfile");
    file.into_temp_path()
}

#[divan::bench]
fn from_file_toml_small(bencher: divan::Bencher) {
    let path = write_temp(".toml", &small_toml());
    bencher.bench(|| from_file::<HashMap<String, String>>(&path).expect("from_file toml small"));
}

#[divan::bench]
fn from_file_toml_wide_200(bencher: divan::Bencher) {
    let path = write_temp(".toml", &wide_toml(200));
    bencher.bench(|| from_file::<HashMap<String, String>>(&path).expect("from_file toml wide"));
}

#[divan::bench]
fn from_file_json_small(bencher: divan::Bencher) {
    let path = write_temp(".json", &small_json());
    bencher.bench(|| from_file::<HashMap<String, String>>(&path).expect("from_file json small"));
}

#[divan::bench]
fn from_file_json_wide_200(bencher: divan::Bencher) {
    let path = write_temp(".json", &wide_json(200));
    bencher.bench(|| from_file::<HashMap<String, String>>(&path).expect("from_file json wide"));
}

// ---------------------------------------------------------------------
// 3. builder merge — 2/3/4-layer file+overlay+env chains
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Default, Settings, Serialize, Deserialize)]
#[settings(prefix = "BENCH_MERGE")]
#[serde(default)]
struct MergeConfig {
    #[setting(default = 1)]
    alpha: u32,
    #[setting(default = 2)]
    beta: u32,
    #[setting(default = 3)]
    gamma: u32,
    #[setting(default = "d")]
    delta: String,
    #[setting(default = "e")]
    epsilon: String,
    #[setting(default = false)]
    zeta: bool,
    #[setting(default = 7)]
    eta: u32,
    #[setting(default = 8)]
    theta: u32,
    #[setting(default = 9)]
    iota: u32,
    #[setting(default = 10)]
    kappa: u32,
}

fn merge_layer_toml(offset: u32) -> String {
    format!("alpha = {offset}\nbeta = {}\ngamma = {}\n", offset + 1, offset + 2)
}

#[divan::bench]
fn builder_merge_2_layer(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .file(&layer_1)
            .env()
            .build()
            .expect("2-layer build")
    });
}

#[divan::bench]
fn builder_merge_3_layer(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .file(&layer_1)
            .overlay_file(&layer_2)
            .env()
            .build()
            .expect("3-layer build")
    });
}

#[divan::bench]
fn builder_merge_4_layer(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    let layer_3 = write_temp(".toml", &merge_layer_toml(30));
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .file(&layer_1)
            .overlay_file(&layer_2)
            .file(&layer_3)
            .env()
            .build()
            .expect("4-layer build")
    });
}

// 6/8-layer: same alternating file/overlay_file shape, extended, to show
// whether the per-layer merge cache (src/file.rs::accumulator) turns the
// per-added-layer cost sub-linear rather than merely shifting the constant.
#[divan::bench]
fn builder_merge_6_layer(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    let layer_3 = write_temp(".toml", &merge_layer_toml(30));
    let layer_4 = write_temp(".toml", &merge_layer_toml(40));
    let layer_5 = write_temp(".toml", &merge_layer_toml(50));
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .file(&layer_1)
            .overlay_file(&layer_2)
            .file(&layer_3)
            .overlay_file(&layer_4)
            .file(&layer_5)
            .env()
            .build()
            .expect("6-layer build")
    });
}

#[divan::bench]
fn builder_merge_8_layer(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    let layer_3 = write_temp(".toml", &merge_layer_toml(30));
    let layer_4 = write_temp(".toml", &merge_layer_toml(40));
    let layer_5 = write_temp(".toml", &merge_layer_toml(50));
    let layer_6 = write_temp(".toml", &merge_layer_toml(60));
    let layer_7 = write_temp(".toml", &merge_layer_toml(70));
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .file(&layer_1)
            .overlay_file(&layer_2)
            .file(&layer_3)
            .overlay_file(&layer_4)
            .file(&layer_5)
            .overlay_file(&layer_6)
            .file(&layer_7)
            .env()
            .build()
            .expect("8-layer build")
    });
}

// task 5(c) wrong-thing-measured check: the builder_merge_* benches above
// read a real temp file per layer inside the timed closure, so their number
// is merge-cost-plus-file-I/O, not merge cost in isolation. This variant
// swaps every file()/overlay_file() layer for mapping()/overlay_mapping() on
// an in-memory struct to see how much of the file-based number is disk I/O.
//
// Layer count and shape are matched to `builder_merge_8_layer` exactly: 7
// layers in the same override/overlay/override/overlay/override/overlay/override
// alternation, seeded by `.mapping()` rather than `.value()` so the first
// call also pays the one-shot deserialize `.file()` pays on its first call
// (`crate::file::from_mapping` vs `crate::from_file`), and every later call
// hits `merge_mapping`/`merge_file` (`src/builder.rs:223-236`) for a genuine
// serde_value round trip. Both arms: 1 one-shot deserialize + 6 round trips.
#[divan::bench]
fn builder_merge_8_layer_in_memory(bencher: divan::Bencher) {
    let layer_2 = MergeConfig {
        alpha: 21,
        beta: 22,
        gamma: 23,
        ..Default::default()
    };
    let layer_4 = MergeConfig {
        alpha: 41,
        beta: 42,
        gamma: 43,
        ..Default::default()
    };
    let layer_6 = MergeConfig {
        alpha: 61,
        beta: 62,
        gamma: 63,
        ..Default::default()
    };
    bencher.bench(|| {
        ConfigBuilder::<MergeConfig>::new()
            .mapping(MergeConfig {
                alpha: 11,
                beta: 12,
                gamma: 13,
                ..Default::default()
            })
            .overlay_mapping(layer_2.clone())
            .mapping(MergeConfig {
                alpha: 31,
                beta: 32,
                gamma: 33,
                ..Default::default()
            })
            .overlay_mapping(layer_4.clone())
            .mapping(MergeConfig {
                alpha: 51,
                beta: 52,
                gamma: 53,
                ..Default::default()
            })
            .overlay_mapping(layer_6.clone())
            .mapping(MergeConfig {
                alpha: 71,
                beta: 72,
                gamma: 73,
                ..Default::default()
            })
            .env()
            .build()
            .expect("8-layer in-memory build")
    });
}

// degenerate control (task 5): a hand-rolled 7-step "merge" that never reads
// the incoming layers or goes through the serde_value medium at all — pure
// `Clone` of the base struct, seven times over. If this control does NOT run
// dramatically faster than `builder_merge_8_layer`, the real bench above is
// not actually measuring merge+deserialize work and its number is void.
#[divan::bench]
fn builder_merge_8_layer_control_skip_merge(bencher: divan::Bencher) {
    let base = MergeConfig::default();
    bencher.bench(|| {
        let mut value = base.clone();
        for _ in 0..7 {
            value = std::hint::black_box(value.clone());
        }
        value
    });
}

// ---------------------------------------------------------------------
// 3b. layered merge — incumbent arms (gates 6/13). Same two TOML layers
// (offset 10, 20 via merge_layer_toml, the same helper builder_merge_3_layer
// above uses) and the same MergeConfig target struct across all three
// libraries. conflaguration's own number for this shape is
// `builder_merge_3_layer` above (file() + overlay_file(), no new arm needed
// — it already exists).
// ---------------------------------------------------------------------

// design-favors: incumbent — config-rs ConfigBuilder, the incumbent this
// crate's own ConfigBuilder is named after. config::Config::builder()'s
// add_source is an override-style merge (later source replaces matching
// keys), not conflaguration's overlay_file semantics (fills gaps, keeps
// existing non-null values) — same layer count/content, different merge
// semantics; noted, not hidden.
#[divan::bench]
fn builder_merge_2_toml_layers_incumbent_config(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    bencher.bench(|| {
        config::Config::builder()
            .add_source(config::File::from(layer_1.to_path_buf()))
            .add_source(config::File::from(layer_2.to_path_buf()))
            .build()
            .expect("config 2-layer build")
            .try_deserialize::<MergeConfig>()
            .expect("config 2-layer deserialize")
    });
}

// design-favors: incumbent — figment, the second layered-merge incumbent.
// figment's merge()/join() distinguish override vs overlay exactly like
// conflaguration's file()/overlay_file(), so this mirrors
// `builder_merge_3_layer`'s semantics precisely: merge(layer_1) sets the
// base, join(layer_2) fills gaps only without clobbering layer_1's values.
#[divan::bench]
fn builder_merge_2_toml_layers_incumbent_figment(bencher: divan::Bencher) {
    let layer_1 = write_temp(".toml", &merge_layer_toml(10));
    let layer_2 = write_temp(".toml", &merge_layer_toml(20));
    bencher.bench(|| {
        Figment::new()
            .merge(Toml::file(&layer_1))
            .join(Toml::file(&layer_2))
            .extract::<MergeConfig>()
            .expect("figment 2-layer extract")
    });
}

// ---------------------------------------------------------------------
// 4. template expansion — 1KB no-match, 1KB 10-match, 100KB
// ---------------------------------------------------------------------

fn template_input(total_bytes: usize, match_count: usize) -> Vec<u8> {
    let token = "{{ env::BENCH_TEMPLATE_VAR }}";
    let mut buffer = Vec::with_capacity(total_bytes);
    let filler = b"the quick brown fox jumps over the lazy dog; ";
    let mut inserted = 0;
    while buffer.len() + token.len() < total_bytes {
        if inserted < match_count && buffer.len() % 97 == 0 {
            buffer.extend_from_slice(token.as_bytes());
            inserted += 1;
        } else {
            buffer.extend_from_slice(filler);
        }
    }
    while buffer.len() < total_bytes {
        buffer.push(b'.');
    }
    buffer
}

#[divan::bench]
fn template_1kb_no_match(bencher: divan::Bencher) {
    let input = template_input(1024, 0);
    bencher.bench(|| render(&input, &EnvResolver).expect("no-match input never errors"));
}

#[divan::bench]
fn template_1kb_10_match(bencher: divan::Bencher) {
    // SAFETY: single-threaded bench binary, set once before the timed loop.
    unsafe { std::env::set_var("BENCH_TEMPLATE_VAR", "x") };
    let input = template_input(1024, 10);
    bencher.bench(|| render(&input, &EnvResolver).expect("template 1kb 10 match"));
}

#[divan::bench]
fn template_100kb_matches(bencher: divan::Bencher) {
    unsafe { std::env::set_var("BENCH_TEMPLATE_VAR", "x") };
    let input = template_input(100 * 1024, 50);
    bencher.bench(|| render(&input, &EnvResolver).expect("template 100kb"));
}

// ---------------------------------------------------------------------
// 5. join_key in isolation
// ---------------------------------------------------------------------

#[divan::bench]
fn join_key_empty_prefix(bencher: divan::Bencher) {
    bencher.bench(|| conflaguration::join_key(std::hint::black_box(""), std::hint::black_box("SEGMENT")));
}

#[divan::bench]
fn join_key_with_prefix(bencher: divan::Bencher) {
    bencher.bench(|| conflaguration::join_key(std::hint::black_box("APP_DATABASE"), std::hint::black_box("HOST")));
}

// degenerate control (task 5): the exact pre-optimization body of
// `join_key`, as its own function taking opaque `&str` parameters (matching
// the original signature) so the compiler cannot const-propagate the
// literal lengths the way it can when `format!` args are written inline at
// the call site. If this bench does NOT show the old two-allocator-op cost
// (1 undersized alloc + 1 grow), the harness is not actually isolating what
// changed and the join_key_with_prefix win is void.
fn join_key_pre_fix(prefix: &str, segment: &str) -> String {
    format!("{prefix}_{segment}")
}

#[divan::bench]
fn join_key_control_old_format_shape(bencher: divan::Bencher) {
    bencher.bench(|| join_key_pre_fix(std::hint::black_box("APP_DATABASE"), std::hint::black_box("HOST")));
}

// ---------------------------------------------------------------------
// 6. end-to-end service boot — gate 7. Composes every gap above (layered
// file merge, env override, template render, validated build) into the one
// number a real service's startup actually pays, so the micro-arms above
// can be read in proportion to it rather than in isolation.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Settings, Validate, Serialize, Deserialize)]
struct DatabaseConfig {
    #[setting(default = "postgres://localhost:5432/app")]
    url: String,
    #[setting(default = 10)]
    pool_size: u32,
}

#[derive(Debug, Clone, Settings, Validate, Serialize, Deserialize)]
#[settings(prefix = "BENCH_E2E_SVC")]
struct ServiceConfig {
    #[setting(default = "0.0.0.0")]
    bind_host: String,
    #[setting(default = 8080)]
    bind_port: u16,
    #[setting(nested)]
    database: DatabaseConfig,
    #[setting(default = false)]
    tls_enabled: bool,
    #[setting(default = 30)]
    timeout_seconds: u64,
    #[setting(default = 4)]
    worker_count: u32,
}

fn e2e_base_toml() -> String {
    "bind_host = \"0.0.0.0\"\n\
     bind_port = 8080\n\
     tls_enabled = false\n\
     timeout_seconds = 30\n\
     worker_count = 4\n\
     \n\
     [database]\n\
     url = \"postgres://localhost:5432/app_dev\"\n\
     pool_size = 5\n"
        .to_owned()
}

fn e2e_override_toml() -> String {
    "bind_host = \"10.0.4.12\"\n\
     tls_enabled = true\n\
     \n\
     [database]\n\
     pool_size = 25\n"
        .to_owned()
}

const DB_URL_TEMPLATE: &[u8] = b"postgres://{{ env::BENCH_E2E_DB_USER }}:{{ env::BENCH_E2E_DB_PASS }}@{{ env::BENCH_E2E_DB_HOST }}:5432/{{ env::BENCH_E2E_DB_NAME }}";

const DB_HOST_TEMPLATE: &[u8] = b"{{ env::BENCH_E2E_DB_HOST }}";

fn set_e2e_boot_env() {
    // SAFETY: single-threaded bench binary; set once before the timed loop,
    // on a prefix no other bench arm reads.
    unsafe {
        std::env::set_var("BENCH_E2E_SVC_WORKER_COUNT", "16");
        std::env::set_var("BENCH_E2E_SVC_TIMEOUT_SECONDS", "45");
        std::env::set_var("BENCH_E2E_DB_USER", "svc_app");
        std::env::set_var("BENCH_E2E_DB_PASS", "s3cr3t-rotated");
        std::env::set_var("BENCH_E2E_DB_HOST", "db-primary.prod.internal");
        std::env::set_var("BENCH_E2E_DB_NAME", "app_production");
    }
}

// the whole thing a real service does once at boot: two config-file layers
// (base + env-specific override) read and merged, an env-var pass over
// worker/timeout, a template render of the database URL (the one path in
// the crate that could plausibly run again, e.g. on secret rotation), and a
// validated build into a nested typed struct. file writes and env vars are
// set in `with_inputs` (untimed setup, called once per iteration but
// excluded from the measured sample); the timed closure only reads, merges,
// renders, and builds.
#[divan::bench]
fn e2e_service_boot(bencher: divan::Bencher) {
    set_e2e_boot_env();
    bencher
        .with_inputs(|| (write_temp(".toml", &e2e_base_toml()), write_temp(".toml", &e2e_override_toml())))
        .bench_values(|(base_path, override_path)| {
            let rendered_url = render(DB_URL_TEMPLATE, &EnvResolver).expect("db url template renders");
            let rendered_url = String::from_utf8(rendered_url.into_owned()).expect("rendered url is utf8");

            ConfigBuilder::<ServiceConfig>::new()
                .file(&base_path)
                .file(&override_path)
                .env()
                .override_with(|config| config.database.url = rendered_url)
                .validate()
                .build()
                .expect("service boot builds")
        });
}

// smaller input size: one file layer, no env-var pass, a one-token template
// render. Same struct and same timed/setup split as `e2e_service_boot`, so
// the two arms show how boot cost scales with layer count.
#[divan::bench]
fn e2e_service_boot_minimal(bencher: divan::Bencher) {
    // SAFETY: single-threaded bench binary; set once before the timed loop.
    unsafe { std::env::set_var("BENCH_E2E_DB_HOST", "db-primary.prod.internal") };
    bencher
        .with_inputs(|| write_temp(".toml", &e2e_base_toml()))
        .bench_values(|base_path| {
            let rendered_host = render(DB_HOST_TEMPLATE, &EnvResolver).expect("db host template renders");
            let rendered_host = String::from_utf8(rendered_host.into_owned()).expect("rendered host is utf8");

            ConfigBuilder::<ServiceConfig>::new()
                .file(&base_path)
                .override_with(|config| config.database.url = format!("postgres://{rendered_host}:5432/app"))
                .validate()
                .build()
                .expect("minimal service boot builds")
        });
}
