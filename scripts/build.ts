import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { targets } from "./targets";

for (const item of targets) {
  const directory = join(import.meta.dir, "..", "packages", item.package, "bin");
  await mkdir(directory, { recursive: true });
  const output = join(directory, "gwt");
  console.log(`Building ${item.package} (${item.target})`);
  const child = Bun.spawn([
    "bun", "build", join(import.meta.dir, "..", "src", "index.ts"),
    "--compile", `--target=${item.target}`, `--outfile=${output}`, "--minify",
  ], { stdout: "inherit", stderr: "inherit" });
  const code = await child.exited;
  if (code !== 0) process.exit(code);
}
