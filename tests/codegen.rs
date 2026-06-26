#![cfg(feature = "derive")]

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
