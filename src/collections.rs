use uuid::Uuid;

use crate::errors::FileOperationError;
use crate::locations::game_data_dir;
use crate::models::{DetectedGame, DlcLoad, ModCollection};
use std::fs::{create_dir_all, read_dir};
use std::path::{Path, PathBuf};

pub fn create_collection_for_game(
    app_id: u32,
    name: String,
) -> Result<ModCollection, FileOperationError> {
    let mc = ModCollection::new(&name);
    save_collection_for_game(app_id, &mc)?;
    Ok(mc)
}

/// Name given to the collection created automatically when a game has none yet.
pub const DEFAULT_COLLECTION_NAME: &str = "Default";

/// Load every collection for a game, creating and persisting an empty default
/// collection when none exist yet. This guarantees a freshly detected game
/// always has at least one selectable collection on first launch, rather than
/// presenting an empty/"no collection" state.
pub fn load_or_create_collections_for_game(
    app_id: u32,
) -> Result<Vec<ModCollection>, FileOperationError> {
    let mut collections = load_collection_for_game(game_data_dir(app_id));
    if collections.is_empty() {
        let default = create_collection_for_game(app_id, DEFAULT_COLLECTION_NAME.to_string())?;
        collections.push(default);
    }
    Ok(collections)
}

pub fn load_collection_for_game(path: PathBuf) -> Vec<ModCollection> {
    let mut mod_collections: Vec<ModCollection> = vec![];
    let Ok(read_result) = read_dir(path.as_path()) else {
        log::warn!("No mod collections found in : {}", path.display());
        return mod_collections;
    };
    for file in read_result {
        if let Ok(entry) = file {
            let load_attempt = ModCollection::load(&entry.path());

            match load_attempt {
                Ok(mc) => {
                    mod_collections.push(mc);
                }
                Err(e) => {
                    log::warn!("Unable to load mod collection: {}", e)
                }
            }
        }
    }

    mod_collections
}

/// Import a collection from an external JSON file into a game's collection
/// store. The imported collection is given a fresh UUID before saving so it can
/// never overwrite an existing collection (even one exported from this same
/// game), and is returned for the UI to select.
pub fn import_collection_for_game(
    app_id: u32,
    path: &Path,
) -> Result<ModCollection, FileOperationError> {
    let mut collection = ModCollection::load(path)?;
    collection.id = Uuid::new_v4();
    save_collection_for_game(app_id, &collection)?;
    Ok(collection)
}

pub fn save_collection_for_game(
    app_id: u32,
    mod_collection: &ModCollection,
) -> Result<(), FileOperationError> {
    let path = PathBuf::from(game_data_dir(app_id));
    if path.exists() {
        format_save(&mod_collection, &path)?;
    } else {
        create_dir_all(game_data_dir(app_id))?;
        format_save(&mod_collection, &path)?;
    }
    Ok(())
}

fn format_save(mod_collection: &ModCollection, path: &PathBuf) -> Result<(), FileOperationError> {
    mod_collection.save(&path.join(format!("{}.json", mod_collection.id.to_string())))?;
    Ok(())
}

pub fn delete_collection_for_game(
    app_id: u32,
    mod_collection_id: Uuid,
) -> Result<(), FileOperationError> {
    let path = game_data_dir(app_id).join(format!("{}.json", mod_collection_id.to_string()));
    std::fs::remove_file(path)?;
    Ok(())
}

pub fn apply_mod_collection_for_game(
    game: &DetectedGame,
    mod_collection: &ModCollection,
) -> Result<(), FileOperationError> {
    let data_path = Path::new(&game.paradox_data_path).join("dlc_load.json");
    let dlc_load_contents = std::fs::read_to_string(&data_path)?;
    let mut dlc_load: DlcLoad = serde_json::from_str(dlc_load_contents.as_str())?;
    dlc_load.enabled_mods = Vec::new();
    for md in &mod_collection.mods {
        if md.enabled {
            dlc_load
                .enabled_mods
                .push(format!("mod/ugc_{}.mod", md.mod_id));
        }
    }

    let dlc_load_contents = serde_json::to_string_pretty(&dlc_load)?;
    std::fs::write(&data_path, dlc_load_contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector;

    #[test]
    fn test_load_correct_collection_for_game() {
        let path = PathBuf::new().join("tests/fixtures/mod_collections/123456");
        let mod_collection = load_collection_for_game(path);
        assert_eq!(mod_collection.len(), 1);
        assert_eq!(mod_collection[0].name, "test-collection");
        assert_eq!(mod_collection[0].mods.iter().len(), 1);
        assert_eq!(mod_collection[0].mods[0].mod_id, "path/to/mod")
    }

    #[test]
    fn test_load_or_create_creates_default_then_reuses_it() {
        // Unlikely app_id so we don't collide with a real game's collections.
        let app_id = 4_294_967_290;
        let dir = game_data_dir(app_id);
        let _ = std::fs::remove_dir_all(&dir);

        let first = load_or_create_collections_for_game(app_id).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].name, DEFAULT_COLLECTION_NAME);

        // A second call must reuse the persisted default, not create another.
        let second = load_or_create_collections_for_game(app_id).unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, first[0].id);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_import_assigns_fresh_id_and_persists() {
        let app_id = 4_294_967_289;
        let dir = game_data_dir(app_id);
        let _ = std::fs::remove_dir_all(&dir);

        // An external export file with a known id.
        let mut exported = ModCollection::new("Imported");
        exported.add_mod("12345".to_string());
        let export_path = std::env::temp_dir().join("ferrous_import_test.json");
        exported.save(&export_path).unwrap();

        let imported = import_collection_for_game(app_id, &export_path).unwrap();
        assert_eq!(imported.name, "Imported");
        assert_eq!(imported.mods.len(), 1);
        // Fresh id so an import never clobbers an existing collection.
        assert_ne!(imported.id, exported.id);
        // Persisted under the new id.
        assert!(dir.join(format!("{}.json", imported.id)).exists());

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&export_path);
    }

    #[test]
    #[ignore = "requires a real Steam install: detect_games() reads live Steam paths"]
    fn test_apply_mod_collection_for_game() {
        let games = detector::detect_games().unwrap();
        let game = games.iter().find(|g| g.game_name == "Stellaris").unwrap();

        apply_mod_collection_for_game(game, &ModCollection::new("Test")).unwrap();
    }
}
