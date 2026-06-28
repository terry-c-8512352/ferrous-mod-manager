<script lang="ts">
  type NavKey = 'all' | 'enabled' | 'conflicts';

  // A tag filter derived from the installed mods' Paradox tags. `color` is the
  // dot colour borrowed from the tag's display category bucket.
  export interface TagFilter {
    tag: string;
    count: number;
    color: string;
  }

  interface Props {
    counts: { all: number; enabled: number; issues: number };
    tags: TagFilter[];
    activeNav: NavKey;
    activeTag: string | null;
    onnav: (key: NavKey) => void;
    ontag: (tag: string) => void;
  }

  let { counts, tags, activeNav, activeTag, onnav, ontag }: Props = $props();

  const navItems: { key: NavKey; label: string; count: number }[] = $derived([
    { key: 'all', label: 'All Mods', count: counts.all },
    { key: 'enabled', label: 'Enabled', count: counts.enabled },
    { key: 'conflicts', label: 'Issues', count: counts.issues },
  ]);
</script>

<div class="sidebar">
  <div class="group">
    {#each navItems as item (item.key)}
      <button class="nav-row" class:active={activeNav === item.key} onclick={() => onnav(item.key)}>
        <span>{item.label}</span>
        <span class="count">{item.count}</span>
      </button>
    {/each}
  </div>

  <div class="group">
    <div class="heading">TAGS</div>
    {#if tags.length === 0}
      <div class="empty">No tags</div>
    {/if}
    {#each tags as t (t.tag)}
      <button
        class="cat-row"
        class:active={activeTag === t.tag}
        onclick={() => ontag(t.tag)}
      >
        <span class="cat-label">
          <span class="dot" style="background:{t.color}"></span>
          {t.tag}
        </span>
        <span class="count">{t.count}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .sidebar {
    width: 210px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--bd);
    background: var(--panel);
    padding: 13px 12px;
    gap: 15px;
    overflow: auto;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .heading {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--faint);
    padding: 2px 10px 5px;
  }

  .empty {
    font-size: 12px;
    color: var(--faint);
    padding: 4px 10px;
  }

  .nav-row,
  .cat-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border: none;
    border-radius: 7px;
    cursor: pointer;
    font-family: inherit;
    font-size: 13px;
    font-weight: 500;
    background: transparent;
    color: var(--ink);
    text-align: left;
  }

  .nav-row {
    padding: 8px 10px;
  }

  .cat-row {
    padding: 7px 10px;
  }

  .nav-row:hover,
  .cat-row:hover {
    background: var(--bd2);
  }

  .nav-row.active,
  .cat-row.active {
    background: var(--acc-weak);
    color: var(--acc-ink);
    font-weight: 600;
  }

  .cat-label {
    display: flex;
    align-items: center;
    gap: 9px;
    text-transform: capitalize;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex: none;
  }

  .nav-row .count {
    font-size: 12px;
    font-weight: 600;
    color: var(--faint);
  }

  .nav-row.active .count {
    color: var(--acc-ink);
  }

  .cat-row .count {
    font-size: 12px;
    color: var(--faint);
  }
</style>
