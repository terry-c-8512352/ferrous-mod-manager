use std::collections::HashMap;

use ferrous_mod_manager::{
    collections::{
        create_collection_for_game, delete_collection_for_game, load_collection_for_game,
        save_collection_for_game,
    },
    locations::game_data_dir,
    models::{DetectedGame, ModCollection, ModConflict, ModDescriptor},
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_games,
            detect_mods,
            load_collections,
            save_collection,
            delete_collection,
            create_collection,
            detect_mod_conflict,
            apply_mod_collection
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&data_dir.join("mod-collections"))?;
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn detect_games() -> Result<Vec<DetectedGame>, String> {
    ferrous_mod_manager::detector::detect_games().map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_mods(game: DetectedGame) -> Vec<ModDescriptor> {
    ferrous_mod_manager::detector::discover_mods(&game)
}

#[tauri::command]
fn create_collection(game: DetectedGame, name: String) -> Result<ModCollection, String> {
    create_collection_for_game(game.app_id, name).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_collections(games: Vec<DetectedGame>) -> HashMap<u32, Vec<ModCollection>> {
    games
        .iter()
        .map(|g| {
            let path = game_data_dir(g.app_id);
            let collections = load_collection_for_game(path);
            (g.app_id, collections)
        })
        .collect()
}

#[tauri::command]
fn save_collection(game: DetectedGame, mod_collection: ModCollection) -> Result<(), String> {
    save_collection_for_game(game.app_id, &mod_collection).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_collection(game: DetectedGame, mod_collection: ModCollection) -> Result<(), String> {
    delete_collection_for_game(game.app_id, mod_collection.id).map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_mod_conflict(mods: Vec<ModDescriptor>) -> Vec<ModConflict> {
    ferrous_mod_manager::conflict::conflict_detection(mods)
}

#[tauri::command]
fn apply_mod_collection(game: DetectedGame, mod_collection: ModCollection) -> Result<(), String> {
    ferrous_mod_manager::collections::apply_mod_collection_for_game(&game, &mod_collection)
        .map_err(|e| e.to_string())
}
