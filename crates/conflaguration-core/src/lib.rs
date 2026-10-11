#![no_std]

mod resolve;

#[cfg(feature = "alloc")]
mod validation;

pub use resolve::{ResolveError, ResolveFromSource, resolve_color};

#[cfg(feature = "alloc")]
pub use validation::{Validate, ValidationError, ValidationMessage};

/// A typed settings input that lends values to its caller.
///
/// A platform environment source can return `COLOR=always` as `Some("always")`
/// without taking ownership of the input bytes.
pub trait SettingsSource {
    /// Looks up one setting key while lending the value from this source.
    fn get(&self, key: &str) -> Option<&str>;

    /// Parses one present scalar from borrowed source bytes.
    fn parse_scalar<Value>(&self, key: &str) -> Result<Option<Value>, Value::Err>
    where
        Value: core::str::FromStr,
    {
        self.get(key).map(str::parse).transpose()
    }

    /// Preserves absent, present-empty, and present-nonempty as separate states.
    fn state(&self, key: &str) -> SettingState<'_> {
        match self.get(key) {
            None => SettingState::Absent,
            Some("") => SettingState::PresentEmpty,
            Some(value) => SettingState::PresentNonempty(value),
        }
    }
}

/// resolves the first matching value from cli, environment, then defaults.
pub struct SettingsLayers<'source, Defaults, Environment, Cli> {
    defaults: &'source Defaults,
    environment: &'source Environment,
    cli: &'source Cli,
}

impl<'source, Defaults, Environment, Cli> SettingsLayers<'source, Defaults, Environment, Cli> {
    /// borrows the three ordered sources without copying keys or values.
    #[must_use]
    pub const fn new(defaults: &'source Defaults, environment: &'source Environment, cli: &'source Cli) -> Self {
        Self { defaults, environment, cli }
    }
}

impl<Defaults, Environment, Cli> SettingsSource for SettingsLayers<'_, Defaults, Environment, Cli>
where
    Defaults: SettingsSource,
    Environment: SettingsSource,
    Cli: SettingsSource,
{
    fn get(&self, key: &str) -> Option<&str> {
        self.cli
            .get(key)
            .or_else(|| self.environment.get(key))
            .or_else(|| self.defaults.get(key))
    }
}

/// One borrowed setting value with presence retained even when empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingState<'value> {
    /// The source has no pair for this key.
    Absent,
    /// The source contains `KEY=` with no value bytes.
    PresentEmpty,
    /// The source contains a nonempty value borrowed from its pair storage.
    PresentNonempty(&'value str),
}

/// A view over borrowed key-value pairs supplied by a platform adapter.
pub struct BorrowedSettings<'source> {
    pairs: &'source [(&'source str, &'source str)],
}

impl<'source> BorrowedSettings<'source> {
    /// Lends a caller-owned pair slice for lookup without copying its values.
    #[must_use]
    pub const fn new(pairs: &'source [(&'source str, &'source str)]) -> Self {
        Self { pairs }
    }
}

impl SettingsSource for BorrowedSettings<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(candidate, _)| *candidate == key)
            .map(|(_, value)| *value)
    }
}
