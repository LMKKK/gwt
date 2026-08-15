# gwt

`gwt` is a small, fast interactive Git worktree switcher written in Rust. It shows every worktree and its staged, modified, conflicted, and untracked file counts, then lets you switch with the keyboard.

## Install

With Cargo:

```sh
cargo install autumnk-gwt
```

Or with npm:

```sh
npm install --global @autumnk/gwt
```

Both install the `gwt` command. Git must be installed and available in `PATH`.

## Shell integration

Add one of these to your shell configuration so selecting a worktree changes the current shell directory:

```sh
# ~/.zshrc
eval "$(gwt init zsh)"

# ~/.bashrc
eval "$(gwt init bash)"
```

Use `gwt list`, move with arrow keys or `j`/`k`, press Enter to select, and press `q` to cancel. `gwt list --select` prints the chosen path and is intended for shell integration. Set `NO_COLOR` to disable color.

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo run -- list
```

`Cargo.toml` is the version source of truth. Keep the npm package versions synchronized, then run `cargo xtask verify-versions`. Release binaries are staged with `cargo xtask stage <rust-target>` and inspected with `cargo xtask sizes` and `cargo xtask pack-check`.

The npm package contains only a minimal Node.js launcher that selects one of six native Rust platform packages. The crates.io package builds the same `gwt` binary from source.

## License

MIT
