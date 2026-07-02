use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::dependency::resolve_load_chain;
use crate::models::{ConflictCategory, ModConflict, ModDescriptor};

/// How many mods must share one file before the overlap is treated as a
/// commonly-replaced vanilla file rather than a meaningful pairwise conflict.
pub const WIDESPREAD_THRESHOLD: usize = 5;

/// Name fragments that signal a mod is designed to override others even when
/// it declares no dependencies (most Workshop patches don't).
const PATCH_HINTS: &[&str] = &["patch", "compat", "fix"];

/// What a file-path overlap between enabled mods actually means for the game.
///
/// Paradox games resolve a shared file wholesale in favour of the mod that
/// loads last, and patch/addon mods rely on exactly that — so an overlap is
/// only a problem when the override is unintentional or points the wrong way.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum ConflictVerdict {
    /// A mod loads before another mod it declares a (transitive) dependency
    /// on: its override points the wrong way. Fixable by reordering.
    OrderingProblem,
    /// The winning mod is designed to override the others (declared
    /// dependency or patch-style name). Working as intended.
    IntendedOverride,
    /// Only cosmetic content (localisation, gfx, sound) — worst case is a
    /// visual or text oddity.
    Cosmetic,
    /// A file so commonly replaced that overlap carries little signal.
    Widespread,
    /// Unrelated mods changing the same gameplay file: the real conflict.
    Genuine,
}

/// A file-overlap conflict enriched with load-order semantics.
#[derive(Debug, Serialize)]
pub struct AssessedConflict {
    pub file_path: PathBuf,
    /// The conflicting mods' names, sorted into load order.
    pub mod_list: Vec<String>,
    pub category: ConflictCategory,
    pub verdict: ConflictVerdict,
    /// The mod whose copy of the file the game will actually use (loads last).
    pub winner: String,
    /// For `OrderingProblem`: the mod that must move later in the load order.
    pub culprit: Option<String>,
    /// Human-readable explanation of the verdict, for tooltips.
    pub reason: String,
}

/// Classify raw file-overlap conflicts against the enabled mods **in load
/// order**. Mods are matched by display name throughout — that is how both
/// `ModConflict::mod_list` and descriptor `dependencies` identify mods.
pub fn assess_conflicts(
    conflicts: Vec<ModConflict>,
    mods_in_load_order: &[ModDescriptor],
) -> Vec<AssessedConflict> {
    let order: HashMap<&str, usize> = mods_in_load_order
        .iter()
        .enumerate()
        .filter_map(|(i, m)| m.name.as_deref().map(|n| (n, i)))
        .collect();

    // name -> names of the mods it transitively depends on. Chains resolve to
    // mod_ids, so map them back through each descriptor's name.
    let id_to_name: HashMap<&str, &str> = mods_in_load_order
        .iter()
        .filter_map(|m| m.name.as_deref().map(|n| (m.mod_id(), n)))
        .collect();
    let depends_on: HashMap<&str, HashSet<&str>> = mods_in_load_order
        .iter()
        .filter_map(|m| {
            let name = m.name.as_deref()?;
            let deps = resolve_load_chain(m.mod_id(), mods_in_load_order)
                .chain
                .iter()
                .filter(|id| id.as_str() != m.mod_id())
                .filter_map(|id| id_to_name.get(id.as_str()).copied())
                .collect();
            Some((name, deps))
        })
        .collect();

    conflicts
        .into_iter()
        .map(|c| assess_one(c, &order, &depends_on))
        .collect()
}

fn assess_one(
    conflict: ModConflict,
    order: &HashMap<&str, usize>,
    depends_on: &HashMap<&str, HashSet<&str>>,
) -> AssessedConflict {
    let ModConflict {
        file_path,
        mut mod_list,
        category,
    } = conflict;
    mod_list.sort_by_key(|n| order.get(n.as_str()).copied().unwrap_or(usize::MAX));
    let winner = mod_list.last().cloned().unwrap_or_default();

    let deps_of = |name: &str| depends_on.get(name);

    // 1. Ordering problem: an earlier mod depends on (is meant to override) a
    // later one. The earlier mod is the culprit that must move down.
    for (i, early) in mod_list.iter().enumerate() {
        for late in &mod_list[i + 1..] {
            if deps_of(early).is_some_and(|deps| deps.contains(late.as_str())) {
                return AssessedConflict {
                    file_path,
                    reason: format!(
                        "{early} loads before {late}, which it overrides — move it below {late}"
                    ),
                    culprit: Some(early.clone()),
                    mod_list,
                    category,
                    verdict: ConflictVerdict::OrderingProblem,
                    winner,
                };
            }
        }
    }

    // 2. Intended override: the winner is designed to replace every loser.
    let winner_lower = winner.to_lowercase();
    let winner_is_patch = PATCH_HINTS.iter().any(|h| winner_lower.contains(h));
    let losers = &mod_list[..mod_list.len().saturating_sub(1)];
    let all_intended = !losers.is_empty()
        && losers.iter().all(|loser| {
            winner_is_patch
                || deps_of(&winner).is_some_and(|deps| deps.contains(loser.as_str()))
                || winner_lower.contains(&loser.to_lowercase())
        });
    if all_intended {
        return AssessedConflict {
            file_path,
            reason: format!("{winner} is designed to override {}", losers.join(", ")),
            culprit: None,
            mod_list,
            category,
            verdict: ConflictVerdict::IntendedOverride,
            winner,
        };
    }

    // 3. Cosmetic content can't break a game — the same buckets that keep
    // achievements enabled.
    if category.is_achievement_safe() {
        return AssessedConflict {
            file_path,
            reason: format!("Cosmetic file — {winner}'s version is used"),
            culprit: None,
            mod_list,
            category,
            verdict: ConflictVerdict::Cosmetic,
            winner,
        };
    }

    // 4. A file half the load order replaces carries little pairwise signal.
    if mod_list.len() >= WIDESPREAD_THRESHOLD {
        return AssessedConflict {
            file_path,
            reason: format!(
                "Commonly replaced file — {} mods change it, {winner} wins",
                mod_list.len()
            ),
            culprit: None,
            mod_list,
            category,
            verdict: ConflictVerdict::Widespread,
            winner,
        };
    }

    // 5. Unrelated mods changing the same gameplay file.
    AssessedConflict {
        file_path,
        reason: format!("Unrelated mods change the same gameplay file — {winner} wins"),
        culprit: None,
        mod_list,
        category,
        verdict: ConflictVerdict::Genuine,
        winner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(name: &str, id: &str, deps: &[&str]) -> ModDescriptor {
        ModDescriptor {
            name: Some(name.to_string()),
            path: Some(format!("/mods/{id}")),
            remote_file_id: Some(id.to_string()),
            supported_version: None,
            tags: None,
            picture: None,
            version: None,
            dependencies: if deps.is_empty() {
                None
            } else {
                Some(deps.iter().map(|d| d.to_string()).collect())
            },
        }
    }

    fn conflict(path: &str, mods: &[&str]) -> ModConflict {
        let file_path = PathBuf::from(path);
        ModConflict {
            category: ConflictCategory::from_path(&file_path),
            file_path,
            mod_list: mods.iter().map(|m| m.to_string()).collect(),
        }
    }

    fn assess(conflict: ModConflict, mods: &[ModDescriptor]) -> AssessedConflict {
        assess_conflicts(vec![conflict], mods)
            .pop()
            .expect("one conflict in, one assessment out")
    }

    #[test]
    fn test_declared_dependency_is_intended_override() {
        let mods = vec![
            descriptor("Alpha", "1", &[]),
            descriptor("Zeta", "2", &["Alpha"]),
        ];
        let a = assess(conflict("common/things/a.txt", &["Alpha", "Zeta"]), &mods);
        assert_eq!(a.verdict, ConflictVerdict::IntendedOverride);
        assert_eq!(a.winner, "Zeta");
    }

    #[test]
    fn test_transitive_dependency_is_intended_override() {
        let mods = vec![
            descriptor("Alpha", "1", &[]),
            descriptor("Middle", "2", &["Alpha"]),
            descriptor("Zeta", "3", &["Middle"]),
        ];
        let a = assess(conflict("common/things/a.txt", &["Alpha", "Zeta"]), &mods);
        assert_eq!(a.verdict, ConflictVerdict::IntendedOverride);
    }

    #[test]
    fn test_patch_hint_name_is_intended_override() {
        let mods = vec![
            descriptor("Warfare Expanded", "1", &[]),
            descriptor("Combat Compatibility Patch", "2", &[]),
        ];
        let a = assess(
            conflict(
                "common/things/a.txt",
                &["Warfare Expanded", "Combat Compatibility Patch"],
            ),
            &mods,
        );
        assert_eq!(a.verdict, ConflictVerdict::IntendedOverride);
    }

    #[test]
    fn test_name_containment_is_intended_override() {
        let mods = vec![
            descriptor("UI Overhaul", "1", &[]),
            descriptor("UI Overhaul + Tiny Outliner", "2", &[]),
        ];
        let a = assess(
            conflict(
                "gfx/interface/outliner.gui",
                &["UI Overhaul", "UI Overhaul + Tiny Outliner"],
            ),
            &mods,
        );
        assert_eq!(a.verdict, ConflictVerdict::IntendedOverride);
        assert_eq!(a.winner, "UI Overhaul + Tiny Outliner");
    }

    #[test]
    fn test_dependent_loading_first_is_ordering_problem() {
        let mods = vec![
            descriptor("Zeta", "2", &["Alpha"]),
            descriptor("Alpha", "1", &[]),
        ];
        let a = assess(conflict("common/things/a.txt", &["Alpha", "Zeta"]), &mods);
        assert_eq!(a.verdict, ConflictVerdict::OrderingProblem);
        assert_eq!(a.culprit.as_deref(), Some("Zeta"));
        assert!(a.reason.contains("move it below Alpha"));
    }

    #[test]
    fn test_localisation_overlap_is_cosmetic() {
        let mods = vec![
            descriptor("Warfare Expanded", "1", &[]),
            descriptor("Galactic Trade", "2", &[]),
        ];
        let a = assess(
            conflict(
                "localisation/english/a_l_english.yml",
                &["Warfare Expanded", "Galactic Trade"],
            ),
            &mods,
        );
        assert_eq!(a.verdict, ConflictVerdict::Cosmetic);
    }

    #[test]
    fn test_widely_shared_file_is_widespread() {
        let names = ["Aaa", "Bbb", "Ccc", "Ddd", "Eee"];
        let mods: Vec<ModDescriptor> = names
            .iter()
            .enumerate()
            .map(|(i, n)| descriptor(n, &i.to_string(), &[]))
            .collect();
        let a = assess(conflict("common/things/a.txt", &names), &mods);
        assert_eq!(a.verdict, ConflictVerdict::Widespread);
    }

    #[test]
    fn test_unrelated_gameplay_overlap_is_genuine() {
        let mods = vec![
            descriptor("Warfare Expanded", "1", &[]),
            descriptor("Galactic Trade", "2", &[]),
        ];
        let a = assess(
            conflict(
                "common/things/a.txt",
                &["Galactic Trade", "Warfare Expanded"],
            ),
            &mods,
        );
        assert_eq!(a.verdict, ConflictVerdict::Genuine);
        // mod_list re-sorted into load order; winner loads last.
        assert_eq!(a.mod_list, vec!["Warfare Expanded", "Galactic Trade"]);
        assert_eq!(a.winner, "Galactic Trade");
    }

    #[test]
    fn test_ordering_problem_outranks_patch_name() {
        // A patch that loads before its base is still broken, however it's named.
        let mods = vec![
            descriptor("Combat Fix", "2", &["Warfare Expanded"]),
            descriptor("Warfare Expanded", "1", &[]),
        ];
        let a = assess(
            conflict("common/things/a.txt", &["Warfare Expanded", "Combat Fix"]),
            &mods,
        );
        assert_eq!(a.verdict, ConflictVerdict::OrderingProblem);
        assert_eq!(a.culprit.as_deref(), Some("Combat Fix"));
    }

    #[test]
    fn test_verdict_serializes_as_plain_string() {
        let json = serde_json::to_string(&ConflictVerdict::IntendedOverride).unwrap();
        assert_eq!(json, "\"IntendedOverride\"");
    }
}
