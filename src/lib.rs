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

pub type Result<T> = std::result::Result<T, Error>;

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

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

pub fn init<T: Settings + Validate>() -> Result<T> {
    let settings = T::from_env()?;
    settings.validate()?;
    Ok(settings)
}

pub fn builder<T>() -> ConfigBuilder<T> {
    ConfigBuilder::new()
}

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
}
