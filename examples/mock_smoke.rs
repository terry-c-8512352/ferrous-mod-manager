// Throwaway: drives the real backend pipeline against a mock $HOME so we can
// confirm the GUI will get sensible data.
// Run with: HOME=<mockhome> cargo run --example mock_smoke -p ferrous-mod-manager
use ferrous_mod_manager::{achievements, conflict, detector};

fn main() {
    let games = detector::detect_games().expect("detect_games");
    println!("Detected {} game(s):", games.len());
    for g in &games {
        println!("  - {} (app_id {})", g.game_name, g.app_id);
    }

    let game = games.first().expect("at least one game");
    let mods = detector::discover_mods(game);
    println!("\nDiscovered {} mod(s) in {}:", mods.len(), game.game_name);

    let statuses = achievements::achievement_status_for_mods(&mods);
    for (m, s) in mods.iter().zip(&statuses) {
        let name = m.name.as_deref().unwrap_or("<unnamed>");
        if s.compatible {
            println!("  [achievements OK ] {name}");
        } else {
            println!(
                "  [achievements OFF] {name}  (affects {:?})",
                s.gameplay_categories
            );
        }
    }

    let blockers = statuses.iter().filter(|s| !s.compatible).count();
    println!("\nIronman/achievements this loadout: {blockers} mod(s) disable them");

    let conflicts = conflict::conflict_detection(mods);
    println!("\n{} file conflict(s):", conflicts.len());
    for c in &conflicts {
        println!(
            "  - {} : {:?}  [{:?}]",
            c.file_path.display(),
            c.mod_list,
            c.category
        );
    }
}
