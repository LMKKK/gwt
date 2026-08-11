# gwt

`gwt` is a small, fast command-line tool for viewing and switching between all worktrees attached to a Git repository. It is developed and tested with Bun, then distributed as standalone macOS and Linux executables through npm.

## Install

```sh
npm install --global @autumn/gwt
```

The package installs the matching standalone executable for macOS or Linux on arm64 or x64. Linux glibc and musl are both supported. Windows is not currently supported.

## Enable directory switching

A child process cannot change its parent shell's directory, so add the integration for your shell to its configuration file.

For zsh (`~/.zshrc`):

```sh
eval "$(gwt init zsh)"
```

For bash (`~/.bashrc`):

```sh
eval "$(gwt init bash)"
```

Restart the shell or source the configuration file. The generated function intercepts only an interactive `gwt list`; every other invocation is forwarded unchanged to the executable.

## Usage

Run this from any directory inside an attached worktree:

```sh
gwt list
```

The initial selection is the current worktree. Use `↑`/`↓` or `k`/`j` to move, Enter to switch, and `q`, Escape, or Ctrl-C to cancel. The table shows the current worktree marker, branch, path, 12-character commit ID, and a status summary. Detached worktrees are labeled `detached`; inaccessible entries are labeled `prunable`, `bare`, or `unavailable`.

In a non-TTY context, `gwt list` prints a plain table and exits without prompting. `NO_COLOR` disables terminal colors. Paths containing spaces and Unicode are supported; paths containing newlines or ending in a newline are outside the initial compatibility target.

Other commands:

```sh
gwt --help
gwt --version
gwt init zsh
gwt init bash
```

## Status counts

Status is summarized as `conflicted`, `staged`, `modified`, and `untracked` file counts. A file with both index and working-tree changes is counted in both `staged` and `modified`. A worktree with no changes is `clean`.

## Common errors

- `Not a Git repository`: run `gwt list` inside a Git repository or one of its attached worktrees.
- `Unable to run Git`: install Git and ensure `git` is available in `PATH`.
- `platform package ... is not installed`: reinstall without npm's `--omit=optional` option.
- Enter prints a path instead of changing directory: install the shell integration shown above.

## Development

Requires Bun 1.2 or later.

```sh
bun install
bun run check
bun run build
bun run pack:check
```

`bun run build` cross-compiles six standalone executables: macOS arm64/x64 and Linux arm64/x64 for glibc and musl. x64 targets use Bun's baseline CPU target. `bun run publish:all` is CI-only and publishes all platform packages before the main package.

The Git integration uses `git worktree list --porcelain -z` and `git status --porcelain=v2 -z`, without constructing shell command strings.
