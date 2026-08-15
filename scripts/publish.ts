import { join } from "node:path";
import { targets } from "./targets";

if (!process.env.CI) {
  console.error("Refusing to publish outside CI. Set CI=true after authenticating npm.");
  process.exit(1);
}

for (const name of [...targets.map((item) => item.package), "gwt"]) {
  console.log(`Publishing @autumn-k/${name}`);
  const child = Bun.spawn(["npm", "publish", "--access", "public", "--provenance"], {
    cwd: join(import.meta.dir, "..", "packages", name), stdout: "inherit", stderr: "inherit",
  });
  const code = await child.exited;
  if (code !== 0) process.exit(code);
}
