<script lang="ts">
  import TrophyIcon from './TrophyIcon.svelte';
  import type { DecoratedMod } from './catalog';

  interface Props {
    mods: DecoratedMod[];
    total: number;
    width: number;
    ontoggle: (modId: string) => void;
  }

  let { mods, total, width, ontoggle }: Props = $props();
</script>

<div class="col" style="width:{width}px">
  <div class="col-header">
    <span class="title">INSTALLED</span>
    <span class="sub">{total} mods</span>
  </div>
  <div class="list">
    {#each mods as mod (mod.mod_id)}
      <div class="row">
        <div class="meta">
          <div class="name">{mod.name}</div>
          <div class="sub-line">{mod.catLabel} · <span class="mono">{mod.sizeLabel}</span></div>
        </div>
        {#if mod.achievementSafe}
          <span class="ach" title={mod.achievementText}>
            <TrophyIcon size={12} title={mod.achievementText} />
          </span>
        {/if}
        {#if mod.hasIssue}
          <span class="issue-dot" style="background:{mod.statusColor}" title={mod.issueText}></span>
        {/if}
        <button
          class="check"
          class:on={mod.inCollection}
          onclick={() => ontoggle(mod.mod_id)}
          title={mod.inCollection ? 'Remove from collection' : 'Add to collection'}
          aria-label={mod.inCollection ? 'Remove from collection' : 'Add to collection'}
        >
          {#if mod.inCollection}✓{/if}
        </button>
      </div>
    {/each}
    {#if mods.length === 0}
      <div class="empty">No mods match.</div>
    {/if}
  </div>
</div>

<style>
  .col {
    flex: none;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-right: 1px solid var(--bd);
    background: var(--surface);
  }

  .col-header {
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 7px;
    padding: 11px 14px 9px;
    border-bottom: 1px solid var(--bd2);
  }

  .title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--muted);
  }

  .sub {
    font-size: 11px;
    color: var(--faint);
  }

  .list {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--bd2);
  }

  .row:hover {
    background: var(--panel);
  }

  .meta {
    min-width: 0;
    flex: 1;
  }

  .name {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub-line {
    font-size: 10.5px;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mono {
    font-family: var(--mono);
  }

  .ach {
    display: flex;
    align-items: center;
    color: var(--trophy);
    flex: none;
  }

  .issue-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }

  .check {
    position: relative;
    width: 18px;
    height: 18px;
    flex: none;
    cursor: pointer;
    border-radius: 5px;
    border: 1.5px solid #c4cad2;
    background: var(--surface);
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .check.on {
    border-color: var(--acc);
    background: var(--acc);
  }

  .empty {
    padding: 24px 14px;
    text-align: center;
    color: var(--faint);
    font-size: 12px;
  }
</style>
