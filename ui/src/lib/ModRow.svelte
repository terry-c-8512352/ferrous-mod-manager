<script lang="ts">
  import type { ResolvedMod, ConflictSeverity } from './types';

  interface Props {
    mod: ResolvedMod;
    enabled: boolean;
    position?: number;
    variant: 'installed' | 'collection';
    conflictSeverity?: ConflictSeverity;
    inCollection?: boolean;
    ontoggle: () => void;
    onmoveup?: () => void;
    onmovedown?: () => void;
    onremove?: () => void;
    onmovetoposition?: (newPos: number) => void;
  }

  let { mod, enabled, position, variant, conflictSeverity = 'none', inCollection = false, ontoggle, onmoveup, onmovedown, onremove, onmovetoposition }: Props = $props();

  let posValue = $state('');
  // Keep input in sync when position prop changes (drag/reorder/initial render)
  $effect(() => { posValue = String(position ?? ''); });

  function handlePosKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      (e.target as HTMLInputElement).blur();
    } else if (e.key === 'Escape') {
      posValue = String(position ?? '');
      (e.target as HTMLInputElement).blur();
    }
  }

  function handlePosBlur() {
    const n = parseInt(posValue, 10);
    if (!isNaN(n) && n !== position && onmovetoposition) {
      onmovetoposition(n);
    } else {
      posValue = String(position ?? '');
    }
  }
</script>

<div
  class="mod-row {variant}"
  class:conflict-high={conflictSeverity === 'high'}
  class:conflict-medium={conflictSeverity === 'medium'}
  class:conflict-low={conflictSeverity === 'low'}
  class:disabled={!enabled}
  class:in-collection={inCollection && variant === 'installed'}
>
  {#if variant === 'collection'}
    <input
      class="position-input"
      type="number"
      min="1"
      bind:value={posValue}
      onkeydown={handlePosKeydown}
      onblur={handlePosBlur}
      onfocus={(e) => (e.target as HTMLInputElement).select()}
      title="Type a number to jump to that position"
    />
  {/if}

  <input type="checkbox" checked={enabled} onchange={ontoggle} />

  <span class="mod-name" title={mod.name}>
    {mod.name ?? mod.mod_id}
    {#if conflictSeverity === 'high'}
      <span class="conflict-icon high" title="This mod has significant file conflicts">⚠</span>
    {:else if conflictSeverity === 'low'}
      <span class="conflict-icon low" title="This mod has minor file overlaps">·</span>
    {/if}
  </span>

  <span class="version">{mod.version ?? '—'}</span>

  {#if variant === 'installed'}
    <span class="source {mod.source}">{mod.source === 'workshop' ? 'Workshop' : 'Local'}</span>
  {/if}

  {#if variant === 'collection'}
    <span class="actions">
      <button class="icon-btn" onclick={onmoveup} title="Move up" disabled={!onmoveup}>↑</button>
      <button class="icon-btn" onclick={onmovedown} title="Move down" disabled={!onmovedown}>↓</button>
      <button class="icon-btn remove" onclick={onremove} title="Remove from collection">✕</button>
    </span>
  {/if}
</div>

<style>
  .mod-row {
    display: grid;
    align-items: center;
    padding: 2px 8px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-h);
    gap: 4px;
  }

  .mod-row.installed {
    grid-template-columns: 20px 1fr 72px 72px;
  }

  .mod-row.collection {
    grid-template-columns: 24px 20px 1fr 72px 68px;
  }

  .mod-row:hover {
    background: var(--accent-bg);
  }

  .mod-row.conflict-high {
    border-left: 2px solid var(--danger);
    padding-left: 6px;
  }

  .mod-row.conflict-medium {
    border-left: 2px solid var(--tertiary);
    padding-left: 6px;
  }

  .mod-row.disabled {
    opacity: 0.45;
  }

  .mod-row.in-collection {
    background: color-mix(in srgb, var(--accent-bg) 60%, transparent);
  }

  .position-input {
    width: 100%;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    color: var(--text);
    font-size: 11px;
    font-family: var(--mono);
    text-align: right;
    padding: 1px 2px;
    box-sizing: border-box;
    /* hide browser spinner arrows */
    appearance: textfield;
    -moz-appearance: textfield;
  }

  .position-input::-webkit-inner-spin-button,
  .position-input::-webkit-outer-spin-button {
    -webkit-appearance: none;
  }

  .position-input:hover {
    border-color: var(--border);
  }

  .position-input:focus {
    outline: none;
    border-color: var(--accent-border);
    background: var(--bg);
    color: var(--text-h);
  }

  input[type='checkbox'] {
    accent-color: var(--accent);
    cursor: pointer;
    width: 14px;
    height: 14px;
  }

  .mod-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .conflict-icon {
    margin-left: 4px;
    font-size: 11px;
  }

  .conflict-icon.high {
    color: var(--danger);
  }

  .conflict-icon.low {
    color: var(--text);
    opacity: 0.5;
    font-size: 14px;
    font-weight: 700;
  }

  .version {
    font-size: 11px;
    color: var(--text);
    text-align: right;
  }

  .source {
    font-size: 11px;
    color: var(--text);
    text-align: right;
  }

  .source.local {
    color: var(--accent);
  }

  .actions {
    display: flex;
    gap: 2px;
    justify-content: flex-end;
  }

  .icon-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text);
    cursor: pointer;
    font-size: 11px;
    padding: 1px 4px;
    line-height: 1.4;
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .icon-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .icon-btn.remove:hover:not(:disabled) {
    background: var(--tertiary-bg);
    border-color: var(--tertiary);
    color: var(--tertiary);
  }
</style>
