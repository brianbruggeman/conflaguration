use conflaguration::Error;
use conflaguration::Result;
use conflaguration::Settings;
use conflaguration::Validate;
use conflaguration::ValidationMessage;

#[derive(Default, Debug, PartialEq)]
#[cfg_attr(any(feature = "toml", feature = "json", feature = "yaml"), derive(serde::Deserialize, serde::Serialize))]
struct TestConfig {
    host: String,
    port: u16,
}

impl Settings for TestConfig {
    const PREFIX: Option<&'static str> = Some("BTEST");

    fn from_env() -> Result<Self> {
        Ok(Self {
            host: conflaguration::resolve_or_parse(&["BTEST_HOST"], "localhost")?,
            port: conflaguration::resolve_or(&["BTEST_PORT"], 8080)?,
        })
    }

    fn from_env_with_prefix(prefix: &str) -> Result<Self> {
        let host_key = format!("{prefix}_HOST");
        let port_key = format!("{prefix}_PORT");
        Ok(Self {
            host: conflaguration::resolve_or_parse(&[&host_key], "localhost")?,
            port: conflaguration::resolve_or(&[&port_key], 8080)?,
        })
    }

    fn override_from_env(&mut self) -> Result<()> {
        self.host = conflaguration::resolve_or_parse(&["BTEST_HOST"], &self.host)?;
        self.port = conflaguration::resolve_or(&["BTEST_PORT"], self.port)?;
        Ok(())
    }

    fn override_from_env_with_prefix(&mut self, prefix: &str) -> Result<()> {
        let host_key = format!("{prefix}_HOST");
        let port_key = format!("{prefix}_PORT");
        self.host = conflaguration::resolve_or_parse(&[&host_key], &self.host)?;
        self.port = conflaguration::resolve_or(&[&port_key], self.port)?;
        Ok(())
    }
}

impl Validate for TestConfig {
    fn validate(&self) -> Result<()> {
        let mut errors = vec![];
        if self.host.is_empty() {
            errors.push(ValidationMessage::new("host", "must not be empty"));
        }
        if self.port == 0 {
            errors.push(ValidationMessage::new("port", "must be > 0"));
        }
        if errors.is_empty() { Ok(()) } else { Err(Error::Validation { errors }) }
    }
}

#[test]
fn env_loads_from_environment() {
    temp_env::with_vars([("BTEST_HOST", Some("example.com")), ("BTEST_PORT", Some("9090"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "example.com");
        assert_eq!(config.port, 9090);
    });
}

#[test]
fn value_default_then_env_overrides() {
    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", Some("3000"))], || {
        let config: TestConfig = conflaguration::builder()
            .value(TestConfig::default())
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "");
        assert_eq!(config.port, 3000);
    });
}

#[test]
fn override_with_mutates_value() {
    temp_env::with_vars([("BTEST_HOST", Some("original.com")), ("BTEST_PORT", Some("8080"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .override_with(|config: &mut TestConfig| {
                config.host = "overridden.com".into();
            })
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "overridden.com");
        assert_eq!(config.port, 8080);
    });
}

#[test]
fn validate_passes_valid_config() {
    temp_env::with_vars([("BTEST_HOST", Some("localhost")), ("BTEST_PORT", Some("8080"))], || {
        let result: Result<TestConfig> = conflaguration::builder().env().validate().build();
        assert!(result.is_ok());
    });
}

#[test]
fn validate_rejects_invalid_config() {
    temp_env::with_vars([("BTEST_HOST", Some("")), ("BTEST_PORT", Some("0"))], || {
        let result: Result<TestConfig> = conflaguration::builder().env().validate().build();
        assert!(matches!(result, Err(Error::Validation { .. })));
    });
}

#[test]
fn build_without_source_returns_no_source() {
    let result: Result<TestConfig> = conflaguration::builder().build();
    assert!(matches!(result, Err(Error::NoSource)));
}

#[test]
fn error_state_short_circuits() {
    temp_env::with_vars([("BTEST_HOST", Some("")), ("BTEST_PORT", Some("0"))], || {
        let mut apply_called = false;
        let result: Result<TestConfig> = conflaguration::builder()
            .env()
            .validate()
            .override_with(|_| {
                apply_called = true;
            })
            .build();
        assert!(!apply_called);
        assert!(matches!(result, Err(Error::Validation { .. })));
    });
}

#[test]
fn env_with_prefix_constructs_from_scratch() {
    temp_env::with_vars(
        [
            ("MYAPP_HOST", Some("prefixed.com")),
            ("MYAPP_PORT", Some("4000")),
            ("BTEST_HOST", None::<&str>),
            ("BTEST_PORT", None::<&str>),
        ],
        || {
            let config: TestConfig = conflaguration::builder()
                .env_with_prefix("MYAPP")
                .build()
                .unwrap_or_else(|err| panic!("build failed: {err}"));
            assert_eq!(config.host, "prefixed.com");
            assert_eq!(config.port, 4000);
        },
    );
}

#[test]
fn env_with_prefix_overrides_existing() {
    temp_env::with_vars(
        [
            ("MYAPP_HOST", Some("prefixed.com")),
            ("MYAPP_PORT", Some("4000")),
            ("BTEST_HOST", Some("original.com")),
            ("BTEST_PORT", Some("8080")),
        ],
        || {
            let config: TestConfig = conflaguration::builder()
                .env()
                .env_with_prefix("MYAPP")
                .build()
                .unwrap_or_else(|err| panic!("build failed: {err}"));
            assert_eq!(config.host, "prefixed.com");
            assert_eq!(config.port, 4000);
        },
    );
}

#[cfg(feature = "toml")]
#[test]
fn file_then_env_then_apply() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "host = \"file.com\"\nport = 1234\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", Some("5555"))], || {
        let config: TestConfig = conflaguration::builder()
            .file(&path)
            .env()
            .override_with(|config: &mut TestConfig| {
                config.host = "cli.com".into();
            })
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "cli.com");
        assert_eq!(config.port, 5555);
    });
}

#[cfg(feature = "toml")]
#[test]
fn file_loads_config() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "host = \"fromfile.com\"\nport = 9999\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", None::<&str>)], || {
        let config: TestConfig = conflaguration::builder()
            .file(&path)
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromfile.com");
        assert_eq!(config.port, 9999);
    });
}

#[cfg(feature = "toml")]
#[test]
fn env_then_file_overwrites_env_state() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "host = \"fromfile.com\"\nport = 1111\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", Some("fromenv.com")), ("BTEST_PORT", Some("2222"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .file(&path)
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromfile.com", "file() after env() should overwrite");
        assert_eq!(config.port, 1111);
    });
}

#[cfg(feature = "toml")]
#[test]
fn file_then_env_overwrites_file_state() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "host = \"fromfile.com\"\nport = 1111\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", Some("fromenv.com")), ("BTEST_PORT", Some("2222"))], || {
        let config: TestConfig = conflaguration::builder()
            .file(&path)
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromenv.com", "env() after file() should overwrite");
        assert_eq!(config.port, 2222);
    });
}

#[cfg(feature = "toml")]
#[test]
fn file_overlay_preserves_keys_absent_from_file() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("partial.toml");
    // partial file: sets only port, omits host
    std::fs::write(&path, "port = 4321\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", Some("fromenv.com")), ("BTEST_PORT", Some("2222"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .file(&path)
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromenv.com", "file omits host, so the env value is kept");
        assert_eq!(config.port, 4321, "file sets port, so it overlays the env value");
    });
}

#[cfg(feature = "toml")]
#[test]
fn file_overlays_second_file() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let base = dir.path().join("base.toml");
    let patch = dir.path().join("patch.toml");
    std::fs::write(&base, "host = \"base.com\"\nport = 1000\n").unwrap_or_else(|err| panic!("write failed: {err}"));
    std::fs::write(&patch, "port = 2000\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", None::<&str>)], || {
        let config: TestConfig = conflaguration::builder()
            .file(&base)
            .file(&patch)
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "base.com", "patch omits host, base value kept");
        assert_eq!(config.port, 2000, "patch sets port, overlays base");
    });
}

#[cfg(feature = "toml")]
#[test]
fn overlay_file_keeps_existing_overwrites_nothing() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
    let path = dir.path().join("overlay.toml");
    // overlay tries to set both, but host already has a real value
    std::fs::write(&path, "host = \"fromfile.com\"\nport = 9999\n").unwrap_or_else(|err| panic!("write failed: {err}"));

    temp_env::with_vars([("BTEST_HOST", Some("fromenv.com")), ("BTEST_PORT", Some("2222"))], || {
        let config: TestConfig = conflaguration::builder()
            .env() // host=fromenv.com, port=2222 (real values)
            .overlay_file(&path) // overlay must not clobber existing non-null values
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromenv.com", "overlay keeps existing value");
        assert_eq!(config.port, 2222, "overlay keeps existing value");
    });
}

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
#[derive(serde::Serialize)]
struct PortPatch {
    port: u16,
}

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
#[test]
fn mapping_overrides_existing() {
    temp_env::with_vars([("BTEST_HOST", Some("fromenv.com")), ("BTEST_PORT", Some("2222"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .mapping(PortPatch { port: 8443 }) // override: port wins, host untouched (not in patch)
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "fromenv.com", "key absent from mapping is left alone");
        assert_eq!(config.port, 8443, "mapping overrides the provided key");
    });
}

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
#[test]
fn value_seeds_then_overlay_fills() {
    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", None::<&str>)], || {
        // seed an owned value with an empty host (default), then overlay-fill it
        let mut patch = std::collections::HashMap::new();
        patch.insert("host", "filled.com");
        let config: TestConfig = conflaguration::builder()
            .value(TestConfig { host: String::new(), port: 7000 })
            .overlay_mapping(patch) // host is "" (non-null) so overlay keeps it
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        // "" is a real (non-null) value, so overlay does NOT replace it
        assert_eq!(config.host, "", "overlay keeps the existing empty-but-present value");
        assert_eq!(config.port, 7000);
    });
}

#[cfg(feature = "toml")]
#[test]
fn file_error_short_circuits() {
    let bad_path = std::path::Path::new("/nonexistent/config.toml");
    let result: Result<TestConfig> = conflaguration::builder().file(bad_path).env().build();
    assert!(matches!(result, Err(Error::Io(_))));
}

#[test]
fn default_builder_constructs_same_as_new() {
    let from_default: conflaguration::ConfigBuilder<TestConfig> = Default::default();
    let from_new: conflaguration::ConfigBuilder<TestConfig> = conflaguration::builder();
    assert!(matches!(from_default.build(), Err(Error::NoSource)));
    assert!(matches!(from_new.build(), Err(Error::NoSource)));
}

#[test]
fn value_after_error_preserves_error() {
    temp_env::with_vars([("BTEST_HOST", Some("")), ("BTEST_PORT", Some("0"))], || {
        let result: Result<TestConfig> = conflaguration::builder().env().validate().value(TestConfig::default()).build();
        assert!(matches!(result, Err(Error::Validation { .. })));
    });
}

#[test]
fn env_after_error_preserves_error() {
    temp_env::with_vars([("BTEST_HOST", Some("")), ("BTEST_PORT", Some("0"))], || {
        let result: Result<TestConfig> = conflaguration::builder().env().validate().env().build();
        assert!(matches!(result, Err(Error::Validation { .. })));
    });
}

#[test]
fn env_with_prefix_after_error_preserves_error() {
    temp_env::with_vars([("BTEST_HOST", Some("")), ("BTEST_PORT", Some("0"))], || {
        let result: Result<TestConfig> = conflaguration::builder().env().validate().env_with_prefix("IGNORED").build();
        assert!(matches!(result, Err(Error::Validation { .. })));
    });
}

#[test]
fn value_replaces_running_state() {
    temp_env::with_vars([("BTEST_HOST", Some("from_env")), ("BTEST_PORT", Some("5555"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .value(TestConfig { host: "seeded".into(), port: 1 })
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "seeded", "value() as a later source replaces the whole running value");
        assert_eq!(config.port, 1);
    });
}

#[test]
fn value_default_then_env_loads_normally() {
    temp_env::with_vars([("BTEST_HOST", Some("env_host")), ("BTEST_PORT", Some("7777"))], || {
        let config: TestConfig = conflaguration::builder()
            .value(TestConfig::default())
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "env_host");
        assert_eq!(config.port, 7777);
    });
}

#[test]
fn validate_on_none_state_is_noop() {
    temp_env::with_vars([("BTEST_HOST", Some("valid")), ("BTEST_PORT", Some("8080"))], || {
        let with_validate: TestConfig = conflaguration::builder()
            .validate()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        let without_validate: TestConfig = conflaguration::builder()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(with_validate.host, without_validate.host);
        assert_eq!(with_validate.port, without_validate.port);
    });
}
