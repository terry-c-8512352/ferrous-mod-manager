<script lang="ts">
  import type { ModCollection } from './types';

  interface Props {
    collections: ModCollection[];
    activeName: string;
    search: string;
    onsearch: (value: string) => void;
    onselect: (name: string) => void;
    oncollections: () => void;
    onplay: () => void;
  }

  let { collections, activeName, search, onsearch, onselect, oncollections, onplay }: Props =
    $props();
</script>

<div class="toolbar">
  <span class="label">PLAYSET</span>
  {#each collections as col (col.id)}
    <button
      class="chip"
      class:active={col.name === activeName}
      onclick={() => onselect(col.name)}
    >
      {col.name}
    </button>
  {/each}

  <span class="spacer"></span>

  <div class="search">
    <span class="search-icon">⌕</span>
    <input
      value={search}
      oninput={(e) => onsearch((e.target as HTMLInputElement).value)}
      placeholder="Search mods…"
    />
  </div>

  <button class="btn" onclick={oncollections}>▦&nbsp; Collections</button>
  <button class="btn play" onclick={onplay}>▶&nbsp; Play</button>
</div>

<style>
  .toolbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--bd);
    background: var(--surface);
  }

  .label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--faint);
    margin-right: 2px;
  }

  .chip {
    padding: 6px 13px;
    border-radius: 8px;
    border: 1px solid var(--bd);
    background: var(--surface);
    color: var(--muted);
    font-family: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .chip.active {
    border-color: var(--acc);
    background: var(--acc-weak);
    color: var(--acc-ink);
  }

  .spacer {
    flex: 1;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 228px;
    padding: 7px 11px;
    border: 1px solid var(--bd);
    border-radius: 8px;
    background: var(--panel);
  }

  .search-icon {
    color: var(--faint);
  }

  .search input {
    border: none;
    outline: none;
    background: transparent;
    font-family: inherit;
    font-size: 13px;
    width: 100%;
    color: var(--ink);
  }

  .btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 15px;
    border-radius: 8px;
    border: 1px solid var(--bd);
    background: var(--surface);
    color: var(--ink);
    font-family: inherit;
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:hover {
    border-color: var(--acc);
    color: var(--acc-ink);
  }

  .btn.play {
    padding: 8px 20px;
    border-color: var(--acc);
    background: var(--acc);
    color: #fff;
    box-shadow: 0 1px 2px rgba(27, 140, 229, 0.45);
  }
</style>
