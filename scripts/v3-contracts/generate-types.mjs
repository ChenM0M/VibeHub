import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compile } from "json-schema-to-typescript";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "../..");
const contractRoot = resolve(projectRoot, "contracts/v3");
const outputRoot = resolve(projectRoot, process.argv[2] ?? "src/v3/contracts/generated");
const schemas = [
  ["event-envelope.schema.json", "V3EventEnvelope"],
  ["application-command.schema.json", "V3ApplicationCommand"],
  ["project-overview-view.schema.json", "ProjectOverviewView"],
  ["project-structure-view.schema.json", "ProjectStructureView"],
  ["task-timeline-view.schema.json", "TaskTimelineView"],
  ["plan-graph-view.schema.json", "PlanGraphView"],
  ["node-brief.schema.json", "NodeBrief"],
  ["worktree-orchestration-view.schema.json", "WorktreeOrchestrationView"],
];

await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });
const exports = [];
for (const [filename, typeName] of schemas) {
  const schema = JSON.parse(await readFile(resolve(contractRoot, filename), "utf8"));
  const outputName = filename.replace(".schema.json", ".ts");
  const source = await compile(schema, typeName, {
    cwd: contractRoot,
    bannerComment: "/* Generated from contracts/v3. Do not edit directly. */",
    style: { singleQuote: false, semi: true, tabWidth: 2, trailingComma: "all" },
    unreachableDefinitions: false,
  });
  await writeFile(resolve(outputRoot, outputName), source, "utf8");
  exports.push(`export type { ${typeName} } from "./${outputName.replace(".ts", "")}";`);
}
await writeFile(resolve(outputRoot, "index.ts"), `${exports.join("\n")}\n`, "utf8");
console.log(`Generated V3 TypeScript contracts at ${outputRoot}`);
