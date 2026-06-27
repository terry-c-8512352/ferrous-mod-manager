<script lang="ts">
  import type { DetectedGame } from './types';

  interface Props {
    games: DetectedGame[];
    selectedGameId: number;
    onselect: (gameId: number) => void;
  }

  let { games, selectedGameId, onselect }: Props = $props();
</script>

<div class="game-selector">
  <label for="game-select">Game</label>
  <select
    id="game-select"
    value={selectedGameId}
    onchange={(e) => onselect(Number((e.target as HTMLSelectElement).value))}
  >
    {#each games as game}
      <option value={game.app_id}>{game.game_name}</option>
    {/each}
  </select>
</div>

<style>
  .game-selector {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    white-space: nowrap;
  }

  select {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text-h);
    font-size: 13px;
    padding: 2px 6px;
    cursor: pointer;
    min-width: 180px;
  }

  select:focus {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
