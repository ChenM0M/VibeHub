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
  ["task-create.schema.json", "V3TaskCreateContract"],
  ["project-settings.schema.json", "V3ProjectSettingsContract"],
  ["agent-spec.schema.json", "V3AgentSpecContract"],
  ["project-overview-view.schema.json", "ProjectOverviewView"],
  ["project-structure-view.schema.json", "ProjectStructureView"],
  ["agent-results-view.schema.json", "AgentResultsView"],
  ["task-timeline-view.schema.json", "TaskTimelineView"],
  ["plan-graph-view.schema.json", "PlanGraphView"],
  ["node-brief.schema.json", "NodeBrief"],
  ["worktree-orchestration-view.schema.json", "WorktreeOrchestrationView"],
];

const exportedDefinitions = new Map([
  ["application-command.schema.json", [
    "V3ApplicationCommand", "PlanNodeAdd", "PlanDependenciesSet", "PlanNodeStateSet",
    "SessionOpen", "EventLog", "SessionClose", "AgentResultRecord", "WorktreeCommand", "Rebuild",
  ]],
  ["task-create.schema.json", ["V3TaskCreateContract", "V3TaskCreateRequest", "V3TaskCreateResult"]],
  ["project-settings.schema.json", [
    "V3ProjectSettingsContract", "V3OutputLanguage", "V3AgentSpecTarget",
    "V3ProjectSettings", "V3ProjectSettingsInspection", "V3ProjectSettingsUpdateRequest",
  ]],
  ["agent-spec.schema.json", [
    "V3AgentSpecContract", "V3AgentSpecArtifactStatus", "V3AgentSpecArtifactInspection",
    "V3AgentSpecInspection", "V3AgentSpecSyncRequest", "V3AgentSpecSyncResult",
  ]],
]);

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
    ignoreMinAndMaxItems: true,
  });
  await writeFile(resolve(outputRoot, outputName), source, "utf8");
  const moduleName = outputName.replace(".ts", "");
  const names = exportedDefinitions.get(filename) ?? [typeName];
  exports.push(`export type { ${names.join(", ")} } from "./${moduleName}";`);
}
await writeFile(resolve(outputRoot, "index.ts"), `${exports.join("\n")}\n`, "utf8");
console.log(`Generated V3 TypeScript contracts at ${outputRoot}`);
