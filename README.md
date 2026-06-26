# Ferrous Mod Manager

A native Linux mod manager for Paradox Interactive games, built with Rust and [Iced](https://iced.rs/).

## Supported Games

| Game | Status |
|------|--------|
| Stellaris | Confirmed Working |
| Europa Universalis IV | Needs Testing |
| Hearts of Iron IV | Needs Testing |
| Crusader Kings III | Needs Testing |
| Victoria 3 | Needs Testing |
| Imperator: Rome | Needs Testing |
| Star Trek: Infinite | Needs Testing |

## Screenshots

<img src="docs/screenshots/mod_collection_overview.png" alt="Mod collection overview" width="700">

<img src="docs/screenshots/mod_conflict_tooltip.png" alt="Mod conflict tooltip" width="700">

## Installation

### From Source

#### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Steam (for game detection and launching)

#### Build & Run

```bash
# Clone the repo
git clone https://github.com/terry-c-8512352/ferrous-mod-manager.git
cd ferrous-mod-manager

# Run in dev mode
cargo run -p app-ui

# Build for release
cargo build --release -p app-ui
```

#### Run Tests

```bash
# Rust tests
cargo test

# Lint
cargo clippy

# Format check
cargo fmt -- --check
```

## Architecture

| Layer | Language | Role |
|-------|----------|------|
| Core library | Rust (`src/`) | Game detection, mod parsing, conflict analysis, load order I/O |
| UI | Rust + Iced (`app-ui/`) | Native desktop window: mod lists, drag-and-drop ordering, conflict visualization |

## Contributing

Looking for some contributors to help maintain this project :)
