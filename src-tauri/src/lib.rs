use std::collections::HashMap;

use ferrous_mod_manager::{
    achievements,
    collections::{
        self, create_collection_for_game, delete_collection_for_game,
        load_or_create_collections_for_game, save_collection_for_game,
    },
    conflict,
    conflict_assessment::{self, AssessedConflict},
    dependency::{self, DependencyReport},
    detector,
    launch::launch_game,
    locations::ModRoots,
    models::{AchievementStatus, DetectedGame, ModCollection, ModDescriptor},
    settings::{self, UserSettings},
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Some Wayland/Nvidia systems hand WebKitGTK a DMA-BUF renderer it can't use,
    // producing a blank or broken window. Disabling it forces a software path that
    // works everywhere, so users no longer need to export this var before launching.
    // Must run before any GTK/WebKit init. Honor an explicit override if the user set one.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // Safe: called at the very start of `run()`, before any threads are spawned.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            detect_games,
            detect_mods,
            load_collections,
            save_collection,
            delete_collection,
            create_collection,
            detect_mod_conflict,
            mod_sizes,
            detect_achievement_compatibility,
            apply_mod_collection,
            enable_mod_with_dependencies,
            disable_mod_with_dependents,
            load_settings,
            save_settings,
            launch
        ])
        .setup(|app| {
            // Save window size/position on close and restore it on launch,
            // instead of resetting to the dimensions in tauri.conf.json.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("mod-collections"))?;
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
    detector::detect_games().map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_mods(game: DetectedGame) -> Vec<ModDescriptor> {
    detector::discover_mods(&game)
}

#[tauri::command]
fn create_collection(game: DetectedGame, name: String) -> Result<ModCollection, String> {
    create_collection_for_game(game.app_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_collections(games: Vec<DetectedGame>) -> HashMap<u32, Vec<ModCollection>> {
    games
        .iter()
        .map(|g| {
            // Create a default collection on first launch instead of an empty state.
            let collections = load_or_create_collections_for_game(g.app_id).unwrap_or_default();
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

/// Scan the enabled mods for file overlaps and classify each one (intended
/// override / ordering problem / cosmetic / widespread / genuine). `mods`
/// must be the enabled set **in collection load order** — verdicts and the
/// winning mod are derived from that order.
#[tauri::command]
fn detect_mod_conflict(mods: Vec<ModDescriptor>) -> Vec<AssessedConflict> {
    let conflicts = conflict::conflict_detection(mods.clone(), &ModRoots::detect());
    conflict_assessment::assess_conflicts(conflicts, &mods)
}

/// On-disk size (bytes) of each mod's file tree, keyed by `mod_id`. Mods without
/// a local `path` (or whose path can't be walked) report 0.
#[tauri::command]
fn mod_sizes(mods: Vec<ModDescriptor>) -> HashMap<String, u64> {
    conflict::mod_sizes(&mods, &ModRoots::detect())
}

#[tauri::command]
fn detect_achievement_compatibility(mods: Vec<ModDescriptor>) -> Vec<AchievementStatus> {
    achievements::achievement_status_for_mods(&mods, &ModRoots::detect())
}

/// The game's data path is re-resolved from the local Steam install by app id;
/// the frontend-supplied paths in `game` are deliberately not trusted for writes.
#[tauri::command]
fn apply_mod_collection(game: DetectedGame, mod_collection: ModCollection) -> Result<(), String> {
    collections::apply_mod_collection_by_app_id(game.app_id, &mod_collection)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct EnableModOutcome {
    collection: ModCollection,
    report: DependencyReport,
}

/// Enable a mod in a collection together with its transitive dependencies
/// (resolved by name against the installed mods), persist the updated
/// collection, and report auto-enabled and missing dependencies.
#[tauri::command]
fn enable_mod_with_dependencies(
    game: DetectedGame,
    mut mod_collection: ModCollection,
    mod_id: String,
    mods: Vec<ModDescriptor>,
) -> Result<EnableModOutcome, String> {
    let report = dependency::enable_with_dependencies(&mut mod_collection, &mod_id, &mods);
    save_collection_for_game(game.app_id, &mod_collection).map_err(|e| e.to_string())?;
    Ok(EnableModOutcome {
        collection: mod_collection,
        report,
    })
}

#[derive(serde::Serialize)]
struct DisableModOutcome {
    collection: ModCollection,
    disabled_dependents: Vec<String>,
}

/// Disable a mod in a collection together with every enabled mod that
/// transitively depends on it (resolved by name against the installed mods),
/// persist the updated collection, and report what was switched off with it.
#[tauri::command]
fn disable_mod_with_dependents(
    game: DetectedGame,
    mut mod_collection: ModCollection,
    mod_id: String,
    mods: Vec<ModDescriptor>,
) -> Result<DisableModOutcome, String> {
    let disabled = dependency::disable_with_dependents(&mut mod_collection, &mod_id, &mods);
    save_collection_for_game(game.app_id, &mod_collection).map_err(|e| e.to_string())?;
    Ok(DisableModOutcome {
        collection: mod_collection,
        disabled_dependents: disabled,
    })
}

/// Load user settings; a missing file yields defaults (first run).
#[tauri::command]
fn load_settings() -> Result<UserSettings, String> {
    settings::load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_settings(settings: UserSettings) -> Result<(), String> {
    settings::save_settings(&settings).map_err(|e| e.to_string())
}

/// Launch the game's executable directly, falling back to `steam://run/<app_id>`.
/// On success the manager window minimizes to get out of the way of the game.
#[tauri::command]
fn launch(window: tauri::Window, game: DetectedGame) -> Result<(), String> {
    launch_game(&game).map_err(|e| e.to_string())?;
    // Best-effort: a failed minimize shouldn't fail the launch.
    let _ = window.minimize();
    Ok(())
}
