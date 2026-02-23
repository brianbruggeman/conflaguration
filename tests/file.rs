#![cfg(all(feature = "derive", any(feature = "toml", feature = "json", feature = "yaml")))]
#![allow(dead_code)]

use std::io::Write;

use conflaguration::Settings;

#[derive(serde::Deserialize, Settings)]
#[settings(prefix = "FILE_TEST")]
struct FileTestConfig {
    #[setting(default = 8080)]
    port: u16,

    #[setting(default = "localhost")]
    host: String,

    #[setting(default = false)]
    debug: bool,
}

#[cfg(feature = "toml")]
#[test]
fn from_toml_str_parses_valid_toml() {
    let input = r#"
port = 3000
host = "tomlhost"
debug = true
"#;
    let config: FileTestConfig = conflaguration::from_toml_str(input).unwrap_or_else(|err| panic!("from_toml_str failed: {err}"));
    assert_eq!(config.port, 3000);
    assert_eq!(config.host, "tomlhost");
    assert!(config.debug);
}

#[cfg(feature = "json")]
#[test]
fn from_json_str_parses_valid_json() {
    let input = r#"{"port": 4000, "host": "jsonhost", "debug": false}"#;
    let config: FileTestConfig = conflaguration::from_json_str(input).unwrap_or_else(|err| panic!("from_json_str failed: {err}"));
    assert_eq!(config.port, 4000);
    assert_eq!(config.host, "jsonhost");
    assert!(!config.debug);
}

#[cfg(feature = "yaml")]
#[test]
fn from_yaml_str_parses_valid_yaml() {
    let input = "port: 5000\nhost: yamlhost\ndebug: true\n";
    let config: FileTestConfig = conflaguration::from_yaml_str(input).unwrap_or_else(|err| panic!("from_yaml_str failed: {err}"));
    assert_eq!(config.port, 5000);
    assert_eq!(config.host, "yamlhost");
    assert!(config.debug);
}

#[cfg(feature = "toml")]
#[test]
fn from_file_detects_toml_extension() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port = 9999\nhost = \"filetoml\"\ndebug = true\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    let config: FileTestConfig = conflaguration::from_file(&path).unwrap_or_else(|err| panic!("from_file failed: {err}"));
    assert_eq!(config.port, 9999);
    assert_eq!(config.host, "filetoml");
    assert!(config.debug);
}

#[cfg(feature = "json")]
#[test]
fn from_file_detects_json_extension() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.json");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, r#"{{"port": 8888, "host": "filejson", "debug": false}}"#).unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    let config: FileTestConfig = conflaguration::from_file(&path).unwrap_or_else(|err| panic!("from_file failed: {err}"));
    assert_eq!(config.port, 8888);
    assert_eq!(config.host, "filejson");
    assert!(!config.debug);
}

#[cfg(feature = "yaml")]
#[test]
fn from_file_detects_yaml_extension() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.yaml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port: 7777\nhost: fileyaml\ndebug: true\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    let config: FileTestConfig = conflaguration::from_file(&path).unwrap_or_else(|err| panic!("from_file failed: {err}"));
    assert_eq!(config.port, 7777);
    assert_eq!(config.host, "fileyaml");
    assert!(config.debug);
}

#[cfg(feature = "yaml")]
#[test]
fn from_file_detects_yml_extension() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.yml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port: 6666\nhost: fileyml\ndebug: false\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    let config: FileTestConfig = conflaguration::from_file(&path).unwrap_or_else(|err| panic!("from_file failed: {err}"));
    assert_eq!(config.port, 6666);
    assert_eq!(config.host, "fileyml");
    assert!(!config.debug);
}

#[cfg(feature = "toml")]
#[test]
fn from_file_returns_error_for_unsupported_format() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.ini");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port=1234").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    let result: conflaguration::Result<FileTestConfig> = conflaguration::from_file(&path);
    assert!(matches!(result, Err(conflaguration::Error::UnsupportedFormat(_))));
}

#[cfg(feature = "toml")]
#[test]
fn from_file_returns_error_for_missing_file() {
    let path = std::path::Path::new("/tmp/conflaguration_nonexistent_test_file.toml");
    let result: conflaguration::Result<FileTestConfig> = conflaguration::from_file(path);
    assert!(matches!(result, Err(conflaguration::Error::Io(_))));
}

#[cfg(feature = "toml")]
#[test]
fn from_file_then_env_layers_correctly() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port = 3000\nhost = \"filehost\"\ndebug = false\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    temp_env::with_vars([("FILE_TEST_PORT", Some("9090")), ("FILE_TEST_HOST", None::<&str>), ("FILE_TEST_DEBUG", None::<&str>)], || {
        let config: FileTestConfig = conflaguration::from_file_then_env(&path).unwrap_or_else(|err| panic!("from_file_then_env failed: {err}"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "filehost");
        assert!(!config.debug);
    });
}

#[cfg(feature = "toml")]
#[test]
fn from_file_then_env_preserves_all_file_values_when_no_env() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port = 3000\nhost = \"filehost\"\ndebug = true\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    temp_env::with_vars([("FILE_TEST_PORT", None::<&str>), ("FILE_TEST_HOST", None::<&str>), ("FILE_TEST_DEBUG", None::<&str>)], || {
        let config: FileTestConfig = conflaguration::from_file_then_env(&path).unwrap_or_else(|err| panic!("from_file_then_env failed: {err}"));
        assert_eq!(config.port, 3000);
        assert_eq!(config.host, "filehost");
        assert!(config.debug);
    });
}

#[cfg(feature = "toml")]
#[test]
fn from_file_then_env_then_applies_cli_overrides() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap_or_else(|err| panic!("create failed: {err}"));
    write!(file, "port = 3000\nhost = \"filehost\"\ndebug = false\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    drop(file);

    temp_env::with_vars([("FILE_TEST_PORT", Some("9090")), ("FILE_TEST_HOST", None::<&str>), ("FILE_TEST_DEBUG", None::<&str>)], || {
        let config: FileTestConfig = conflaguration::from_file_then_env_then(&path, |config: &mut FileTestConfig| {
            config.host = "clihost".into();
            config.debug = true;
        })
        .unwrap_or_else(|err| panic!("from_file_then_env_then failed: {err}"));
        assert_eq!(config.port, 9090);
        assert_eq!(config.host, "clihost");
        assert!(config.debug);
    });
}
