import { describe, expect, test } from "bun:test";
import { runCli, VERSION } from "../src/cli";
import { shellInit } from "../src/shell";

function output() {
  let value = "";
  return { stream: { write(chunk: string) { value += chunk; return true; }, isTTY: false } as unknown as NodeJS.WriteStream, get: () => value };
}

describe("CLI", () => {
  test("prints help and version", async () => {
    const stdout = output(); const stderr = output();
    expect(await runCli(["--version"], { stdin: process.stdin, stdout: stdout.stream, stderr: stderr.stream })).toBe(0);
    expect(stdout.get()).toBe(`${VERSION}\n`);
  });

  test("rejects unknown commands", async () => {
    const stdout = output(); const stderr = output();
    expect(await runCli(["wat"], { stdin: process.stdin, stdout: stdout.stream, stderr: stderr.stream })).toBe(2);
    expect(stderr.get()).toContain("unknown command 'wat'");
  });

  test("prints an ANSI-free table when stdout is not a TTY", async () => {
    const stdout = output(); const stderr = output();
    expect(await runCli(["list"], { stdin: process.stdin, stdout: stdout.stream, stderr: stderr.stream })).toBe(0);
    expect(stdout.get()).toContain("BRANCH");
    expect(stdout.get()).not.toMatch(/\x1b\[[0-9;]*m/);
  });
});

describe("shell initialization", () => {
  test.each(["zsh", "bash"] as const)("generates valid %s syntax", async (shell) => {
    const script = shellInit(shell);
    expect(script).toContain('builtin cd -- "$_gwt_path"');
    const proc = Bun.spawn([shell, "-n"], { stdin: "pipe", stdout: "pipe", stderr: "pipe" });
    proc.stdin.write(script);
    proc.stdin.end();
    expect(await proc.exited).toBe(0);
  });
});
