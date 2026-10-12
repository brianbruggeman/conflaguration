#![cfg(all(feature = "derive", any(feature = "default", feature = "std")))]

use conflaguration::ConfigCodegen;

#[derive(ConfigCodegen)]
#[settings(prefix = "APP")]
struct BuildConfig {
    pool_threads: u32,
    tracing: bool,
    log_level: String,
}

#[derive(ConfigCodegen)]
struct NoPrefix {
    workers: u16,
}

#[test]
fn const_module_renders_each_scalar_with_its_type() {
    let config = BuildConfig {
        pool_threads: 8,
        tracing: false,
        log_level: "info".into(),
    };
    let module = config.to_const_module();
    assert!(module.contains("pub const POOL_THREADS: u32 = 8;"), "got: {module}");
    assert!(module.contains("pub const TRACING: bool = false;"), "got: {module}");
    assert!(module.contains("pub const LOG_LEVEL: &str = \"info\";"), "got: {module}");
    assert!(module.starts_with("// generated"), "has a do-not-edit header, got: {module}");
}

#[test]
fn env_keys_derive_from_prefix_and_field_names() {
    let keys = BuildConfig::env_keys();
    assert_eq!(keys, vec!["APP_POOL_THREADS", "APP_TRACING", "APP_LOG_LEVEL"]);
}

#[test]
fn env_keys_without_prefix_use_bare_field_names() {
    assert_eq!(NoPrefix::env_keys(), vec!["WORKERS"]);
}

#[derive(ConfigCodegen)]
struct LiteralWidths {
    four_digits: u32,
    five_digits: usize,
    seven_digits: u64,
    negative: i64,
    widest: u64,
    flag: bool,
    label: String,
}

#[test]
fn integer_consts_group_digits_in_threes_at_five_or_more_digits() {
    let config = LiteralWidths {
        four_digits: 8192,
        five_digits: 65536,
        seven_digits: 1_234_567,
        negative: -1_234_567,
        widest: u64::MAX,
        flag: true,
        label: "wide".into(),
    };
    let module = config.to_const_module();
    assert!(module.contains("pub const FOUR_DIGITS: u32 = 8192;"), "got: {module}");
    assert!(module.contains("pub const FIVE_DIGITS: usize = 65_536;"), "got: {module}");
    assert!(module.contains("pub const SEVEN_DIGITS: u64 = 1_234_567;"), "got: {module}");
    assert!(module.contains("pub const NEGATIVE: i64 = -1_234_567;"), "got: {module}");
    assert!(module.contains("pub const WIDEST: u64 = 18_446_744_073_709_551_615;"), "got: {module}");
}

#[test]
fn bool_and_string_consts_are_not_regrouped() {
    let config = LiteralWidths {
        four_digits: 1,
        five_digits: 1,
        seven_digits: 1,
        negative: 1,
        widest: 1,
        flag: true,
        label: "12345".into(),
    };
    let module = config.to_const_module();
    assert!(module.contains("pub const FLAG: bool = true;"), "got: {module}");
    assert!(module.contains(r#"pub const LABEL: &str = "12345";"#), "got: {module}");
}

#[test]
fn string_values_are_quoted_and_escaped() {
    let config = BuildConfig {
        pool_threads: 1,
        tracing: true,
        log_level: r#"a"b"#.into(),
    };
    let module = config.to_const_module();
    // Debug formatting escapes the embedded quote
    assert!(module.contains(r#"pub const LOG_LEVEL: &str = "a\"b";"#), "got: {module}");
    assert!(module.contains("pub const TRACING: bool = true;"), "got: {module}");
}
