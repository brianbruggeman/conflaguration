//! Typed configuration from environment variables, files, and fluent builders.
//!
//! ```rust,ignore
//! use conflaguration::{Settings, Validate, init};
//!
//! #[derive(Settings, Validate)]
//! #[settings(prefix = "APP")]
//! struct Config {
//!     #[setting(default = 8080)]
//!     port: u16,
//!     #[setting(default = "localhost")]
//!     host: String,
//! }
//!
//! let config: Config = init().unwrap();
//! ```

mod builder;

pub use builder::ConfigBuilder;
pub use environs::FromEnvStr;
pub use environs::load;
pub use environs::load_override;
pub use environs::load_override_path;
pub use environs::load_path;
pub use environs::resolve;
pub use environs::resolve_or;
pub use environs::resolve_or_else;
pub use environs::resolve_or_parse;

#[cfg(feature = "derive")]
pub use conflaguration_derive::ConfigDisplay;
#[cfg(feature = "derive")]
pub use conflaguration_derive::Settings;
#[cfg(feature = "derive")]
pub use conflaguration_derive::Validate;

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use serde::de::DeserializeOwned;

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
mod file;

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file;
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file_then_env;
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file_then_env_then;
#[cfg(feature = "json")]
pub use file::from_json_str;
#[cfg(feature = "toml")]
pub use file::from_toml_str;
#[cfg(feature = "yaml")]
pub use file::from_yaml_str;

/// Crate-level result type.
pub type Result<T> = std::result::Result<T, Error>;

/// A single validation failure, keyed by dotted field path.
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    pub path: String,
    pub message: String,
}

impl ValidationMessage {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn prepend_path(&mut self, prefix: &str) {
        if self.path.is_empty() {
            self.path = prefix.to_string();
        } else {
            self.path = format!("{prefix}.{}", self.path);
        }
    }
}

impl std::fmt::Display for ValidationMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.path.is_empty() {
            write!(formatter, "{}", self.message)
        } else {
            write!(formatter, "{}: {}", self.path, self.message)
        }
    }
}

/// All error types produced by conflaguration.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Env(#[from] environs::Error),

    #[error("validation failed:\n{}", errors.iter().map(|err| format!("  - {err}")).collect::<Vec<_>>().join("\n"))]
    Validation { errors: Vec<ValidationMessage> },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unsupported config format: {0}")]
    UnsupportedFormat(String),

    #[error("no config source provided to builder")]
    NoSource,

    #[cfg(feature = "toml")]
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),

    #[cfg(feature = "json")]
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(feature = "yaml")]
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Construct a typed config struct from environment variables.
///
/// Derive with `#[derive(Settings)]` or implement manually.
pub trait Settings: Sized {
    const PREFIX: Option<&'static str> = None;

    fn from_env() -> Result<Self>;

    fn from_env_with_prefix(_prefix: &str) -> Result<Self> {
        Self::from_env()
    }

    fn override_from_env(&mut self) -> Result<()> {
        Ok(())
    }

    fn override_from_env_with_prefix(&mut self, _prefix: &str) -> Result<()> {
        Ok(())
    }
}

/// Validate a config struct after construction.
///
/// Derive with `#[derive(Validate)]` to cascade into nested fields,
/// or implement manually for custom rules.
pub trait Validate {
    fn validate(&self) -> Result<()>;
}

/// Construct from env and validate in one step.
pub fn init<T: Settings + Validate>() -> Result<T> {
    let settings = T::from_env()?;
    settings.validate()?;
    Ok(settings)
}

/// Start a fluent builder for layered config: defaults, env, files, overrides.
pub fn builder<T>() -> ConfigBuilder<T> {
    ConfigBuilder::new()
}

/// Resolve an env var through a custom parse function. Errors if no key is set.
pub fn resolve_with<T, E, F>(keys: &[&str], parse_fn: F) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnOnce(&str) -> std::result::Result<T, E>,
{
    Ok(environs::resolve_with(keys, parse_fn)?)
}

/// Resolve through a custom parse function, falling back to `default` if no key is set.
pub fn resolve_with_or<T, E, F>(keys: &[&str], parse_fn: F, default: T) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnOnce(&str) -> std::result::Result<T, E>,
{
    match environs::resolve_with(keys, parse_fn) {
        Ok(val) => Ok(val),
        Err(environs::Error::NotFound { .. }) => Ok(default),
        Err(err) => Err(err.into()),
    }
}

/// Resolve through a custom parse function; if no key is set, parse `default_str` instead.
pub fn resolve_with_or_str<T, E, F>(keys: &[&str], parse_fn: F, default_str: &str) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnOnce(&str) -> std::result::Result<T, E>,
{
    let mut matched_key: Option<&str> = None;
    let raw = keys.iter().find_map(|key| {
        std::env::var(key).ok().map(|val| {
            matched_key = Some(key);
            val
        })
    });
    let input = raw.as_deref().unwrap_or(default_str);
    parse_fn(input).map_err(|source| {
        Error::Env(environs::Error::Parse {
            key: matched_key.unwrap_or("<default>").to_string(),
            expected: std::any::type_name::<T>(),
            got: input.to_string(),
            source: Box::new(source),
            location: environs::Location::default(),
        })
    })
}

/// Render config fields with their env var keys for debugging/logging.
///
/// Derive with `#[derive(ConfigDisplay)]` for automatic implementation.
pub trait ConfigDisplay {
    fn fmt_config(&self, formatter: &mut std::fmt::Formatter<'_>, depth: usize) -> std::fmt::Result;

    fn fmt_config_with_prefix(&self, formatter: &mut std::fmt::Formatter<'_>, depth: usize, _prefix: &str) -> std::fmt::Result {
        self.fmt_config(formatter, depth)
    }

    fn display(&self) -> ConfigView<'_, Self>
    where
        Self: Sized,
    {
        ConfigView(self)
    }

    fn display_with_prefix<'a>(&'a self, prefix: &'a str) -> ConfigPrefixView<'a, Self>
    where
        Self: Sized,
    {
        ConfigPrefixView { inner: self, prefix }
    }
}

pub struct ConfigView<'a, T: ConfigDisplay + ?Sized>(&'a T);

impl<T: ConfigDisplay + ?Sized> std::fmt::Display for ConfigView<'_, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_config(formatter, 0)
    }
}

pub struct ConfigPrefixView<'a, T: ConfigDisplay + ?Sized> {
    inner: &'a T,
    prefix: &'a str,
}

impl<T: ConfigDisplay + ?Sized> std::fmt::Display for ConfigPrefixView<'_, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_config_with_prefix(formatter, 0, self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestConfig {
        port: u16,
        name: String,
    }

    impl Settings for TestConfig {
        fn from_env() -> Result<Self> {
            Ok(Self {
                port: resolve_or(&["TEST_CONFLAG_PORT"], 8080)?,
                name: resolve_or_parse(&["TEST_CONFLAG_NAME"], "default")?,
            })
        }
    }

    impl Validate for TestConfig {
        fn validate(&self) -> Result<()> {
            let mut errors = vec![];
            if self.port == 0 {
                errors.push(ValidationMessage::new("port", "must be > 0"));
            }
            if self.name.is_empty() {
                errors.push(ValidationMessage::new("name", "must not be empty"));
            }
            if errors.is_empty() { Ok(()) } else { Err(Error::Validation { errors }) }
        }
    }

    #[test]
    fn from_env_with_defaults() {
        temp_env::with_vars([("TEST_CONFLAG_PORT", None::<&str>), ("TEST_CONFLAG_NAME", None::<&str>)], || {
            let config = TestConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            assert_eq!(config.port, 8080);
            assert_eq!(config.name, "default");
        });
    }

    #[test]
    fn from_env_reads_environment() {
        temp_env::with_vars([("TEST_CONFLAG_PORT", Some("3000")), ("TEST_CONFLAG_NAME", Some("myapp"))], || {
            let config = TestConfig::from_env().unwrap_or_else(|err| panic!("from_env failed: {err}"));
            assert_eq!(config.port, 3000);
            assert_eq!(config.name, "myapp");
        });
    }

    #[test]
    fn validate_passes_on_valid_config() {
        let config = TestConfig { port: 8080, name: "app".into() };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_collects_all_errors() {
        let config = TestConfig { port: 0, name: String::new() };
        let err = config.validate().unwrap_err();
        match err {
            Error::Validation { errors } => {
                assert_eq!(errors.len(), 2);
                assert_eq!(errors[0].path, "port");
                assert_eq!(errors[0].message, "must be > 0");
                assert_eq!(errors[1].path, "name");
                assert_eq!(errors[1].message, "must not be empty");
            }
            other => panic!("expected Validation error, got {other}"),
        }
    }

    #[test]
    fn init_combines_from_env_and_validate() {
        temp_env::with_vars([("TEST_CONFLAG_PORT", Some("9090")), ("TEST_CONFLAG_NAME", Some("production"))], || {
            let config: TestConfig = init().unwrap_or_else(|_| panic!("init failed"));
            assert_eq!(config.port, 9090);
            assert_eq!(config.name, "production");
        });
    }

    #[test]
    fn init_propagates_validation_error() {
        temp_env::with_vars([("TEST_CONFLAG_PORT", Some("0")), ("TEST_CONFLAG_NAME", Some(""))], || {
            let result: Result<TestConfig> = init();
            assert!(matches!(result, Err(Error::Validation { .. })));
        });
    }

    #[test]
    fn env_error_propagates_through() {
        temp_env::with_vars([("TEST_CONFLAG_PORT", Some("banana")), ("TEST_CONFLAG_NAME", None::<&str>)], || {
            let result = TestConfig::from_env();
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn validation_displays_paths() {
        let err = Error::Validation {
            errors: vec![ValidationMessage::new("port", "must be > 0"), ValidationMessage::new("database.host", "must not be empty")],
        };
        let msg = err.to_string();
        assert!(msg.contains("validation failed:"));
        assert!(msg.contains("  - port: must be > 0"));
        assert!(msg.contains("  - database.host: must not be empty"));
    }

    #[test]
    fn validation_message_prepend_path() {
        let mut msg = ValidationMessage::new("port", "must be > 0");
        msg.prepend_path("database");
        assert_eq!(msg.path, "database.port");
        assert_eq!(msg.to_string(), "database.port: must be > 0");

        msg.prepend_path("app");
        assert_eq!(msg.path, "app.database.port");
    }

    #[test]
    fn validation_message_prepend_empty_path() {
        let mut msg = ValidationMessage::new("", "something went wrong");
        assert_eq!(msg.to_string(), "something went wrong");

        msg.prepend_path("config");
        assert_eq!(msg.path, "config");
        assert_eq!(msg.to_string(), "config: something went wrong");
    }

    fn infallible_parse(value: &str) -> std::result::Result<Vec<String>, std::convert::Infallible> {
        Ok(value.split(',').map(|s| s.trim().to_string()).collect())
    }

    fn fallible_parse(value: &str) -> std::result::Result<u16, std::num::ParseIntError> {
        value.parse()
    }

    #[test]
    fn resolve_with_returns_parsed_value() {
        temp_env::with_vars([("TEST_RW_HAPPY", Some("a,b,c"))], || {
            let result = resolve_with(&["TEST_RW_HAPPY"], infallible_parse);
            assert_eq!(result.unwrap_or_else(|err| panic!("resolve_with failed: {err}")), vec!["a", "b", "c"]);
        });
    }

    #[test]
    fn resolve_with_errors_on_missing_key() {
        temp_env::with_vars([("TEST_RW_MISS", None::<&str>)], || {
            let result = resolve_with(&["TEST_RW_MISS"], infallible_parse);
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn resolve_with_propagates_parse_error() {
        temp_env::with_vars([("TEST_RW_BAD", Some("notanumber"))], || {
            let result = resolve_with(&["TEST_RW_BAD"], fallible_parse);
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn resolve_with_or_returns_parsed_when_set() {
        temp_env::with_vars([("TEST_RWO_HIT", Some("42"))], || {
            let result = resolve_with_or(&["TEST_RWO_HIT"], fallible_parse, 9999);
            assert_eq!(result.unwrap_or_else(|err| panic!("resolve_with_or failed: {err}")), 42);
        });
    }

    #[test]
    fn resolve_with_or_returns_default_when_missing() {
        temp_env::with_vars([("TEST_RWO_MISS", None::<&str>)], || {
            let result = resolve_with_or(&["TEST_RWO_MISS"], fallible_parse, 9999);
            assert_eq!(result.unwrap_or_else(|err| panic!("resolve_with_or failed: {err}")), 9999);
        });
    }

    #[test]
    fn resolve_with_or_propagates_parse_error_despite_default() {
        temp_env::with_vars([("TEST_RWO_BAD", Some("banana"))], || {
            let result = resolve_with_or(&["TEST_RWO_BAD"], fallible_parse, 9999);
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn resolve_with_or_str_returns_parsed_when_set() {
        temp_env::with_vars([("TEST_RWOS_HIT", Some("a,b"))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_HIT"], infallible_parse, "x,y");
            assert_eq!(result.unwrap_or_else(|err| panic!("resolve_with_or_str failed: {err}")), vec!["a", "b"]);
        });
    }

    #[test]
    fn resolve_with_or_str_uses_default_str_when_missing() {
        temp_env::with_vars([("TEST_RWOS_MISS", None::<&str>)], || {
            let result = resolve_with_or_str(&["TEST_RWOS_MISS"], infallible_parse, "x,y");
            assert_eq!(result.unwrap_or_else(|err| panic!("resolve_with_or_str failed: {err}")), vec!["x", "y"]);
        });
    }

    #[test]
    fn resolve_with_or_str_propagates_error_on_env_value() {
        temp_env::with_vars([("TEST_RWOS_BAD", Some("notanumber"))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_BAD"], fallible_parse, "8080");
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn resolve_with_or_str_propagates_error_on_default_str() {
        temp_env::with_vars([("TEST_RWOS_BAD_DEF", None::<&str>)], || {
            let result = resolve_with_or_str(&["TEST_RWOS_BAD_DEF"], fallible_parse, "banana");
            assert!(matches!(result, Err(Error::Env(_))));
        });
    }

    #[test]
    fn resolve_with_or_str_error_shows_default_key_when_missing() {
        temp_env::with_vars([("TEST_RWOS_ERR_KEY", None::<&str>)], || {
            let result = resolve_with_or_str(&["TEST_RWOS_ERR_KEY"], fallible_parse, "banana");
            let msg = result.unwrap_err().to_string();
            assert!(msg.contains("<default>"), "expected <default> in error, got: {msg}");
        });
    }

    #[test]
    fn resolve_with_or_str_error_shows_env_key_when_set() {
        temp_env::with_vars([("TEST_RWOS_ERR_ENV", Some("banana"))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_ERR_ENV"], fallible_parse, "8080");
            let msg = result.unwrap_err().to_string();
            assert!(msg.contains("TEST_RWOS_ERR_ENV"), "expected key name in error, got: {msg}");
        });
    }

    #[test]
    fn resolve_with_cascades_to_second_key() {
        temp_env::with_vars([("TEST_RW_CASC_A", None::<&str>), ("TEST_RW_CASC_B", Some("1,2"))], || {
            let result = resolve_with(&["TEST_RW_CASC_A", "TEST_RW_CASC_B"], infallible_parse);
            assert_eq!(result.unwrap_or_else(|err| panic!("failed: {err}")), vec!["1", "2"]);
        });
    }

    #[test]
    fn resolve_with_cascade_errors_on_first_bad_value() {
        temp_env::with_vars([("TEST_RW_CASC_BAD", Some("notanumber")), ("TEST_RW_CASC_GOOD", Some("42"))], || {
            let result = resolve_with(&["TEST_RW_CASC_BAD", "TEST_RW_CASC_GOOD"], fallible_parse);
            assert!(result.is_err(), "should error on first matched key even if second is valid");
        });
    }

    #[test]
    fn resolve_with_or_cascades_to_second_key() {
        temp_env::with_vars([("TEST_RWO_CASC_A", None::<&str>), ("TEST_RWO_CASC_B", Some("99"))], || {
            let result = resolve_with_or(&["TEST_RWO_CASC_A", "TEST_RWO_CASC_B"], fallible_parse, 0);
            assert_eq!(result.unwrap_or_else(|err| panic!("failed: {err}")), 99);
        });
    }

    #[test]
    fn resolve_with_or_str_cascades_to_second_key() {
        temp_env::with_vars([("TEST_RWOS_CASC_A", None::<&str>), ("TEST_RWOS_CASC_B", Some("a,b"))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_CASC_A", "TEST_RWOS_CASC_B"], infallible_parse, "x");
            assert_eq!(result.unwrap_or_else(|err| panic!("failed: {err}")), vec!["a", "b"]);
        });
    }

    #[test]
    fn resolve_with_or_str_cascade_errors_on_first_bad_value() {
        temp_env::with_vars([("TEST_RWOS_CASC_BAD", Some("nope")), ("TEST_RWOS_CASC_OK", Some("42"))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_CASC_BAD", "TEST_RWOS_CASC_OK"], fallible_parse, "0");
            assert!(result.is_err(), "should error on first matched key");
        });
    }

    #[test]
    fn resolve_with_receives_empty_string_when_env_set_empty() {
        temp_env::with_vars([("TEST_RW_EMPTY", Some(""))], || {
            let result = resolve_with(&["TEST_RW_EMPTY"], infallible_parse);
            assert_eq!(result.unwrap_or_else(|err| panic!("failed: {err}")), vec![""]);
        });
    }

    #[test]
    fn resolve_with_or_receives_empty_string_not_default() {
        temp_env::with_vars([("TEST_RWO_EMPTY", Some(""))], || {
            let result = resolve_with_or(&["TEST_RWO_EMPTY"], fallible_parse, 9999);
            assert!(result.is_err(), "empty string should be parsed, not treated as missing");
        });
    }

    #[test]
    fn resolve_with_or_str_receives_empty_string_not_default() {
        temp_env::with_vars([("TEST_RWOS_EMPTY", Some(""))], || {
            let result = resolve_with_or_str(&["TEST_RWOS_EMPTY"], fallible_parse, "8080");
            assert!(result.is_err(), "empty string should be parsed, not fall to default_str");
        });
    }

    #[test]
    fn resolve_with_or_str_empty_keys_uses_default_str() {
        let result = resolve_with_or_str::<Vec<String>, _, _>(&[], infallible_parse, "a,b");
        assert_eq!(result.unwrap_or_else(|err| panic!("failed: {err}")), vec!["a", "b"]);
    }

    #[test]
    fn resolve_with_or_str_empty_keys_error_shows_default_key() {
        let result = resolve_with_or_str::<u16, _, _>(&[], fallible_parse, "banana");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("<default>"), "expected <default> in error, got: {msg}");
    }

    struct ManualConfig {
        value: String,
    }

    impl Settings for ManualConfig {
        fn from_env() -> Result<Self> {
            Ok(Self {
                value: resolve_or_parse(&["TEST_MANUAL_VAL"], "default")?,
            })
        }
    }

    #[test]
    fn trait_default_from_env_with_prefix_delegates_to_from_env() {
        temp_env::with_vars([("TEST_MANUAL_VAL", Some("hello"))], || {
            let config = ManualConfig::from_env_with_prefix("IGNORED").unwrap_or_else(|err| panic!("failed: {err}"));
            assert_eq!(config.value, "hello");
        });
    }

    #[test]
    fn trait_default_override_from_env_is_noop() {
        let mut config = ManualConfig { value: "original".into() };
        config.override_from_env().unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.value, "original");
    }

    #[test]
    fn trait_default_override_from_env_with_prefix_is_noop() {
        let mut config = ManualConfig { value: "original".into() };
        config
            .override_from_env_with_prefix("IGNORED")
            .unwrap_or_else(|err| panic!("failed: {err}"));
        assert_eq!(config.value, "original");
    }

    struct DisplayableConfig {
        port: u16,
    }

    impl ConfigDisplay for DisplayableConfig {
        fn fmt_config(&self, formatter: &mut std::fmt::Formatter<'_>, _depth: usize) -> std::fmt::Result {
            write!(formatter, "port={}", self.port)
        }
    }

    #[test]
    fn config_view_display_delegates_to_fmt_config() {
        let config = DisplayableConfig { port: 8080 };
        let view = config.display();
        assert_eq!(format!("{view}"), "port=8080");
    }

    #[test]
    fn config_prefix_view_uses_default_fmt_config_with_prefix() {
        let config = DisplayableConfig { port: 3000 };
        let view = config.display_with_prefix("APP");
        assert_eq!(format!("{view}"), "port=3000");
    }
}
