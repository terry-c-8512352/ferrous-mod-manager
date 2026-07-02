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

/// Disable `target_id` in `collection` together with every enabled mod that
/// transitively depends on it (matched by name, the same way descriptors
/// declare dependencies). Returns the display names of the dependents that
/// were switched off — the target itself excluded.
///
/// The cascade propagates through already-disabled dependents, so a mod whose
/// requirement chain reaches the target still switches off even when an
/// intermediate link was disabled beforehand.
pub fn disable_with_dependents(
    collection: &mut ModCollection,
    target_id: &str,
    installed: &[ModDescriptor],
) -> Vec<String> {
    if let Some(entry) = collection.mods.iter_mut().find(|e| e.mod_id == target_id) {
        entry.enabled = false;
    }

    // Dependents reference the target by display name; an uninstalled or
    // unnamed target can't be depended on, so there is nothing to cascade.
    let Some(target_name) = installed
        .iter()
        .find(|m| m.mod_id() == target_id)
        .and_then(|m| m.name.as_deref())
    else {
        return Vec::new();
    };

    // Reverse edges: dependency name -> mods that declare it.
    let mut dependents_of: HashMap<&str, Vec<&ModDescriptor>> = HashMap::new();
    for m in installed {
        for dep_name in m.dependencies.iter().flatten() {
            dependents_of.entry(dep_name.as_str()).or_default().push(m);
        }
    }

    let mut deactivated = Vec::new();
    let mut visited: HashSet<&str> = HashSet::from([target_name]);
    let mut queue = vec![target_name];
    while let Some(name) = queue.pop() {
        for dependent in dependents_of.get(name).into_iter().flatten() {
            let Some(dependent_name) = dependent.name.as_deref() else {
                continue;
            };
            if !visited.insert(dependent_name) {
                continue;
            }
            if let Some(entry) = collection
                .mods
                .iter_mut()
                .find(|e| e.mod_id == dependent.mod_id() && e.enabled)
            {
                entry.enabled = false;
                deactivated.push(dependent_name.to_string());
            }
            queue.push(dependent_name);
        }
    }
    deactivated
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
        col.toggle_mod("1"); // disable target
        col.toggle_mod("2"); // disable dependency
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

    /// Collection where every listed mod is added and enabled.
    fn enabled_collection(ids: &[&str]) -> ModCollection {
        let mut col = ModCollection::new("test");
        for id in ids {
            col.add_mod(id.to_string());
        }
        col
    }

    fn enabled_ids(collection: &ModCollection) -> Vec<&str> {
        collection
            .mods
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.mod_id.as_str())
            .collect()
    }

    #[test]
    fn test_disable_cascades_to_transitive_dependents() {
        // C depends on B depends on A; X is unrelated.
        let installed = vec![
            descriptor("A", "1", &[]),
            descriptor("B", "2", &["A"]),
            descriptor("C", "3", &["B"]),
            descriptor("X", "9", &[]),
        ];
        let mut col = enabled_collection(&["1", "2", "3", "9"]);
        let mut disabled = disable_with_dependents(&mut col, "1", &installed);
        disabled.sort();
        assert_eq!(disabled, vec!["B", "C"]);
        assert_eq!(enabled_ids(&col), vec!["9"]);
    }

    #[test]
    fn test_disable_cascades_through_already_disabled_dependent() {
        let installed = vec![
            descriptor("A", "1", &[]),
            descriptor("B", "2", &["A"]),
            descriptor("C", "3", &["B"]),
        ];
        let mut col = enabled_collection(&["1", "2", "3"]);
        col.toggle_mod("2"); // B already off
        let disabled = disable_with_dependents(&mut col, "1", &installed);
        // C still requires A via B, so it switches off; B is only reported once off.
        assert_eq!(disabled, vec!["C"]);
        assert!(enabled_ids(&col).is_empty());
    }

    #[test]
    fn test_disable_leaves_non_dependents_enabled() {
        let installed = vec![
            descriptor("A", "1", &[]),
            descriptor("B", "2", &["A"]),
            descriptor("X", "9", &[]),
        ];
        let mut col = enabled_collection(&["1", "2", "9"]);
        let disabled = disable_with_dependents(&mut col, "2", &installed);
        // Disabling the dependent must not touch the dependency or bystanders.
        assert!(disabled.is_empty());
        assert_eq!(enabled_ids(&col), vec!["1", "9"]);
    }

    #[test]
    fn test_disable_cycle_terminates_and_disables_both() {
        let installed = vec![descriptor("A", "1", &["B"]), descriptor("B", "2", &["A"])];
        let mut col = enabled_collection(&["1", "2"]);
        let disabled = disable_with_dependents(&mut col, "1", &installed);
        assert_eq!(disabled, vec!["B"]);
        assert!(enabled_ids(&col).is_empty());
    }

    #[test]
    fn test_disable_uninstalled_target_only_disables_its_entry() {
        let installed = vec![descriptor("A", "1", &[])];
        let mut col = enabled_collection(&["999", "1"]);
        let disabled = disable_with_dependents(&mut col, "999", &installed);
        assert!(disabled.is_empty());
        assert_eq!(enabled_ids(&col), vec!["1"]);
    }
}
