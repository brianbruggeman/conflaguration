use crate::Error;
use crate::Result;
use crate::Settings;
use crate::Validate;

/// Fluent builder for layered configuration.
///
/// Sources are applied in call order; later sources override earlier ones.
/// Errors short-circuit — once an error occurs, subsequent sources are skipped.
///
/// ```rust,ignore
/// let config: MyConfig = conflaguration::builder()
///     .defaults()
///     .file("config.toml")
///     .env()
///     .validate()
///     .build()?;
/// ```
pub struct ConfigBuilder<T> {
    state: Option<Result<T>>,
}

impl<T> Default for ConfigBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ConfigBuilder<T> {
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Seed with `T::default()`. Only applies if no prior source was set.
    pub fn defaults(self) -> Self
    where
        T: Default,
    {
        match self.state {
            Some(Err(_)) => self,
            None => Self { state: Some(Ok(T::default())) },
            _ => self,
        }
    }

    /// Load from environment variables. Overrides existing values if state is already set.
    pub fn env(self) -> Self
    where
        T: Settings,
    {
        match self.state {
            Some(Err(_)) => self,
            Some(Ok(mut value)) => {
                let result = value.override_from_env().map(|()| value);
                Self { state: Some(result) }
            }
            None => Self { state: Some(T::from_env()) },
        }
    }

    /// Load from environment variables using a runtime prefix instead of the struct's static prefix.
    pub fn env_with_prefix(self, prefix: &str) -> Self
    where
        T: Settings,
    {
        match self.state {
            Some(Err(_)) => self,
            Some(Ok(mut value)) => {
                let result = value.override_from_env_with_prefix(prefix).map(|()| value);
                Self { state: Some(result) }
            }
            None => Self {
                state: Some(T::from_env_with_prefix(prefix)),
            },
        }
    }

    /// Mutate the current value in-place. Skipped if state is error or empty.
    pub fn apply<F: FnOnce(&mut T)>(self, func: F) -> Self {
        match self.state {
            Some(Ok(mut value)) => {
                func(&mut value);
                Self { state: Some(Ok(value)) }
            }
            other => Self { state: other },
        }
    }

    /// Run validation. Converts ok state to error state if validation fails.
    pub fn validate(self) -> Self
    where
        T: Validate,
    {
        match self.state {
            Some(Ok(value)) => {
                let result = value.validate().map(|()| value);
                Self { state: Some(result) }
            }
            other => Self { state: other },
        }
    }

    /// Consume the builder and return the config or the first error encountered.
    pub fn build(self) -> Result<T> {
        match self.state {
            Some(result) => result,
            None => Err(Error::NoSource),
        }
    }
}

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
impl<T> ConfigBuilder<T>
where
    T: serde::de::DeserializeOwned,
{
    /// Load from a config file. Format detected by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`).
    pub fn file(self, path: impl AsRef<std::path::Path>) -> Self {
        match self.state {
            Some(Err(_)) => self,
            _ => Self {
                state: Some(crate::from_file(path.as_ref())),
            },
        }
    }
}
