use core::fmt;

use crate::SettingsSource;

/// One missing or invalid field in a borrowed settings source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    /// The source contains no pair for this field.
    Missing { field: &'static str },
    /// The pair's bytes cannot be parsed as the field type.
    Invalid { field: &'static str },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { field } => write!(formatter, "missing setting: {field}"),
            Self::Invalid { field } => write!(formatter, "invalid setting: {field}"),
        }
    }
}

impl core::error::Error for ResolveError {}

/// Builds typed settings while borrowing values from a caller-owned source.
pub trait ResolveFromSource<'source>: Sized {
    /// Resolves each field without taking ownership of the source values.
    fn resolve_from<Source>(source: &'source Source) -> Result<Self, ResolveError>
    where
        Source: SettingsSource + ?Sized;
}

/// resolves whether terminal color is enabled from borrowed environment pairs.
///
/// `CARGO_VAL_COLOR=false` disables color when present; any `NO_COLOR` entry,
/// including an empty value, disables color after that setting is read.
pub fn resolve_color<Source>(source: &Source, generated_default: bool) -> Result<bool, ResolveError>
where
    Source: crate::SettingsSource + ?Sized,
{
    let configured = match source.get("CARGO_VAL_COLOR") {
        None => generated_default,
        Some(value) => value
            .parse::<bool>()
            .map_err(|_| ResolveError::Invalid { field: "CARGO_VAL_COLOR" })?,
    };
    Ok(configured && matches!(source.state("NO_COLOR"), crate::SettingState::Absent))
}
