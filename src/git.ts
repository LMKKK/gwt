import { existsSync, realpathSync } from "node:fs";
import { relative, resolve } from "node:path";
import type { Availability, StatusCounts, WorktreeRecord, WorktreeView } from "./types";

export class GitError extends Error {
  constructor(message: string, readonly causeText?: string) {
    super(message);
    this.name = "GitError";
  }
}

export function parseWorktreePorcelain(input: Uint8Array | string): WorktreeRecord[] {
  const text = typeof input === "string" ? input : new TextDecoder().decode(input);
  const fields = text.split("\0");
  const records: WorktreeRecord[] = [];
  let current: WorktreeRecord | undefined;

  for (const field of fields) {
    if (field === "") {
      if (current) {
        records.push(current);
        current = undefined;
      }
      continue;
    }

    const separator = field.indexOf(" ");
    const key = separator === -1 ? field : field.slice(0, separator);
    const value = separator === -1 ? "" : field.slice(separator + 1);
    if (key === "worktree") {
      if (current) records.push(current);
      current = { path: value, head: "", detached: false, bare: false };
      continue;
    }
    if (!current) continue;

    switch (key) {
      case "HEAD": current.head = value; break;
      case "branch": current.branch = value.replace(/^refs\/heads\//, ""); break;
      case "detached": current.detached = true; break;
      case "bare": current.bare = true; break;
      case "locked": current.locked = value; break;
      case "prunable": current.prunable = value; break;
    }
  }
  if (current) records.push(current);
  return records;
}

export function parseStatusPorcelainV2(input: Uint8Array | string): StatusCounts {
  const text = typeof input === "string" ? input : new TextDecoder().decode(input);
  const counts: StatusCounts = { conflicted: 0, staged: 0, modified: 0, untracked: 0 };
  const entries = text.split("\0");

  for (let index = 0; index < entries.length; index++) {
    const entry = entries[index];
    if (!entry) continue;
    if (entry.startsWith("? ")) {
      counts.untracked++;
      continue;
    }
    if (entry.startsWith("u ")) {
      counts.conflicted++;
      continue;
    }
    if (entry.startsWith("1 ") || entry.startsWith("2 ")) {
      const xy = entry.slice(2, 4);
      if (xy[0] !== ".") counts.staged++;
      if (xy[1] !== ".") counts.modified++;
      // Renames/copies have a second NUL-delimited path.
      if (entry.startsWith("2 ")) index++;
    }
  }
  return counts;
}

async function runGit(args: string[], cwd: string): Promise<Uint8Array> {
  const child = (() => {
    try {
      return Bun.spawn(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe" });
    } catch (error) {
      throw new GitError("Unable to run Git. Make sure 'git' is installed and available in PATH.", String(error));
    }
  })();
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(child.stdout).bytes(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  if (exitCode !== 0) {
    const detail = stderr.trim();
    if (detail.includes("not a git repository")) {
      throw new GitError(`Not a Git repository: ${cwd}`, detail);
    }
    throw new GitError(`Git command failed: git ${args.join(" ")}${detail ? `\n${detail}` : ""}`, detail);
  }
  return stdout;
}

function availabilityOf(record: WorktreeRecord): Availability {
  if (record.prunable !== undefined) return "prunable";
  if (record.bare) return "bare";
  if (!existsSync(record.path)) return "unavailable";
  return "available";
}

function samePathOrInside(candidate: string, root: string): boolean {
  const canonical = (path: string) => {
    try { return realpathSync(path); } catch { return resolve(path); }
  };
  const difference = relative(canonical(root), canonical(candidate));
  return difference === "" || (!difference.startsWith("..") && !difference.startsWith("/"));
}

export async function listWorktrees(cwd = process.cwd()): Promise<WorktreeView[]> {
  const records = parseWorktreePorcelain(await runGit(["worktree", "list", "--porcelain", "-z"], cwd));
  const currentIndex = records.reduce((best, record, index) => {
    if (!samePathOrInside(cwd, record.path)) return best;
    return best === -1 || record.path.length > records[best]!.path.length ? index : best;
  }, -1);

  return Promise.all(records.map(async (record, index): Promise<WorktreeView> => {
    const availability = availabilityOf(record);
    if (availability !== "available") return { ...record, current: index === currentIndex, availability };
    try {
      const output = await runGit(["status", "--porcelain=v2", "-z", "--untracked-files=all"], record.path);
      return { ...record, current: index === currentIndex, availability, status: parseStatusPorcelainV2(output) };
    } catch {
      return { ...record, current: index === currentIndex, availability: "unavailable" };
    }
  }));
}
