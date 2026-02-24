use crate::Error;
use crate::Result;
use crate::Settings;
use crate::Validate;

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

    pub fn apply<F: FnOnce(&mut T)>(self, func: F) -> Self {
        match self.state {
            Some(Ok(mut value)) => {
                func(&mut value);
                Self { state: Some(Ok(value)) }
            }
            other => Self { state: other },
        }
    }

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
    pub fn file(self, path: impl AsRef<std::path::Path>) -> Self {
        match self.state {
            Some(Err(_)) => self,
            _ => Self {
                state: Some(crate::from_file(path.as_ref())),
            },
        }
    }
}
