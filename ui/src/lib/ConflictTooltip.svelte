<script lang="ts">
  import type { ConflictCategory } from './types';

  interface ConflictGroup {
    other_mod_name: string;
    other_position: number;
    this_position: number;
    this_wins: boolean;
    file_count: number;
    categories: Map<ConflictCategory, number>;
    files: string[];
  }

  interface Props {
    modName: string;
    conflicts: ConflictGroup[];
    anchor: DOMRect;
    onmouseenter?: () => void;
    onmouseleave?: () => void;
  }

  let { modName, conflicts, anchor, onmouseenter, onmouseleave }: Props = $props();

  const TOOLTIP_WIDTH = 340;
  const MARGIN = 8;

  const left = $derived(Math.max(MARGIN, anchor.left - TOOLTIP_WIDTH - MARGIN));
  const top = $derived(anchor.top);

  let expandedMod = $state<string | null>(null);

  function toggleExpand(name: string) {
    expandedMod = expandedMod === name ? null : name;
  }

  const CATEGORY_LABELS: Record<ConflictCategory, string> = {
    GameData: 'game data',
    Events: 'events',
    Map: 'map',
    Defines: 'defines',
    Localisation: 'localisation',
    Gfx: 'gfx',
    Sound: 'sound',
    Other: 'other',
  };

  const CATEGORY_SEVERITY: Record<ConflictCategory, 'high' | 'medium' | 'low'> = {
    GameData: 'high',
    Events: 'high',
    Map: 'high',
    Defines: 'medium',
    Other: 'medium',
    Localisation: 'low',
    Gfx: 'low',
    Sound: 'low',
  };

  function sortedCategories(cats: Map<ConflictCategory, number>): [ConflictCategory, number][] {
    const order: ConflictCategory[] = ['GameData', 'Events', 'Map', 'Defines', 'Other', 'Localisation', 'Gfx', 'Sound'];
    return [...cats.entries()].sort((a, b) => order.indexOf(a[0]) - order.indexOf(b[0]));
  }

  function summaryLabel(cats: Map<ConflictCategory, number>): string | null {
    const total = [...cats.values()].reduce((a, b) => a + b, 0);
    const lowCount = (cats.get('Localisation') ?? 0) + (cats.get('Gfx') ?? 0) + (cats.get('Sound') ?? 0);
    if (lowCount > total * 0.7 && total > 2) return 'Mostly cosmetic (likely benign)';
    const definesCount = cats.get('Defines') ?? 0;
    if (definesCount > total * 0.7 && total > 2) return 'Mostly defines (usually intentional)';
    return null;
  }
</script>

<div
  class="tooltip"
  style="left: {left}px; top: {top}px; width: {TOOLTIP_WIDTH}px;"
  role="tooltip"
  {onmouseenter}
  {onmouseleave}
>
  <div class="tooltip-title">{modName}</div>

  {#if conflicts.length === 0}
    <div class="no-conflicts">No active conflicts in this collection.</div>
  {:else}
    <div class="conflict-list">
      {#each conflicts as c}
        <div class="conflict-item">
          <div class="conflict-header">
            <span class="mod-name" title={c.other_mod_name}>{c.other_mod_name}</span>
            <button class="expand-btn" onclick={() => toggleExpand(c.other_mod_name)}
              title={expandedMod === c.other_mod_name ? 'Hide files' : 'Show files'}
            >{expandedMod === c.other_mod_name ? '▾' : '▸'}</button>
          </div>

          <div class="category-badges">
            {#each sortedCategories(c.categories) as [cat, count]}
              <span class="badge sev-{CATEGORY_SEVERITY[cat]}">{count} {CATEGORY_LABELS[cat]}</span>
            {/each}
          </div>

          <div class="conflict-meta">
            <span class="file-count">{c.file_count} file{c.file_count === 1 ? '' : 's'}</span>
            <span class="verdict" class:wins={c.this_wins} class:loses={!c.this_wins}>
              {#if c.this_wins}
                ✓ You win <span class="dim">(pos. {c.this_position} &gt; {c.other_position})</span>
              {:else}
                ✗ They win <span class="dim">(pos. {c.other_position} &gt; {c.this_position})</span>
              {/if}
            </span>
          </div>

          {#if summaryLabel(c.categories)}
            <div class="summary-hint">{summaryLabel(c.categories)}</div>
          {/if}

          {#if expandedMod === c.other_mod_name}
            <div class="file-list">
              {#each c.files.toSorted() as file}
                <div class="file-path">{file}</div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <div class="tooltip-footer">Higher position loads last and takes priority</div>
  {/if}
</div>

<style>
  .tooltip {
    position: fixed;
    z-index: 1000;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    box-shadow: var(--shadow);
    font-size: 12px;
    pointer-events: auto;
    max-height: 420px;
    overflow-y: auto;
  }

  .tooltip-title {
    padding: 8px 10px 6px;
    font-weight: 600;
    font-size: 12px;
    color: var(--text-h);
    border-bottom: 1px solid var(--border);
    background: var(--code-bg);
    border-radius: 2px 4px 0 0;
  }

  .no-conflicts {
    padding: 10px;
    color: var(--text);
    opacity: 0.7;
    font-style: italic;
  }

  .conflict-list {
    padding: 6px 0;
  }

  .conflict-item {
    padding: 5px 10px;
    border-bottom: 1px solid var(--border);
  }

  .conflict-item:last-child {
    border-bottom: none;
  }

  .conflict-header {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .mod-name {
    font-weight: 600;
    color: var(--text-h);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .expand-btn {
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 10px;
    padding: 0 2px;
    opacity: 0.6;
    flex-shrink: 0;
  }

  .expand-btn:hover {
    opacity: 1;
    color: var(--accent);
  }

  .category-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
    margin: 3px 0;
  }

  .badge {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 2px;
    white-space: nowrap;
  }

  .badge.sev-high {
    background: var(--danger-bg);
    color: var(--danger);
  }

  .badge.sev-medium {
    background: var(--tertiary-bg);
    color: var(--tertiary);
  }

  .badge.sev-low {
    background: var(--code-bg);
    color: var(--text);
    opacity: 0.7;
  }

  .conflict-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .file-count {
    color: var(--text);
    opacity: 0.7;
    white-space: nowrap;
  }

  .verdict {
    flex: 1;
  }

  .verdict.wins {
    color: var(--accent);
  }

  .verdict.loses {
    color: var(--tertiary);
  }

  .dim {
    opacity: 0.65;
    font-weight: 400;
  }

  .summary-hint {
    font-size: 10px;
    color: var(--text);
    opacity: 0.6;
    font-style: italic;
    margin-top: 2px;
  }

  .file-list {
    margin-top: 4px;
    padding: 4px 0;
    border-top: 1px solid var(--border);
    max-height: 150px;
    overflow-y: auto;
  }

  .file-path {
    font-size: 10px;
    font-family: var(--mono);
    color: var(--text);
    padding: 1px 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tooltip-footer {
    padding: 5px 10px;
    font-size: 10px;
    color: var(--text);
    opacity: 0.55;
    border-top: 1px solid var(--border);
    background: var(--code-bg);
    border-radius: 0 0 4px 4px;
    font-style: italic;
  }
</style>
