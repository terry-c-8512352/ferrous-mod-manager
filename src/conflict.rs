use crate::models::{ConflictCategory, ModConflict, ModDescriptor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn scan_mods(mod_list: Vec<ModDescriptor>) -> HashMap<PathBuf, Vec<String>> {
    let mut file_map: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for game_mod in mod_list {
        if let Some(path) = game_mod.path {
            for entry in WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.map_err(|err| log::warn!("Skipping entry: {err}")).ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }
                if let Ok(relative) = entry.path().strip_prefix(&path) {
                    if relative == Path::new("descriptor.mod") {
                        continue;
                    }
                    let mod_name = game_mod.name.clone().unwrap_or_else(|| path.clone());
                    file_map
                        .entry(relative.to_path_buf())
                        .or_default()
                        .push(mod_name);
                }
            }
        }
    }

    file_map
}

fn categorize_path(file_path: &Path) -> ConflictCategory {
    match file_path
        .components()
        .next()
        .and_then(|c| c.as_os_str().to_str())
    {
        Some("common") => {
            match file_path
                .components()
                .nth(1)
                .and_then(|c| c.as_os_str().to_str())
            {
                Some("defines") => ConflictCategory::Defines,
                _ => ConflictCategory::GameData,
            }
        }
        Some("localisation" | "localization") => ConflictCategory::Localisation,
        Some("events") => ConflictCategory::Events,
        Some("gfx" | "interface" | "fonts" | "dlc_metadata") => ConflictCategory::Gfx,
        Some("sound" | "music") => ConflictCategory::Sound,
        Some("map" | "map_data") => ConflictCategory::Map,
        _ => ConflictCategory::Other,
    }
}

pub fn conflict_detection(mods: Vec<ModDescriptor>) -> Vec<ModConflict> {
    let file_map = scan_mods(mods);
    let mut list_of_conflicts: Vec<ModConflict> = Vec::new();
    for (file_path, mod_list) in file_map {
        if mod_list.len() > 1 {
            let mod_category = categorize_path(&file_path);
            list_of_conflicts.push(ModConflict {
                file_path,
                mod_list,
                category: mod_category,
            });
        }
    }

    list_of_conflicts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModDescriptor;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/fixtures/conflict");
        path.push(name);
        path.to_string_lossy().into_owned()
    }

    fn make_mod(name: &str) -> ModDescriptor {
        ModDescriptor {
            name: Some(name.to_string()),
            path: Some(fixture_path(name)),
            remote_file_id: None,
            supported_version: None,
            tags: None,
            picture: None,
            version: None,
            dependencies: None,
        }
    }

    #[test]
    fn test_conflict_detected() {
        let mods = vec![make_mod("mod_a"), make_mod("mod_b")];
        let file_map = scan_mods(mods);

        let conflict_path = PathBuf::from("common/traits/foo.txt");
        let conflicting_mods = file_map.get(&conflict_path).expect("file should be in map");
        assert_eq!(conflicting_mods.len(), 2, "foo.txt should be in both mods");
        assert!(conflicting_mods.contains(&"mod_a".to_string()));
        assert!(conflicting_mods.contains(&"mod_b".to_string()));
    }

    #[test]
    fn test_no_conflict_for_unique_files() {
        let mods = vec![make_mod("mod_a"), make_mod("mod_b")];
        let file_map = scan_mods(mods);

        let unique_path = PathBuf::from("events/my_event.txt");
        let mods_with_file = file_map.get(&unique_path).expect("file should be in map");
        assert_eq!(
            mods_with_file.len(),
            1,
            "my_event.txt should only be in mod_b"
        );
    }

    #[test]
    fn test_conflict_detection_finds_conflicts() {
        let mods = vec![make_mod("mod_a"), make_mod("mod_b")];
        let conflicts = conflict_detection(mods);

        let conflict = conflicts
            .iter()
            .find(|c| c.file_path == PathBuf::from("common/traits/foo.txt"))
            .expect("expected a conflict on foo.txt");

        assert!(conflict.mod_list.contains(&"mod_a".to_string()));
        assert!(conflict.mod_list.contains(&"mod_b".to_string()));
    }

    #[test]
    fn test_conflict_detection_no_conflicts() {
        let mods = vec![make_mod("mod_a")];
        let conflicts = conflict_detection(mods);

        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_categorize_defines() {
        assert_eq!(
            categorize_path(Path::new("common/defines/00_defines.txt")),
            ConflictCategory::Defines
        );
    }

    #[test]
    fn test_categorize_game_data() {
        assert_eq!(
            categorize_path(Path::new("common/traits/leader_traits.txt")),
            ConflictCategory::GameData
        );
    }

    #[test]
    fn test_categorize_localisation() {
        assert_eq!(
            categorize_path(Path::new("localisation/english/l_english.yml")),
            ConflictCategory::Localisation
        );
        assert_eq!(
            categorize_path(Path::new("localization/english/l_english.yml")),
            ConflictCategory::Localisation
        );
    }

    #[test]
    fn test_categorize_events() {
        assert_eq!(
            categorize_path(Path::new("events/my_event.txt")),
            ConflictCategory::Events
        );
    }

    #[test]
    fn test_categorize_gfx() {
        assert_eq!(
            categorize_path(Path::new("gfx/models/ship.mesh")),
            ConflictCategory::Gfx
        );
        assert_eq!(
            categorize_path(Path::new("interface/topbar.gui")),
            ConflictCategory::Gfx
        );
    }

    #[test]
    fn test_categorize_sound() {
        assert_eq!(
            categorize_path(Path::new("sound/effects/boom.wav")),
            ConflictCategory::Sound
        );
    }

    #[test]
    fn test_categorize_map() {
        assert_eq!(
            categorize_path(Path::new("map/galaxy/setup.txt")),
            ConflictCategory::Map
        );
    }

    #[test]
    fn test_categorize_other() {
        assert_eq!(
            categorize_path(Path::new("flags/custom_flag.tga")),
            ConflictCategory::Other
        );
    }
}
