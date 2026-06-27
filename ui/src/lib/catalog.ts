// Display-layer derivations for the three-column manager. Ported 1:1 from the
// canonical design mockup ("Mod Manager.dc.html") and the merged Iced redesign
// (`app-ui/src/main.rs` `category_of`/`cat_meta`/status classification), adapted
// to the data the Tauri backend actually exposes.

import type { ModConflict, ResolvedMod } from './types';

// ---------------------------------------------------------------------------
// Categories (bucketed from Paradox `tags`)
// ---------------------------------------------------------------------------

export type CategoryKey =
  | 'interface'
  | 'gameplay'
  | 'graphics'
  | 'utility'
  | 'audio'
  | 'other';

export const CATEGORY_ORDER: CategoryKey[] = [
  'interface',
  'gameplay',
  'graphics',
  'utility',
  'audio',
];

export interface CatMeta {
  label: string;
  color: string;
  bg: string;
}

const CAT_META: Record<CategoryKey, CatMeta> = {
  interface: { label: 'Interface', color: '#2f7fd1', bg: '#e8f1fb' },
  gameplay: { label: 'Gameplay', color: '#1f9d6b', bg: '#e6f5ee' },
  graphics: { label: 'Graphics', color: '#7a5bd0', bg: '#efeafb' },
  utility: { label: 'Utility', color: '#5f6b7a', bg: '#eceff3' },
  audio: { label: 'Audio', color: '#c2871a', bg: '#fbf2e0' },
  other: { label: 'Other', color: '#6c7480', bg: '#eef0f3' },
};

export function catMeta(key: CategoryKey): CatMeta {
  return CAT_META[key];
}

/** Bucket a mod into a display category from its Paradox tags. First match wins. */
export function categoryOf(tags: string[] | undefined): CategoryKey {
  if (!tags) return 'other';
  for (const t of tags) {
    const tl = t.toLowerCase();
    if (tl.includes('interface') || tl.includes('tooltip') || tl === 'ui') {
      return 'interface';
    }
    if (
      tl.includes('graphic') ||
      tl.includes('portrait') ||
      tl.includes('shipset') ||
      tl.includes('visual') ||
      tl.includes('namelist')
    ) {
      return 'graphics';
    }
    if (tl.includes('sound') || tl.includes('music') || tl.includes('audio')) {
      return 'audio';
    }
    if (
      tl.includes('utilit') ||
      tl.includes('fix') ||
      tl.includes('performance') ||
      tl.includes('quality') ||
      tl.includes('cheat')
    ) {
      return 'utility';
    }
    if (
      tl.includes('gameplay') ||
      tl.includes('balance') ||
      tl.includes('overhaul') ||
      tl.includes('event') ||
      tl.includes('econom') ||
      tl.includes('military') ||
      tl.includes('species') ||
      tl.includes('galaxy') ||
      tl.includes('diplomac') ||
      tl.includes('technolog') ||
      tl.includes('origin') ||
      tl.includes('building') ||
      tl.includes('conversion')
    ) {
      return 'gameplay';
    }
  }
  return 'other';
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

/** Human-readable byte size, e.g. "4.3 MB"; "—" for zero. Mirrors `human_size`. */
export function humanSize(bytes: number): string {
  const KB = 1024;
  if (bytes === 0) return '—';
  if (bytes < KB) return `${bytes} B`;
  if (bytes < KB * KB) return `${Math.round(bytes / KB)} KB`;
  if (bytes < KB * KB * KB) {
    const mb = bytes / (KB * KB);
    return mb < 10 ? `${mb.toFixed(1)} MB` : `${Math.round(mb)} MB`;
  }
  return `${(bytes / (KB * KB * KB)).toFixed(1)} GB`;
}

/** 1-2 uppercase initials from a mod name for its thumbnail. */
export function initials(name: string): string {
  const inits = name
    .split(/\s+/)
    .filter((w) => /[a-z0-9]/i.test(w[0] ?? ''))
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase();
  return inits || 'M';
}

// ---------------------------------------------------------------------------
// Per-mod status classification
// ---------------------------------------------------------------------------

export type ModStatus = 'ok' | 'off' | 'conflict' | 'dep';

export interface StatusMeta {
  color: string;
  bg: string;
}

const STATUS_META: Record<ModStatus, StatusMeta> = {
  ok: { color: '#1f9d6b', bg: '#e6f5ee' },
  off: { color: '#9aa2ad', bg: '#eef0f3' },
  conflict: { color: '#df5648', bg: '#fbe9e7' },
  dep: { color: '#d6890f', bg: '#fbf1de' },
};

export function statusMeta(status: ModStatus): StatusMeta {
  return STATUS_META[status];
}

export interface DecoratedMod {
  mod_id: string;
  name: string;
  version: string; // "v1.2" or ""
  category: CategoryKey;
  catLabel: string;
  catColor: string;
  catBg: string;
  initials: string;
  sizeBytes: number;
  sizeLabel: string;
  inCollection: boolean;
  enabled: boolean;
  loadNo: string; // "01", or "" when not enabled
  status: ModStatus;
  statusText: string;
  statusColor: string;
  statusBg: string;
  hasIssue: boolean; // conflict or dependency problem
  issueText: string;
}

/** Lookups shared across every mod in a single decoration pass. */
export interface DecorateContext {
  /** mod_id -> on-disk size in bytes. */
  sizes: Map<string, number>;
  /** mod_id -> { inCollection, enabled, loadNo } for the active collection. */
  collectionState: Map<
    string,
    { inCollection: boolean; enabled: boolean; loadNo: string }
  >;
  /** mod name -> the installed mod with that name (dependency resolution). */
  installedByName: Map<string, ResolvedMod>;
  /** names of mods that are enabled in the active collection. */
  enabledNames: Set<string>;
  /** mod name -> conflicts it participates in (already filtered to enabled mods). */
  conflictsByName: Map<string, ModConflict[]>;
}

export function decorateMod(mod: ResolvedMod, ctx: DecorateContext): DecoratedMod {
  const name = mod.name ?? mod.mod_id;
  const cat = categoryOf(mod.tags);
  const meta = CAT_META[cat];
  const cState = ctx.collectionState.get(mod.mod_id) ?? {
    inCollection: false,
    enabled: false,
    loadNo: '',
  };
  const enabled = cState.enabled;

  // Conflicts: only meaningful for enabled mods (backend conflict detection is
  // run against the enabled set, so any hit here is a live collision).
  let conflictActive = false;
  let issueText = '';
  if (enabled) {
    const cs = ctx.conflictsByName.get(name);
    if (cs && cs.length > 0) {
      conflictActive = true;
      const c = cs[0];
      const other = c.mod_list.find((n) => n !== name) ?? '(unknown)';
      issueText = `Conflicts with ${other} — ${c.file_path}`;
    }
  }

  // Dependencies: Paradox `.mod` dependencies reference other mods by display
  // name. Missing = not installed; disabled = installed but not enabled.
  let depKind: '' | 'missing' | 'disabled' = '';
  for (const dep of mod.dependencies ?? []) {
    const o = ctx.installedByName.get(dep);
    if (!o) {
      depKind = 'missing';
      if (!conflictActive) issueText = `Requires ${dep} — not installed`;
    } else if (!ctx.enabledNames.has(dep)) {
      if (!depKind) depKind = 'disabled';
      if (!conflictActive) issueText = `Requires ${dep} — currently disabled`;
    }
  }

  let status: ModStatus = enabled ? 'ok' : 'off';
  let statusText = enabled ? 'Enabled' : 'Disabled';
  if (conflictActive) {
    status = 'conflict';
    statusText = 'Conflict';
  } else if (depKind === 'missing') {
    status = 'dep';
    statusText = 'Missing dep';
  } else if (depKind === 'disabled') {
    status = 'dep';
    statusText = 'Needs dep';
  }

  const sMeta = STATUS_META[status];
  const sizeBytes = ctx.sizes.get(mod.mod_id) ?? 0;

  return {
    mod_id: mod.mod_id,
    name,
    version: mod.version ? `v${mod.version}` : '',
    category: cat,
    catLabel: meta.label,
    catColor: meta.color,
    catBg: meta.bg,
    initials: initials(name),
    sizeBytes,
    sizeLabel: humanSize(sizeBytes),
    inCollection: cState.inCollection,
    enabled,
    loadNo: cState.loadNo,
    status,
    statusText,
    statusColor: sMeta.color,
    statusBg: sMeta.bg,
    hasIssue: status === 'conflict' || status === 'dep',
    issueText,
  };
}
