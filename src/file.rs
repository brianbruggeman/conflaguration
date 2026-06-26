use std::path::Path;

use crate::Result;
use crate::Settings;

/// Deserialize from a TOML string.
#[cfg(feature = "toml")]
pub fn from_toml_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(toml::from_str(contents)?)
}

/// Deserialize from a JSON string.
#[cfg(feature = "json")]
pub fn from_json_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(serde_json::from_str(contents)?)
}

/// Deserialize from a YAML string.
#[cfg(feature = "yaml")]
pub fn from_yaml_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(serde_yaml::from_str(contents)?)
}

/// Load config from a file, detecting format by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`).
/// Uppercase or mixed-case extensions are rejected as unsupported.
pub fn from_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let contents = std::fs::read_to_string(path)?;
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match extension {
        #[cfg(feature = "toml")]
        "toml" => from_toml_str(&contents),

        #[cfg(not(feature = "toml"))]
        "toml" => Err(crate::Error::UnsupportedFormat("toml (enable the 'toml' feature)".into())),

        #[cfg(feature = "json")]
        "json" => from_json_str(&contents),

        #[cfg(not(feature = "json"))]
        "json" => Err(crate::Error::UnsupportedFormat("json (enable the 'json' feature)".into())),

        #[cfg(feature = "yaml")]
        "yaml" | "yml" => from_yaml_str(&contents),

        #[cfg(not(feature = "yaml"))]
        "yaml" | "yml" => Err(crate::Error::UnsupportedFormat("yaml (enable the 'yaml' feature)".into())),

        other => Err(crate::Error::UnsupportedFormat(other.into())),
    }
}

// parse a config file into a generic json value, the medium used to overlay
// one file's present keys onto an already-constructed config.
fn file_to_value(path: &Path) -> Result<serde_json::Value> {
    let contents = std::fs::read_to_string(path)?;
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match extension {
        #[cfg(feature = "toml")]
        "toml" => Ok(toml::from_str(&contents)?),

        #[cfg(not(feature = "toml"))]
        "toml" => Err(crate::Error::UnsupportedFormat("toml (enable the 'toml' feature)".into())),

        #[cfg(feature = "json")]
        "json" => Ok(serde_json::from_str(&contents)?),

        #[cfg(not(feature = "json"))]
        "json" => Err(crate::Error::UnsupportedFormat("json (enable the 'json' feature)".into())),

        #[cfg(feature = "yaml")]
        "yaml" | "yml" => Ok(serde_yaml::from_str(&contents)?),

        #[cfg(not(feature = "yaml"))]
        "yaml" | "yml" => Err(crate::Error::UnsupportedFormat("yaml (enable the 'yaml' feature)".into())),

        other => Err(crate::Error::UnsupportedFormat(other.into())),
    }
}

/// How an incoming layer combines with the running value, per key.
#[derive(Clone, Copy)]
pub(crate) enum MergeMode {
    /// Replace an existing value and insert new keys.
    Override,
    /// Insert new keys, but keep an existing non-null value untouched.
    Overlay,
}

// deep-merge `incoming` onto `base` per `mode`. Objects recurse; a key absent
// from base is always inserted; a present non-null scalar is replaced only under
// Override.
fn merge_value(base: &mut serde_json::Value, incoming: serde_json::Value, mode: MergeMode) {
    match (base, incoming) {
        (serde_json::Value::Object(base_map), serde_json::Value::Object(incoming_map)) => {
            for (key, value) in incoming_map {
                merge_value(base_map.entry(key).or_insert(serde_json::Value::Null), value, mode);
            }
        }
        (base_slot, incoming) => match mode {
            MergeMode::Override => *base_slot = incoming,
            MergeMode::Overlay => {
                if base_slot.is_null() {
                    *base_slot = incoming;
                }
            }
        },
    }
}

/// Build a value from an in-memory mapping (used as a builder's first source).
pub(crate) fn from_mapping<T, M>(incoming: &M) -> Result<T>
where
    T: serde::de::DeserializeOwned,
    M: serde::Serialize,
{
    let value = serde_json::to_value(incoming)?;
    Ok(serde_json::from_value(value)?)
}

/// Combine a config file's keys with an existing value per `mode`.
pub(crate) fn merge_file<T>(base: &T, path: &Path, mode: MergeMode) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut base_value = serde_json::to_value(base)?;
    let file_value = file_to_value(path)?;
    merge_value(&mut base_value, file_value, mode);
    Ok(serde_json::from_value(base_value)?)
}

/// Combine an in-memory mapping's keys with an existing value per `mode`.
pub(crate) fn merge_mapping<T, M>(base: &T, incoming: &M, mode: MergeMode) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    M: serde::Serialize,
{
    let mut base_value = serde_json::to_value(base)?;
    let incoming_value = serde_json::to_value(incoming)?;
    merge_value(&mut base_value, incoming_value, mode);
    Ok(serde_json::from_value(base_value)?)
}

/// Load from file, then override with environment variables.
pub fn from_file_then_env<T: serde::de::DeserializeOwned + Settings>(path: &Path) -> Result<T> {
    let mut config: T = from_file(path)?;
    config.override_from_env()?;
    Ok(config)
}

/// Load from file, override with env, then apply a final mutation (e.g. CLI overrides).
pub fn from_file_then_env_then<T, F>(path: &Path, apply: F) -> Result<T>
where
    T: serde::de::DeserializeOwned + Settings,
    F: FnOnce(&mut T),
{
    let mut config: T = from_file(path)?;
    config.override_from_env()?;
    apply(&mut config);
    Ok(config)
}
