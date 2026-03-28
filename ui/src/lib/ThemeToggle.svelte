<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  type Theme = 'light' | 'dark' | 'system';

  const STORAGE_KEY = 'ferrous-theme';

  function getSystemPreference(): 'light' | 'dark' {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function applyTheme(theme: Theme) {
    const root = document.documentElement;
    root.classList.remove('light', 'dark');
    if (theme === 'light') root.classList.add('light');
    if (theme === 'dark') root.classList.add('dark');
  }

  // Tell GTK/WebKit to use the matching window decoration theme.
  // Wrapped in try/catch so it silently does nothing in browser dev mode.
  async function syncWindowTheme(theme: Theme) {
    try {
      const win = getCurrentWindow();
      await win.setTheme(theme === 'system' ? null : theme);
    } catch {
      // Not running inside Tauri (e.g. `npm run dev` in browser)
    }
  }

  function loadSaved(): Theme {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'light' || saved === 'dark' || saved === 'system') return saved;
    return 'system';
  }

  let theme = $state<Theme>(loadSaved());
  $effect(() => {
    applyTheme(theme);
    syncWindowTheme(theme);
  });

  const effectiveTheme = $derived(theme === 'system' ? getSystemPreference() : theme);

  const icon = $derived(effectiveTheme === 'dark' ? '☀' : '☾');
  const label = $derived(
    theme === 'system'
      ? `System (${effectiveTheme})`
      : theme === 'dark' ? 'Dark' : 'Light'
  );

  function toggle() {
    const next: Theme =
      theme === 'system' ? (getSystemPreference() === 'dark' ? 'light' : 'dark')
      : theme === 'dark' ? 'light'
      : 'dark';
    theme = next;
    localStorage.setItem(STORAGE_KEY, next);
  }
</script>

<button class="theme-toggle" onclick={toggle} title="Toggle theme: {label}">
  <span class="icon">{icon}</span>
  <span class="text">{label}</span>
</button>

<style>
  .theme-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text-h);
    cursor: pointer;
    font-size: 12px;
    padding: 4px 8px;
    white-space: nowrap;
  }

  .theme-toggle:hover {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .icon {
    font-size: 13px;
    line-height: 1;
  }
</style>
