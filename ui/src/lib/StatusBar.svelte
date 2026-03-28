<script lang="ts">
  interface Props {
    totalMods: number;
    enabledMods: number;
    significantConflicts: number;
    minorConflicts: number;
    activeGameName: string;
  }

  let { totalMods, enabledMods, significantConflicts, minorConflicts, activeGameName }: Props = $props();
</script>

<div class="status-bar">
  <span class="game-name">{activeGameName}</span>
  <span class="sep">·</span>
  <span>{totalMods} mods installed</span>
  <span class="sep">·</span>
  <span>{enabledMods} enabled</span>
  {#if significantConflicts > 0}
    <span class="sep">·</span>
    <span class="conflicts-significant">{significantConflicts} significant conflict{significantConflicts === 1 ? '' : 's'}</span>
  {/if}
  {#if minorConflicts > 0}
    <span class="sep">·</span>
    <span class="conflicts-minor">{minorConflicts} minor overlap{minorConflicts === 1 ? '' : 's'}</span>
  {/if}
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text);
    background: var(--code-bg);
    flex-shrink: 0;
  }

  .game-name {
    font-weight: 600;
    color: var(--text-h);
  }

  .sep {
    opacity: 0.4;
  }

  .conflicts-significant {
    color: var(--danger);
    font-weight: 500;
  }

  .conflicts-minor {
    color: var(--text);
    opacity: 0.7;
  }
</style>
