import { describe, expect, test } from "bun:test";
import { renderTable, truncate } from "../src/table";
import { formatStatus } from "../src/status";
import { parseKey, parseKeys } from "../src/tui";
import type { WorktreeView } from "../src/types";

const clean: WorktreeView = {
  path: "/repo/work tree-你好", head: "1234567890abcdef", branch: "main", detached: false,
  bare: false, current: true, availability: "available",
  status: { conflicted: 0, staged: 0, modified: 0, untracked: 0 },
};

const ANSI = /\x1b\[[0-9;]*m/g;
const stripAnsi = (value: string) => value.replace(ANSI, "");

describe("status and table", () => {
  test("formats clean and dirty status", () => {
    expect(formatStatus(clean)).toBe("clean");
    expect(formatStatus({ ...clean, status: { conflicted: 1, staged: 2, modified: 3, untracked: 4 } }))
      .toBe("1 conflicted, 2 staged, 3 modified, 4 untracked");
    expect(formatStatus({ ...clean, availability: "prunable" })).toBe("prunable");
  });

  test("truncates by terminal display width", () => {
    expect(Bun.stringWidth(truncate("worktree-你好", 8))).toBeLessThanOrEqual(8);
    expect(truncate("abc", 8)).toBe("abc");
  });

  test("renders a narrow table without exceeding width when feasible", () => {
    const lines = renderTable([clean], 50);
    expect(lines).toHaveLength(2);
    expect(lines[1]).toContain("1234567890ab");
    expect(Bun.stringWidth(lines[1]!)).toBeLessThanOrEqual(50);
  });

  test("keeps color-disabled output byte-for-byte compatible", () => {
    expect(renderTable([clean], Number.POSITIVE_INFINITY, { color: false })).toEqual([
      "   BRANCH  PATH                  COMMIT        STATUS",
      "*  main    /repo/work tree-你好  1234567890ab  clean",
    ]);
  });

  test("styles only after laying out ANSI-free cells", () => {
    const lines = renderTable([{ ...clean, path: "/仓库/你好" }], 42, { color: true, selected: 0 });
    expect(lines.join("\n")).toMatch(ANSI);
    expect(lines[0]).toContain("\x1b[37m");
    for (const line of lines) {
      expect(Bun.stringWidth(stripAnsi(line))).toBeLessThanOrEqual(42);
    }
    expect(stripAnsi(lines[1]!)).toContain("你好");
    expect(lines[1]).not.toContain("\x1b[7m");
  });

  test("uses semantic colors for each status category", () => {
    const { branch: _branch, ...withoutBranch } = clean;
    const variants: WorktreeView[] = [
      clean,
      { ...withoutBranch, current: false, detached: true, status: { conflicted: 0, staged: 1, modified: 2, untracked: 3 } },
      { ...clean, current: false, status: { conflicted: 1, staged: 1, modified: 1, untracked: 1 } },
      { ...clean, current: false, availability: "unavailable" },
      { ...clean, current: false, availability: "prunable" },
      { ...clean, current: false, availability: "bare", bare: true },
      { ...clean, current: false, locked: "busy" },
    ];
    const lines = renderTable(variants, Number.POSITIVE_INFINITY, { color: true });
    expect(lines[1]).toContain("\x1b[32m*");
    expect(lines[1]).toContain("\x1b[36mmain");
    expect(lines[1]).toContain("\x1b[2m1234567890ab");
    expect(lines[1]).toContain("\x1b[32mclean");
    expect(lines[2]).toContain("\x1b[33mdetached");
    expect(lines[2]).toContain("\x1b[33m1 staged");
    expect(lines[2]).toContain("\x1b[94m2 modified");
    expect(lines[2]).toContain("\x1b[31m3 untracked");
    expect(lines[3]).toContain("\x1b[31m1 conflicted");
    expect(lines[3]).toContain("\x1b[33m1 staged");
    expect(lines[3]).toContain("\x1b[94m1 modified");
    expect(lines[3]).toContain("\x1b[31m1 untracked");
    expect(lines[4]).toContain("\x1b[31munavailable");
    expect(lines[5]).toContain("\x1b[31mprunable");
    expect(lines[6]).toContain("\x1b[33mbare");
    expect(lines[7]).toContain("\x1b[33mlocked: busy");
  });
});

describe("keys", () => {
  test.each([["j", "down"], ["\x1b[B", "down"], ["k", "up"], ["\x1b[A", "up"], ["\r", "select"], ["q", "cancel"], ["\x03", "cancel"]] as const)("%p is %s", (input, expected) => {
    expect(parseKey(input)).toBe(expected);
  });

  test("parses multiple keys from one terminal read", () => {
    expect(parseKeys("j\x1b[A\r")).toEqual(["down", "up", "select"]);
  });
});
