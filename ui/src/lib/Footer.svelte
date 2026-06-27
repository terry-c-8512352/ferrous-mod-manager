<script lang="ts">
  interface Props {
    total: number;
    enabled: number;
    conflicts: number;
    depWarnings: number;
    storage: string;
  }

  let { total, enabled, conflicts, depWarnings, storage }: Props = $props();
</script>

<div class="footer">
  <span><b>{total}</b> installed</span>
  <span><b>{enabled}</b> enabled</span>
  <span class="stat">
    <span class="dot" style="background:#df5648"></span>
    <b style="color:#c8432f">{conflicts}</b> conflicts
  </span>
  <span class="stat">
    <span class="dot" style="background:#d6890f"></span>
    <b style="color:#b06d09">{depWarnings}</b> dependency warnings
  </span>
  <span class="storage mono">{storage} on disk</span>
</div>

<style>
  .footer {
    flex: none;
    height: 42px;
    border-top: 1px solid var(--bd);
    background: var(--panel);
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    font-size: 12px;
    color: var(--muted);
  }

  .footer b {
    color: var(--ink);
  }

  .stat {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }

  .storage {
    margin-left: auto;
  }

  .mono {
    font-family: var(--mono);
  }
</style>
