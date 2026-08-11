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
});

describe("keys", () => {
  test.each([["j", "down"], ["\x1b[B", "down"], ["k", "up"], ["\x1b[A", "up"], ["\r", "select"], ["q", "cancel"], ["\x03", "cancel"]] as const)("%p is %s", (input, expected) => {
    expect(parseKey(input)).toBe(expected);
  });

  test("parses multiple keys from one terminal read", () => {
    expect(parseKeys("j\x1b[A\r")).toEqual(["down", "up", "select"]);
  });
});
