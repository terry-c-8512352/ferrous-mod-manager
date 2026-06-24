# app-ui — native Iced desktop UI

The Ferrous Mod Manager desktop app, built on [Iced](https://iced.rs) (MIT).
The sole UI — it replaced the former Svelte + Tauri webview (now removed). Calls
the `ferrous-mod-manager` core **directly in-process** — no IPC, no
hand-maintained TypeScript type mirror.

## What's implemented

- Game selector + collection selector (create / delete).
- Two-pane main view: **installed mods** (filterable, add/remove, add-all /
  remove-all) ↔ **active collection** (drag-to-reorder, enable/disable,
  enable-all / disable-all, remove).
- Live **conflict detection** for the active loadout, with a per-mod severity
  pill and an expandable detail panel grouped by opposing mod.
- **Achievement / ironman** status per mod.
- **Apply** (writes the game's `dlc_load.json`) with success/error toasts.
- Light/dark theming, bundled Exo 2 typography, Lucide SVG icons.

Not yet ported (tracked for follow-up): the full collections **manager** screen
(rename, multi-select delete, JSON import/export), **Launch** (still a stub),
list virtualization for very large mod sets, and tweened animations.

## Run

Against your real install, from the repo root:

```bash
cargo run -p app-ui
```

Against a generated mock home (no Steam / no game install needed):

```bash
just mock-ui
```

> The `just mock-ui` recipe overrides `$HOME` to point at a fake home, while
> pinning `RUSTUP_HOME`/`CARGO_HOME` to the real ones — `cargo`/`rustup` read
> `$HOME` for their own config and won't start otherwise.

Build with `--no-default-features` to drop the wgpu (GPU) renderer and use
iced's `tiny-skia` CPU renderer — needed for headless rendering.
