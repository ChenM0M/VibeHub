import { mkdir, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildFixtureTree } from "./fixture-data.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "../..");
const outputRoot = resolve(projectRoot, process.argv[2] ?? "fixtures/v3");

await rm(outputRoot, { recursive: true, force: true });
for (const [relativePath, content] of buildFixtureTree()) {
  const outputPath = resolve(outputRoot, relativePath);
  await mkdir(dirname(outputPath), { recursive: true });
  await writeFile(outputPath, content, "utf8");
}

console.log(`Generated V3 fixtures at ${outputRoot}`);
