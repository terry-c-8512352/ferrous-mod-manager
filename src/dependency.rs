use crate::models::{ModCollection, ModDescriptor, ModEntry};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// A dependency declared in a `.mod` descriptor that is not installed.
/// Paradox descriptors reference dependencies by display name, so `name` is
/// the missing mod's name and `required_by` the name of the mod declaring it.
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
pub struct MissingDependency {
    pub name: String,
    pub required_by: String,
}

/// Result of resolving a mod's transitive dependency closure.
#[derive(Debug, Serialize, Default)]
pub struct DependencyResolution {
    /// `mod_id`s in load order: every dependency precedes its dependents,
    /// the target mod comes last. Deduplicated (diamond dependencies appear once).
    pub chain: Vec<String>,
    pub missing: Vec<MissingDependency>,
}

/// What `enable_with_dependencies` did to the collection, for surfacing in the UI.
#[derive(Debug, Serialize, Default)]
pub struct DependencyReport {
    /// Display names of dependencies that were newly added to the collection
    /// or switched from disabled to enabled (the target mod itself excluded).
    pub activated_dependencies: Vec<String>,
    pub missing: Vec<MissingDependency>,
}

/// Resolve the full load chain for `target_id`: the target plus its transitive
/// dependencies, ordered so every dependency comes before the mods that need it.
/// Dependencies are matched against installed mods by name; unmatched names are
/// reported as missing. Cyclic declarations terminate rather than error — the
/// cycle members end up adjacent in declaration order.
pub fn resolve_load_chain(target_id: &str, installed: &[ModDescriptor]) -> DependencyResolution {
    let by_name: HashMap<&str, &ModDescriptor> = installed
        .iter()
        .filter_map(|m| m.name.as_deref().map(|n| (n, m)))
        .collect();

    let mut resolution = DependencyResolution::default();
    let Some(target) = installed.iter().find(|m| m.mod_id() == target_id) else {
        // Collections may hold entries for mods no longer installed; the chain
        // is just the target itself and nothing can be resolved for it.
        resolution.chain.push(target_id.to_string());
        return resolution;
    };

    let mut visited: HashSet<&str> = HashSet::new();
    let mut missing_seen: HashSet<&str> = HashSet::new();
    visit(
        target,
        &by_name,
        &mut visited,
        &mut missing_seen,
        &mut resolution,
    );
    resolution
}

/// Post-order DFS: pushing a mod after recursing into its dependencies yields
/// a topological order (dependencies first). `visited` is marked on entry so
/// diamonds resolve once and cycles terminate.
fn visit<'a>(
    desc: &'a ModDescriptor,
    by_name: &HashMap<&'a str, &'a ModDescriptor>,
    visited: &mut HashSet<&'a str>,
    missing_seen: &mut HashSet<&'a str>,
    resolution: &mut DependencyResolution,
) {
    if !visited.insert(desc.mod_id()) {
        return;
    }
    for dep_name in desc.dependencies.iter().flatten() {
        match by_name.get(dep_name.as_str()) {
            Some(dep) => visit(dep, by_name, visited, missing_seen, resolution),
            None => {
                if missing_seen.insert(dep_name) {
                    resolution.missing.push(MissingDependency {
                        name: dep_name.clone(),
                        required_by: desc
                            .name
                            .clone()
                            .unwrap_or_else(|| desc.mod_id().to_string()),
                    });
                }
            }
        }
    }
    resolution.chain.push(desc.mod_id().to_string());
}

/// Enable `target_id` in `collection` along with its transitive dependencies.
///
/// The target keeps its position (or is appended if new); each dependency is
/// enabled where it stands if already ordered before its dependent, and is
/// otherwise inserted or moved to sit just before it. Mods outside the chain
/// are never reordered.
pub fn enable_with_dependencies(
    collection: &mut ModCollection,
    target_id: &str,
    installed: &[ModDescriptor],
) -> DependencyReport {
    let resolution = resolve_load_chain(target_id, installed);
    let name_of = |id: &str| {
        installed
            .iter()
            .find(|m| m.mod_id() == id)
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| id.to_string())
    };

    let mut activated = Vec::new();

    // Everything in the chain must end up before this index; start at the target.
    let mut limit = match collection.mods.iter().position(|e| e.mod_id == target_id) {
        Some(idx) => {
            collection.mods[idx].enabled = true;
            idx
        }
        None => {
            collection.mods.push(ModEntry {
                mod_id: target_id.to_string(),
                enabled: true,
            });
            collection.mods.len() - 1
        }
    };

    // Walk dependencies nearest-first (reverse chain order, skipping the target,
    // which sits last), keeping each strictly before the one processed previously.
    for dep_id in resolution.chain.iter().rev().skip(1) {
        match collection.mods.iter().position(|e| e.mod_id == *dep_id) {
            Some(idx) if idx < limit => {
                if !collection.mods[idx].enabled {
                    collection.mods[idx].enabled = true;
                    activated.push(name_of(dep_id));
                }
                limit = idx;
            }
            Some(idx) => {
                // Ordered after its dependent — pull it up to just before.
                let mut entry = collection.mods.remove(idx);
                if !entry.enabled {
                    entry.enabled = true;
                    activated.push(name_of(dep_id));
                }
                collection.mods.insert(limit, entry);
            }
            None => {
                collection.mods.insert(
                    limit,
                    ModEntry {
                        mod_id: dep_id.to_string(),
                        enabled: true,
                    },
                );
                activated.push(name_of(dep_id));
            }
        }
    }

    DependencyReport {
        activated_dependencies: activated,
        missing: resolution.missing,
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
            supported_version: Some("v4.2.*".to_string()),
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

    fn ids(collection: &ModCollection) -> Vec<&str> {
        collection.mods.iter().map(|e| e.mod_id.as_str()).collect()
    }

    #[test]
    fn test_chain_orders_transitive_dependencies_first() {
        let installed = vec![
            descriptor("A", "1", &["B"]),
            descriptor("B", "2", &["C"]),
            descriptor("C", "3", &[]),
        ];
        let res = resolve_load_chain("1", &installed);
        assert_eq!(res.chain, vec!["3", "2", "1"]);
        assert!(res.missing.is_empty());
    }

    #[test]
    fn test_diamond_dependency_resolved_once() {
        let installed = vec![
            descriptor("A", "1", &["B", "C"]),
            descriptor("B", "2", &["D"]),
            descriptor("C", "3", &["D"]),
            descriptor("D", "4", &[]),
        ];
        let res = resolve_load_chain("1", &installed);
        assert_eq!(res.chain, vec!["4", "2", "3", "1"]);
    }

    #[test]
    fn test_cycle_terminates() {
        let installed = vec![descriptor("A", "1", &["B"]), descriptor("B", "2", &["A"])];
        let res = resolve_load_chain("1", &installed);
        assert_eq!(res.chain.len(), 2);
    }

    #[test]
    fn test_missing_dependency_reported_with_requirer() {
        let installed = vec![
            descriptor("A", "1", &["B"]),
            descriptor("B", "2", &["Ghost Mod"]),
        ];
        let res = resolve_load_chain("1", &installed);
        assert_eq!(
            res.missing,
            vec![MissingDependency {
                name: "Ghost Mod".to_string(),
                required_by: "B".to_string(),
            }]
        );
        assert_eq!(res.chain, vec!["2", "1"]);
    }

    #[test]
    fn test_uninstalled_target_yields_bare_chain() {
        let res = resolve_load_chain("999", &[]);
        assert_eq!(res.chain, vec!["999"]);
        assert!(res.missing.is_empty());
    }

    #[test]
    fn test_enable_inserts_missing_dependencies_before_target() {
        let installed = vec![
            descriptor("A", "1", &["B"]),
            descriptor("B", "2", &["C"]),
            descriptor("C", "3", &[]),
        ];
        let mut col = ModCollection::new("test");
        let report = enable_with_dependencies(&mut col, "1", &installed);
        assert_eq!(ids(&col), vec!["3", "2", "1"]);
        assert!(col.mods.iter().all(|e| e.enabled));
        assert_eq!(report.activated_dependencies, vec!["B", "C"]);
        assert!(report.missing.is_empty());
    }

    #[test]
    fn test_enable_moves_misordered_dependency_and_keeps_others_in_place() {
        let installed = vec![
            descriptor("A", "1", &["B"]),
            descriptor("B", "2", &[]),
            descriptor("X", "9", &[]),
        ];
        let mut col = ModCollection::new("test");
        col.add_mod("1".to_string());
        col.add_mod("9".to_string());
        col.add_mod("2".to_string());
        col.toggle_mod("1".to_string()); // disable target
        col.toggle_mod("2".to_string()); // disable dependency
        let report = enable_with_dependencies(&mut col, "1", &installed);
        // B pulled to just before A; unrelated X keeps its relative position.
        assert_eq!(ids(&col), vec!["2", "1", "9"]);
        assert!(col.mods[0].enabled && col.mods[1].enabled);
        assert_eq!(report.activated_dependencies, vec!["B"]);
    }

    #[test]
    fn test_enable_leaves_satisfied_order_untouched() {
        let installed = vec![descriptor("A", "1", &["B"]), descriptor("B", "2", &[])];
        let mut col = ModCollection::new("test");
        col.add_mod("2".to_string());
        col.add_mod("1".to_string());
        let report = enable_with_dependencies(&mut col, "1", &installed);
        assert_eq!(ids(&col), vec!["2", "1"]);
        assert!(report.activated_dependencies.is_empty());
    }

    #[test]
    fn test_enable_surfaces_missing_dependencies() {
        let installed = vec![descriptor("A", "1", &["Ghost Mod"])];
        let mut col = ModCollection::new("test");
        let report = enable_with_dependencies(&mut col, "1", &installed);
        assert_eq!(ids(&col), vec!["1"]);
        assert!(col.mods[0].enabled);
        assert_eq!(report.missing[0].name, "Ghost Mod");
        assert_eq!(report.missing[0].required_by, "A");
    }
}
