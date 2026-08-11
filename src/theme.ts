import { createColors } from "picocolors";
import type { WorktreeView } from "./types";

export interface TerminalTheme {
  header(value: string): string;
  current(value: string): string;
  branch(value: string): string;
  detached(value: string): string;
  commit(value: string): string;
  path(value: string, selected: boolean): string;
  status(value: string, worktree: WorktreeView): string;
  selection(value: string): string;
  hint(value: string): string;
  key(value: string): string;
}

export function createTerminalTheme(color: boolean): TerminalTheme {
  const colors = createColors(color);
  const boldWhite = (value: string) => colors.bold(colors.white(value));
  const boldCyan = (value: string) => colors.bold(colors.cyan(value));

  return {
    header: boldWhite,
    current: (value) => colors.bold(colors.green(value)),
    branch: colors.cyan,
    detached: colors.yellow,
    commit: colors.dim,
    path: (value, selected) => selected ? colors.bold(value) : value,
    status: (value, worktree) => {
      if (worktree.availability === "prunable" || worktree.availability === "unavailable") {
        return colors.red(value);
      }
      if (worktree.availability === "bare") return colors.yellow(value);
      if (!worktree.status) return colors.red(value);

      const parts: string[] = [];
      if (worktree.status.conflicted) parts.push(colors.red(`${worktree.status.conflicted} conflicted`));
      if (worktree.status.staged) parts.push(colors.yellow(`${worktree.status.staged} staged`));
      if (worktree.status.modified) parts.push(colors.blueBright(`${worktree.status.modified} modified`));
      if (worktree.status.untracked) parts.push(colors.red(`${worktree.status.untracked} untracked`));
      if (worktree.locked !== undefined) {
        parts.push(colors.yellow(worktree.locked ? `locked: ${worktree.locked}` : "locked"));
      }
      return parts.length ? parts.join(", ") : colors.green(value);
    },
    selection: boldCyan,
    hint: colors.dim,
    key: colors.bold,
  };
}
