use crate::locations::ModRoots;
use crate::models::{AchievementStatus, ConflictCategory, ModDescriptor};
use std::collections::BTreeSet;
use std::path::Path;
use walkdir::WalkDir;

/// Determine whether a single mod keeps achievements (and ironman saves) enabled.
///
/// Paradox games disable achievements whenever an active mod changes the gameplay
/// checksum. Mods that only touch cosmetic content — graphics, sound, and
/// localisation text — leave the checksum untouched and stay achievement-safe.
/// Anything touching game data, defines, events, the map, or an unrecognised path
/// is treated as achievement-disabling. The returned `gameplay_categories` are the
/// distinct offending categories, so callers can explain *why* achievements break.
pub fn achievement_status(game_mod: &ModDescriptor, roots: &ModRoots) -> AchievementStatus {
    let mod_id = game_mod.mod_id().to_string();

    // Descriptor paths are untrusted Workshop content; refuse to walk anything
    // outside the known mod directories. Without inspectable files we assume
    // the mod affects gameplay so we never falsely promise achievements.
    let Some(path) = game_mod.path.as_deref().and_then(|p| roots.checked_path(p)) else {
        return AchievementStatus {
            mod_id,
            compatible: false,
            gameplay_categories: Vec::new(),
        };
    };

    let mut gameplay_categories: BTreeSet<ConflictCategory> = BTreeSet::new();
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
            let category = ConflictCategory::from_path(relative);
            if !category.is_achievement_safe() {
                gameplay_categories.insert(category);
            }
        }
    }

    AchievementStatus {
        mod_id,
        compatible: gameplay_categories.is_empty(),
        gameplay_categories: gameplay_categories.into_iter().collect(),
    }
}

/// Classify a batch of mods, preserving input order.
pub fn achievement_status_for_mods(
    mods: &[ModDescriptor],
    roots: &ModRoots,
) -> Vec<AchievementStatus> {
    mods.iter().map(|m| achievement_status(m, roots)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path(rel: &str) -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(rel);
        path.to_string_lossy().into_owned()
    }

    fn fixture_roots() -> ModRoots {
        ModRoots::from_roots([
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/achievements")
        ])
    }

    fn make_mod(name: &str, path: Option<String>) -> ModDescriptor {
        ModDescriptor {
            name: Some(name.to_string()),
            path,
            remote_file_id: None,
            supported_version: None,
            tags: None,
            picture: None,
            version: None,
            dependencies: None,
        }
    }

    #[test]
    fn test_cosmetic_only_mod_keeps_achievements() {
        let m = make_mod(
            "cosmetic",
            Some(fixture_path("tests/fixtures/achievements/cosmetic_mod")),
        );
        let status = achievement_status(&m, &fixture_roots());
        assert!(
            status.compatible,
            "a mod touching only gfx/sound/localisation should keep achievements"
        );
        assert!(status.gameplay_categories.is_empty());
    }

    #[test]
    fn test_gameplay_mod_disables_achievements() {
        // gameplay_mod mixes a cosmetic gfx file with a common/traits file; the
        // single gameplay file must be enough to disable achievements.
        let m = make_mod(
            "gameplay",
            Some(fixture_path("tests/fixtures/achievements/gameplay_mod")),
        );
        let status = achievement_status(&m, &fixture_roots());
        assert!(!status.compatible);
        assert_eq!(status.gameplay_categories, vec![ConflictCategory::GameData]);
    }

    #[test]
    fn test_descriptor_mod_is_ignored() {
        // The cosmetic fixture also contains a descriptor.mod at its root, which
        // would categorise as `Other`; it must be skipped so the mod stays safe.
        let m = make_mod(
            "cosmetic",
            Some(fixture_path("tests/fixtures/achievements/cosmetic_mod")),
        );
        assert!(achievement_status(&m, &fixture_roots()).compatible);
    }

    #[test]
    fn test_mod_without_path_assumed_incompatible() {
        let m = make_mod("no_path", None);
        let status = achievement_status(&m, &fixture_roots());
        assert!(!status.compatible);
        assert!(status.gameplay_categories.is_empty());
    }

    #[test]
    fn test_mod_with_disallowed_path_assumed_incompatible() {
        // A path outside the allowed mod roots must not be walked; the mod is
        // treated like one we can't inspect.
        let m = make_mod("evil", Some("/etc".to_string()));
        let status = achievement_status(&m, &fixture_roots());
        assert!(!status.compatible);
        assert!(status.gameplay_categories.is_empty());
    }

    #[test]
    fn test_mod_id_is_carried_through() {
        let m = make_mod(
            "gameplay",
            Some(fixture_path("tests/fixtures/achievements/gameplay_mod")),
        );
        let status = achievement_status(&m, &fixture_roots());
        assert_eq!(status.mod_id, m.mod_id());
    }

    #[test]
    fn test_status_for_mods_preserves_order_and_count() {
        let mods = vec![
            make_mod(
                "gameplay",
                Some(fixture_path("tests/fixtures/achievements/gameplay_mod")),
            ),
            make_mod(
                "cosmetic",
                Some(fixture_path("tests/fixtures/achievements/cosmetic_mod")),
            ),
        ];
        let statuses = achievement_status_for_mods(&mods, &fixture_roots());
        assert_eq!(statuses.len(), 2);
        assert!(!statuses[0].compatible);
        assert!(statuses[1].compatible);
    }
}
