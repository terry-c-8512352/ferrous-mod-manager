use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::errors::DetectionError;
use crate::models::{DetectedGame, ModDescriptor};
use crate::parser::{mod_descriptor, vdf};

pub fn detect_games() -> Result<Vec<DetectedGame>, DetectionError> {
    let home = dirs::home_dir().ok_or(DetectionError::NoHomeDir)?;
    detect_games_from_home(&home)
}

fn detect_games_from_home(home: &Path) -> Result<Vec<DetectedGame>, DetectionError> {
    // (game_name, steam_folder, paradox_folder)
    let game_info: HashMap<u32, (&str, &str, &str)> = HashMap::from([
        (281990, ("Stellaris", "Stellaris", "Stellaris")),
        (
            236850,
            (
                "Europa Universalis IV",
                "Europa Universalis IV",
                "Europa Universalis IV",
            ),
        ),
        (
            394360,
            (
                "Hearts of Iron IV",
                "Hearts of Iron IV",
                "Hearts of Iron IV",
            ),
        ),
        (
            1158310,
            (
                "Crusader Kings III",
                "Crusader Kings III",
                "Crusader Kings III",
            ),
        ),
        (529340, ("Victoria 3", "Victoria 3", "Victoria 3")),
        (
            859580,
            ("Imperator: Rome", "Imperator Rome", "Imperator Rome"),
        ),
    ]);

    let vdf_path = home
        .join(".steam")
        .join("steam")
        .join("steamapps")
        .join("libraryfolders.vdf");

    let content = fs::read_to_string(&vdf_path)?;
    let libraries = vdf::parse_vdf_file(&content)?;

    let mut detected = Vec::new();

    for library in libraries {
        for app_id in &library.apps {
            if let Some(&(game_name, folder_name, paradox_folder)) = game_info.get(app_id) {
                let install_path = Path::new(&library.path)
                    .join("steamapps")
                    .join("common")
                    .join(folder_name)
                    .to_string_lossy()
                    .into_owned();

                let paradox_data_path = home
                    .join(".local")
                    .join("share")
                    .join("Paradox Interactive")
                    .join(paradox_folder)
                    .to_string_lossy()
                    .into_owned();

                detected.push(DetectedGame {
                    app_id: *app_id,
                    install_path,
                    game_name: game_name.to_string(),
                    paradox_data_path,
                });
            }
        }
    }

    Ok(detected)
}

pub fn discover_mods(game: &DetectedGame) -> Vec<ModDescriptor> {
    let mod_dir = std::path::Path::new(&game.paradox_data_path).join("mod");

    let entries = match fs::read_dir(&mod_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? == "mod" {
                let content = fs::read_to_string(&path).ok()?;
                mod_descriptor::parse_mod_file(&content).ok()
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_games_missing_vdf_returns_io_error() {
        let result = detect_games_from_home(Path::new("/nonexistent/home"));
        assert!(matches!(result, Err(DetectionError::Io(_))));
    }

    #[test]
    fn test_detect_games_finds_known_games() {
        let home = Path::new("tests/fixtures/fake_home");
        let games = detect_games_from_home(home).unwrap();
        assert_eq!(games.len(), 2);
        assert!(games.iter().any(|g| g.game_name == "Stellaris"));
        assert!(games.iter().any(|g| g.game_name == "Crusader Kings III"));
    }

    #[test]
    fn test_detect_games_ignores_unknown_app_ids() {
        let home = Path::new("tests/fixtures/fake_home_no_games");
        let games = detect_games_from_home(home).unwrap();
        assert!(games.is_empty());
    }

    #[test]
    fn test_detect_games_populates_paths_correctly() {
        let home = Path::new("tests/fixtures/fake_home");
        let games = detect_games_from_home(home).unwrap();
        let stellaris = games.iter().find(|g| g.game_name == "Stellaris").unwrap();

        assert_eq!(stellaris.app_id, 281990);
        assert_eq!(
            stellaris.install_path,
            "/fake/steam/library/steamapps/common/Stellaris"
        );
        assert_eq!(
            stellaris.paradox_data_path,
            "tests/fixtures/fake_home/.local/share/Paradox Interactive/Stellaris"
        );
    }

    fn make_game(paradox_data_path: &str) -> DetectedGame {
        DetectedGame {
            app_id: 281990,
            install_path: String::new(),
            game_name: "Stellaris".to_string(),
            paradox_data_path: paradox_data_path.to_string(),
        }
    }

    #[test]
    fn test_discover_mods_missing_directory_returns_empty() {
        let game = make_game("/nonexistent/path/that/does/not/exist");
        let mods = discover_mods(&game);
        assert!(mods.is_empty());
    }

    #[test]
    fn test_discover_mods_returns_valid_mods() {
        let game = make_game("tests/fixtures/discover");
        let mods = discover_mods(&game);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].name.as_deref().unwrap(), "Valid Test Mod");
    }

    #[test]
    fn test_discover_mods_skips_invalid_mod_files() {
        // fixtures/discover/mod/ contains invalid.mod (missing required fields)
        // it should be silently skipped, not cause a panic or error
        let game = make_game("tests/fixtures/discover");
        let mods = discover_mods(&game);
        assert!(
            mods.iter()
                .all(|m| m.name.as_deref() != Some("Incomplete Mod"))
        );
    }

    #[test]
    fn test_discover_mods_ignores_non_mod_files() {
        // fixtures/discover/mod/ also contains readme.txt
        // only .mod files should be parsed
        let game = make_game("tests/fixtures/discover");
        let mods = discover_mods(&game);
        assert_eq!(mods.len(), 1);
    }
}
