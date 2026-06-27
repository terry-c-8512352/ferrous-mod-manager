# Ferrous Mod Manager task runner.
#
# Install just:  cargo install just   (or `apt install just`, `brew install just`)
# List recipes:  just            (the default recipe below)
#
# The `mock-*` recipes let you run the whole app on a box with no Steam and no
# Paradox game installed, by pointing it at a generated fake $HOME. Overriding
# $HOME hides rustup's toolchain from cargo, so we pin RUSTUP_HOME/CARGO_HOME to
# the real ones (captured here before any recipe overrides $HOME).

export RUSTUP_HOME := env_var_or_default('RUSTUP_HOME', env_var('HOME') / '.rustup')
export CARGO_HOME := env_var_or_default('CARGO_HOME', env_var('HOME') / '.cargo')

mock_home := justfile_directory() / 'mock-home'

# Show available recipes.
default:
    @just --list

# (Re)generate the fake $HOME with mock Steam libraries and Stellaris mods.
gen-mock:
    cargo run --quiet --example gen_mock_home -- '{{mock_home}}'

# Run the backend pipeline (detect -> achievements -> conflicts) against the mock $HOME.
mock-smoke: gen-mock
    HOME='{{mock_home}}' cargo run --quiet --example mock_smoke -p ferrous-mod-manager

# Launch the real Tauri GUI against the mock $HOME.
# WEBKIT_DISABLE_DMABUF_RENDERER=1 avoids a blank webview on software/!GPU stacks.
mock-gui: gen-mock
    HOME='{{mock_home}}' WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo tauri dev

# Remove the generated mock $HOME.
clean-mock:
    rm -rf '{{mock_home}}'

# Run the Rust test suite.
test:
    cargo test

# Lint and format-check.
check:
    cargo clippy
    cargo fmt -- --check
