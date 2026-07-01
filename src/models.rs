use crate::errors::FileOperationError;
use crate::fsutil::{MAX_READ_BYTES, read_to_string_limited, write_atomic};
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConflictCategory {
    Defines,
    GameData,
    Localisation,
    Events,
    Gfx,
    Map,
    Sound,
    Other,
}

impl ConflictCategory {
    /// Bucket a mod-relative file path into a category based on its top-level
    /// Paradox directory (e.g. `common/defines/...` -> `Defines`). Unrecognised
    /// paths fall through to `Other`.
    pub fn from_path(file_path: &Path) -> ConflictCategory {
        match file_path
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
        {
            Some("common") => {
                match file_path
                    .components()
                    .nth(1)
                    .and_then(|c| c.as_os_str().to_str())
                {
                    Some("defines") => ConflictCategory::Defines,
                    _ => ConflictCategory::GameData,
                }
            }
            Some("localisation" | "localization") => ConflictCategory::Localisation,
            Some("events") => ConflictCategory::Events,
            Some("gfx" | "interface" | "fonts" | "dlc_metadata") => ConflictCategory::Gfx,
            Some("sound" | "music") => ConflictCategory::Sound,
            Some("map" | "map_data") => ConflictCategory::Map,
            _ => ConflictCategory::Other,
        }
    }

    /// Whether a file in this category leaves the gameplay checksum untouched.
    /// Paradox games keep achievements (and ironman saves) enabled only when
    /// every active mod touches purely cosmetic content: graphics, sound, and
    /// localisation text. Everything else — game data, defines, events, the map,
    /// and unrecognised paths — changes the checksum and disables achievements.
    pub fn is_achievement_safe(&self) -> bool {
        matches!(
            self,
            ConflictCategory::Localisation | ConflictCategory::Gfx | ConflictCategory::Sound
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModDescriptor {
    pub name: Option<String>,              //Required
    pub path: Option<String>,              //Required
    pub remote_file_id: Option<String>,    //Requried
    pub supported_version: Option<String>, //Required
    pub tags: Option<Vec<String>>,
    pub picture: Option<String>,
    pub version: Option<String>,
    pub dependencies: Option<Vec<String>>,
}

impl ModDescriptor {
    pub fn mod_id(&self) -> &str {
        self.remote_file_id
            .as_deref()
            .or(self.path.as_deref())
            .unwrap_or("")
    }
}

#[derive(Debug)]
pub struct LibraryVdf {
    pub idx: u32,
    pub path: String,
    pub apps: Vec<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DetectedGame {
    pub app_id: u32,
    pub install_path: String,
    pub game_name: String,
    pub paradox_data_path: String,
}

#[derive(Debug, Serialize)]
pub struct ModConflict {
    pub file_path: PathBuf,
    pub mod_list: Vec<String>,
    pub category: ConflictCategory,
}

/// Whether a single mod keeps achievements (and ironman saves) enabled.
/// `gameplay_categories` lists the distinct checksum-affecting categories the
/// mod touches, so the UI can explain *why* achievements would be disabled; it
/// is empty when the mod is `compatible`.
#[derive(Debug, Serialize)]
pub struct AchievementStatus {
    pub mod_id: String,
    pub compatible: bool,
    pub gameplay_categories: Vec<ConflictCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModCollection {
    pub id: Uuid,
    pub name: String,
    pub mods: Vec<ModEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModEntry {
    pub mod_id: String,
    pub enabled: bool,
}

impl ModCollection {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            mods: vec![],
        }
    }

    pub fn add_mod(&mut self, mod_id: String) {
        self.mods.push(ModEntry {
            mod_id,
            enabled: true,
        });
    }

    pub fn toggle_mod(&mut self, mod_id: String) {
        let found_mod = self.mods.iter_mut().find(|m| m.mod_id == mod_id);
        match found_mod {
            None => log::warn!("toggle_mod: mod_id '{}' not found", mod_id),
            Some(entry) => entry.enabled = !entry.enabled,
        }
    }

    pub fn move_mod(&mut self, old_loc: usize, new_loc: usize) {
        let entry = self.mods.remove(old_loc);
        self.mods.insert(new_loc, entry);
    }

    pub fn save(&self, path: &Path) -> Result<(), FileOperationError> {
        let contents = serde_json::to_string_pretty(&self)?;
        write_atomic(path, &contents)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<ModCollection, FileOperationError> {
        let contents = read_to_string_limited(path, MAX_READ_BYTES)?;
        let mod_collection: ModCollection = serde_json::from_str(&contents)?;
        Ok(mod_collection)
    }
}

/// The game's `dlc_load.json`. Only `enabled_mods` is managed by this app;
/// `disabled_dlcs` and any field a future game version adds are round-tripped
/// untouched via `extra` instead of being silently dropped on write, and a
/// file missing either known field still parses.
#[derive(Debug, Serialize, Deserialize)]
pub struct DlcLoad {
    #[serde(default)]
    pub enabled_mods: Vec<String>,
    #[serde(default)]
    pub disabled_dlcs: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load() {
        let mod_id = "random_path_to_mod/mod_id".to_string();
        let path = std::env::temp_dir().join("test_collection.json");
        let mut mod_collection = ModCollection::new("Test Mod Collection");
        mod_collection.add_mod(mod_id.clone());
        assert!(mod_collection.save(&path).is_ok());
        let loaded_collection = ModCollection::load(&path);
        assert!(&loaded_collection.is_ok());
        assert_eq!(loaded_collection.unwrap().mods[0].mod_id, mod_id);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_dlc_load_round_trip_preserves_unknown_fields() {
        // Fields this app doesn't model (added by the game/launcher) must
        // survive a load-modify-save cycle.
        let input = r#"{
            "disabled_dlcs": ["dlc/dlc001.dlc"],
            "enabled_mods": ["mod/ugc_1.mod"],
            "some_future_field": {"nested": true}
        }"#;
        let mut dlc_load: DlcLoad = serde_json::from_str(input).unwrap();
        dlc_load.enabled_mods = vec!["mod/ugc_2.mod".to_string()];
        let output = serde_json::to_string(&dlc_load).unwrap();
        let round_tripped: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(round_tripped["some_future_field"]["nested"], true);
        assert_eq!(round_tripped["disabled_dlcs"][0], "dlc/dlc001.dlc");
        assert_eq!(round_tripped["enabled_mods"][0], "mod/ugc_2.mod");
    }

    #[test]
    fn test_dlc_load_parses_with_missing_fields() {
        // A dlc_load.json missing a known field must not hard-fail the apply.
        let dlc_load: DlcLoad = serde_json::from_str("{}").unwrap();
        assert!(dlc_load.enabled_mods.is_empty());
        assert!(dlc_load.disabled_dlcs.is_empty());
    }
}
