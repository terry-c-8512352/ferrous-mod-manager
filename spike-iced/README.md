# Iced UI spike (THROWAWAY — Phase 0)

De-risking spike for the UI re-platform off Svelte + Tauri to native **Iced**.
See the plan: `~/.claude/plans/i-want-to-reconsider-curious-robin.md`.

It rebuilds **only the Collection panel** — deliberately the hardest screen —
wired **directly** to the `ferrous-mod-manager` core (no Tauri, no IPC). It
exercises the three risky capabilities at once:

- **Drag-to-reorder** — grab the dotted handle on a row and drag it over another
  row. (Chevron up/down buttons also reorder, as a click fallback.)
- **Conflict visualisation** — click a severity-coloured `⚠ N conflicts` pill to
  expand a detail panel **grouped by the opposing mod**, listing each colliding
  file with a category chip + severity colour.
- **Light/dark theming** — toggle top-right (moon/sun).
- Achievement/ironman pills per mod.

### Polish pass

This is the *designed* revision (not the first functional mockup), to show
Iced's real aesthetic ceiling rather than its defaults:

- **Bundled Inter** typography (Regular/Medium/SemiBold/Bold, SIL OFL — see
  `assets/fonts/OFL.txt`) with a deliberate type scale.
- Hand-tuned dark/light **palette**, 8px spacing rhythm, elevated cards with
  subtle borders + shadow, soft tinted **pill** badges.
- Real **Lucide SVG icons** (ISC licensed), not unicode glyphs.

Still intentionally *not* covered (Iced-specific follow-ups, flagged in the plan):
list virtualization for large mod sets, and **tweened animations** — the dragged
row elevates via a state change, but Iced needs a manual time subscription for
real transitions.

## Run it (needs a real display — won't run headless)

From the repo root, generate a self-contained mock `$HOME` so there's real data
to render (5 mods: a high-severity `common/defines` conflict, a low-severity
`localisation` conflict, and 3 achievement-blockers):

```bash
cargo run --example gen_mock_home -- /tmp/mock-home
cargo build -p spike-iced
HOME=/tmp/mock-home ./target/debug/spike-iced
```

> Run the built binary with `HOME=` overridden rather than `HOME=… cargo run`
> — `cargo`/`rustup` read `$HOME` for their own config and will refuse to start.

On Wayland, if the window misbehaves, the wgpu renderer falls back to software
automatically; if needed force it with `WGPU_BACKEND=gl` or run under XWayland.

To point it at your **real** Stellaris install instead, just run
`./target/debug/spike-iced` with your normal `$HOME`.

## Decision gate

Compare side-by-side with the current Svelte UI
(`WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo tauri dev`). Proceed to the full
migration only if this native version looks **at least as good** and the
drag/expand interactions feel right.

## Cleanup

Throwaway. To remove: delete `spike-iced/` and drop it from `members` in the
root `Cargo.toml`.
