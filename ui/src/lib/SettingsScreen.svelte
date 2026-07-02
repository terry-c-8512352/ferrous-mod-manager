<script lang="ts">
  import type { UserSettings } from './types';

  interface Props {
    settings: UserSettings;
    onback: () => void;
    onchange: (settings: UserSettings) => void;
  }

  let { settings, onback, onchange }: Props = $props();
</script>

<div class="settings">
  <div class="settings-header">
    <button class="btn-back" onclick={onback}>←&nbsp; Back to mods</button>
    <h2>Settings</h2>
  </div>

  <div class="setting-list">
    <label class="setting-row">
      <input
        type="checkbox"
        class="setting-check"
        checked={settings.auto_dependencies}
        onchange={(e) =>
          onchange({ ...settings, auto_dependencies: e.currentTarget.checked })}
      />
      <div class="setting-info">
        <div class="setting-name">Handle dependencies automatically</div>
        <div class="setting-desc">
          Enabling a mod also enables the mods it depends on, and disabling a mod
          also disables the mods that depend on it. Turn this off to toggle mods
          strictly one at a time (missing-dependency warnings still show).
        </div>
      </div>
    </label>
  </div>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .settings-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--code-bg);
    flex-shrink: 0;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-h);
    flex: 1;
  }

  .btn-back {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text-h);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
    white-space: nowrap;
  }

  .btn-back:hover {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .setting-list {
    overflow-y: auto;
    flex: 1;
    padding: 8px 0;
  }

  .setting-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--bd2);
    cursor: pointer;
  }

  .setting-row:hover {
    background: var(--panel);
  }

  .setting-check {
    margin-top: 2px;
    accent-color: var(--acc);
    flex: none;
  }

  .setting-info {
    min-width: 0;
  }

  .setting-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
  }

  .setting-desc {
    margin-top: 2px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--muted);
    max-width: 560px;
  }
</style>
