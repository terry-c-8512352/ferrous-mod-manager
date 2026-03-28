<script lang="ts">
    import ThemeToggle from "./lib/ThemeToggle.svelte";
    import GameSelector from "./lib/GameSelector.svelte";
    import CollectionSelector from "./lib/CollectionSelector.svelte";
    import CollectionsManager from "./lib/CollectionsManager.svelte";
    import ModList from "./lib/ModList.svelte";
    import CollectionPanel from "./lib/CollectionPanel.svelte";
    import StatusBar from "./lib/StatusBar.svelte";
    import Toast from "./lib/Toast.svelte";
    import type {
        DetectedGame,
        ModCollection,
        ModConflict,
        ModDescriptor,
        ResolvedMod,
        ConflictSeverity,
    } from "./lib/types";
    import { resolveModId, conflictSeverityForFile } from "./lib/types";
    import { invoke } from "@tauri-apps/api/core";

    let errorMessage = $state("");
    let successMessage = $state("");

    let selectedGameId: number = $state<number>(0);
    let games = $state<DetectedGame[]>([]);
    let collectionsByGame = $state<Record<number, ModCollection[]>>({});

    invoke<DetectedGame[]>("detect_games")
        .then((detected) => {
            games = detected;
            if (games.length > 0) {
                selectedGameId = games[0].app_id;
            }
            return invoke<Record<number, ModCollection[]>>("load_collections", {
                games,
            });
        })
        .then((data) => {
            collectionsByGame = data;
            const gameCols = collectionsByGame[selectedGameId];
            if (gameCols?.[0]) selectedCollectionName = gameCols[0].name;
        })
        .catch((error) => console.error(`Unable to load games: ${error}`));

    let installedMods = $state<ResolvedMod[]>([]);
    $effect(() => {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (game) {
            invoke<ModDescriptor[]>("detect_mods", { game })
                .then(
                    (mods) =>
                        (installedMods = mods.map((m) => ({
                            ...m,
                            mod_id: resolveModId(m),
                            source: m.remote_file_id ? "workshop" : "local",
                        }))),
                )
                .catch((err) => console.error(`Unable to load mods: ${err}`));
        }
    });

    let selectedCollectionName = $state("");
    let view = $state<"main" | "collections">("main");

    const collections = $derived(collectionsByGame[selectedGameId] ?? []);
    const activeCollection = $derived(
        collections.find((c) => c.name === selectedCollectionName) ??
            collections[0],
    );
    const collectionModIdSet = $derived(
        new Set((activeCollection?.mods ?? []).map((m) => m.mod_id)),
    );
    const resolvedModMap = $derived(
        new Map(installedMods.map((m) => [m.mod_id, m])),
    );
    let conflicts = $state<ModConflict[]>([]);
    $effect(() => {
        const enabledModIds = new Set(
            (activeCollection?.mods ?? [])
                .filter((m) => m.enabled)
                .map((m) => m.mod_id),
        );
        const activeMods = installedMods.filter((m) =>
            enabledModIds.has(m.mod_id),
        );
        if (activeMods.length === 0) {
            conflicts = [];
            return;
        }
        invoke<ModConflict[]>("detect_mod_conflict", { mods: activeMods })
            .then((result) => (conflicts = result))
            .catch((err) =>
                console.error(`Unable to detect conflicts: ${err}`),
            );
    });
    const enabledCount = $derived(
        (activeCollection?.mods ?? []).filter((m) => m.enabled).length,
    );
    // Count unique mods at each severity tier
    const conflictCounts = $derived(() => {
        const modMaxSeverity = new Map<string, ConflictSeverity>();
        const order: Record<ConflictSeverity, number> = { none: 0, low: 1, medium: 2, high: 3 };
        for (const c of conflicts) {
            const sev = conflictSeverityForFile(c);
            for (const name of c.mod_list) {
                const current = modMaxSeverity.get(name) ?? 'none';
                if (order[sev] > order[current]) {
                    modMaxSeverity.set(name, sev);
                }
            }
        }
        let significant = 0;
        let minor = 0;
        for (const sev of modMaxSeverity.values()) {
            if (sev === 'high' || sev === 'medium') significant++;
            else if (sev === 'low') minor++;
        }
        return { significant, minor };
    });
    const activeGameName = $derived(
        games.find((g) => g.app_id === selectedGameId)?.game_name ?? "",
    );

    function saveCollection(col: ModCollection) {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game) return;
        invoke("save_collection", { game, modCollection: col }).catch((err) => {
            console.error(`Failed to save collection: ${err}`);
            errorMessage = `Failed to save "${col.name}": ${err}`;
        });
    }

    function switchGame(gameId: number) {
        selectedGameId = gameId;
        const gameCols = collectionsByGame[gameId];
        selectedCollectionName = gameCols?.[0]?.name ?? "";
    }

    function switchCollection(name: string) {
        selectedCollectionName = name;
    }

    function toggleModInCollection(modId: string) {
        if (!activeCollection) return;
        const idx = activeCollection.mods.findIndex((m) => m.mod_id === modId);
        if (idx === -1) {
            activeCollection.mods.push({ mod_id: modId, enabled: true });
        } else {
            activeCollection.mods.splice(idx, 1);
        }
        saveCollection(activeCollection);
    }

    function toggleModEnabled(modId: string) {
        if (!activeCollection) return;
        const entry = activeCollection.mods.find((m) => m.mod_id === modId);
        if (entry) entry.enabled = !entry.enabled;
        saveCollection(activeCollection);
    }

    function moveMod(from: number, to: number) {
        if (!activeCollection) return;
        const [item] = activeCollection.mods.splice(from, 1);
        activeCollection.mods.splice(to, 0, item);
        saveCollection(activeCollection);
    }

    function removeMod(modId: string) {
        if (!activeCollection) return;
        const idx = activeCollection.mods.findIndex((m) => m.mod_id === modId);
        if (idx !== -1) activeCollection.mods.splice(idx, 1);
        saveCollection(activeCollection);
    }

    function enableAll() {
        if (!activeCollection) return;
        for (const entry of activeCollection.mods) entry.enabled = true;
        saveCollection(activeCollection);
    }

    function disableAll() {
        if (!activeCollection) return;
        for (const entry of activeCollection.mods) entry.enabled = false;
        saveCollection(activeCollection);
    }

    function addAllMods() {
        if (!activeCollection) return;
        const existing = new Set(activeCollection.mods.map((m) => m.mod_id));
        for (const mod of installedMods) {
            if (!existing.has(mod.mod_id)) {
                activeCollection.mods.push({
                    mod_id: mod.mod_id,
                    enabled: true,
                });
            }
        }
        saveCollection(activeCollection);
    }

    function removeAllMods() {
        if (!activeCollection) return;
        activeCollection.mods = [];
        saveCollection(activeCollection);
    }

    function deleteCollectionFromDisk(
        col: ModCollection,
        onFailure: () => void,
    ) {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game) return;
        invoke("delete_collection", { game, modCollection: col }).catch(
            (err) => {
                console.error(`Failed to delete collection: ${err}`);
                errorMessage = `Failed to delete "${col.name}": ${err}`;
                onFailure();
            },
        );
    }

    function deleteCollection(name?: string) {
        const cols = collectionsByGame[selectedGameId];
        if (!cols || cols.length <= 1) return;
        const target = name ?? selectedCollectionName;
        const idx = cols.findIndex((c) => c.name === target);
        if (idx === -1) return;
        const col = cols[idx];
        const previousSelection = selectedCollectionName;
        cols.splice(idx, 1);
        if (target === selectedCollectionName) {
            selectedCollectionName = cols[Math.max(0, idx - 1)].name;
        }
        deleteCollectionFromDisk(col, () => {
            collectionsByGame[selectedGameId].splice(idx, 0, col);
            selectedCollectionName = previousSelection;
        });
    }

    function importCollections(imported: ModCollection[]) {
        if (!collectionsByGame[selectedGameId])
            collectionsByGame[selectedGameId] = [];
        const existing = new Set(
            collectionsByGame[selectedGameId].map((c) => c.name),
        );
        for (const col of imported) {
            if (!existing.has(col.name)) {
                collectionsByGame[selectedGameId].push(col);
                existing.add(col.name);
                saveCollection(col);
            }
        }
    }

    function deleteCollections(names: string[]) {
        const cols = collectionsByGame[selectedGameId];
        if (!cols) return;
        // Must keep at least one collection
        const toDelete = new Set(
            names.filter((n) => cols.some((c) => c.name === n)),
        );
        if (toDelete.size >= cols.length)
            toDelete.delete([...toDelete][toDelete.size - 1]);
        const deletedCols = cols.filter((c) => toDelete.has(c.name));
        const remaining = cols.filter((c) => !toDelete.has(c.name));
        const previousSelection = selectedCollectionName;
        collectionsByGame[selectedGameId] = remaining;
        if (toDelete.has(selectedCollectionName)) {
            selectedCollectionName = remaining[0].name;
        }
        for (const col of deletedCols) {
            deleteCollectionFromDisk(col, () => {
                collectionsByGame[selectedGameId] = [
                    ...collectionsByGame[selectedGameId],
                    ...deletedCols,
                ];
                selectedCollectionName = previousSelection;
            });
        }
    }

    function createCollection(name: string) {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game) return;
        invoke<ModCollection>("create_collection", { game, name })
            .then((newCol) => {
                if (!collectionsByGame[selectedGameId])
                    collectionsByGame[selectedGameId] = [];
                collectionsByGame[selectedGameId].push(newCol);
                selectedCollectionName = newCol.name;
            })
            .catch((err) => {
                console.error(`Failed to create collection: ${err}`);
                errorMessage = `Failed to create "${name}": ${err}`;
            });
    }

    function renameCollection(oldName: string, newName: string) {
        const cols = collectionsByGame[selectedGameId];
        if (!cols) return;
        const col = cols.find((c) => c.name === oldName);
        if (!col) return;
        col.name = newName;
        if (selectedCollectionName === oldName)
            selectedCollectionName = newName;
        saveCollection(col);
    }

    function applyCollection() {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game || !activeCollection) return;
        invoke("apply_mod_collection", {
            game,
            modCollection: activeCollection,
        })
            .then(() => {
                successMessage = `Applied "${activeCollection!.name}" to ${game.game_name}`;
            })
            .catch((err) => {
                errorMessage = `Failed to apply collection: ${err}`;
            });
    }

    function launchGame() {
        alert("Launch: not implemented yet");
    }
</script>

<div class="app-shell">
    <div class="top-bar">
        <GameSelector {games} {selectedGameId} onselect={switchGame} />
        <div class="top-bar-divider"></div>
        {#if view === "collections"}
            <button class="manager-btn" onclick={() => (view = "main")}
                >Manager</button
            >
        {:else}
            <CollectionSelector
                onapply={applyCollection}
                onlaunch={launchGame}
                onmanage={() => (view = "collections")}
            />
        {/if}
        <div class="top-bar-spacer"></div>
        <ThemeToggle />
    </div>

    {#if view === "collections"}
        <CollectionsManager
            {collections}
            onback={() => (view = "main")}
            oncreate={createCollection}
            ondelete={deleteCollection}
            ondeleteall={deleteCollections}
            onrename={renameCollection}
            onselect={switchCollection}
            onimport={importCollections}
        />
    {:else}
        <div class="main-area">
            <ModList
                mods={installedMods}
                {collectionModIdSet}
                ontogglecollection={toggleModInCollection}
                onaddall={addAllMods}
                onremoveall={removeAllMods}
            />
            {#if activeCollection}
                <CollectionPanel
                    collection={activeCollection}
                    {collections}
                    {resolvedModMap}
                    {conflicts}
                    ontoggle={toggleModEnabled}
                    onmove={moveMod}
                    onremove={removeMod}
                    onselect={switchCollection}
                    oncreate={createCollection}
                    ondelete={deleteCollection}
                    onenableall={enableAll}
                    ondisableall={disableAll}
                />
            {:else}
                <div class="no-collection">No collection selected.</div>
            {/if}
        </div>

        <StatusBar
            totalMods={installedMods.length}
            enabledMods={enabledCount}
            significantConflicts={conflictCounts().significant}
            minorConflicts={conflictCounts().minor}
            {activeGameName}
        />
    {/if}
</div>

<Toast message={errorMessage} onclear={() => (errorMessage = "")} />
<Toast message={successMessage} variant="success" onclear={() => (successMessage = "")} />

<style>
    .app-shell {
        display: flex;
        flex-direction: column;
        height: 100vh;
        overflow: hidden;
        font-size: 14px;
    }

    .top-bar {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 6px 12px;
        border-bottom: 1px solid var(--border);
        background: var(--code-bg);
        flex-shrink: 0;
    }

    .manager-btn {
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

    .manager-btn:hover {
        background: var(--accent-bg);
        border-color: var(--accent-border);
        color: var(--accent);
    }

    .top-bar-divider {
        width: 1px;
        height: 22px;
        background: var(--border);
        margin: 0 4px;
    }

    .top-bar-spacer {
        flex: 1;
    }

    .main-area {
        display: flex;
        flex: 1;
        overflow: hidden;
    }

    .no-collection {
        flex: 1;
        display: flex;
        align-items: center;
        justify-content: center;
        color: var(--text);
        opacity: 0.5;
        font-size: 13px;
    }
</style>
