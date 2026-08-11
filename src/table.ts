import type { WorktreeView } from "./types";
import { formatStatus } from "./status";

const HEADERS = ["", "BRANCH", "PATH", "COMMIT", "STATUS"] as const;

function width(value: string): number {
  return Bun.stringWidth(value.replace(/\x1b\[[0-9;]*m/g, ""));
}

export function truncate(value: string, maxWidth: number): string {
  if (maxWidth <= 0) return "";
  if (width(value) <= maxWidth) return value;
  if (maxWidth === 1) return "…";
  let result = "";
  for (const character of value) {
    if (width(result + character) > maxWidth - 1) break;
    result += character;
  }
  return result + "…";
}

function pad(value: string, target: number): string {
  return value + " ".repeat(Math.max(0, target - width(value)));
}

function rows(worktrees: WorktreeView[]): string[][] {
  return worktrees.map((worktree) => [
    worktree.current ? "*" : "",
    worktree.branch ?? "detached",
    worktree.path,
    worktree.head ? worktree.head.slice(0, 12) : "-",
    formatStatus(worktree),
  ]);
}

export function renderTable(worktrees: WorktreeView[], terminalWidth = 120): string[] {
  const data = rows(worktrees);
  const allRows = [Array.from(HEADERS), ...data];
  const natural = HEADERS.map((_, column) => Math.max(...allRows.map((row) => width(row[column] ?? ""))));
  const gaps = (HEADERS.length - 1) * 2;
  const minimums = [1, 3, 3, 4, 3];
  const widths = [...natural];
  while (widths.reduce((sum, item) => sum + item, gaps) > terminalWidth) {
    let candidate = -1;
    let excess = 0;
    for (let column = 0; column < widths.length; column++) {
      const room = widths[column]! - minimums[column]!;
      if (room > excess) { candidate = column; excess = room; }
    }
    if (candidate === -1) break;
    widths[candidate]!--;
  }
  const format = (row: readonly string[]) => row.map((cell, column) => {
    const clipped = truncate(cell, widths[column]!);
    return column === row.length - 1 ? clipped : pad(clipped, widths[column]!);
  }).join("  ");
  return [format(HEADERS), ...data.map(format)];
}
