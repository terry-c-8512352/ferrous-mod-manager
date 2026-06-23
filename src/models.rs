use crate::errors::FileOperationError;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::{read_to_string, write};
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

#[derive(Debug, Serialize, Deserialize)]
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
        write(path, contents)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<ModCollection, FileOperationError> {
        let contents = read_to_string(path)?;
        let mod_collection: ModCollection = serde_json::from_str(&contents)?;
        Ok(mod_collection)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DlcLoad {
    pub enabled_mods: Vec<String>,
    pub disabled_dlcs: Vec<String>,
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
}
