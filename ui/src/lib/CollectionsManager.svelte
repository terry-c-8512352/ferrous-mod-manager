<script lang="ts">
  import type { ModCollection } from './types';

  interface Props {
    collections: ModCollection[];
    onback: () => void;
    oncreate: (name: string) => void;
    ondelete: (name: string) => void;
    ondeleteall: (names: string[]) => void;
    onrename: (oldName: string, newName: string) => void;
    onselect: (name: string) => void;
    onimport: (imported: ModCollection[]) => void;
  }

  let { collections, onback, oncreate, ondelete, ondeleteall, onrename, onselect, onimport }: Props = $props();

  let editingName = $state<string | null>(null);
  let editValue = $state('');
  let selected = $state<Set<string>>(new Set());
  let creatingNew = $state(false);
  let newName = $state('');

  function focusOnMount(el: HTMLElement) {
    el.focus();
  }

  const allSelectable = $derived(collections.length > 1 ? collections : []);
  const allChecked = $derived(allSelectable.length > 0 && allSelectable.every(c => selected.has(c.name)));
  const someChecked = $derived(selected.size > 0);

  function toggleSelect(name: string) {
    const next = new Set(selected);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    selected = next;
  }

  function toggleAll() {
    if (allChecked) {
      selected = new Set();
    } else {
      selected = new Set(allSelectable.map(c => c.name));
    }
  }

  function deleteSelected() {
    const names = [...selected];
    selected = new Set();
    ondeleteall(names);
  }

  function startRename(name: string) {
    editingName = name;
    editValue = name;
  }

  function commitRename(oldName: string) {
    const trimmed = editValue.trim();
    if (trimmed && trimmed !== oldName && !collections.some(c => c.name === trimmed)) {
      onrename(oldName, trimmed);
    }
    editingName = null;
  }

  function cancelRename() {
    editingName = null;
  }

  function handleKeydown(e: KeyboardEvent, oldName: string) {
    if (e.key === 'Enter') commitRename(oldName);
    if (e.key === 'Escape') cancelRename();
  }

  function startCreate() {
    creatingNew = true;
    newName = '';
  }

  function commitCreate() {
    const trimmed = newName.trim();
    if (trimmed && !collections.some(c => c.name === trimmed)) {
      oncreate(trimmed);
    }
    creatingNew = false;
    newName = '';
  }

  function cancelCreate() {
    creatingNew = false;
    newName = '';
  }

  function handleCreateKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') commitCreate();
    if (e.key === 'Escape') cancelCreate();
  }

  function handleSelect(name: string) {
    onselect(name);
    onback();
  }

  function exportCollections() {
    const data = JSON.stringify(collections, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'collections.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  let fileInput: HTMLInputElement;

  function triggerImport() {
    fileInput.click();
  }

  async function handleImportFile(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const parsed: ModCollection[] = JSON.parse(text);
      if (!Array.isArray(parsed)) throw new Error('Expected an array');
      onimport(parsed);
    } catch {
      alert('Invalid collections file.');
    }
    (e.target as HTMLInputElement).value = '';
  }
</script>

<div class="manager">
  <div class="manager-header">
    <h2>Mod Collections</h2>
    {#if someChecked}
      <button class="btn-delete-sel" onclick={deleteSelected}>
        Delete Selected ({selected.size})
      </button>
    {/if}
    <button class="btn-action" onclick={triggerImport}>Import</button>
    <button class="btn-action" onclick={exportCollections}>Export</button>
    <button class="btn-create" onclick={startCreate} disabled={creatingNew}>+ New Collection</button>
  </div>
  <input
    bind:this={fileInput}
    type="file"
    accept=".json"
    style="display:none"
    onchange={handleImportFile}
  />

  <div class="collection-list">
    {#if collections.length > 1}
      <div class="collection-row header-row">
        <input
          type="checkbox"
          class="row-check"
          checked={allChecked}
          onchange={toggleAll}
          title="Select all"
        />
        <span class="select-all-label">Select all</span>
      </div>
    {/if}

    {#each collections as col (col.name)}
      <div class="collection-row" class:row-selected={selected.has(col.name)}>
        <input
          type="checkbox"
          class="row-check"
          checked={selected.has(col.name)}
          onchange={() => toggleSelect(col.name)}
          disabled={collections.length <= 1}
        />
        <div class="col-info">
          {#if editingName === col.name}
            <input
              class="rename-input"
              bind:value={editValue}
              onblur={() => commitRename(col.name)}
              onkeydown={(e) => handleKeydown(e, col.name)}
              use:focusOnMount
            />
          {:else}
            <button class="col-name" onclick={() => handleSelect(col.name)}>
              {col.name}
            </button>
          {/if}
          <span class="col-count">{col.mods.length} mod{col.mods.length === 1 ? '' : 's'}</span>
        </div>

        <div class="col-actions">
          <button class="action-btn" onclick={() => startRename(col.name)} title="Rename">
            ✎
          </button>
          <button
            class="action-btn action-delete"
            onclick={() => ondelete(col.name)}
            title="Delete"
            disabled={collections.length <= 1}
          >
            ✕
          </button>
        </div>
      </div>
    {/each}

    {#if creatingNew}
      <div class="collection-row">
        <input
          class="rename-input"
          placeholder="Collection name..."
          bind:value={newName}
          onblur={commitCreate}
          onkeydown={handleCreateKeydown}
          use:focusOnMount
        />
        <button class="action-btn" onclick={cancelCreate} title="Cancel">✕</button>
      </div>
    {/if}

    {#if collections.length === 0 && !creatingNew}
      <div class="empty">No collections yet. Create one to get started.</div>
    {/if}
  </div>
</div>

<style>
  .manager {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .manager-header {
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

  .btn-action {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text-h);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
  }

  .btn-action:hover {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .btn-create {
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    border-radius: 2px;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
  }

  .btn-create:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-delete-sel {
    background: var(--tertiary-bg);
    border: 1px solid var(--tertiary);
    border-radius: 2px;
    color: var(--tertiary);
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
  }

  .btn-delete-sel:hover {
    background: var(--tertiary);
    color: white;
  }

  .collection-list {
    overflow-y: auto;
    flex: 1;
    padding: 8px 0;
  }

  .collection-row {
    display: flex;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    gap: 12px;
  }

  .collection-row:hover {
    background: var(--accent-bg);
  }

  .row-selected {
    background: var(--accent-bg);
  }

  .header-row {
    padding: 6px 16px;
    background: var(--code-bg);
    font-size: 11px;
    color: var(--text);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .header-row:hover {
    background: var(--code-bg);
  }

  .select-all-label {
    font-size: 11px;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .row-check {
    flex-shrink: 0;
    cursor: pointer;
    accent-color: var(--accent);
  }

  .row-check:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .col-info {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }

  .col-name {
    background: none;
    border: none;
    color: var(--text-h);
    cursor: pointer;
    font-size: 14px;
    font-weight: 500;
    font-family: var(--sans);
    padding: 0;
    text-align: left;
  }

  .col-name:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .col-count {
    font-size: 11px;
    color: var(--text);
    opacity: 0.7;
  }

  .rename-input {
    background: var(--bg);
    border: 1px solid var(--accent-border);
    border-radius: 2px;
    color: var(--text-h);
    font-size: 14px;
    font-family: var(--sans);
    font-weight: 500;
    padding: 2px 6px;
    outline: none;
    min-width: 200px;
  }

  .col-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .action-btn {
    background: none;
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
    padding: 3px 8px;
    line-height: 1;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--accent-bg);
    border-color: var(--accent-border);
    color: var(--accent);
  }

  .action-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .action-delete:hover:not(:disabled) {
    background: var(--tertiary-bg);
    border-color: var(--tertiary);
    color: var(--tertiary);
  }

  .empty {
    padding: 32px;
    text-align: center;
    color: var(--text);
    opacity: 0.6;
    font-size: 13px;
  }
</style>
