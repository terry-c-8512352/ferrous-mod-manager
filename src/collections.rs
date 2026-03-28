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
    fn test_apply_mod_collection_for_game() {
        let games = detector::detect_games().unwrap();
        let game = games.iter().find(|g| g.game_name == "Stellaris").unwrap();

        apply_mod_collection_for_game(game, &ModCollection::new("Test")).unwrap();
    }
}
