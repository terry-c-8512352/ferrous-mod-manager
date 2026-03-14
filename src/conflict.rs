use crate::models::{ModConflict, ModDescriptor};
use std::collections::HashMap;
use std::path::PathBuf;
use walkdir::WalkDir;

fn scan_mods(mod_list: Vec<ModDescriptor>) -> HashMap<PathBuf, Vec<String>> {
    let mut file_map: HashMap<PathBuf, Vec<String>> = HashMap::new();

    for game_mod in mod_list {
        if let Some(path) = game_mod.path {
            for entry in WalkDir::new(&path) {
                let entry = entry.unwrap(); // TODO: Fix this so we capture the error better.
                if !entry.file_type().is_file() {
                    continue;
                }
                if let Ok(relative) = entry.path().strip_prefix(&path) {
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

pub fn conflict_detection(mods: Vec<ModDescriptor>) -> Vec<ModConflict> {
    let file_map = scan_mods(mods);

    let mut list_of_conflicts: Vec<ModConflict> = Vec::new();

    for (file_path, mod_list) in file_map {
        if mod_list.len() > 1 {
            list_of_conflicts.push(ModConflict {
                file_path,
                mod_list,
            });
        }
    }

    dbg!(&list_of_conflicts);

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
}
