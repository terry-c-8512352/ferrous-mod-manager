use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::LaunchError;
use crate::models::DetectedGame;

/// Launch a detected game.
///
/// We prefer running the game's own executable directly out of its Steam
/// install directory, which skips the Paradox launcher that
/// `steam://run/<app_id>` would otherwise pop up. When the binary can't be
/// located (unknown game, an unusual install layout, or a non-Linux platform
/// where the executable name differs), we fall back to the `steam://` URL
/// handler so launching still works.
///
/// The child is spawned detached; we do not wait for the game to exit.
pub fn launch_game(game: &DetectedGame) -> Result<(), LaunchError> {
    if let Some(binary) = resolve_game_binary(game) {
        let mut command = Command::new(&binary);
        // Paradox binaries expect their working directory to be the install
        // root (they resolve game data relative to it), even when the
        // executable itself lives under `binaries/`.
        command.current_dir(&game.install_path);
        command.spawn()?;
        return Ok(());
    }

    launch_via_steam(game.app_id)
}

/// Relative paths (within the game's Steam install dir) to the real game
/// executable on Linux, in priority order. Paradox ships the binary either at
/// the install root or under `binaries/`, alongside the launcher we skip.
fn game_binary_candidates(app_id: u32) -> &'static [&'static str] {
    match app_id {
        281990 => &["stellaris"],                       // Stellaris
        236850 => &["eu4"],                             // Europa Universalis IV
        394360 => &["hoi4"],                            // Hearts of Iron IV
        1158310 => &["binaries/ck3", "ck3"],            // Crusader Kings III
        529340 => &["binaries/victoria3", "victoria3"], // Victoria 3
        859580 => &["binaries/imperator", "imperator"], // Imperator: Rome
        _ => &[],
    }
}

/// First existing executable file among the game's known binary candidates.
fn resolve_game_binary(game: &DetectedGame) -> Option<PathBuf> {
    let install = Path::new(&game.install_path);
    game_binary_candidates(game.app_id)
        .iter()
        .map(|rel| install.join(rel))
        .find(|candidate| candidate.is_file())
}

/// Launch through the platform's URL opener, which Steam registers as the
/// handler for `steam:` URLs on install (works whether Steam is running or not
/// and without hardcoding the Steam executable path):
///
/// - Linux: `xdg-open`
/// - macOS: `open`
/// - Windows: `cmd /C start`
fn launch_via_steam(app_id: u32) -> Result<(), LaunchError> {
    let url = format!("steam://run/{app_id}");

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("cmd");
        // The empty "" is `start`'s window-title argument; without it a quoted
        // URL would be consumed as the title instead of the thing to open.
        c.args(["/C", "start", "", &url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(&url);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(&url);
        c
    };

    command.spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(app_id: u32, install_path: &str) -> DetectedGame {
        DetectedGame {
            app_id,
            install_path: install_path.to_string(),
            game_name: "Test".to_string(),
            paradox_data_path: String::new(),
        }
    }

    #[test]
    fn resolves_binary_at_install_root() {
        let install = "tests/fixtures/launch/stellaris_install";
        let resolved = resolve_game_binary(&game(281990, install));
        assert_eq!(resolved, Some(Path::new(install).join("stellaris")));
    }

    #[test]
    fn resolves_binary_under_binaries_subdir() {
        let install = "tests/fixtures/launch/ck3_install";
        let resolved = resolve_game_binary(&game(1158310, install));
        assert_eq!(
            resolved,
            Some(Path::new(install).join("binaries").join("ck3"))
        );
    }

    #[test]
    fn returns_none_when_binary_missing() {
        let resolved = resolve_game_binary(&game(281990, "tests/fixtures/launch/empty_install"));
        assert_eq!(resolved, None);
    }

    #[test]
    fn returns_none_for_unknown_game() {
        // The Stellaris binary exists here, but the app id isn't a known game.
        let resolved =
            resolve_game_binary(&game(999999, "tests/fixtures/launch/stellaris_install"));
        assert_eq!(resolved, None);
    }
}
