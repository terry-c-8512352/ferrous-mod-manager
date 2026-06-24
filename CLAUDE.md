# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Ferrous Mod Manager — a native Linux mod manager for Paradox Interactive games (Stellaris confirmed working; EU4, HOI4, CK3, Vic3, Imperator: Rome, Star Trek: Infinite need testing). Detects Steam library installs, parses Paradox `.mod` descriptor files, finds file-level conflicts between installed mods, and manages enable/order "collections" that get applied to the game's `dlc_load.json`.

## Commands

```bash
# Run the UI in dev mode
cargo run -p app-ui

# Build release binary
cargo build --release -p app-ui

# Run against a generated fake $HOME (no Steam / no game install needed)
just mock-ui

# Rust tests (run from repo root; workspace covers both crates)
cargo test

# Run a single test
cargo test test_detect_games_finds_known_games

# Lint / format
cargo clippy
cargo fmt -- --check
```

The `app-ui` crate has a `wgpu` feature, **on by default** (GPU renderer for the shipped app). Build with `--no-default-features` to drop wgpu and use iced's `tiny-skia` CPU renderer — needed for headless rendering, where the software-GL stack can segfault inside wgpu.

## Architecture

Two-crate workspace:

| Layer | Location | Language | Role |
|-------|----------|----------|------|
| Core library | `src/` (crate `ferrous-mod-manager`) | Rust | Game detection, mod parsing, conflict analysis, collection persistence — no UI dependencies |
| UI | `app-ui/` (crate `app-ui`) | Rust + Iced | Native desktop window: mod lists, drag-and-drop ordering, conflict visualization |

`Cargo.toml` at the repo root defines a workspace with members `.` and `app-ui`; `app-ui` depends on the core crate via a path dependency and calls its functions **directly, in-process** — there is no IPC layer. Core library types (`DetectedGame`, `ModDescriptor`, `ModCollection`, `ModConflict`, ...) are used straight from the core crate, so there is no separate type mirror to keep in sync.

### Core library modules (`src/`)

- `detector.rs` — finds Steam's `libraryfolders.vdf`, cross-references known Paradox app IDs to determine installed games and their `paradox_data_path` (`~/.local/share/Paradox Interactive/<Game>`), then scans that game's `mod/` directory for `.mod` descriptor files.
- `parser/vdf.rs` — `nom`-based parser for Steam's VDF format (library folders + app IDs).
- `parser/mod_descriptor.rs` — `nom`-based parser for Paradox `.mod` descriptor syntax (`key="value"` and `key={ "a" "b" }` blocks).
- `models.rs` — shared data types and `ModCollection` save/load (JSON) plus list mutation helpers (`add_mod`, `toggle_mod`, `move_mod`).
- `collections.rs` — per-game collection CRUD on disk (one JSON file per collection, named by UUID, under `locations::game_data_dir(app_id)`) and `apply_mod_collection_for_game`, which writes the enabled mod's workshop IDs into the game's `dlc_load.json` as `mod/ugc_<id>.mod` entries — this is the actual "activate this mod list in-game" step.
- `conflict.rs` — walks every mod's file tree (`walkdir`), builds a map of relative file path → owning mod names, and reports any path owned by more than one mod as a `ModConflict`. `categorize_path` buckets conflicts by top-level Paradox directory (`common/defines`, `localisation`, `events`, `gfx`/`interface`, `sound`/`music`, `map`, else `Other`) to drive severity in the UI.
- `locations.rs` — central source of truth for on-disk data paths (`dirs::data_local_dir()/ferrous-mod-manager/mod-collections/<app_id>`).
- `errors.rs` — `thiserror` error enums per concern (`ModParseError`, `DetectionError`, `VdfParseError`, `FileOperationError`); the UI consumes these `Result`s directly and surfaces failures as toast messages.

A mod's identity (`ModDescriptor::mod_id`) is its Steam Workshop `remote_file_id` if present, otherwise falls back to its local `path` — local and workshop mods share the same `ModEntry.mod_id` keying scheme in collections.

### UI (`app-ui/src/main.rs`)

Single-file Iced application (`App` model + `update`/`view`). It calls core library functions directly (`detect_games`, `discover_mods`, the `collections::*` CRUD, `conflict_detection`, `achievement_status_for_mods`) — no command layer. Keep new game/mod logic in `src/`, not here; `main.rs` is presentation + local UI state only.

### Tests

Rust tests are colocated with their modules (`#[cfg(test)] mod tests` at the bottom of each file) and rely heavily on fixtures under `tests/fixtures/` (fake Steam home directories, fake mod directories, fake collection JSON) rather than mocking — e.g. `detector.rs` tests point `detect_games_from_home` at `tests/fixtures/fake_home`. When adding detection/parsing logic, prefer adding a fixture over mocking the filesystem.
