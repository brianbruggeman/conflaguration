use crate::Error;
use crate::Result;
#[cfg(any(feature = "default", feature = "std"))]
use crate::Settings;
use crate::Validate;

/// Fluent builder for layered configuration.
///
/// Sources apply in call order. The *first* source builds the value in full
/// ([`value`](Self::value), [`file`](Self::file), [`mapping`](Self::mapping), or
/// [`env`](Self::env) from empty). Each later source provides a set of keys and
/// combines them one of two ways:
///
/// - **override** ([`file`](Self::file), [`mapping`](Self::mapping),
///   [`env`](Self::env)) — replace existing keys, insert new ones.
/// - **overlay** ([`overlay_file`](Self::overlay_file),
///   [`overlay_mapping`](Self::overlay_mapping)) — insert keys the value lacks,
///   but keep existing non-null values untouched.
///
/// So order is precedence for overrides (later wins per key), and overlays fill
/// gaps without clobbering. [`override_with`](Self::override_with) is the closure
/// escape hatch for values computed from the running state.
///
/// Errors short-circuit — once an error occurs, subsequent sources are skipped.
///
/// Layering a file/env chain needs the `toml`/`json`/`yaml`/`derive` features;
/// this example sticks to the feature-free core (`value`/`override_with`/`build`)
/// so it runs under any feature set — see the crate root tutorial for the full
/// file+env layering walkthrough.
///
/// ```rust
/// use conflaguration::ConfigBuilder;
///
/// #[derive(Debug, Default, PartialEq)]
/// struct Counters {
///     hits: u32,
///     misses: u32,
/// }
///
/// // struct -> fluent -> struct
/// let counters: Counters = ConfigBuilder::new()
///     .value(Counters::default())     // seed from an owned value
///     .override_with(|running| running.hits = 5)
///     .build()?;
///
/// assert_eq!(counters, Counters { hits: 5, misses: 0 });
/// # Ok::<(), conflaguration::Error>(())
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
    /// Create an empty builder with no source configured yet.
    pub fn new() -> Self {
        Self { state: None }
    }

    /// Set the running value to an owned `T` — the struct↔fluent entry point.
    /// As the first source it seeds the chain; later it replaces the whole value.
    /// (Use `.value(T::default())` for a defaults base.)
    pub fn value(self, value: T) -> Self {
        match self.state {
            Some(Err(_)) => self,
            _ => Self { state: Some(Ok(value)) },
        }
    }

    /// Apply environment variables.
    ///
    /// From empty state, constructs the value in full (defaults fill unset keys).
    /// Over an existing state, overlays per key: each variable that is *set*
    /// overrides its field; unset variables leave the current value untouched.
    #[cfg(any(feature = "default", feature = "std"))]
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
    #[cfg(any(feature = "default", feature = "std"))]
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

    /// Explicitly override fields with a closure (e.g. CLI flags) — the escape
    /// hatch for forcing values regardless of source. Runs last in a layer
    /// chain; skipped if state is error or empty.
    pub fn override_with<F: FnOnce(&mut T)>(self, func: F) -> Self {
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

#[cfg(all(any(feature = "default", feature = "std"), any(feature = "toml", feature = "json", feature = "yaml")))]
impl<T> ConfigBuilder<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    /// Apply a config file, **overriding** existing keys and inserting new ones.
    /// Format detected by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`).
    /// As the first source it deserializes the file in full.
    pub fn file(self, path: impl AsRef<std::path::Path>) -> Self {
        self.merge_from_file(path.as_ref(), crate::file::MergeMode::Override)
    }

    /// Apply a config file as an **overlay**: insert keys the running value lacks,
    /// but keep existing non-null values untouched.
    pub fn overlay_file(self, path: impl AsRef<std::path::Path>) -> Self {
        self.merge_from_file(path.as_ref(), crate::file::MergeMode::Overlay)
    }

    /// Apply an in-memory mapping (any `Serialize`), **overriding** existing keys
    /// and inserting new ones. As the first source it must deserialize to a full `T`.
    ///
    /// Contrast with [`overlay_mapping`](Self::overlay_mapping): override always
    /// replaces a key the incoming mapping sets; overlay only fills a key the
    /// running value is missing (`()` from an empty slot, or a serialized
    /// `Option::None` — see [`ConfigBuilder`] for the merge rules).
    ///
    /// ```rust
    /// use conflaguration::{ConfigBuilder, Result, Settings, Validate};
    ///
    /// #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
    /// struct Config {
    ///     host: String,
    ///     port: Option<u16>,
    /// }
    ///
    /// impl Settings for Config {
    ///     fn from_env() -> Result<Self> { Ok(Self::default()) }
    /// }
    /// impl Validate for Config {
    ///     fn validate(&self) -> Result<()> { Ok(()) }
    /// }
    ///
    /// let base = Config { host: "localhost".to_string(), port: None };
    ///
    /// // overlay: host is already set (not empty), so it's kept; port is
    /// // empty (None), so the incoming value fills it.
    /// let after_overlay: Config = ConfigBuilder::new()
    ///     .value(base)
    ///     .overlay_mapping(Config { host: "ignored-by-overlay".to_string(), port: Some(9090) })
    ///     .build()?;
    /// assert_eq!(after_overlay, Config { host: "localhost".to_string(), port: Some(9090) });
    ///
    /// // override: replaces every key the incoming mapping sets, regardless
    /// // of the running value.
    /// let after_override: Config = ConfigBuilder::new()
    ///     .value(after_overlay)
    ///     .mapping(Config { host: "prod".to_string(), port: Some(1) })
    ///     .build()?;
    /// assert_eq!(after_override, Config { host: "prod".to_string(), port: Some(1) });
    /// # Ok::<(), conflaguration::Error>(())
    /// ```
    pub fn mapping<M: serde::Serialize>(self, map: M) -> Self {
        self.merge_from_mapping(&map, crate::file::MergeMode::Override)
    }

    /// Apply an in-memory mapping as an **overlay**: insert missing keys, keep
    /// existing non-null values untouched.
    pub fn overlay_mapping<M: serde::Serialize>(self, map: M) -> Self {
        self.merge_from_mapping(&map, crate::file::MergeMode::Overlay)
    }

    fn merge_from_file(self, path: &std::path::Path, mode: crate::file::MergeMode) -> Self {
        match self.state {
            Some(Err(_)) => self,
            None => Self { state: Some(crate::from_file(path)) },
            Some(Ok(base)) => Self {
                state: Some(crate::file::merge_file(&base, path, mode)),
            },
        }
    }

    fn merge_from_mapping<M: serde::Serialize>(self, map: &M, mode: crate::file::MergeMode) -> Self {
        match self.state {
            Some(Err(_)) => self,
            None => Self {
                state: Some(crate::file::from_mapping(map)),
            },
            Some(Ok(base)) => Self {
                state: Some(crate::file::merge_mapping(&base, map, mode)),
            },
        }
    }
}
