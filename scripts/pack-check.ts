import { readdir } from "node:fs/promises";
import { join } from "node:path";

const packagesDir = join(import.meta.dir, "..", "packages");
for (const entry of (await readdir(packagesDir, { withFileTypes: true })).filter((item) => item.isDirectory())) {
  const cwd = join(packagesDir, entry.name);
  console.log(`Checking npm package ${entry.name}`);
  const child = Bun.spawn(["npm", "pack", "--dry-run"], { cwd, stdout: "inherit", stderr: "inherit" });
  const code = await child.exited;
  if (code !== 0) process.exit(code);
}
