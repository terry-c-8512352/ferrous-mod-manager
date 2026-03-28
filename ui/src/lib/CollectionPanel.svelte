<script lang="ts">
    import type { ModCollection, ResolvedMod, ModConflict, ConflictSeverity, ConflictCategory } from "./types";
    import { conflictSeverityForFile } from "./types";
    import ModRow from "./ModRow.svelte";
    import ConflictTooltip from "./ConflictTooltip.svelte";

    interface Props {
        collection: ModCollection;
        collections: ModCollection[];
        resolvedModMap: Map<string, ResolvedMod>;
        conflicts: ModConflict[];
        ontoggle: (modId: string) => void;
        onmove: (from: number, to: number) => void;
        onremove: (modId: string) => void;
        onselect: (name: string) => void;
        oncreate: (name: string) => void;
        ondelete: () => void;
        onenableall: () => void;
        ondisableall: () => void;
    }

    let {
        collection,
        collections,
        resolvedModMap,
        conflicts,
        ontoggle,
        onmove,
        onremove,
        onselect,
        oncreate,
        ondelete,
        onenableall,
        ondisableall,
    }: Props = $props();

    let searchQuery = $state("");
    let hoveredModId = $state<string | null>(null);
    let tooltipAnchor = $state<DOMRect | null>(null);
    let creatingNew = $state(false);
    let newName = $state("");

    function focusOnMount(el: HTMLElement) {
        el.focus();
    }

    function startCreate() {
        creatingNew = true;
        newName = "";
    }

    function commitCreate() {
        const trimmed = newName.trim();
        if (trimmed && !collections.some((c) => c.name === trimmed)) {
            oncreate(trimmed);
        }
        creatingNew = false;
        newName = "";
    }

    function cancelCreate() {
        creatingNew = false;
        newName = "";
    }

    function handleCreateKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") commitCreate();
        if (e.key === "Escape") cancelCreate();
    }

    // Map of mod_id → position (1-based) for all mods in the collection
    const positionMap = $derived(
        new Map(collection.mods.map((m, i) => [m.mod_id, i + 1])),
    );

    // Backend returns mod names in mod_list, but we key by mod_id — build a reverse map
    const nameToModId = $derived(
        new Map(
            [...resolvedModMap.values()].map((m) => [m.name ?? m.mod_id, m.mod_id]),
        ),
    );

    // Per-mod maximum conflict severity (highest severity across all its conflicts)
    const modSeverityMap = $derived(() => {
        const severities = new Map<string, ConflictSeverity>();
        const order: Record<ConflictSeverity, number> = { none: 0, low: 1, medium: 2, high: 3 };
        for (const c of conflicts) {
            const fileSeverity = conflictSeverityForFile(c);
            for (const name of c.mod_list) {
                const modId = nameToModId.get(name) ?? name;
                const current = severities.get(modId) ?? 'none';
                if (order[fileSeverity] > order[current]) {
                    severities.set(modId, fileSeverity);
                }
            }
        }
        return severities;
    });

    const filteredEntries = $derived(() => {
        const q = searchQuery.trim().toLowerCase();
        return collection.mods
            .map((entry, idx) => ({ entry, idx }))
            .filter(({ entry }) => {
                if (q === "") return true;
                const mod = resolvedModMap.get(entry.mod_id);
                return (mod?.name ?? entry.mod_id).toLowerCase().includes(q);
            });
    });

    // Compute conflict details for the currently hovered mod, grouped by opposing mod
    const hoveredConflicts = $derived(() => {
        if (!hoveredModId) return [];
        const thisPos = positionMap.get(hoveredModId);
        if (thisPos === undefined) return [];

        const grouped = new Map<
            string,
            {
                other_mod_name: string;
                other_position: number;
                this_position: number;
                this_wins: boolean;
                file_count: number;
                categories: Map<ConflictCategory, number>;
                files: string[];
            }
        >();

        // Translate the hovered mod_id back to name for matching against mod_list
        const hoveredName =
            resolvedModMap.get(hoveredModId!)?.name ?? hoveredModId!;

        for (const c of conflicts) {
            if (!c.mod_list.includes(hoveredName)) continue;
            for (const otherName of c.mod_list) {
                if (otherName === hoveredName) continue;
                const otherId = nameToModId.get(otherName) ?? otherName;
                if (!positionMap.has(otherId)) continue;
                const otherPos = positionMap.get(otherId)!;
                if (!grouped.has(otherId)) {
                    grouped.set(otherId, {
                        other_mod_name:
                            resolvedModMap.get(otherId)?.name ?? otherName,
                        other_position: otherPos,
                        this_position: thisPos,
                        this_wins: thisPos > otherPos,
                        file_count: 0,
                        categories: new Map(),
                        files: [],
                    });
                }
                const g = grouped.get(otherId)!;
                g.file_count++;
                g.categories.set(c.category, (g.categories.get(c.category) ?? 0) + 1);
                g.files.push(c.file_path);
            }
        }

        return [...grouped.values()].sort((a, b) => b.file_count - a.file_count);
    });

    // Drag-and-drop state
    // draggingIdx is a plain variable (not $state) — making it reactive would
    // trigger a re-render on dragstart, which cancels the browser's drag gesture.
    let draggingIdx: number | null = null;
    let dragOverIdx = $state<number | null>(null);

    function onDragStart(idx: number, e: DragEvent) {
        draggingIdx = idx;
        e.dataTransfer!.effectAllowed = "move";
        // Required by some WebKit builds for the drag to register
        e.dataTransfer!.setData("text/plain", String(idx));
        hoveredModId = null;
        tooltipAnchor = null;
    }

    function onDragOver(idx: number, e: DragEvent) {
        e.preventDefault();
        e.dataTransfer!.dropEffect = "move";
        dragOverIdx = idx;
    }

    function onDragLeave() {
        dragOverIdx = null;
    }

    function onDrop(idx: number, e: DragEvent) {
        e.preventDefault();
        if (draggingIdx !== null && draggingIdx !== idx) {
            onmove(draggingIdx, idx);
        }
        draggingIdx = null;
        dragOverIdx = null;
    }

    function onDragEnd() {
        draggingIdx = null;
        dragOverIdx = null;
    }

    let hideTimer: ReturnType<typeof setTimeout> | null = null;

    function scheduleHide() {
        hideTimer = setTimeout(() => {
            hoveredModId = null;
            tooltipAnchor = null;
        }, 120);
    }

    function cancelHide() {
        if (hideTimer !== null) {
            clearTimeout(hideTimer);
            hideTimer = null;
        }
    }

    function onRowEnter(modId: string, event: MouseEvent) {
        if (draggingIdx !== null) return; // suppress tooltip while dragging
        cancelHide();
        hoveredModId = modId;
        tooltipAnchor = (
            event.currentTarget as HTMLElement
        ).getBoundingClientRect();
    }

    function onRowLeave() {
        scheduleHide();
    }
</script>

<div class="panel">
    <div class="panel-header">
        <span class="label">Collection</span>
        {#if creatingNew}
            <input
                class="new-name-input"
                placeholder="Collection name..."
                bind:value={newName}
                onblur={commitCreate}
                onkeydown={handleCreateKeydown}
                use:focusOnMount
            />
        {:else}
            <select
                value={collection.name}
                onchange={(e) =>
                    onselect((e.target as HTMLSelectElement).value)}
            >
                {#each collections as col}
                    <option value={col.name}>{col.name}</option>
                {/each}
            </select>
            <span class="count">({collection.mods.length})</span>
        {/if}
        <button
            class="hdr-btn"
            onclick={startCreate}
            title="New collection"
            disabled={creatingNew}>+</button
        >
        <button
            class="hdr-btn hdr-btn-delete"
            onclick={ondelete}
            title="Delete collection"
            disabled={collections.length <= 1 || creatingNew}>✕</button
        >
    </div>

    <div class="search-bar">
        <input
            type="search"
            placeholder="Search collection..."
            bind:value={searchQuery}
        />
        <button class="bar-btn" onclick={onenableall}>Enable All</button>
        <button class="bar-btn" onclick={ondisableall}>Disable All</button>
    </div>

    <div class="column-headers">
        <span class="right">#</span>
        <span></span>
        <span>Name</span>
        <span class="right">Version</span>
        <span></span>
    </div>

    <div class="mod-list">
        {#each filteredEntries() as { entry, idx } (entry.mod_id)}
            {@const mod = resolvedModMap.get(entry.mod_id)}
            {#if mod}
                <div
                    class="row-wrapper"
                    class:drag-over={dragOverIdx === idx}
                    draggable="true"
                    ondragstart={(e) => onDragStart(idx, e)}
                    ondragover={(e) => onDragOver(idx, e)}
                    ondragleave={onDragLeave}
                    ondrop={(e) => onDrop(idx, e)}
                    ondragend={onDragEnd}
                    onmouseenter={(e) => onRowEnter(entry.mod_id, e)}
                    onmouseleave={onRowLeave}
                    role="row"
                    tabindex="-1"
                >
                    <ModRow
                        {mod}
                        enabled={entry.enabled}
                        position={idx + 1}
                        variant="collection"
                        conflictSeverity={modSeverityMap().get(entry.mod_id) ?? 'none'}
                        ontoggle={() => ontoggle(entry.mod_id)}
                        onmoveup={idx > 0
                            ? () => onmove(idx, idx - 1)
                            : undefined}
                        onmovedown={idx < collection.mods.length - 1
                            ? () => onmove(idx, idx + 1)
                            : undefined}
                        onremove={() => onremove(entry.mod_id)}
                        onmovetoposition={(newPos) => {
                            const clamped = Math.max(
                                1,
                                Math.min(newPos, collection.mods.length),
                            );
                            onmove(idx, clamped - 1);
                        }}
                    />
                </div>
            {:else}
                <div class="missing-row">
                    <span class="position">{idx + 1}</span>
                    <span class="missing-id">{entry.mod_id} (not found)</span>
                </div>
            {/if}
        {/each}
        {#if collection.mods.length === 0}
            <div class="empty">
                No mods in this collection.<br />
                <small>Enable mods from the left panel to add them.</small>
            </div>
        {:else if filteredEntries().length === 0}
            <div class="empty">No mods match your search.</div>
        {/if}
    </div>
</div>

{#if hoveredModId && tooltipAnchor && (modSeverityMap().get(hoveredModId) ?? 'none') !== 'none'}
    {@const mod = resolvedModMap.get(hoveredModId)}
    {#if mod}
        <ConflictTooltip
            modName={mod.name ?? hoveredModId}
            conflicts={hoveredConflicts()}
            anchor={tooltipAnchor}
            onmouseenter={cancelHide}
            onmouseleave={scheduleHide}
        />
    {/if}
{/if}

<style>
    .panel {
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        min-width: 0;
    }

    .panel-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 3px 8px;
        font-size: 11px;
        font-weight: 700;
        text-transform: uppercase;
        letter-spacing: 0.6px;
        border-bottom: 1px solid var(--border);
        background: var(--code-bg);
        color: var(--text);
        flex-shrink: 0;
    }

    .label {
        white-space: nowrap;
    }

    .panel-header select {
        background: var(--bg);
        border: 1px solid var(--border);
        border-radius: 2px;
        color: var(--text-h);
        font-size: 12px;
        font-weight: 500;
        font-family: var(--sans);
        padding: 1px 4px;
        cursor: pointer;
        text-transform: none;
        letter-spacing: 0;
    }

    .panel-header select:focus {
        outline: 1px solid var(--accent-border);
    }

    .new-name-input {
        flex: 1;
        background: var(--bg);
        border: 1px solid var(--accent-border);
        border-radius: 2px;
        color: var(--text-h);
        font-size: 12px;
        font-family: var(--sans);
        padding: 2px 6px;
        outline: none;
        min-width: 0;
    }

    .count {
        font-weight: 400;
        opacity: 0.6;
        flex: 1;
    }

    .hdr-btn {
        background: none;
        border: 1px solid var(--border);
        border-radius: 2px;
        color: var(--text);
        cursor: pointer;
        font-size: 13px;
        line-height: 1;
        padding: 1px 5px;
    }

    .hdr-btn:hover:not(:disabled) {
        background: var(--accent-bg);
        border-color: var(--accent-border);
        color: var(--accent);
    }

    .hdr-btn:disabled {
        opacity: 0.3;
        cursor: default;
    }

    .hdr-btn-delete:hover:not(:disabled) {
        background: var(--tertiary-bg);
        border-color: var(--tertiary);
        color: var(--tertiary);
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
        box-sizing: border-box;
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
        grid-template-columns: 24px 20px 1fr 72px 68px;
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

    .row-wrapper {
        display: block;
        cursor: grab;
    }

    .row-wrapper:active {
        cursor: grabbing;
    }

    .row-wrapper.drag-over {
        border-top: 2px solid var(--accent);
    }

    .empty {
        padding: 24px 16px;
        text-align: center;
        color: var(--text);
        font-size: 13px;
        opacity: 0.6;
        line-height: 1.6;
    }

    .missing-row {
        display: grid;
        grid-template-columns: 24px 1fr;
        gap: 4px;
        padding: 3px 8px;
        font-size: 12px;
        border-bottom: 1px solid var(--border);
        opacity: 0.5;
    }

    .position {
        text-align: right;
        font-size: 11px;
        color: var(--text);
    }

    .missing-id {
        color: var(--tertiary);
        font-style: italic;
    }
</style>
