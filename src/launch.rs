use std::process::Command;

use crate::errors::LaunchError;

/// Launch a Steam game by its app id through the `steam://` URL handler.
///
/// This delegates to the platform's URL opener (Steam registers itself as the
/// handler for `steam:` URLs on install), so it works whether Steam is running
/// or not and without hardcoding the Steam executable path:
///
/// - Linux: `xdg-open`
/// - macOS: `open`
/// - Windows: `cmd /C start`
///
/// The child is spawned detached; we do not wait for the game to exit.
pub fn launch_game(app_id: u32) -> Result<(), LaunchError> {
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
