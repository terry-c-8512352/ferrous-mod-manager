<script lang="ts">
    import ThemeToggle from "./lib/ThemeToggle.svelte";
    import GameSelector from "./lib/GameSelector.svelte";
    import CollectionsManager from "./lib/CollectionsManager.svelte";
    import Toolbar from "./lib/Toolbar.svelte";
    import Sidebar from "./lib/Sidebar.svelte";
    import InstalledList from "./lib/InstalledList.svelte";
    import LoadOrder from "./lib/LoadOrder.svelte";
    import Footer from "./lib/Footer.svelte";
    import Toast from "./lib/Toast.svelte";
    import type {
        DetectedGame,
        ModCollection,
        ModConflict,
        ModDescriptor,
        ResolvedMod,
    } from "./lib/types";
    import { resolveModId } from "./lib/types";
    import {
        decorateMod,
        categoryOf,
        catMeta,
        humanSize,
        type DecorateContext,
        type DecoratedMod,
    } from "./lib/catalog";
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

    // Per-mod on-disk size (bytes), keyed by mod_id. Recomputed when the game's
    // installed set changes.
    let modSizes = $state<Map<string, number>>(new Map());
    $effect(() => {
        if (installedMods.length === 0) {
            modSizes = new Map();
            return;
        }
        invoke<Record<string, number>>("mod_sizes", { mods: installedMods })
            .then((r) => (modSizes = new Map(Object.entries(r))))
            .catch((err) => console.error(`Unable to load sizes: ${err}`));
    });

    let selectedCollectionName = $state("");
    let view = $state<"main" | "collections">("main");

    // Width (px) of the Installed column. The Load Order column is flex:1, so
    // dragging the divider between them resizes both. Persisted across sessions.
    const INSTALLED_WIDTH_KEY = "installedColumnWidth";
    const INSTALLED_WIDTH_MIN = 240;
    const INSTALLED_WIDTH_MAX = 720;
    function clampWidth(px: number) {
        return Math.min(INSTALLED_WIDTH_MAX, Math.max(INSTALLED_WIDTH_MIN, px));
    }
    let installedWidth = $state(
        clampWidth(Number(localStorage.getItem(INSTALLED_WIDTH_KEY)) || 312),
    );

    function startResize(event: PointerEvent) {
        event.preventDefault();
        const startX = event.clientX;
        const startWidth = installedWidth;
        const onMove = (e: PointerEvent) => {
            installedWidth = clampWidth(startWidth + (e.clientX - startX));
        };
        const onUp = () => {
            window.removeEventListener("pointermove", onMove);
            window.removeEventListener("pointerup", onUp);
            document.body.style.cursor = "";
            document.body.style.userSelect = "";
            localStorage.setItem(INSTALLED_WIDTH_KEY, String(installedWidth));
        };
        window.addEventListener("pointermove", onMove);
        window.addEventListener("pointerup", onUp);
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
    }

    // Sidebar / toolbar filter state (drives the installed column only).
    let search = $state("");
    let nav = $state<"all" | "enabled" | "conflicts">("all");
    let tagFilter = $state<string | null>(null);

    const collections = $derived(collectionsByGame[selectedGameId] ?? []);
    const activeCollection = $derived(
        collections.find((c) => c.name === selectedCollectionName) ??
            collections[0],
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

    // --- Derivation context shared by both columns -------------------------

    const installedByName = $derived.by(() => {
        const m = new Map<string, ResolvedMod>();
        for (const mod of installedMods) m.set(mod.name ?? mod.mod_id, mod);
        return m;
    });

    const enabledNames = $derived.by(() => {
        const s = new Set<string>();
        for (const e of activeCollection?.mods ?? []) {
            if (!e.enabled) continue;
            s.add(resolvedModMap.get(e.mod_id)?.name ?? e.mod_id);
        }
        return s;
    });

    const conflictsByName = $derived.by(() => {
        const m = new Map<string, ModConflict[]>();
        for (const c of conflicts) {
            for (const name of c.mod_list) {
                let arr = m.get(name);
                if (!arr) {
                    arr = [];
                    m.set(name, arr);
                }
                arr.push(c);
            }
        }
        return m;
    });

    const collectionState = $derived.by(() => {
        const m = new Map<
            string,
            { inCollection: boolean; enabled: boolean; loadNo: string }
        >();
        let n = 0;
        for (const e of activeCollection?.mods ?? []) {
            let loadNo = "";
            if (e.enabled) {
                n++;
                loadNo = String(n).padStart(2, "0");
            }
            m.set(e.mod_id, { inCollection: true, enabled: e.enabled, loadNo });
        }
        return m;
    });

    const ctx = $derived.by<DecorateContext>(() => ({
        sizes: modSizes,
        collectionState,
        installedByName,
        enabledNames,
        conflictsByName,
    }));

    const decoratedInstalled = $derived(
        installedMods.map((m) => decorateMod(m, ctx)),
    );

    // The active collection's mods, in load order. Entries that reference an
    // uninstalled mod still render (as an "Other" placeholder).
    const decoratedCollection = $derived.by(() => {
        const list: DecoratedMod[] = [];
        for (const e of activeCollection?.mods ?? []) {
            const mod: ResolvedMod = resolvedModMap.get(e.mod_id) ?? {
                mod_id: e.mod_id,
                name: e.mod_id,
                source: "workshop",
            };
            list.push(decorateMod(mod, ctx));
        }
        return list;
    });

    const visibleInstalled = $derived.by(() => {
        const q = search.trim().toLowerCase();
        return decoratedInstalled.filter((d) => {
            if (q && !d.name.toLowerCase().includes(q)) return false;
            if (tagFilter && !d.tags.includes(tagFilter)) return false;
            if (nav === "enabled" && !d.enabled) return false;
            if (nav === "conflicts" && !d.hasIssue) return false;
            return true;
        });
    });

    // --- Counts ------------------------------------------------------------

    const enabledCount = $derived(
        (activeCollection?.mods ?? []).filter((m) => m.enabled).length,
    );

    // Sidebar filters generated from the installed mods' Paradox tags. Each
    // distinct tag becomes a filter row; the dot colour is borrowed from the
    // tag's display-category bucket. Sorted by frequency, then alphabetically.
    const tagFilters = $derived.by(() => {
        const counts = new Map<string, number>();
        for (const m of installedMods) {
            for (const t of m.tags ?? []) {
                counts.set(t, (counts.get(t) ?? 0) + 1);
            }
        }
        return [...counts.entries()]
            .map(([tag, count]) => ({
                tag,
                count,
                color: catMeta(categoryOf([tag])).color,
            }))
            .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag));
    });

    // Unique unordered pairs of enabled mods that collide on at least one file.
    const conflictPairCount = $derived.by(() => {
        const set = new Set<string>();
        for (const c of conflicts) {
            const ns = c.mod_list;
            for (let i = 0; i < ns.length; i++)
                for (let j = i + 1; j < ns.length; j++)
                    set.add([ns[i], ns[j]].sort().join("|"));
        }
        return set.size;
    });

    const depWarnCount = $derived.by(() => {
        let n = 0;
        for (const e of activeCollection?.mods ?? []) {
            if (!e.enabled) continue;
            const deps = resolvedModMap.get(e.mod_id)?.dependencies ?? [];
            for (const dep of deps) {
                if (!installedByName.has(dep) || !enabledNames.has(dep)) n++;
            }
        }
        return n;
    });

    // Total on-disk size of every installed mod, independent of which are
    // enabled — computed as soon as sizes load, not as mods are turned on.
    const storageLabel = $derived.by(() => {
        let total = 0;
        for (const m of installedMods) total += modSizes.get(m.mod_id) ?? 0;
        return humanSize(total);
    });

    const navCounts = $derived({
        all: installedMods.length,
        enabled: enabledCount,
        issues: conflictPairCount + depWarnCount,
    });

    const activeGameName = $derived(
        games.find((g) => g.app_id === selectedGameId)?.game_name ?? "",
    );

    // --- Backend-mutating handlers -----------------------------------------

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

    function deleteCollection(name: string = selectedCollectionName) {
        const cols = collectionsByGame[selectedGameId];
        if (!cols || cols.length <= 1) return;
        const idx = cols.findIndex((c) => c.name === name);
        if (idx === -1) return;
        const col = cols[idx];
        const previousSelection = selectedCollectionName;
        cols.splice(idx, 1);
        if (name === selectedCollectionName) {
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

    // Apply the active collection to dlc_load.json without launching.
    function applyCollection() {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game || !activeCollection) return;
        invoke("apply_mod_collection", {
            game,
            modCollection: activeCollection,
        })
            .then(() => {
                successMessage = `Applied "${activeCollection.name}" to ${game.game_name}`;
            })
            .catch((err) => {
                errorMessage = `Failed to apply: ${err}`;
            });
    }

    // Apply the active collection to dlc_load.json, then launch via Steam.
    function launchGame() {
        const game = games.find((g) => g.app_id === selectedGameId);
        if (!game || !activeCollection) return;
        invoke("apply_mod_collection", {
            game,
            modCollection: activeCollection,
        })
            .then(() => invoke("launch", { game }))
            .then(() => {
                successMessage = `Launching ${game.game_name}…`;
            })
            .catch((err) => {
                errorMessage = `Failed to launch: ${err}`;
            });
    }
</script>

<div class="app-shell">
    <div class="title-bar">
        <GameSelector {games} {selectedGameId} onselect={switchGame} />
        <span class="title-spacer"></span>
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
        <Toolbar
            {collections}
            activeName={activeCollection?.name ?? ""}
            {search}
            onsearch={(v) => (search = v)}
            onselect={switchCollection}
            oncollections={() => (view = "collections")}
            onapply={applyCollection}
            onplay={launchGame}
        />

        <div class="body">
            <Sidebar
                counts={navCounts}
                tags={tagFilters}
                activeNav={nav}
                activeTag={tagFilter}
                onnav={(k) => (nav = k)}
                ontag={(t) => (tagFilter = tagFilter === t ? null : t)}
            />
            <InstalledList
                mods={visibleInstalled}
                total={installedMods.length}
                width={installedWidth}
                ontoggle={toggleModInCollection}
            />
            <div
                class="col-resizer"
                role="separator"
                aria-orientation="vertical"
                aria-label="Resize installed mods column"
                onpointerdown={startResize}
            ></div>
            <LoadOrder
                mods={decoratedCollection}
                {enabledCount}
                ontoggle={toggleModEnabled}
                onmove={moveMod}
                onremove={removeMod}
            />
        </div>

        <Footer
            total={installedMods.length}
            enabled={enabledCount}
            conflicts={conflictPairCount}
            depWarnings={depWarnCount}
            storage={storageLabel}
        />
    {/if}
</div>

<Toast message={errorMessage} onclear={() => (errorMessage = "")} />
<Toast
    message={successMessage}
    variant="success"
    onclear={() => (successMessage = "")}
/>

<style>
    .app-shell {
        display: flex;
        flex-direction: column;
        height: 100vh;
        overflow: hidden;
        font-family: var(--sans);
        color: var(--ink);
        background: var(--surface);
    }

    .title-bar {
        flex: none;
        height: 40px;
        display: flex;
        align-items: center;
        gap: 11px;
        padding: 0 14px;
        border-bottom: 1px solid var(--bd);
        background: var(--panel);
    }

    .title-spacer {
        flex: 1;
    }

    .body {
        flex: 1;
        display: flex;
        min-height: 0;
    }

    /* Drag handle sitting between the Installed and Load Order columns.
       Narrow hit target with a wider invisible grab area via margin. */
    .col-resizer {
        flex: none;
        width: 5px;
        margin: 0 -2px;
        cursor: col-resize;
        background: transparent;
        z-index: 1;
        transition: background 0.12s ease;
    }

    .col-resizer:hover,
    .col-resizer:active {
        background: var(--accent, #4a8cff);
    }
</style>
