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

/// The canonicalized directory roots that mod content may legitimately live
/// under. Descriptor `path=` values come from third-party `.mod` files
/// (Steam Workshop content), so they are untrusted: before walking one we
/// require it to resolve inside these roots, otherwise a malicious descriptor
/// (`path="/"`) could send `WalkDir` across the whole filesystem.
#[derive(Debug, Clone)]
pub struct ModRoots(Vec<PathBuf>);

impl ModRoots {
    /// Roots for the current user: the Paradox data dir (local mods) plus
    /// every Steam library's `steamapps/workshop/content` dir (workshop mods).
    pub fn detect() -> Self {
        match dirs::home_dir() {
            Some(home) => Self::for_home(&home),
            None => Self(Vec::new()),
        }
    }

    pub fn for_home(home: &Path) -> Self {
        let mut roots = vec![paradox_data_root(home)];
        if let Some(vdf_path) = steam_library_vdf_candidates(home)
            .into_iter()
            .find(|p| p.exists())
        {
            match crate::fsutil::read_to_string_limited(&vdf_path, crate::fsutil::MAX_READ_BYTES)
                .map_err(|e| e.to_string())
                .and_then(|c| crate::parser::vdf::parse_vdf_file(&c).map_err(|e| e.to_string()))
            {
                Ok(libraries) => {
                    for library in libraries {
                        roots.push(
                            Path::new(&library.path)
                                .join("steamapps")
                                .join("workshop")
                                .join("content"),
                        );
                    }
                }
                Err(e) => log::warn!("Could not read Steam libraries for mod roots: {e}"),
            }
        }
        Self::from_roots(roots)
    }

    /// Build from explicit roots (used by tests and tools that already know
    /// where mods live). Roots that don't exist are dropped.
    pub fn from_roots(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self(
            roots
                .into_iter()
                .filter_map(|r| r.canonicalize().ok())
                .collect(),
        )
    }

    /// Canonicalize an untrusted mod path (resolving symlinks and `..`) and
    /// return it only if it lies under one of the allowed roots.
    pub fn checked_path(&self, path: &str) -> Option<PathBuf> {
        let canonical = Path::new(path).canonicalize().ok()?;
        if self.0.iter().any(|root| canonical.starts_with(root)) {
            Some(canonical)
        } else {
            log::warn!("Refusing to scan mod path outside known mod directories: {path}");
            None
        }
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_checked_path_accepts_paths_inside_roots() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conflict");
        let roots = ModRoots::from_roots([base.clone()]);
        let inside = base.join("mod_a").to_string_lossy().into_owned();
        assert!(roots.checked_path(&inside).is_some());
    }

    #[test]
    fn test_checked_path_rejects_paths_outside_roots() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conflict");
        let roots = ModRoots::from_roots([base]);
        assert!(roots.checked_path("/").is_none());
        assert!(roots.checked_path("/etc").is_none());
        // Escaping an allowed root with `..` must not work either.
        let escape = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/conflict/mod_a/../../fake_home")
            .to_string_lossy()
            .into_owned();
        assert!(roots.checked_path(&escape).is_none());
    }

    #[test]
    fn test_checked_path_rejects_symlink_escaping_root() {
        let base = std::env::temp_dir().join(format!("modroots_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let link = base.join("sneaky");
        std::os::unix::fs::symlink("/etc", &link).unwrap();

        let roots = ModRoots::from_roots([base.clone()]);
        assert!(
            roots.checked_path(&link.to_string_lossy()).is_none(),
            "a symlink pointing outside the root must be rejected"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

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
