# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Ferrous Mod Manager — a native Linux mod manager for Paradox Interactive games (Stellaris confirmed working; EU4, HOI4, CK3, Vic3, Imperator: Rome, Star Trek: Infinite need testing). Detects Steam library installs, parses Paradox `.mod` descriptor files, finds file-level conflicts between installed mods, and manages enable/order "collections" that get applied to the game's `dlc_load.json`.

## Commands

```bash
# Run in dev mode
cargo tauri dev

# Build release bundle
cargo tauri build

# Rust tests (run from repo root; workspace covers both crates)
cargo test

# Run a single test
cargo test test_detect_games_finds_known_games

# Lint / format
cargo clippy
cargo fmt -- --check

# Frontend (from ui/)
npm install
npm run dev      # standalone Vite dev server (normally launched by `cargo tauri dev` instead)
npm run build
npm run check    # svelte-check + tsc, no separate lint script

# Run against a generated fake $HOME (no Steam / no game install needed)
just mock-gui    # gen mock $HOME, then `cargo tauri dev` pointed at it
just mock-smoke  # backend-only pipeline (detect -> achievements -> conflicts)
```

`WEBKIT_DISABLE_DMABUF_RENDERER=1` works around WebKitGTK rendering issues (blank/broken window) on some Wayland/Nvidia systems. The binary now sets this itself at the top of `run()` (`src-tauri/src/lib.rs`, Linux-only) when not already set, so it no longer needs to be exported manually. Override with `WEBKIT_DISABLE_DMABUF_RENDERER=0` to force the native DMA-BUF path.

### Blank window in a VM (no GPU)

In a VM without a usable GPU, the dev window can come up **blank** with `libEGL`/`MESA-LOADER` warnings ending in `ZINK: failed to choose pdev` and `failed to create dri2 screen`. Cause: Mesa prefers the Zink (GL-on-Vulkan) driver, the VM has no Vulkan device, and instead of falling back to software it hands WebKit a dead GL context. Force the llvmpipe software rasterizer:

```bash
LIBGL_ALWAYS_SOFTWARE=1 \
GALLIUM_DRIVER=llvmpipe \
WEBKIT_DISABLE_COMPOSITING_MODE=1 \
cargo tauri dev
```

Requires Mesa's `swrast_dri.so` (package `libgl1-mesa-dri`). If still blank, prepend `GDK_BACKEND=x11` to route GTK through XWayland.

## Architecture

Three-layer workspace:

| Layer | Location | Language | Role |
|-------|----------|----------|------|
| Core library | `src/` (crate `ferrous-mod-manager`) | Rust | Game detection, mod parsing, conflict analysis, collection persistence — no Tauri/UI dependencies |
| Tauri shell | `src-tauri/` (crate `app`, lib `app_lib`) | Rust | Desktop window + IPC; thin `#[tauri::command]` wrappers around the core library |
| UI | `ui/` | Svelte 5 + TypeScript | Mod lists, drag-and-drop ordering, conflict visualization |

`Cargo.toml` at the repo root defines a workspace with members `.` and `src-tauri`; `src-tauri` depends on the core crate via a path dependency. Core library types (`DetectedGame`, `ModDescriptor`, `ModCollection`, `ModConflict`, ...) are `Serialize`/`Deserialize` and cross the Tauri IPC boundary directly — `ui/src/lib/types.ts` is a hand-maintained mirror of those shapes and must be kept in sync manually when Rust models change.

### Core library modules (`src/`)

- `detector.rs` — finds Steam's `libraryfolders.vdf`, cross-references known Paradox app IDs to determine installed games and their `paradox_data_path` (`~/.local/share/Paradox Interactive/<Game>`), then scans that game's `mod/` directory for `.mod` descriptor files.
- `parser/vdf.rs` — `nom`-based parser for Steam's VDF format (library folders + app IDs).
- `parser/mod_descriptor.rs` — `nom`-based parser for Paradox `.mod` descriptor syntax (`key="value"` and `key={ "a" "b" }` blocks); parses `tags`, `version`, and `dependencies`.
- `models.rs` — shared data types and `ModCollection` save/load (JSON) plus list mutation helpers (`add_mod`, `toggle_mod`, `move_mod`).
- `collections.rs` — per-game collection CRUD on disk (one JSON file per collection, named by UUID, under `locations::game_data_dir(app_id)`) and `apply_mod_collection_for_game`, which writes the enabled mod's workshop IDs into the game's `dlc_load.json` as `mod/ugc_<id>.mod` entries — this is the actual "activate this mod list in-game" step.
- `conflict.rs` — walks every mod's file tree (`walkdir`), builds a map of relative file path → owning mod names, and reports any path owned by more than one mod as a `ModConflict`. `categorize_path` buckets conflicts by top-level Paradox directory (`common/defines`, `localisation`, `events`, `gfx`/`interface`, `sound`/`music`, `map`, else `Other`) to drive severity in the UI. `mod_size_bytes` totals a mod's on-disk size.
- `dependency.rs` — transitive dependency resolution. `resolve_load_chain` DFS-walks a mod's `dependencies` (matched against installed mods **by name**, since that's how Paradox descriptors declare them) into a topological load chain (dependencies first, cycle-safe) plus a list of `MissingDependency` (not-installed names with who required them). `enable_with_dependencies` applies that chain to a `ModCollection`: enables/inserts every dependency, moving one only if it sits after its dependent, and reports what it auto-activated. Exposed to the UI via the `enable_mod_with_dependencies` Tauri command, which also persists the collection.
- `launch.rs` — `launch_game(app_id)` opens `steam://run/<app_id>` via the platform URL handler (cross-platform: `cmd start` on Windows, `open` on macOS, `xdg-open` on Linux).
- `locations.rs` — central source of truth for on-disk data paths (`dirs::data_local_dir()/ferrous-mod-manager/mod-collections/<app_id>`); `paradox_data_root()` abstracts the per-OS Paradox data directory (Linux/macOS/Windows). Also home of `ModRoots`: the canonicalized set of directories mod content may live under (Paradox data dir + every Steam library's `workshop/content`). Descriptor `path=` values are untrusted Workshop input, so every function that walks a mod's file tree (`conflict.rs`, `achievements.rs`) takes a `&ModRoots` and refuses paths that don't resolve inside them — tests build one with `ModRoots::from_roots([fixture dir])`.
- `fsutil.rs` — hardened filesystem primitives: `write_atomic` (temp file + fsync + rename; no truncated configs on crash, replaces rather than follows symlinks) used for all collection/`dlc_load.json` writes, and `read_to_string_limited` (size-capped at `MAX_READ_BYTES`) used for every parsed-file read.
- `errors.rs` — `thiserror` error enums per concern (`ModParseError`, `DetectionError`, `VdfParseError`, `FileOperationError`, `LaunchError`); Tauri commands flatten these to `String` at the IPC boundary.

A mod's identity (`ModDescriptor::mod_id`) is its Steam Workshop `remote_file_id` if present, otherwise falls back to its local `path` — local and workshop mods share the same `ModEntry.mod_id` keying scheme in collections.

### Tauri shell (`src-tauri/src/lib.rs`)

Single file registering all `#[tauri::command]`s (`detect_games`, `detect_mods`, `load_collections`, `save_collection`, `delete_collection`, `create_collection`, `detect_mod_conflict`, `mod_sizes`, `detect_achievement_compatibility`, `apply_mod_collection`, `enable_mod_with_dependencies`, `launch`). Each command is a near-direct pass-through to a core library function — keep new game/mod logic in `src/`, not here. `load_collections` uses `load_or_create_collections_for_game`, so a game with no saved collections gets a default one on first launch. Security posture: webview input is untrusted — `apply_mod_collection` re-resolves the game by `app_id` instead of using frontend-supplied paths, the walk commands pass `ModRoots::detect()`, and there is deliberately no command taking a raw filesystem path (a former `import_collection(path)` was removed; import is client-side via file input). `tauri.conf.json` sets a restrictive CSP (`default-src 'self'` + Tauri IPC `connect-src`) — keep it when touching webview config.

### Tests

Rust tests are colocated with their modules (`#[cfg(test)] mod tests` at the bottom of each file) and rely heavily on fixtures under `tests/fixtures/` (fake Steam home directories, fake mod directories, fake collection JSON) rather than mocking — e.g. `detector.rs` tests point `detect_games_from_home` at `tests/fixtures/fake_home`. When adding detection/parsing logic, prefer adding a fixture over mocking the filesystem.
