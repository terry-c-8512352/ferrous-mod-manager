<script lang="ts">
  interface Props {
    onapply: () => void;
    onlaunch: () => void;
    onmanage: () => void;
  }

  let { onapply, onlaunch, onmanage }: Props = $props();
</script>

<div class="collection-selector">
  <button class="btn" onclick={onmanage}>Collections ▾</button>

  <div class="divider"></div>


  <button class="btn btn-primary" onclick={onapply}>Apply</button>
  <button class="btn btn-primary" onclick={onlaunch}>Launch Game</button>
</div>

<style>
  .collection-selector {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
  }

  .divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 2px;
  }

  .btn {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text-h);
    cursor: pointer;
    font-size: 12px;
    font-family: var(--sans);
    padding: 4px 10px;
    white-space: nowrap;
  }

  .btn:hover {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .btn-primary {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
    font-weight: 500;
  }

  .btn-primary:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
