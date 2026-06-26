use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("ferrous-mod-manager")
        .join("mod-collections")
}

pub fn game_data_dir(app_id: u32) -> PathBuf {
    data_dir().join(app_id.to_string())
}

/// Root directory holding Paradox Interactive's per-game data, derived from the
/// user's home directory so it stays testable against fixture homes.
///
/// - Linux: `~/.local/share/Paradox Interactive`
/// - Windows: `%USERPROFILE%\Documents\Paradox Interactive`
pub fn paradox_data_root(home: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        home.join("Documents").join("Paradox Interactive")
    }
    #[cfg(not(windows))]
    {
        home.join(".local")
            .join("share")
            .join("Paradox Interactive")
    }
}

/// Candidate locations for Steam's `libraryfolders.vdf`, in priority order. The
/// caller uses the first one that exists on disk.
///
/// On Linux these are home-relative (native, flatpak, and the older
/// `~/.local/share/Steam` layout). On Windows Steam is rarely under the user
/// home, so the standard `Program Files` install locations are used instead.
#[cfg_attr(windows, allow(unused_variables))]
pub fn steam_library_vdf_candidates(home: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\libraryfolders.vdf"),
            PathBuf::from(r"C:\Program Files\Steam\steamapps\libraryfolders.vdf"),
        ]
    }
    #[cfg(not(windows))]
    {
        let steamapps_vdf = |base: PathBuf| base.join("steamapps").join("libraryfolders.vdf");
        vec![
            steamapps_vdf(home.join(".steam").join("steam")),
            steamapps_vdf(home.join(".local").join("share").join("Steam")),
            steamapps_vdf(
                home.join(".var")
                    .join("app")
                    .join("com.valvesoftware.Steam")
                    .join(".local")
                    .join("share")
                    .join("Steam"),
            ),
        ]
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_paradox_data_root_linux_layout() {
        let root = paradox_data_root(Path::new("/home/user"));
        assert_eq!(
            root,
            Path::new("/home/user/.local/share/Paradox Interactive")
        );
    }

    #[test]
    fn test_steam_vdf_candidates_prefers_native_path() {
        let candidates = steam_library_vdf_candidates(Path::new("/home/user"));
        assert_eq!(
            candidates.first().unwrap(),
            Path::new("/home/user/.steam/steam/steamapps/libraryfolders.vdf")
        );
        // Flatpak layout is included as a fallback.
        assert!(
            candidates
                .iter()
                .any(|p| p.to_string_lossy().contains("com.valvesoftware.Steam"))
        );
    }
}
