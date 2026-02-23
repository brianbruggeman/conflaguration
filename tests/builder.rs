use conflaguration::Error;
use conflaguration::Result;
use conflaguration::Settings;
use conflaguration::Validate;
use conflaguration::ValidationMessage;

#[derive(Default, Debug, PartialEq)]
#[cfg_attr(any(feature = "toml", feature = "json", feature = "yaml"), derive(serde::Deserialize))]
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
fn defaults_then_env_overrides() {
    temp_env::with_vars([("BTEST_HOST", None::<&str>), ("BTEST_PORT", Some("3000"))], || {
        let config: TestConfig = conflaguration::builder()
            .defaults()
            .env()
            .build()
            .unwrap_or_else(|err| panic!("build failed: {err}"));
        assert_eq!(config.host, "");
        assert_eq!(config.port, 3000);
    });
}

#[test]
fn apply_mutates_value() {
    temp_env::with_vars([("BTEST_HOST", Some("original.com")), ("BTEST_PORT", Some("8080"))], || {
        let config: TestConfig = conflaguration::builder()
            .env()
            .apply(|config: &mut TestConfig| {
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
            .apply(|_| {
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
            .apply(|config: &mut TestConfig| {
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
fn file_error_short_circuits() {
    let bad_path = std::path::Path::new("/nonexistent/config.toml");
    let result: Result<TestConfig> = conflaguration::builder().file(bad_path).env().build();
    assert!(matches!(result, Err(Error::Io(_))));
}
