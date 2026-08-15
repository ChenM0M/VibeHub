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
  ["project-memory.schema.json", "ProjectMemoryProjection"],
  ["agent-spec.schema.json", "V3AgentSpecContract"],
  ["agent-profile.schema.json", "V3AgentProfileContract"],
  ["workspace-state.schema.json", "WorkspaceStateContract"],
  ["session-task-routing.schema.json", "SessionTaskRoutingContract"],
  ["project-overview-view.schema.json", "ProjectOverviewView"],
  ["project-structure-view.schema.json", "ProjectStructureView"],
  ["agent-results-view.schema.json", "AgentResultsView"],
  ["task-timeline-view.schema.json", "TaskTimelineView"],
  ["plan-graph-view.schema.json", "PlanGraphView"],
  ["node-brief.schema.json", "NodeBrief"],
  ["worktree-orchestration-view.schema.json", "WorktreeOrchestrationView"],
  ["usage-overview.schema.json", "UsageOverview"],
];

const exportedDefinitions = new Map([
  ["application-command.schema.json", [
    "V3ApplicationCommand", "PlanNodeAdd", "PlanDependenciesSet", "PlanNodeStateSet",
    "SessionOpen", "EventLog", "SessionClose", "AgentResultRecord", "WorktreeCommand", "Rebuild",
  ]],
  ["task-create.schema.json", ["V3TaskCreateContract", "V3TaskCreateInitialPlanNode", "V3TaskCreateRequest", "V3TaskCreateResult"]],
  ["project-settings.schema.json", [
    "V3ProjectSettingsContract", "V3OutputLanguage", "V3AgentSpecTarget",
    "V3ProjectSettings", "V3ProjectSettingsInspection", "V3ProjectSettingsUpdateRequest",
  ]],
  ["project-memory.schema.json", ["ProjectMemoryProjection", "MemoryEntry"]],
  ["agent-spec.schema.json", [
    "V3AgentSpecContract", "V3AgentSpecArtifactStatus", "V3AgentSpecSyncStatus", "V3AgentSpecArtifactInspection",
    "HostConfigScope", "HostConfigStatus", "HostMcpSyncStatus", "GlobalMcpMigrationStatus",
    "ProjectScopeInspection", "EffectiveAgentDeclaration", "McpHostConfigInspection",
    "HostMcpSyncResult", "GlobalMcpMigrationResult",
    "V3AgentSpecInspection", "V3AgentSpecSyncRequest", "V3AgentSpecSyncResult",
  ]],
  ["agent-profile.schema.json", [
    "V3AgentProfileContract", "AgentKind", "AgentProfileReadResult", "AgentProfileDiscoverResult",
    "AgentProfileSummary", "AgentProfileCommand", "CreateCommand", "CloneCommand", "RenameCommand", "DeleteCommand",
    "AgentProfileSaveResult", "AgentProfileDiagnosticsResult", "AgentProfileDocument", "RuntimeTarget", "NativePath",
    "ConfigSource", "ConfigRevision", "CredentialReference", "ThinkingProfile", "ModelProfile",
    "ProviderProfile", "ManagedProfileFields", "DefaultState", "DefaultProjection",
    "PreservationSummary", "ProtocolCapability", "SchemaCapability", "LaunchSpec",
  ]],
  ["workspace-state.schema.json", [
    "WorkspaceStateContract", "WorkspaceState", "WorkspaceProjectIdentity", "WorkspaceProjectUiContext",
    "WorkspaceStateProvenance", "WorkspaceStateDiagnostic", "WorkspaceStateReadResult",
    "WorkspaceStateSaveRequest", "WorkspaceStateSaveResult",
  ]],
  ["session-task-routing.schema.json", [
    "SessionTaskRoutingContract", "BindingStatus", "BindingFreshness", "BindingSource", "RouteAction",
    "RouteTrigger", "HostCapabilityState", "HostCapabilities", "SessionTaskIdentity", "SessionTaskBinding",
    "TaskRouteCandidate", "RouteOption", "RouteRequest", "TaskRouteDecision",
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
