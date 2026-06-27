<script lang="ts">
  import { CATEGORY_ORDER, catMeta, type CategoryKey } from './catalog';

  type NavKey = 'all' | 'enabled' | 'conflicts';

  interface Props {
    counts: { all: number; enabled: number; issues: number };
    categoryCounts: Record<CategoryKey, number>;
    activeNav: NavKey;
    activeCategory: CategoryKey | null;
    onnav: (key: NavKey) => void;
    oncategory: (key: CategoryKey) => void;
  }

  let { counts, categoryCounts, activeNav, activeCategory, onnav, oncategory }: Props = $props();

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
    <div class="heading">CATEGORIES</div>
    {#each CATEGORY_ORDER as key (key)}
      {@const meta = catMeta(key)}
      <button
        class="cat-row"
        class:active={activeCategory === key}
        onclick={() => oncategory(key)}
      >
        <span class="cat-label">
          <span class="dot" style="background:{meta.color}"></span>
          {meta.label}
        </span>
        <span class="count">{categoryCounts[key] ?? 0}</span>
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
