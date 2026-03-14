use ferrous_mod_manager::conflict;
use ferrous_mod_manager::detector;

fn main() {
    match detector::detect_games() {
        Ok(games) => {
            if games.is_empty() {
                println!("No supported Paradox games found.");
                return;
            }
            for game in &games {
                println!("Found: {} (appid {})", game.game_name, game.app_id);
                println!("  Install path:     {}", game.install_path);
                println!("  Paradox data dir: {}", game.paradox_data_path);

                let mods = detector::discover_mods(game);
                println!("  Mods: {}", mods.len());
                for m in &mods {
                    println!("    - {}", m.name.as_deref().unwrap_or("(unnamed)"));
                }

                let mod_conflicts = conflict::conflict_detection(mods);

                for m_conflict in mod_conflicts {
                    println!("Game File {}", m_conflict.file_path.display());
                    for mod_name in m_conflict.mod_list {
                        println!("// Mods affecting file {}", mod_name)
                    }
                }
            }
        }
        Err(e) => eprintln!("Error detecting games: {e}"),
    }
}
