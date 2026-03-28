<script lang="ts">
    import type { ResolvedMod } from "./types";
    import ModRow from "./ModRow.svelte";

    interface Props {
        mods: ResolvedMod[];
        collectionModIdSet: Set<string>;
        ontogglecollection: (modId: string) => void;
        onaddall: () => void;
        onremoveall: () => void;
    }

    let {
        mods,
        collectionModIdSet,
        ontogglecollection,
        onaddall,
        onremoveall,
    }: Props = $props();

    let searchQuery = $state("");

    const filteredMods = $derived(
        (searchQuery.trim() === ""
            ? [...mods]
            : mods.filter(
                  (m) =>
                      (m.name ?? "")
                          .toLowerCase()
                          .includes(searchQuery.toLowerCase()) ||
                      m.mod_id
                          .toLowerCase()
                          .includes(searchQuery.toLowerCase()),
              )
        ).sort((a, b) =>
            (a.name ?? a.mod_id).localeCompare(b.name ?? b.mod_id),
        ),
    );
</script>

<div class="panel">
    <div class="panel-header">
        Installed Mods <span class="count">({mods.length})</span>
    </div>

    <div class="search-bar">
        <input
            type="search"
            placeholder="Search mods..."
            bind:value={searchQuery}
        />
        <button class="bar-btn" onclick={onaddall}>Add All</button>
        <button class="bar-btn" onclick={onremoveall}>Remove All</button>
    </div>

    <div class="column-headers">
        <span></span>
        <span>Name</span>
        <span class="right">Version</span>
        <span class="right">Source</span>
    </div>

    <div class="mod-list">
        {#each filteredMods as mod (mod.mod_id)}
            <ModRow
                {mod}
                enabled={collectionModIdSet.has(mod.mod_id)}
                variant="installed"
                inCollection={collectionModIdSet.has(mod.mod_id)}
                ontoggle={() => ontogglecollection(mod.mod_id)}
            />
        {/each}
        {#if filteredMods.length === 0}
            <div class="empty">No mods match your search.</div>
        {/if}
    </div>
</div>

<style>
    .panel {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border-right: 1px solid var(--border);
        min-width: 0;
    }

    .panel-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 4px 8px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.6px;
        border-bottom: 1px solid var(--border);
        background: var(--code-bg);
        color: var(--text);
        flex-shrink: 0;
    }

    .count {
        font-weight: 400;
        opacity: 0.6;
    }

    .search-bar {
        display: flex;
        align-items: center;
        gap: 4px;
        padding: 5px 8px;
        border-bottom: 1px solid var(--border);
        flex-shrink: 0;
    }

    .search-bar input {
        flex: 1;
        background: var(--bg);
        border: 1px solid var(--border);
        border-radius: 2px;
        color: var(--text-h);
        font-size: 12px;
        padding: 4px 8px;
    }

    .search-bar input:focus {
        outline: 2px solid var(--accent);
        outline-offset: 1px;
        border-color: var(--accent-border);
    }

    .bar-btn {
        flex-shrink: 0;
        background: var(--code-bg);
        border: 1px solid var(--border);
        border-radius: 2px;
        color: var(--text-h);
        cursor: pointer;
        font-size: 11px;
        font-family: var(--sans);
        padding: 3px 8px;
        white-space: nowrap;
    }

    .bar-btn:hover {
        background: var(--accent-bg);
        border-color: var(--accent-border);
        color: var(--accent);
    }

    .column-headers {
        display: grid;
        grid-template-columns: 20px 1fr 72px 72px;
        gap: 4px;
        padding: 3px 8px;
        font-size: 10px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.4px;
        color: var(--text);
        border-bottom: 1px solid var(--border);
        background: var(--code-bg);
        flex-shrink: 0;
    }

    .right {
        text-align: right;
    }

    .mod-list {
        overflow-y: auto;
        flex: 1;
    }

    .empty {
        padding: 16px;
        text-align: center;
        color: var(--text);
        font-size: 13px;
        opacity: 0.6;
    }
</style>
