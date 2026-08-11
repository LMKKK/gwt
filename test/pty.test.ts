import { afterEach, describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";

describe("shell integration in a pseudo-terminal", () => {
  const directories: string[] = [];
  afterEach(async () => Promise.all(directories.splice(0).map((path) => rm(path, { recursive: true, force: true }))));

  async function git(cwd: string, ...args: string[]) {
    const child = Bun.spawn(["git", ...args], { cwd, stdout: "ignore", stderr: "pipe", env: {
      ...process.env, GIT_AUTHOR_NAME: "gwt test", GIT_AUTHOR_EMAIL: "gwt@example.invalid",
      GIT_COMMITTER_NAME: "gwt test", GIT_COMMITTER_EMAIL: "gwt@example.invalid",
    } });
    const [stderr, code] = await Promise.all([new Response(child.stderr).text(), child.exited]);
    if (code !== 0) throw new Error(stderr);
  }

  for (const shell of ["zsh", "bash"] as const) {
    test(`${shell}: selection changes directory and cancellation keeps it`, async () => {
      const expectBinary = Bun.which("expect");
      const shellBinary = Bun.which(shell);
      const bunBinary = Bun.which("bun");
      if (!expectBinary || !shellBinary || !bunBinary) return;

      const base = await mkdtemp(join(tmpdir(), `gwt-${shell}-pty-`));
      directories.push(base);
      const main = join(base, "main");
      const linked = join(base, "work tree-你好");
      const bin = join(base, "bin");
      await mkdir(main);
      await mkdir(bin);
      await git(main, "init", "-b", "main");
      await git(main, "commit", "--allow-empty", "-m", "initial");
      await git(main, "worktree", "add", "-b", "feature", linked);
      const launcher = join(bin, "gwt");
      await Bun.write(launcher, `#!/bin/sh\nexec "${bunBinary}" "${join(import.meta.dir, "..", "src", "index.ts")}" "$@"\n`);
      await chmod(launcher, 0o755);
      const expectedPath = await realpath(linked);
      const flags = shell === "zsh" ? "-f" : "--noprofile --norc";
      const initialPrompt = shell === "zsh" ? "[%#$] " : "bash[^#$]*[$#] ";
      const script = `
set timeout 15
spawn -noecho ${shellBinary} ${flags}
expect -re {${initialPrompt}}
send -- "PS1=GWT_PROMPT\\r"
expect "GWT_PROMPT"
send -- "eval \\\"\\\$(gwt init ${shell})\\\"\\r"
expect "GWT_PROMPT"
send -- "cd \\\"${main}\\\"\\r"
expect "GWT_PROMPT"
send -- "gwt list\\r"
expect "Enter select"
send -- "j\\r"
expect "GWT_PROMPT"
send -- "printf \\\"__PWD__%s\\\\n\\\" \\\"\\\$PWD\\\"\\r"
expect "__PWD__${expectedPath}"
expect "GWT_PROMPT"
send -- "gwt list\\r"
expect "Enter select"
send -- "q"
expect "GWT_PROMPT"
send -- "printf \\\"__CANCEL__%s\\\\n\\\" \\\"\\\$PWD\\\"\\r"
expect "__CANCEL__${expectedPath}"
send -- "exit\\r"
expect eof
`;
      const child = Bun.spawn([expectBinary, "-c", script], {
        env: { ...process.env, PATH: `${bin}:${process.env.PATH ?? ""}` }, stdout: "pipe", stderr: "pipe",
      });
      const [stdout, stderr, code] = await Promise.all([
        new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited,
      ]);
      expect(code, `${stdout}\n${stderr}`).toBe(0);
      expect(stdout).toContain(`__PWD__${expectedPath}`);
      expect(stdout).toContain(`__CANCEL__${expectedPath}`);
      expect(stdout).toContain("\x1b[?25h");
    }, 30_000);
  }
});
