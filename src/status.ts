import type { WorktreeView } from "./types";

export function formatStatus(worktree: WorktreeView): string {
  if (worktree.availability !== "available") return worktree.availability;
  const counts = worktree.status;
  if (!counts) return "unavailable";
  const parts: string[] = [];
  if (counts.conflicted) parts.push(`${counts.conflicted} conflicted`);
  if (counts.staged) parts.push(`${counts.staged} staged`);
  if (counts.modified) parts.push(`${counts.modified} modified`);
  if (counts.untracked) parts.push(`${counts.untracked} untracked`);
  if (worktree.locked !== undefined) parts.push(worktree.locked ? `locked: ${worktree.locked}` : "locked");
  return parts.length ? parts.join(", ") : "clean";
}
