use std::path::Path;

use serde_value::Value;

use crate::Result;
use crate::lenient_de::Lenient;
use crate::Settings;
use crate::Tree;

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

/// Deserialize from an XML string. Element text becomes the typed field and
/// repeated sibling elements fill a `Vec` field.
///
/// ```rust
/// #[derive(Debug, PartialEq, serde::Deserialize)]
/// struct Config {
///     port: u16,
///     tags: Vec<String>,
/// }
///
/// let config: Config = conflaguration::from_xml_str(
///     "<config><port>8080</port><tags>a</tags><tags>b</tags></config>",
/// )?;
/// assert_eq!(config, Config { port: 8080, tags: vec!["a".to_string(), "b".to_string()] });
/// # Ok::<(), conflaguration::Error>(())
/// ```
#[cfg(feature = "xml")]
pub fn from_xml_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(quick_xml::de::from_str(contents)?)
}

/// Deserialize from a RON string.
#[cfg(feature = "ron")]
pub fn from_ron_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(ron::from_str(contents)?)
}

/// Deserialize from a JSON5 string.
#[cfg(feature = "json5")]
pub fn from_json5_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(json5::from_str(contents)?)
}

/// Load config from a file, detecting format by lowercase extension (`.toml`, `.json`, `.yaml`, `.yml`, `.xml`, `.ron`, `.json5`).
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

        #[cfg(feature = "xml")]
        "xml" => from_xml_str(&contents),

        #[cfg(not(feature = "xml"))]
        "xml" => Err(crate::Error::UnsupportedFormat("xml (enable the 'xml' feature)".into())),

        #[cfg(feature = "ron")]
        "ron" => from_ron_str(&contents),

        #[cfg(not(feature = "ron"))]
        "ron" => Err(crate::Error::UnsupportedFormat("ron (enable the 'ron' feature)".into())),

        #[cfg(feature = "json5")]
        "json5" => from_json5_str(&contents),

        #[cfg(not(feature = "json5"))]
        "json5" => Err(crate::Error::UnsupportedFormat("json5 (enable the 'json5' feature)".into())),

        other => Err(crate::Error::UnsupportedFormat(other.into())),
    }
}

// parse a config file into a format-neutral value, the medium used to overlay
// one file's present keys onto an already-constructed config.
fn file_to_value(path: &Path) -> Result<Value> {
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

        #[cfg(feature = "xml")]
        "xml" => Ok(quick_xml::de::from_str::<crate::Tree>(&contents)?.into_value()),

        #[cfg(not(feature = "xml"))]
        "xml" => Err(crate::Error::UnsupportedFormat("xml (enable the 'xml' feature)".into())),

        #[cfg(feature = "ron")]
        "ron" => Ok(ron::from_str(&contents)?),

        #[cfg(not(feature = "ron"))]
        "ron" => Err(crate::Error::UnsupportedFormat("ron (enable the 'ron' feature)".into())),

        #[cfg(feature = "json5")]
        "json5" => Ok(json5::from_str(&contents)?),

        #[cfg(not(feature = "json5"))]
        "json5" => Err(crate::Error::UnsupportedFormat("json5 (enable the 'json5' feature)".into())),

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

// the serde data model splits "absent" two ways: `()` from a fresh slot and a
// serialized `Option::None`. both count as empty for overlay purposes.
fn is_empty(value: &Value) -> bool {
    matches!(value, Value::Unit | Value::Option(None))
}

// deep-merge `incoming` onto `base` per `mode`. Maps recurse; a key absent
// from base is always inserted; a present non-empty scalar is replaced only under
// Override.
fn merge_value(base: &mut Value, incoming: Value, mode: MergeMode) {
    match (base, incoming) {
        (Value::Map(base_map), Value::Map(incoming_map)) => {
            for (key, value) in incoming_map {
                merge_value(base_map.entry(key).or_insert(Value::Unit), value, mode);
            }
        }
        (base_slot, incoming) => match mode {
            MergeMode::Override => *base_slot = incoming,
            MergeMode::Overlay => {
                if is_empty(base_slot) {
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
    let value = serde_value::to_value(incoming)?;
    Ok(T::deserialize(value)?)
}

/// Combine a config file's keys with an existing value per `mode`.
pub(crate) fn merge_file<T>(base: &T, path: &Path, mode: MergeMode) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut base_value = serde_value::to_value(base)?;
    let file_value = file_to_value(path)?;
    merge_value(&mut base_value, file_value, mode);
    Ok(T::deserialize(Lenient(base_value))?)
}

/// Combine an in-memory mapping's keys with an existing value per `mode`.
pub(crate) fn merge_mapping<T, M>(base: &T, incoming: &M, mode: MergeMode) -> Result<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    M: serde::Serialize,
{
    let mut base_value = serde_value::to_value(base)?;
    let incoming_value = serde_value::to_value(incoming)?;
    merge_value(&mut base_value, incoming_value, mode);
    Ok(T::deserialize(Lenient(base_value))?)
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

/// Deserialize a parsed [`Tree`] into `T`, reading text where a bool, number,
/// list or option is wanted, as the file loaders do.
///
/// ```rust
/// # #[cfg(feature = "xml")]
/// # fn main() -> Result<(), conflaguration::Error> {
/// use conflaguration::{Tree, from_tree, from_xml_str};
///
/// #[derive(serde::Deserialize)]
/// struct Settings {
///     port: u16,
/// }
///
/// let tree: Tree = from_xml_str("<settings><port>8080</port></settings>")?;
/// let settings: Settings = from_tree(tree)?;
/// assert_eq!(settings.port, 8080);
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "xml"))]
/// # fn main() {}
/// ```
pub fn from_tree<T: serde::de::DeserializeOwned>(tree: Tree) -> Result<T> {
    Ok(T::deserialize(tree.into_lenient())?)
}
