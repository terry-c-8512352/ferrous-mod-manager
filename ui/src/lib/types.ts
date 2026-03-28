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

export type ConflictCategory =
  | 'Defines'
  | 'GameData'
  | 'Localisation'
  | 'Events'
  | 'Gfx'
  | 'Map'
  | 'Sound'
  | 'Other';

export interface ModConflict {
  file_path: string;
  mod_list: string[];
  category: ConflictCategory;
}

export type ConflictSeverity = 'none' | 'low' | 'medium' | 'high';

const HIGH_CATEGORIES: Set<ConflictCategory> = new Set(['GameData', 'Events', 'Map']);
const MEDIUM_CATEGORIES: Set<ConflictCategory> = new Set(['Defines', 'Other']);
const HOTNESS_THRESHOLD = 5;

export function conflictSeverityForFile(conflict: ModConflict): ConflictSeverity {
  const isHot = conflict.mod_list.length >= HOTNESS_THRESHOLD;
  if (isHot) return 'low';
  if (HIGH_CATEGORIES.has(conflict.category)) return 'high';
  if (MEDIUM_CATEGORIES.has(conflict.category)) return 'medium';
  return 'low';
}

export interface ResolvedMod extends ModDescriptor {
  mod_id: string;
  source: 'workshop' | 'local';
}

export function resolveModId(mod: ModDescriptor): string {
  return mod.remote_file_id ?? mod.path ?? '';
}
