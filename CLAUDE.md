# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Ferrous Mod Manager — a native Linux mod manager for Paradox Interactive games (Stellaris confirmed working; EU4, HOI4, CK3, Vic3, Imperator: Rome, Star Trek: Infinite need testing). Detects Steam library installs, parses Paradox `.mod` descriptor files, finds file-level conflicts between installed mods, and manages enable/order "collections" that get applied to the game's `dlc_load.json`.

## Commands

```bash
# Run in dev mode (Wayland note below)
WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo tauri dev

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
```

`WEBKIT_DISABLE_DMABUF_RENDERER=1` is required on some Wayland systems to avoid WebKitGTK rendering issues.

### Blank window in a VM (no GPU)

In a VM without a usable GPU, the dev window can come up **blank** with `libEGL`/`MESA-LOADER` warnings ending in `ZINK: failed to choose pdev` and `failed to create dri2 screen`. Cause: Mesa prefers the Zink (GL-on-Vulkan) driver, the VM has no Vulkan device, and instead of falling back to software it hands WebKit a dead GL context. Force the llvmpipe software rasterizer:

```bash
LIBGL_ALWAYS_SOFTWARE=1 \
GALLIUM_DRIVER=llvmpipe \
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
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
- `parser/mod_descriptor.rs` — `nom`-based parser for Paradox `.mod` descriptor syntax (`key="value"` and `key={ "a" "b" }` blocks).
- `models.rs` — shared data types and `ModCollection` save/load (JSON) plus list mutation helpers (`add_mod`, `toggle_mod`, `move_mod`).
- `collections.rs` — per-game collection CRUD on disk (one JSON file per collection, named by UUID, under `locations::game_data_dir(app_id)`) and `apply_mod_collection_for_game`, which writes the enabled mod's workshop IDs into the game's `dlc_load.json` as `mod/ugc_<id>.mod` entries — this is the actual "activate this mod list in-game" step.
- `conflict.rs` — walks every mod's file tree (`walkdir`), builds a map of relative file path → owning mod names, and reports any path owned by more than one mod as a `ModConflict`. `categorize_path` buckets conflicts by top-level Paradox directory (`common/defines`, `localisation`, `events`, `gfx`/`interface`, `sound`/`music`, `map`, else `Other`) to drive severity in the UI.
- `locations.rs` — central source of truth for on-disk data paths (`dirs::data_local_dir()/ferrous-mod-manager/mod-collections/<app_id>`).
- `errors.rs` — `thiserror` error enums per concern (`ModParseError`, `DetectionError`, `VdfParseError`, `FileOperationError`); Tauri commands flatten these to `String` at the IPC boundary.

A mod's identity (`ModDescriptor::mod_id`) is its Steam Workshop `remote_file_id` if present, otherwise falls back to its local `path` — local and workshop mods share the same `ModEntry.mod_id` keying scheme in collections.

### Tauri shell (`src-tauri/src/lib.rs`)

Single file registering all `#[tauri::command]`s (`detect_games`, `detect_mods`, `load_collections`, `save_collection`, `delete_collection`, `create_collection`, `detect_mod_conflict`, `apply_mod_collection`). Each command is a near-direct pass-through to a core library function — keep new game/mod logic in `src/`, not here.

### Tests

Rust tests are colocated with their modules (`#[cfg(test)] mod tests` at the bottom of each file) and rely heavily on fixtures under `tests/fixtures/` (fake Steam home directories, fake mod directories, fake collection JSON) rather than mocking — e.g. `detector.rs` tests point `detect_games_from_home` at `tests/fixtures/fake_home`. When adding detection/parsing logic, prefer adding a fixture over mocking the filesystem.
