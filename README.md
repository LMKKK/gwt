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

Use `gwt list`, move with arrow keys or `j`/`k`, press Enter to select, and press `q` to cancel. Use `gwt new` to select an unused local branch, edit the new worktree path (the branch name is the default), create it, and switch to it. Branches already checked out in another worktree remain visible with that worktree's path, but cannot be selected. Relative paths are resolved from the directory where you invoked `gwt`; `~` and shell variables are not expanded.

Use `gwt remove` to select a linked worktree and confirm its removal. The main worktree, the worktree containing the current directory, and unavailable or prunable entries are excluded. Dirty and locked worktrees remain visible, but Git refuses to remove them because `gwt` never uses `--force`. Removing a worktree does not delete its local branch.

In the leftmost column, `M` marks Git's main worktree and `*` marks the worktree containing the current directory. `M*` means the main worktree is also current.

The internal `gwt list --select` and `gwt new --select` commands print a selected or newly created absolute path for shell integration. Their interactive interfaces are written to stderr. Set `NO_COLOR` to disable color.

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
