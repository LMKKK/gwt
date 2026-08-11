import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, mkdir, realpath, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { listWorktrees, parseStatusPorcelainV2, parseWorktreePorcelain } from "../src/git";

describe("parseWorktreePorcelain", () => {
  test("parses branch, detached, bare, locked and prunable records", () => {
    const input = [
      "worktree /repo", "HEAD abcdef", "branch refs/heads/main", "locked maintenance", "",
      "worktree /repo/detached", "HEAD 123456", "detached", "",
      "worktree /repo/missing", "HEAD fedcba", "prunable gitdir file points to non-existent location", "",
      "worktree /repo.git", "HEAD 000000", "bare", "", "",
    ].join("\0");
    expect(parseWorktreePorcelain(input)).toEqual([
      { path: "/repo", head: "abcdef", branch: "main", detached: false, bare: false, locked: "maintenance" },
      { path: "/repo/detached", head: "123456", detached: true, bare: false },
      { path: "/repo/missing", head: "fedcba", detached: false, bare: false, prunable: "gitdir file points to non-existent location" },
      { path: "/repo.git", head: "000000", detached: false, bare: true },
    ]);
  });
});

describe("parseStatusPorcelainV2", () => {
  test("counts status dimensions independently", () => {
    const input = [
      "1 M. N... 100644 100644 100644 a a staged.txt",
      "1 .M N... 100644 100644 100644 a a modified.txt",
      "1 MM N... 100644 100644 100644 a a both.txt",
      "? untracked.txt",
      "u UU N... 100644 100644 100644 100644 a b c conflict.txt",
      "",
    ].join("\0");
    expect(parseStatusPorcelainV2(input)).toEqual({ conflicted: 1, staged: 2, modified: 2, untracked: 1 });
  });

  test("skips the original path following a rename", () => {
    const input = "2 R. N... 100644 100644 100644 a a R100 new name\0old name\0? next\0";
    expect(parseStatusPorcelainV2(input)).toEqual({ conflicted: 0, staged: 1, modified: 0, untracked: 1 });
  });
});

describe("real Git repository", () => {
  const directories: string[] = [];
  afterEach(async () => Promise.all(directories.splice(0).map((path) => rm(path, { recursive: true, force: true }))));

  async function git(cwd: string, ...args: string[]) {
    const proc = Bun.spawn(["git", ...args], { cwd, stdout: "pipe", stderr: "pipe", env: {
      ...process.env, GIT_AUTHOR_NAME: "gwt test", GIT_AUTHOR_EMAIL: "gwt@example.invalid",
      GIT_COMMITTER_NAME: "gwt test", GIT_COMMITTER_EMAIL: "gwt@example.invalid",
    } });
    const [stdout, stderr, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
    if (code !== 0) throw new Error(stderr);
    return stdout;
  }

  test("lists worktrees from a subdirectory and preserves spaces and Unicode", async () => {
    const base = await mkdtemp(join(tmpdir(), "gwt-test-"));
    directories.push(base);
    const root = join(base, "main");
    await mkdir(root);
    await git(root, "init", "-b", "main");
    await Bun.write(join(root, "tracked.txt"), "initial\n");
    await git(root, "add", "tracked.txt");
    await git(root, "commit", "-m", "initial");
    const linked = join(base, "work tree-你好");
    await git(root, "worktree", "add", "-b", "feature", linked);
    await Bun.write(join(linked, "tracked.txt"), "changed\n");
    await Bun.write(join(linked, "new file.txt"), "new\n");
    await git(linked, "add", "new file.txt");
    await Bun.write(join(linked, "untracked.txt"), "new\n");
    const nested = join(linked, "nested");
    await mkdir(nested);

    const worktrees = await listWorktrees(nested);
    const realRoot = await realpath(root);
    expect(worktrees).toHaveLength(2);
    const current = worktrees.find((item) => item.current);
    expect(current?.path).toBe(await realpath(linked));
    expect(current?.branch).toBe("feature");
    expect(current?.status).toEqual({ conflicted: 0, staged: 1, modified: 1, untracked: 1 });
    expect(worktrees.find((item) => item.path === realRoot)?.status).toEqual({ conflicted: 0, staged: 0, modified: 0, untracked: 0 });
  });

  test("reports a bare repository without attempting status", async () => {
    const bare = await mkdtemp(join(tmpdir(), "gwt-bare-"));
    directories.push(bare);
    await git(bare, "init", "--bare");
    const worktrees = await listWorktrees(bare);
    expect(worktrees).toHaveLength(1);
    expect(worktrees[0]?.bare).toBe(true);
    expect(worktrees[0]?.availability).toBe("bare");
  });

  test("counts a real merge conflict", async () => {
    const root = await mkdtemp(join(tmpdir(), "gwt-conflict-"));
    directories.push(root);
    await git(root, "init", "-b", "main");
    await Bun.write(join(root, "conflict.txt"), "base\n");
    await git(root, "add", "conflict.txt");
    await git(root, "commit", "-m", "base");
    await git(root, "switch", "-c", "other");
    await Bun.write(join(root, "conflict.txt"), "other\n");
    await git(root, "commit", "-am", "other");
    await git(root, "switch", "main");
    await Bun.write(join(root, "conflict.txt"), "main\n");
    await git(root, "commit", "-am", "main");
    const merge = Bun.spawn(["git", "merge", "other"], { cwd: root, stdout: "ignore", stderr: "ignore" });
    expect(await merge.exited).not.toBe(0);

    const worktrees = await listWorktrees(root);
    expect(worktrees[0]?.status).toEqual({ conflicted: 1, staged: 0, modified: 0, untracked: 0 });
  });
});
