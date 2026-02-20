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
            }
        }
        Err(e) => eprintln!("Error detecting games: {e}"),
    }
}
