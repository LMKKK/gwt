import { listWorktrees, GitError } from "./git";
import { shellInit } from "./shell";
import { renderTable } from "./table";
import { selectWorktree } from "./tui";

export const VERSION = "0.1.0";

const HELP = `gwt ${VERSION} — switch between Git worktrees

Usage:
  gwt list          List worktrees (interactive in a terminal)
  gwt init zsh      Print zsh integration
  gwt init bash     Print bash integration
  gwt --help        Show help
  gwt --version     Show version

Install shell integration to make Enter change the current shell directory:
  eval "$(gwt init zsh)"`;

export interface CliIO {
  stdout: NodeJS.WriteStream;
  stderr: NodeJS.WriteStream;
  stdin: NodeJS.ReadStream;
}

export async function runCli(args: string[], io: CliIO = process): Promise<number> {
  if (args.length === 0 || args[0] === "--help" || args[0] === "-h" || args[0] === "help") {
    io.stdout.write(HELP + "\n");
    return 0;
  }
  if (args[0] === "--version" || args[0] === "-v") {
    io.stdout.write(VERSION + "\n");
    return 0;
  }
  if (args[0] === "init") {
    const shell = args[1];
    if ((shell !== "zsh" && shell !== "bash") || args.length !== 2) {
      io.stderr.write("Usage: gwt init zsh|bash\n");
      return 2;
    }
    io.stdout.write(shellInit(shell) + "\n");
    return 0;
  }
  if (args[0] === "list") {
    const select = args[1] === "--select" && args.length === 2;
    if ((!select && args.length !== 1) || (args.length > 2)) {
      io.stderr.write("Usage: gwt list\n");
      return 2;
    }
    try {
      const worktrees = await listWorktrees();
      const interactive = io.stdin.isTTY && (select ? io.stderr.isTTY : io.stdout.isTTY);
      if (interactive) {
        const path = await selectWorktree(worktrees, { input: io.stdin, output: select ? io.stderr : io.stdout });
        if (select && path) io.stdout.write(path + "\n");
      } else {
        if (select) {
          io.stderr.write("gwt list --select requires an interactive terminal.\n");
          return 2;
        }
        io.stdout.write(renderTable(worktrees, Number.POSITIVE_INFINITY).join("\n") + "\n");
      }
      return 0;
    } catch (error) {
      const message = error instanceof GitError ? error.message : `Unexpected error: ${String(error)}`;
      io.stderr.write(`gwt: ${message}\n`);
      return 1;
    }
  }
  io.stderr.write(`gwt: unknown command '${args[0]}'. Run 'gwt --help' for usage.\n`);
  return 2;
}
