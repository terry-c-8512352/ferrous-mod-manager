export interface DetectedGame {
  app_id: number;
  install_path: string;
  game_name: string;
  paradox_data_path: string;
}

export interface ModDescriptor {
  name?: string;
  path?: string;
  remote_file_id?: string;
  supported_version?: string;
  tags?: string[];
  picture?: string;
  version?: string;
  dependencies?: string[];
}

export interface ModEntry {
  mod_id: string;
  enabled: boolean;
}

export interface ModCollection {
  id: string;
  name: string;
  mods: ModEntry[];
}

export interface MissingDependency {
  name: string;
  required_by: string;
}

export interface DependencyReport {
  activated_dependencies: string[];
  missing: MissingDependency[];
}

export interface EnableModOutcome {
  collection: ModCollection;
  report: DependencyReport;
}

export interface DisableModOutcome {
  collection: ModCollection;
  disabled_dependents: string[];
}

export interface UserSettings {
  auto_dependencies: boolean;
}

export function defaultSettings(): UserSettings {
  return { auto_dependencies: true };
}

export type ConflictCategory =
  | 'Defines'
  | 'GameData'
  | 'Localisation'
  | 'Events'
  | 'Gfx'
  | 'Map'
  | 'Sound'
  | 'Other';

// What a file overlap between enabled mods actually means. Mirrors
// `ConflictVerdict`/`AssessedConflict` in src/conflict_assessment.rs: only
// OrderingProblem and Genuine are real issues; the rest are informational.
export type ConflictVerdict =
  | 'OrderingProblem'
  | 'IntendedOverride'
  | 'Cosmetic'
  | 'Widespread'
  | 'Genuine';

export interface AssessedConflict {
  file_path: string;
  mod_list: string[]; // in load order
  category: ConflictCategory;
  verdict: ConflictVerdict;
  winner: string; // the mod whose copy of the file the game uses
  culprit: string | null; // OrderingProblem: the mod that must move later
  reason: string;
}

export interface AchievementStatus {
  mod_id: string;
  compatible: boolean;
  gameplay_categories: ConflictCategory[];
}

const GAMEPLAY_CATEGORY_LABELS: Record<ConflictCategory, string> = {
  GameData: 'game data',
  Events: 'events',
  Map: 'map',
  Defines: 'defines',
  Other: 'other files',
  Localisation: 'localisation',
  Gfx: 'gfx',
  Sound: 'sound',
};

// Human-readable explanation of a mod's achievement/ironman status, used for
// the per-row indicator tooltip. Undefined status (still loading) reads as safe.
export function achievementSummary(status: AchievementStatus | undefined): string {
  if (!status || status.compatible) {
    return 'Achievement-compatible — cosmetic only (gfx, sound, text)';
  }
  const cats = status.gameplay_categories.map((c) => GAMEPLAY_CATEGORY_LABELS[c] ?? c);
  const detail = cats.length ? ` — affects ${cats.join(', ')}` : '';
  return `Disables achievements & ironman${detail}`;
}

export interface ResolvedMod extends ModDescriptor {
  mod_id: string;
  source: 'workshop' | 'local';
}

export function resolveModId(mod: ModDescriptor): string {
  return mod.remote_file_id ?? mod.path ?? '';
}
