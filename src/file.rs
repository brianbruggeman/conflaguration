use std::path::Path;

use crate::Result;
use crate::Settings;

#[cfg(feature = "toml")]
pub fn from_toml_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(toml::from_str(contents)?)
}

#[cfg(feature = "json")]
pub fn from_json_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(serde_json::from_str(contents)?)
}

#[cfg(feature = "yaml")]
pub fn from_yaml_str<T: serde::de::DeserializeOwned>(contents: &str) -> Result<T> {
    Ok(serde_yaml::from_str(contents)?)
}

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

pub fn from_file_then_env<T: serde::de::DeserializeOwned + Settings>(path: &Path) -> Result<T> {
    let mut config: T = from_file(path)?;
    config.override_from_env()?;
    Ok(config)
}

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
