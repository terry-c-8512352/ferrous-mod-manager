use std::fs::create_dir_all;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::FileOperationError;
use crate::fsutil::{MAX_READ_BYTES, read_to_string_limited, write_atomic};
use crate::locations;

/// User-level preferences, persisted as one JSON file (`settings.json`) next
/// to the collection store. `#[serde(default)]` keeps files written by older
/// versions loadable as new settings are added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserSettings {
    /// Automatically enable a mod's dependencies when enabling it, and disable
    /// its dependents when disabling it. On by default; when off, toggling a
    /// mod affects only that mod.
    pub auto_dependencies: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            auto_dependencies: true,
        }
    }
}

/// Load settings from the default location. A missing file is a normal first
/// run and yields defaults; a malformed one is an error so a user's hand edits
/// aren't silently thrown away.
pub fn load_settings() -> Result<UserSettings, FileOperationError> {
    load_settings_from(&locations::settings_file())
}

pub fn load_settings_from(path: &Path) -> Result<UserSettings, FileOperationError> {
    if !path.exists() {
        return Ok(UserSettings::default());
    }
    let contents = read_to_string_limited(path, MAX_READ_BYTES)?;
    Ok(serde_json::from_str(&contents)?)
}

pub fn save_settings(settings: &UserSettings) -> Result<(), FileOperationError> {
    save_settings_to(settings, &locations::settings_file())
}

pub fn save_settings_to(settings: &UserSettings, path: &Path) -> Result<(), FileOperationError> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    write_atomic(path, &serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_settings_path(tag: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("fmm_settings_test_{}_{tag}", std::process::id()))
            .join("settings.json")
    }

    #[test]
    fn test_missing_file_yields_defaults() {
        let settings = load_settings_from(Path::new("/nonexistent/settings.json")).unwrap();
        assert_eq!(settings, UserSettings::default());
        assert!(settings.auto_dependencies);
    }

    #[test]
    fn test_settings_round_trip() {
        let path = temp_settings_path("round_trip");
        let settings = UserSettings {
            auto_dependencies: false,
        };
        save_settings_to(&settings, &path).unwrap();
        assert_eq!(load_settings_from(&path).unwrap(), settings);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_missing_fields_fall_back_to_defaults() {
        // A file from an older version without newer keys must still load.
        let path = temp_settings_path("partial");
        create_dir_all(path.parent().unwrap()).unwrap();
        write_atomic(&path, "{}").unwrap();
        let settings = load_settings_from(&path).unwrap();
        assert!(settings.auto_dependencies);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn test_malformed_file_is_an_error() {
        let path = temp_settings_path("malformed");
        create_dir_all(path.parent().unwrap()).unwrap();
        write_atomic(&path, "not json").unwrap();
        assert!(load_settings_from(&path).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
