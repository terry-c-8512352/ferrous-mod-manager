# app-ui — native Iced desktop UI

The Ferrous Mod Manager desktop app, built on [Iced](https://iced.rs) (MIT).
Replaces the Svelte + Tauri webview. Calls the `ferrous-mod-manager` core
**directly in-process** — no IPC, no hand-maintained TypeScript type mirror.

## What's implemented

- Game selector + collection selector (create / delete).
- Two-pane main view: **installed mods** (filterable, add/remove, add-all /
  remove-all) ↔ **active collection** (drag-to-reorder, enable/disable,
  enable-all / disable-all, remove).
- Live **conflict detection** for the active loadout, with a per-mod severity
  pill and an expandable detail panel grouped by opposing mod.
- **Achievement / ironman** status per mod.
- **Apply** (writes the game's `dlc_load.json`) with success/error toasts.
- Light/dark theming, bundled Inter typography, Lucide SVG icons.

Not yet ported (tracked for follow-up): the full collections **manager** screen
(rename, multi-select delete, JSON import/export), **Launch** (still a stub),
list virtualization for very large mod sets, and tweened animations.

## Run

Needs a real display (won't run headless). Against a generated mock home:

```bash
cargo run --example gen_mock_home -- /tmp/mock-home   # from repo root
cargo build -p app-ui
HOME=/tmp/mock-home ./target/debug/app-ui
```

> Run the built binary with `HOME=` overridden rather than `HOME=… cargo run`
> — `cargo`/`rustup` read `$HOME` for their own config and will refuse to start.

Against your real install, just run `./target/debug/app-ui` with your normal
`$HOME`.

## Status

This crate is the migration target. `src-tauri/` and `ui/` are kept temporarily
as a reference and will be removed once this app reaches parity (see the plan at
`~/.claude/plans/i-want-to-reconsider-curious-robin.md`).
