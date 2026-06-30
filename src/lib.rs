//! Typed configuration from environment variables, files, and fluent builders.
//!
//! # Tutorial
//!
//! ## 1. Describe the config as a struct
//!
//! Derive [`Settings`] to resolve fields from the environment, and [`Validate`]
//! to check them afterward. A struct-level `prefix` is prepended to every key.
//! [`init`] is just `from_env()` then `validate()`, returning the first error
//! from either — so propagate it with `?`, never `unwrap`.
//!
//! ```rust,ignore
//! use conflaguration::{Settings, Validate, init};
//!
//! #[derive(Settings, Validate)]
//! #[settings(prefix = "APP")]
//! struct Config {
//!     #[setting(default = 8080)]
//!     port: u16,                         // reads APP_PORT
//!
//!     #[setting(default = "localhost")]
//!     host: String,                      // reads APP_HOST
//! }
//!
//! fn main() -> conflaguration::Result<()> {
//!     let config: Config = init()?;      // from_env + validate, `?` surfaces failures
//!     println!("listening on {}:{}", config.host, config.port);
//!     Ok(())
//! }
//! ```
//!
//! ## 2. Handle the error instead of unwrapping
//!
//! [`init`] can fail two ways, and the [`Error`] enum tells you which: a bad or
//! missing env var ([`Error::Env`]) or a failed validation rule
//! ([`Error::Validation`]). Match on it to react meaningfully:
//!
//! ```rust,ignore
//! use conflaguration::{Error, init};
//!
//! match init::<Config>() {
//!     Ok(config) => run(config),
//!     // a var was missing or didn't parse; env_err names the key, type, and value
//!     Err(Error::Env(env_err)) => eprintln!("bad environment: {env_err}"),
//!     // your validate() rules rejected fields; each carries a dotted path
//!     Err(Error::Validation { errors }) => {
//!         for failure in errors {
//!             eprintln!("{}: {}", failure.path, failure.message);
//!         }
//!     }
//!     Err(other) => eprintln!("config error: {other}"),
//! }
//! ```
//!
//! ## 3. Nest and flatten sub-structs
//!
//! Any field whose type also derives [`Settings`] becomes a sub-section. The
//! env key is built by accumulating the parent prefix with a segment; the four
//! forms below cover every layout. The same `Database` type is reused three
//! times without collisions because each field supplies its own segment.
//!
//! ```rust,ignore
//! use conflaguration::{Settings, Validate, init};
//!
//! #[derive(Settings, Validate)]
//! struct Database {
//!     #[setting(default = "localhost")]
//!     host: String,
//!     #[setting(default = 5432)]
//!     port: u16,
//! }
//!
//! #[derive(Settings, Validate)]
//! struct Tls {
//!     #[setting(default = false)]
//!     enabled: bool,
//! }
//!
//! #[derive(Settings, Validate)]
//! #[settings(prefix = "APP")]
//! struct Config {
//!     #[setting(envs = "DATABASE_URL", override)]
//!     url: String,                       // DATABASE_URL          (exact key, ignores prefix)
//!
//!     #[setting(nested)]
//!     primary: Database,                 // APP_PRIMARY_HOST/PORT  (segment = field name)
//!
//!     #[setting(nested, prefix = "RO")]
//!     replica: Database,                 // APP_RO_HOST/PORT       (segment renamed)
//!
//!     #[setting(nested, override_prefix = "PG")]
//!     audit: Database,                   // PG_HOST/PORT           (absolute, ignores APP)
//!
//!     #[setting(flatten)]
//!     tls: Tls,                          // APP_ENABLED            (merged, no segment)
//! }
//!
//! fn main() -> conflaguration::Result<()> {
//!     let config: Config = init()?;
//!     Ok(())
//! }
//! ```
//!
//! A sub-struct's own `#[settings(prefix = "...")]` is used only when it is
//! resolved directly as a root; once embedded, the embedding field decides the
//! namespace.
//!
//! ## 4. Layer sources with the builder
//!
//! [`init`] reads one source: the environment. Real deployments stack several —
//! a checked-in base file, a per-environment overlay, and finally real env vars.
//! The [`builder`] composes them.
//!
//! The first source builds the value in full; each later source provides a set
//! of keys and combines them one of two ways:
//!
//! - **override** ([`file`](ConfigBuilder::file), [`mapping`](ConfigBuilder::mapping),
//!   [`env`](ConfigBuilder::env)) — replace existing keys, insert new ones.
//! - **overlay** ([`overlay_file`](ConfigBuilder::overlay_file),
//!   [`overlay_mapping`](ConfigBuilder::overlay_mapping)) — insert keys the value
//!   lacks, keep existing non-null values.
//!
//! So order is precedence for overrides (last wins per key); overlays fill gaps.
//!
//! ```rust,ignore
//! fn main() -> conflaguration::Result<()> {
//!     conflaguration::load()?;            // optional: pull a .env file into the process
//!
//!     let config: Config = conflaguration::builder()
//!         .file("base.toml")              // base layer (full)
//!         .file("prod.toml")              // override: prod's keys win
//!         .overlay_mapping(fallbacks)     // overlay: fill only keys still unset
//!         .env()                          // override: set env vars win
//!         .validate()
//!         .build()?;
//!     Ok(())
//! }
//! ```
//!
//! [`value`](ConfigBuilder::value) seeds the chain from an owned `T` (the
//! struct↔fluent flop), and [`build`](ConfigBuilder::build) hands it back.
//! File/mapping sources require the type to also implement
//! [`Serialize`](serde::Serialize) (the merge reads the current value back);
//! see [`ConfigBuilder`] for the precise per-method rules.
//!
//! ## 5. Compile-time sizing with a build script
//!
//! For values that must be `const` (array sizes, capacities) but still want a
//! runtime override surface, resolve a [`Settings`] struct at build time and
//! bake it into constants with `#[derive(ConfigCodegen)]` plus the [`codegen`]
//! build module. The const is the compile-time default; the same struct
//! resolves the runtime override. See `examples/codegen` (and `examples/sizing`
//! for the hand-rolled TOML variant).
//!
//! [`Settings`]: trait@Settings
//! [`Validate`]: trait@Validate

#![warn(missing_docs)]

mod builder;
pub mod template;

pub use builder::ConfigBuilder;

/// Trait for parsing a value from an environment-variable string. Implemented
/// for the standard scalar types; implement it to support custom field types.
pub use environs::FromEnvStr;

/// Load a `.env` file from the default location into the process environment.
/// Existing variables are left untouched; missing file is a no-op.
pub use environs::load;
/// Like [`load`] but values in the `.env` file overwrite existing variables.
pub use environs::load_override;
/// Like [`load_override`] but reads the `.env` file from an explicit path.
pub use environs::load_override_path;
/// Like [`load`] but reads the `.env` file from an explicit path.
pub use environs::load_path;

/// Resolve the first set key in `keys`, parsing it into `T`. Errors if none set.
pub use environs::resolve;
/// Resolve the first set key, falling back to `default` when none is set.
/// Parse errors on a present key still propagate.
pub use environs::resolve_or;
/// Resolve the first set key, computing the fallback lazily when none is set.
pub use environs::resolve_or_else;
/// Resolve the first set key, parsing `default_str` as the fallback when none is set.
pub use environs::resolve_or_parse;

/// Derive macro: implement [`ConfigDisplay`] for a struct.
#[cfg(feature = "derive")]
pub use conflaguration_derive::ConfigDisplay;
/// Derive macro: implement the [`Settings`](trait@Settings) trait for a struct.
/// Resolves each field from the environment; see below for the attribute list.
#[cfg(feature = "derive")]
pub use conflaguration_derive::Settings;
/// Derive macro: implement [`Validate`], cascading into `nested`/`flatten` fields.
#[cfg(feature = "derive")]
pub use conflaguration_derive::Validate;

/// Derive macro: implement [`ConfigCodegen`] for a flat scalar struct.
#[cfg(feature = "derive")]
pub use conflaguration_derive::ConfigCodegen;

/// Re-exported so generated and downstream code can name the file-loader bound.
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use serde::de::DeserializeOwned;

#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
mod file;

/// Load and deserialize a config file, detecting the format by lowercase
/// extension (`.toml`, `.json`, `.yaml`, `.yml`). Other extensions are rejected.
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file;
/// Load from a file, then overlay environment variables on top.
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file_then_env;
/// Load from a file, overlay env, then apply a final mutation (e.g. CLI flags).
#[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
pub use file::from_file_then_env_then;
/// Deserialize a config value from a JSON string.
#[cfg(feature = "json")]
pub use file::from_json_str;
/// Deserialize a config value from a TOML string.
#[cfg(feature = "toml")]
pub use file::from_toml_str;
/// Deserialize a config value from a YAML string.
#[cfg(feature = "yaml")]
pub use file::from_yaml_str;

/// Crate-level result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Join an accumulated prefix with a key segment. Empty prefix yields the bare
/// segment so root structs without a prefix produce clean keys.
#[doc(hidden)]
#[must_use]
pub fn join_key(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}_{segment}")
    }
}

/// A single validation failure, keyed by dotted field path.
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Dotted path to the offending field, e.g. `database.host`.
    pub path: String,
    /// Human-readable failure reason.
    pub message: String,
}

impl ValidationMessage {
    /// Build a message for `path` with the given failure `message`.
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Prepend a parent segment to the path, building the dotted chain as
    /// errors bubble up through nested structs.
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
    /// Environment lookup or parse failure from the `environs` layer.
    #[error(transparent)]
    Env(#[from] environs::Error),

    /// One or more fields failed validation.
    #[error("validation failed:\n{}", errors.iter().map(|err| format!("  - {err}")).collect::<Vec<_>>().join("\n"))]
    Validation {
        /// The collected per-field failures.
        errors: Vec<ValidationMessage>,
    },

    /// Reading a config file from disk failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// File extension did not match a supported format.
    #[error("unsupported config format: {0}")]
    UnsupportedFormat(String),

    /// The builder was finalized without any source configured.
    #[error("no config source provided to builder")]
    NoSource,

    /// TOML deserialization failed.
    #[cfg(feature = "toml")]
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),

    /// JSON deserialization failed.
    #[cfg(feature = "json")]
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    /// Serializing a config into the file-overlay merge medium failed.
    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[error("value serialize: {0}")]
    ValueSerialize(#[from] serde_value::SerializerError),

    /// Deserializing a config back out of the file-overlay merge medium failed.
    #[cfg(any(feature = "toml", feature = "json", feature = "yaml"))]
    #[error("value deserialize: {0}")]
    ValueDeserialize(#[from] serde_value::DeserializerError),

    /// YAML deserialization failed.
    #[cfg(feature = "yaml")]
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Construct a typed config struct from environment variables.
///
/// Derive with `#[derive(Settings)]` or implement manually.
///
/// Env keys are built by accumulating a prefix down the struct hierarchy. A
/// scalar field contributes its uppercased name as the final segment; a nested
/// field contributes its name (or chosen segment) and recurses. So a `host`
/// field two levels deep under prefix `APP` resolves `APP_<SEGMENT>_HOST`.
pub trait Settings: Sized {
    /// The struct's own prefix, used only when it is the root of resolution.
    /// When embedded as a nested field, the embedding field decides the prefix.
    const PREFIX: Option<&'static str> = None;

    /// Resolve using the struct's own [`PREFIX`](Settings::PREFIX) as the root.
    fn from_env() -> Result<Self>;

    /// Resolve using `prefix` as the accumulated root instead of [`PREFIX`](Settings::PREFIX).
    /// Used by parent structs to push their namespace into nested fields.
    fn from_env_with_prefix(_prefix: &str) -> Result<Self> {
        Self::from_env()
    }

    /// Overwrite only the fields whose env keys are currently set, leaving the
    /// rest untouched. Useful for layering env over file-loaded defaults.
    fn override_from_env(&mut self) -> Result<()> {
        Ok(())
    }

    /// [`override_from_env`](Settings::override_from_env) with an explicit accumulated prefix.
    fn override_from_env_with_prefix(&mut self, _prefix: &str) -> Result<()> {
        Ok(())
    }
}

/// Validate a config struct after construction.
///
/// Derive with `#[derive(Validate)]` to cascade into `nested` and `flatten`
/// fields, or implement manually for custom rules.
pub trait Validate {
    /// Check the constructed config, returning [`Error::Validation`] with every
    /// failure collected, or `Ok(())` when all rules pass.
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
        std::env::var(key).ok().inspect(|_| {
            matched_key = Some(key);
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

/// Emit a resolved config as build-time artifacts — compile-time `const`s and
/// `cargo:rustc-cfg` directives — for the two-tier "build-time default const +
/// runtime override" pattern (see the [`codegen`] module).
///
/// Derive with `#[derive(ConfigCodegen)]` on a flat struct of scalar fields.
pub trait ConfigCodegen {
    /// Render the value as a `pub const` module (with a do-not-edit header),
    /// suitable to write into `OUT_DIR` and `include!` from the crate.
    fn to_const_module(&self) -> String;

    /// Print `cargo:rustc-cfg=<prefix>_<field>` directives: a bare flag for each
    /// `true` bool, a `="value"` form for everything else.
    fn emit_cfg(&self, prefix: &str);

    /// The environment keys this config reads, for `rerun-if-env-changed`.
    fn env_keys() -> Vec<String>
    where
        Self: Sized;
}

/// Render config fields with their env var keys for debugging/logging.
///
/// Derive with `#[derive(ConfigDisplay)]` for automatic implementation.
pub trait ConfigDisplay {
    /// Write each field at the given indentation `depth`, using the struct's
    /// own prefix for the displayed env keys.
    fn fmt_config(&self, formatter: &mut std::fmt::Formatter<'_>, depth: usize) -> std::fmt::Result;

    /// Like [`fmt_config`](ConfigDisplay::fmt_config) but with an explicit
    /// accumulated `prefix` for the displayed keys.
    fn fmt_config_with_prefix(&self, formatter: &mut std::fmt::Formatter<'_>, depth: usize, _prefix: &str) -> std::fmt::Result {
        self.fmt_config(formatter, depth)
    }

    /// Wrap `self` in a [`Display`](std::fmt::Display) adapter that renders the
    /// config with its env keys.
    fn display(&self) -> ConfigView<'_, Self>
    where
        Self: Sized,
    {
        ConfigView(self)
    }

    /// Like [`display`](ConfigDisplay::display) but renders keys under a runtime
    /// `prefix` instead of the struct's static one.
    fn display_with_prefix<'a>(&'a self, prefix: &'a str) -> ConfigPrefixView<'a, Self>
    where
        Self: Sized,
    {
        ConfigPrefixView { inner: self, prefix }
    }
}

/// [`Display`](std::fmt::Display) adapter returned by [`ConfigDisplay::display`].
pub struct ConfigView<'a, T: ConfigDisplay + ?Sized>(&'a T);

impl<T: ConfigDisplay + ?Sized> std::fmt::Display for ConfigView<'_, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_config(formatter, 0)
    }
}

/// [`Display`](std::fmt::Display) adapter returned by
/// [`ConfigDisplay::display_with_prefix`], rendering keys under a runtime prefix.
pub struct ConfigPrefixView<'a, T: ConfigDisplay + ?Sized> {
    inner: &'a T,
    prefix: &'a str,
}

impl<T: ConfigDisplay + ?Sized> std::fmt::Display for ConfigPrefixView<'_, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_config_with_prefix(formatter, 0, self.prefix)
    }
}

/// Build-script helpers for the two-tier pattern: resolve a config at build time,
/// bake it into compile-time constants, and emit the cargo directives that keep
/// the generated file fresh. Pair with `#[derive(ConfigCodegen)]`.
///
/// ```rust,ignore
/// // build.rs
/// use conflaguration::{Settings, codegen};
///
/// #[derive(Settings, conflaguration::ConfigCodegen)]
/// #[settings(prefix = "APP")]
/// struct Build {
///     #[setting(default = 8)]
///     pool_threads: u32,
/// }
///
/// fn main() -> conflaguration::Result<()> {
///     let build = Build::from_env()?;               // defaults + APP_* at build time
///     codegen::write_consts(&build, "build_config.rs")?;
///     build.emit_cfg("app");                         // cargo:rustc-cfg=app_* directives
///     codegen::rerun_for::<Build>();                 // rerun when APP_* change
///     Ok(())
/// }
/// ```
#[cfg(feature = "codegen")]
pub mod codegen {
    use std::path::Path;
    use std::path::PathBuf;

    use crate::ConfigCodegen;

    /// Write `value`'s [`to_const_module`](ConfigCodegen::to_const_module) output
    /// to `OUT_DIR/<file_name>`, returning the path written. Call from `build.rs`.
    pub fn write_consts<T: ConfigCodegen>(value: &T, file_name: &str) -> std::io::Result<PathBuf> {
        let out_dir = std::env::var_os("OUT_DIR").ok_or_else(|| std::io::Error::other("OUT_DIR not set (call from a build script)"))?;
        let path = Path::new(&out_dir).join(file_name);
        std::fs::write(&path, value.to_const_module())?;
        Ok(path)
    }

    /// Emit `cargo:rerun-if-changed` for a path the build depends on.
    pub fn rerun_if_changed(path: impl AsRef<Path>) {
        println!("cargo:rerun-if-changed={}", path.as_ref().display());
    }

    /// Emit `cargo:rerun-if-env-changed` for each key.
    pub fn rerun_if_env_changed(keys: &[&str]) {
        for key in keys {
            println!("cargo:rerun-if-env-changed={key}");
        }
    }

    /// Emit `cargo:rerun-if-env-changed` for every env key `T` reads, derived
    /// from its [`Settings`](crate::Settings) attributes — no hand-kept list.
    pub fn rerun_for<T: ConfigCodegen>() {
        for key in T::env_keys() {
            println!("cargo:rerun-if-env-changed={key}");
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

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
