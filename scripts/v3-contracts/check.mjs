import { execFile } from "node:child_process";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, relative, resolve } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import ts from "typescript";
import { buildFixtureTree, CONTRACT_FILES, CONTRACT_VERSION, sha256 } from "./fixture-data.mjs";

const execFileAsync = promisify(execFile);
const scriptDir = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(scriptDir, "../..");
const contractRoot = resolve(projectRoot, "contracts/v3");
const fixtureRoot = resolve(projectRoot, "fixtures/v3");
const generatedRoot = resolve(projectRoot, "src/v3/contracts/generated");
const schemaNames = [
  "common.schema.json",
  "event-envelope.schema.json",
  "application-command.schema.json",
  "task-create.schema.json",
  "project-settings.schema.json",
  "project-memory.schema.json",
  "agent-spec.schema.json",
  "agent-profile.schema.json",
  "workspace-state.schema.json",
  "session-task-routing.schema.json",
  "project-overview-view.schema.json",
  "project-structure-view.schema.json",
  "agent-results-view.schema.json",
  "task-timeline-view.schema.json",
  "plan-graph-view.schema.json",
  "node-brief.schema.json",
  "worktree-orchestration-view.schema.json",
  "usage-overview.schema.json",
  "task-list-view.schema.json",
  "task-commits-view.schema.json",
  "commit-tasks-view.schema.json",
];
const requiredScenarios = [
  "FX-EMPTY", "FX-HAPPY", "FX-NO-DOCS", "FX-PARALLEL", "FX-REWORK", "FX-STALE",
  "FX-PARTIAL", "FX-ERROR", "FX-WIN-PATHS", "FX-MAC-PATHS", "FX-LARGE", "FX-COVERAGE-GAP",
];

const failures = [];
const passed = [];
const assert = (condition, message) => condition ? passed.push(message) : failures.push(message);
const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));
const portableRelative = (root, path) => relative(root, path).replaceAll("\\", "/");

async function listFiles(root) {
  const result = [];
  for (const entry of await readdir(root, { withFileTypes: true })) {
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) result.push(...await listFiles(path));
    else result.push(path);
  }
  return result.sort();
}

const schemas = new Map();
const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: true });
addFormats(ajv);
for (const name of schemaNames) {
  const schema = await readJson(resolve(contractRoot, name));
  schemas.set(name, schema);
  assert(schema.$schema === "https://json-schema.org/draft/2020-12/schema", `${name} declares JSON Schema 2020-12`);
  assert(schema.$id?.includes(`/v3/${CONTRACT_VERSION}/`), `${name} has a versioned stable $id`);
  assert(ajv.validateSchema(schema), `${name} passes meta-schema validation`);
  ajv.addSchema(schema);
}

const manifest = await readJson(resolve(fixtureRoot, "manifest.json"));
assert(manifest.schema_version === CONTRACT_VERSION, "fixture manifest matches contract version");
assert(JSON.stringify(manifest.scenarios.map((item) => item.scenario_id)) === JSON.stringify(requiredScenarios), "fixture manifest covers all 12 scenarios in stable order");
assert(new Set(manifest.scenarios.flatMap((item) => item.criteria)).size >= 8, "fixture manifest maps scenarios to M0 criteria");

for (const scenario of manifest.scenarios) {
  const scenarioManifest = await readJson(resolve(fixtureRoot, scenario.manifest));
  assert(scenarioManifest.contracts.length === Object.keys(CONTRACT_FILES).length, `${scenario.scenario_id} covers all view contracts`);
  for (const contract of scenarioManifest.contracts) {
    const content = await readFile(resolve(fixtureRoot, scenario.scenario_id, contract.file), "utf8");
    const instance = JSON.parse(content);
    const validator = ajv.getSchema(schemas.get(contract.schema).$id);
    const valid = validator(instance);
    assert(valid, `${scenario.scenario_id}/${contract.file} validates${valid ? "" : `: ${ajv.errorsText(validator.errors)}`}`);
    assert(sha256(content) === contract.sha256, `${scenario.scenario_id}/${contract.file} matches manifest SHA-256`);
  }

  const structure = await readJson(resolve(fixtureRoot, scenario.scenario_id, "project-structure.json"));
  const overview = await readJson(resolve(fixtureRoot, scenario.scenario_id, "project-overview.json"));
  const architectureNodeIds = structure.architecture_nodes.map((node) => node.node_id);
  const architectureEdgeIds = structure.architecture_edges.map((edge) => edge.edge_id);
  const architectureNodeIdSet = new Set(architectureNodeIds);
  const architectureRelations = structure.architecture_edges.map((edge) => `${edge.from_node_id}|${edge.kind}|${edge.to_node_id}`);
  assert(architectureNodeIdSet.size === architectureNodeIds.length, `${scenario.scenario_id} architecture node IDs are unique`);
  assert(new Set(architectureEdgeIds).size === architectureEdgeIds.length, `${scenario.scenario_id} architecture edge IDs are unique`);
  assert(structure.architecture_edges.every((edge) => architectureNodeIdSet.has(edge.from_node_id) && architectureNodeIdSet.has(edge.to_node_id)), `${scenario.scenario_id} architecture edge endpoints exist`);
  assert(structure.architecture_edges.every((edge) => edge.from_node_id !== edge.to_node_id), `${scenario.scenario_id} architecture has no self-edges`);
  assert(new Set(architectureRelations).size === architectureRelations.length, `${scenario.scenario_id} architecture relations are unique`);
  assert(structure.architecture_nodes.every((node) => node.evidence_refs.length > 0), `${scenario.scenario_id} architecture nodes carry evidence`);
  assert(structure.architecture_edges.every((edge) => edge.evidence_refs.length > 0), `${scenario.scenario_id} architecture edges carry evidence`);
  assert(overview.architecture.modules === structure.architecture_nodes.filter((node) => node.kind !== "workspace").length, `${scenario.scenario_id} overview module count matches architecture graph`);
  assert(overview.architecture.relationships === structure.architecture_edges.length, `${scenario.scenario_id} overview relationship count matches architecture graph`);
}

for (const sentinelPath of manifest.invalid_sentinels) {
  const sentinel = await readJson(resolve(fixtureRoot, sentinelPath));
  const validator = ajv.getSchema(schemas.get(sentinel.expected_schema).$id);
  const valid = validator(sentinel.instance);
  const matchingError = validator.errors?.some((item) => item.keyword === sentinel.expected_keyword && item.instancePath === sentinel.expected_instance_path);
  assert(!valid && matchingError, `${sentinelPath} fails at expected keyword/path`);
}

const expectedTree = buildFixtureTree();
const diskFiles = (await listFiles(fixtureRoot)).map((path) => portableRelative(fixtureRoot, path));
assert(JSON.stringify(diskFiles) === JSON.stringify([...expectedTree.keys()].sort()), "fixture tree contains only deterministic generator output");
for (const [relativePath, expected] of expectedTree) {
  const actual = await readFile(resolve(fixtureRoot, relativePath), "utf8");
  assert(actual === expected, `${relativePath} is byte-identical to fixed-seed generation`);
}

const tempRoot = await mkdtemp(resolve(tmpdir(), "vibehub-v3-types-"));
try {
  await execFileAsync(process.execPath, [resolve(scriptDir, "generate-types.mjs"), tempRoot], { cwd: projectRoot });
  const expectedFiles = (await listFiles(tempRoot)).map((path) => portableRelative(tempRoot, path));
  const generatedFiles = (await listFiles(generatedRoot)).map((path) => portableRelative(generatedRoot, path));
  assert(JSON.stringify(expectedFiles) === JSON.stringify(generatedFiles), "generated TypeScript file set has no drift");
  for (const relativePath of expectedFiles) {
    assert(await readFile(resolve(tempRoot, relativePath), "utf8") === await readFile(resolve(generatedRoot, relativePath), "utf8"), `${relativePath} generated TypeScript has no drift`);
  }
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

const v3Sources = (await listFiles(resolve(projectRoot, "src/v3"))).filter((path) => path.endsWith(".ts") || path.endsWith(".tsx"));
const forbiddenDependencies = [
  [/from\s+["'][^"']*\.ya?ml["']/i, "V2 YAML import"],
  [/from\s+["'][^"']*(?:vibehub-core|\.vibehub\/)[^"']*["']/, "V2/core state import"],
];
for (const sourcePath of v3Sources) {
  const source = await readFile(sourcePath, "utf8");
  for (const [pattern, label] of forbiddenDependencies) assert(!pattern.test(source), `${relative(projectRoot, sourcePath)} has no ${label}`);
}
const repositorySource = await readFile(resolve(projectRoot, "src/v3/contracts/fixtureRepository.ts"), "utf8");
for (const filename of Object.values(CONTRACT_FILES)) assert(repositorySource.includes(filename), `fixture repository loads ${filename}`);
assert(repositorySource.includes("Promise.all"), "fixture repository exposes one view-bundle load boundary");

const homeSource = await readFile(resolve(projectRoot, "src/pages/Home.tsx"), "utf8");
const cockpitSource = await readFile(resolve(projectRoot, "src/v3/app/V3Cockpit.tsx"), "utf8");
const storeSource = await readFile(resolve(projectRoot, "src/v3/stores/v3Store.ts"), "utf8");
const productionViewsSource = await readFile(resolve(projectRoot, "src/services/v3ProductionViews.ts"), "utf8");
const legacyV2Source = await readFile(resolve(projectRoot, "src/services/legacyV2.ts"), "utf8");
const localAgentUsageSource = await readFile(resolve(projectRoot, "src-tauri/src/local_agent_usage.rs"), "utf8");
const sourceMatrixSource = await readFile(resolve(projectRoot, "docs/v3/production-data-source-matrix.md"), "utf8");
const usageCapabilitySource = await readFile(resolve(projectRoot, "docs/v3/usage-source-capabilities.md"), "utf8");
const usageSource = await readFile(resolve(projectRoot, "src/v3/components/task/AIUsagePanel.tsx"), "utf8");
const lifecycleSource = await readFile(resolve(projectRoot, "src/v3/components/project/ProjectLifecycleModal.tsx"), "utf8");
const structureSource = await readFile(resolve(projectRoot, "src/v3/components/project/StructureArchitecture.tsx"), "utf8");
const projectCardSource = await readFile(resolve(projectRoot, "src/components/ProjectCard.tsx"), "utf8");
const planGraphSource = await readFile(resolve(projectRoot, "src/v3/components/task/PlanGraph.tsx"), "utf8");
const projectSetupSource = await readFile(resolve(projectRoot, "src/v3/components/project/ProjectSetupModal.tsx"), "utf8");
const generatedContractsSource = await readFile(resolve(projectRoot, "src/v3/contracts/generated/index.ts"), "utf8");
const rendererSources = [homeSource, storeSource, structureSource, planGraphSource, projectSetupSource].join("\n");
assert(planGraphSource.includes("onNodesChange={handleNodesChange}"), "PlanGraph feeds React Flow node changes back into its controlled state");
assert(planGraphSource.includes("measured: nodeMeasurements[node.node_id]"), "PlanGraph preserves measured node dimensions for MiniMap rendering");
assert(planGraphSource.includes("nodeColor={(node) => minimapNodeColor"), "PlanGraph gives MiniMap explicit state colors");
assert(planGraphSource.includes('maxHeight: "calc(100% - 12rem)"'), "PlanGraph reserves space between the inspector panel and MiniMap");
assert(planGraphSource.includes("style={{ width: 180, height: 120 }}"), "PlanGraph keeps MiniMap compact enough for short graph canvases");
for (const legacySurface of ["state.yaml", "agent-view", "adapters/", "commands/", "hooks/", "skills/"]) {
  assert(!rendererSources.includes(legacySurface), `renderer/source does not reference or generate retired ${legacySurface} surface`);
}
for (const generatedExport of ["task-create", "project-settings", "agent-spec", "agent-profile", "application-command"]) {
  assert(generatedContractsSource.includes(`./${generatedExport}`), `generated contract exports include ${generatedExport}`);
}
assert(/projectSettingsApi=\{productionProjectSettingsApi\}/.test(homeSource), "production route injects the real project settings/spec API");
assert(/planApi=\{\{[\s\S]{0,250}v3PlanAddNode[\s\S]{0,100}v3PlanSetDependencies[\s\S]{0,100}v3PlanSetState/.test(homeSource), "production route injects the real plan mutation API");
assert(!/initialSourceMode="fixture"[\s\S]{0,500}(projectSettingsApi|planApi)=/.test(homeSource), "fixture route receives no settings/spec/plan writes");
assert(/initialSourceMode="production"[\s\S]{0,300}projectPath=\{selectedProject\.path\}[\s\S]{0,1200}debugMode=\{V3_DEBUG_ENABLED\}/.test(homeSource), "production V3 project route preserves the M1 debug and usage entry points with an explicit production source");
assert(/if \(showV3Playground\)[\s\S]{0,500}<V3Cockpit[\s\S]{0,200}initialSourceMode="fixture"[\s\S]{0,100}debugMode/.test(homeSource), "fixture controls remain available through the explicit playground route");
assert(/const \[sourceMode, setSourceMode\] = useState<V3SourceMode>\(initialSourceMode\)/.test(cockpitSource), "cockpit source mode remains independent from debug visibility");
assert(/const activateScenario[\s\S]{0,200}setSourceMode\("fixture"\)[\s\S]{0,100}selectScenario\(scenario\)/.test(cockpitSource), "scenario interaction establishes fixture mode before selecting fixture data");
assert(/if \(sourceMode === "production"\)[\s\S]{0,500}selectProject\(/.test(cockpitSource), "production loader runs only inside the explicit production source branch");
assert(/selectScenario: \(scenario\) => \{[\s\S]{0,250}loadRequestId \+= 1;[\s\S]{0,100}legacyRequestId \+= 1;[\s\S]{0,100}usageRequestId \+= 1;/.test(storeSource), "scenario selection invalidates all three read channels");
assert(/selectScenario: \(scenario\) => \{[\s\S]*?set\(\{[\s\S]*?bundle: null,[\s\S]*?selectedTaskId: null,[\s\S]*?selectedNodeId: null,[\s\S]*?currentView: "project-overview",[\s\S]*?navStack: \[\][\s\S]*?\}\);/.test(storeSource), "scenario selection clears cross-source bundle, selection, and navigation state");
assert(/selectProject: \(projectPath[\s\S]{0,250}loadRequestId \+= 1;[\s\S]{0,100}legacyRequestId \+= 1;[\s\S]{0,100}usageRequestId \+= 1;/.test(storeSource), "project selection invalidates all three read channels");
assert(/selectProject: \(projectPath[\s\S]{0,1800}layoutRequestId \+= 1;[\s\S]{0,160}lifecycleRequestId \+= 1;[\s\S]{0,1800}inspectProjectLayout\(\)/.test(storeSource), "production project selection invalidates lifecycle requests and inspects before loading views");
assert(/if \(status\.state === "v3"\)[\s\S]{0,300}loadCurrentBundle\(\)[\s\S]{0,200}loadLegacyArchive\(\)[\s\S]{0,200}loadUsage\(\)/.test(storeSource), "only an authoritative V3 inspection enables production read models");
assert(/absent: "initialize"[\s\S]{0,100}v2: "migrate"[\s\S]{0,100}migration_interrupted: "recover"/.test(storeSource), "lifecycle actions map only from compatible inspected states");
assert(/set\(\{ lifecycleAction: null, lifecycleResult: result \}\);[\s\S]{0,100}inspectProjectLayout\(\)/.test(storeSource), "successful lifecycle writes always re-inspect before loading views");
assert(!/initialSourceMode="fixture"[\s\S]{0,300}lifecycleApi=/.test(homeSource), "fixture playground receives no production lifecycle write API");
assert(!/initialSourceMode="fixture"[\s\S]{0,500}(revealProjectFile|openProjectFile|createTask)=/.test(homeSource), "fixture playground receives no production project file or task mutation actions");
assert(/initialSourceMode="production"[\s\S]{0,900}createTask=\{tauriApi\.v3CreateTask\}/.test(homeSource), "production V3 route injects the bounded task creation action");
assert(/sourceMode === "production" && createTask[\s\S]{0,250}(创建任务|v3\.cockpit\.create\.submit)/.test(cockpitSource), "production V3 exposes a user-reachable task creation entry");
assert(!/VibehubCockpitDialog|VibeHub Cockpit|isCockpitOpen|LayoutDashboard/.test(projectCardSource), "project card no longer reaches or mounts the legacy Cockpit");
assert(/initialSourceMode="production"[\s\S]{0,800}revealProjectFile=\{tauriApi\.vibehubRevealProjectFile\}[\s\S]{0,150}openProjectFile=\{tauriApi\.vibehubOpenProjectFile\}/.test(homeSource), "production V3 route injects bounded project file actions");
assert(/onRevealProjectFile=\{revealProjectFile\}[\s\S]{0,150}onOpenProjectFile=\{openProjectFile\}/.test(cockpitSource), "cockpit forwards project file actions only to the structure surface");
assert(/selectedNode\.kind === "file" && onOpenProjectFile/.test(structureSource), "structure surface restricts default-app open to files");
assert(/role="alert">\{fileActionError\}/.test(structureSource), "structure surface keeps project file action failures visible");
assert(/status\.state === "conflict"[\s\S]{0,500}(不会提供强制初始化或迁移|v3\.lifecycle\.noRepairCandidate)/.test(lifecycleSource), "conflict lifecycle UI fails closed without force controls");
assert(/nextAction !== "migrate" \|\| migrationConfirmed/.test(lifecycleSource), "V2 migration requires explicit archive acknowledgement");
for (const [pattern, label] of [
  [/usage\?\.total_tokens[\s\S]{0,200}(费用不可用|v3\.cockpit\.usageValue|v3\.cockpit\.usageEmpty)/, "project token summary"],
  [/AIUsagePanel/, "AI usage panel"],
  [/(项目 AI 用量总览|v3\.cockpit\.projectUsage)/, "project usage drawer"],
]) assert(pattern.test(cockpitSource), `M1 cockpit preserves ${label}`);
for (const [pattern, label] of [
  [/(AI 工具用量|v3\.usage\.projectTitle|v3\.usage\.taskTitle)/, "usage title"],
  [/(总 Token|v3\.usage\.totalToken)/, "token metric"],
  [/(预估成本|v3\.usage\.estimatedCost)/, "cost metric"],
  [/(会话明细|v3\.usage\.sessionDetails)/, "session interaction"],
  [/usage\.claude_app[\s\S]{0,100}usage\.cursor/, "explicit unsupported Claude App and Cursor sources"],
  [/tool\.tokens \?[^\n]*: tool\.status/, "unsupported sources render status instead of zero tokens"],
]) assert(pattern.test(usageSource), `M1 usage panel preserves ${label}`);

// G04: Final production gate — declared source classes and command isolation
for (const classification of ["Real", "Derived", "Fixture", "Mock", "Placeholder", "Unsupported"]) {
  assert(sourceMatrixSource.includes(`**${classification}**`), `G04 data-source matrix declares the ${classification.toLowerCase()} classification`);
}
assert(sourceMatrixSource.includes("**There is no V3 production mock data path.**"), "G04 data-source matrix forbids a production mock path");
assert(sourceMatrixSource.includes("No fixture command escalation."), "G04 data-source matrix declares fixture command isolation");
assert(/productionLoader=\{loadV3ProductionViews\}/.test(homeSource), "production route injects the dedicated V3 production view loader");
assert(/legacyLoader=\{loadLegacyV2Archive\}[\s\S]{0,120}usageLoader=\{tauriApi\.vibehubReadLocalAgentUsage\}/.test(homeSource), "production route injects independent legacy and usage readers");
assert(/invoke<NativeV3ViewBundle>\("v3_load_view_bundle"/.test(productionViewsSource), "production view loader invokes only v3_load_view_bundle");
assert(/projectOverview: native\.project_overview[\s\S]{0,300}nodeBrief: native\.node_brief/.test(productionViewsSource), "production view loader maps the complete native V3 view bundle");
assert(/invoke<LegacyV2Archive>\("legacy_v2_load_archive"/.test(legacyV2Source), "legacy-v2 reader uses its independent archive command");
const fixtureRouteMatch = homeSource.match(/if \(showV3Playground\) \{([\s\S]*?)\n\s*\}\n\s*\n\s*if \(!config\)/);
const fixtureRouteSource = fixtureRouteMatch?.[1] ?? "";
assert(Boolean(fixtureRouteMatch), "fixture playground has a dedicated Home route block");
assert(!/\b(?:productionLoader|legacyLoader|usageLoader|lifecycleApi|openLegacyFile|revealProjectFile|openProjectFile|createTask)\s*=/.test(fixtureRouteSource), "fixture route injects no production read, lifecycle, file, or task callbacks");
assert(/selectScenario: \(scenario\) => \{[\s\S]{0,700}productionLoader = null;[\s\S]{0,120}legacyLoader = null;[\s\S]{0,120}usageLoader = null;[\s\S]{0,120}lifecycleApi = null;/.test(storeSource), "fixture scenario selection clears all production dependencies before loading fixture data");
assert(/let (?:mut )?claude_app = unsupported_source\(\s*"claude_app"/.test(localAgentUsageSource), "Claude App usage remains an explicit unsupported source");
assert(/let (?:mut )?cursor = unsupported_source\(\s*"cursor"/.test(localAgentUsageSource), "Cursor usage remains an explicit unsupported source");
const usageAggregation = localAgentUsageSource.match(/let non_cached_total_tokens =[\s\S]*?let primary_metric =/);
assert(Boolean(usageAggregation) && !/\bclaude_app\b|\bcursor\b/.test(usageAggregation?.[0] ?? ""), "unsupported Claude App and Cursor sources are excluded from usage aggregation");
assert(/let sources = \[&claude_code, &codex, &opencode\];/.test(localAgentUsageSource), "usage freshness and completeness derive only from supported local sources");
assert(usageCapabilitySource.includes('explicit `claude_app` and `cursor` summaries with `status: "unsupported"`'), "usage capability contract documents explicit unsupported-source summaries");
for (const [pattern, label] of [
  [/FlaskConical/, "fixture scenario control"],
  [/showScenarioDropdown/, "scenario dropdown interaction"],
  [/(调试模式|v3\.cockpit\.debugMode)/, "debug indicator"],
  [/setShowSettingsModal\(true\)/, "settings interaction"],
  [/ProjectSetupModal/, "setup/settings modal"],
]) assert(pattern.test(cockpitSource), `M1 cockpit preserves ${label}`);

// D08: Static production gate — old Cockpit removal and clean break
const mainSource = await readFile(resolve(projectRoot, "src-tauri/src/main.rs"), "utf8");
const tauriSource = await readFile(resolve(projectRoot, "src/services/tauri.ts"), "utf8");
const typesSource = await readFile(resolve(projectRoot, "src/types/index.ts"), "utf8");

// 1. Production route cannot import or reach old Cockpit components
assert(!/VibehubCockpitDialog|VibehubProjectCenter|ProjectDetailBoard|ProjectStructureExplorer/.test(homeSource), "production route does not import old Cockpit components");

// 2. No retired canonical-write or protocol-era commands in production Tauri registration
const retiredCommands = [
  "vibehub_init", "vibehub_start_task", "vibehub_start_task_intake", "vibehub_build_context_pack",
  "vibehub_generate_agent_view", "vibehub_get_agent_adapter_status", "vibehub_update_agent_adapter_config",
  "vibehub_sync_agent_adapters", "vibehub_check_workspace_drift", "vibehub_sync_workspace",
  "vibehub_workflow_explain", "vibehub_switch_task", "vibehub_classify_file_ownership",
  "vibehub_query_task_neighbors", "vibehub_debug_dump", "vibehub_build_handoff",
  "vibehub_generate_review_evidence", "vibehub_read_overview", "vibehub_read_project_digest",
  "vibehub_list_prompt_templates", "vibehub_render_prompt", "vibehub_validate_phase",
  "vibehub_pause_phase", "vibehub_read_vibehub_file", "vibehub_reveal_vibehub_file",
  "vibehub_set_project_locale",
];
for (const cmd of retiredCommands) {
  assert(!new RegExp(`commands::${cmd}\\b`).test(mainSource), `retired ${cmd} is not registered in main.rs`);
}

// 3. Retired TypeScript wrappers have no residual
const retiredWrappers = [
  "vibehubInit", "vibehubStartTask", "vibehubStartTaskIntake", "vibehubGenerateAgentView",
  "vibehubGetAgentAdapterStatus", "vibehubUpdateAgentAdapterConfig", "vibehubSyncAgentAdapters",
  "vibehubCheckWorkspaceDrift", "vibehubSyncWorkspace", "vibehubBuildContextPack",
  "vibehubBuildHandoff", "vibehubDebugDump", "vibehubGenerateReviewEvidence",
  "vibehubReadOverview", "vibehubReadProjectDigest", "vibehubListPromptTemplates",
  "vibehubRenderPrompt", "vibehubValidatePhase", "vibehubPausePhase",
  "vibehubReadVibehubFile", "vibehubRevealVibehubFile", "vibehubSetProjectLocale",
];
for (const wrapper of retiredWrappers) {
  assert(!new RegExp(`\\b${wrapper}\\b`).test(tauriSource), `retired TS wrapper ${wrapper} has no residual in tauri.ts`);
}

// 4. Retained vibehub commands are still registered in production
for (const retained of ["vibehub_read_local_agent_usage", "vibehub_open_vibehub_file", "vibehub_reveal_project_file", "vibehub_open_project_file"]) {
  assert(new RegExp(`commands::${retained}\\b`).test(mainSource), `retained ${retained} is still registered in main.rs`);
}

// 5. Desktop binary shares the CLI dispatcher for headless mode (no duplicate implementation)
assert(/dispatcher::dispatch\b/.test(mainSource), "desktop binary shares the CLI dispatcher for headless mode");

// 6. No retired old-Cockpit-exclusive types in types/index.ts
const retiredTypes = [
  "VibehubCockpitStatus", "VibehubCockpitOverview", "VibehubContextViewData", "VibehubReviewViewData",
  "VibehubHandoffViewData", "VibehubDiffViewData", "VibehubArchiveViewData", "VibehubProjectDigest",
  "VibehubGitBranchesView", "VibehubFlowDetail", "VibehubEventTimelineItem", "VibehubActiveTask",
  "WorkspaceDriftReport", "PhaseValidationResult", "PhaseSetResult", "AgentAdapterStatus",
  "AgentTool", "VibehubPromptTemplateId", "VibehubPromptTemplateOption", "VibehubPromptRenderResult",
  "VibehubFileReadResult", "VibehubStartTaskResult", "VibehubSyncReport", "ResearchStatus",
];
for (const type of retiredTypes) {
  assert(!new RegExp(`\\b${type}\\b`).test(typesSource), `retired type ${type} has no residual in types/index.ts`);
}

// 7. Production V3 does not import V2 schema/domain; legacy-v2 read-only reader stays independent
assert(!/from\s+['"]@\/v3\//.test(legacyV2Source), "legacy-v2 reader does not import V3 modules");
assert(!/from\s+['"]@\/types['"]/.test(legacyV2Source), "legacy-v2 reader uses its own contracts, not the shared types module");

// 8. Retained production usage type still exists
assert(/\bLocalAgentUsageOverview\b/.test(typesSource), "production LocalAgentUsageOverview type is retained");

const transpiledRepository = ts.transpileModule(repositorySource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
}).outputText;
const repositoryModule = await import(`data:text/javascript;base64,${Buffer.from(transpiledRepository).toString("base64")}`);
const repository = repositoryModule.createV3FixtureRepository(async (fixturePath) => {
  const relativePath = fixturePath.replace(/^\/fixtures\/v3\//, "");
  return readJson(resolve(fixtureRoot, relativePath));
});

const eventValidator = ajv.getSchema(schemas.get("event-envelope.schema.json").$id);
const validEvent = {
  event_id: "evt.contract.roundtrip",
  event_type: "session.opened",
  event_version: "1.0",
  aggregate_id: "session.main",
  aggregate_version: 1,
  expected_version: 0,
  idempotency_key: "idem.contract.roundtrip",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
  actor: "contract-check",
  evidence_grade: "hard_observed",
  occurred_at: "2026-07-12T00:00:00.000Z",
  recorded_at: "2026-07-12T00:00:00.000Z",
  payload: {},
};
assert(eventValidator(validEvent), `event envelope round-trip sample validates: ${ajv.errorsText(eventValidator.errors)}`);
assert(!eventValidator({ ...validEvent, aggregate_version: 0 }), "event envelope rejects aggregate version zero");

const taskCreateValidator = ajv.getSchema(schemas.get("task-create.schema.json").$id);
assert(taskCreateValidator({ title: "Contract", intent: "Close DTO drift", acceptance_criteria: ["Generated types are canonical"] }), "task create request validates");
assert(taskCreateValidator({
  status: "created",
  task_id: "task.contract",
  task_path: ".vibehub/tasks/task.contract/task.yaml",
  current_pointer_path: ".vibehub/tasks/current/task.yaml",
  current_pointer_updated: false,
  initial_node_id: "node.contract.initial",
  lifecycle_version: 1,
}), "task create result includes initial node and lifecycle version");
assert(taskCreateValidator({
  status: "created",
  task_id: "task.lightweight",
  task_path: ".vibehub/tasks/task.lightweight/task.yaml",
  current_pointer_path: ".vibehub/tasks/current/task.yaml",
  current_pointer_updated: false,
  initial_node_id: null,
  lifecycle_version: 2,
}), "lightweight task create result has no synthetic initial node");
assert(!taskCreateValidator({ title: "Contract", intent: "Missing criteria", acceptance_criteria: [] }), "task create rejects empty acceptance criteria");

const routingSchema = schemas.get("session-task-routing.schema.json");
const routingValidator = ajv.getSchema(routingSchema.$id);
const routeDecisionValidator = ajv.compile({ $ref: `${routingSchema.$id}#/$defs/TaskRouteDecision` });
assert(routingValidator({
  schema_version: "1.0",
  identity: {
    project_id: "project.contract",
    interaction_id: "interaction.contract",
    session_id: "session.contract",
  },
  intent: "inspect the routing contract",
  trigger: "ordinary_continuation",
  candidates: [],
  host_capabilities: {
    state: "known",
    supports_session_binding: true,
    supports_binding_preconditions: true,
    provider: "codex",
  },
}), `session-task routing request validates: ${ajv.errorsText(routingValidator.errors)}`);
assert(routeDecisionValidator({
  schema_version: "1.0",
  project_id: "project.contract",
  interaction_id: "interaction.contract",
  session_id: "session.contract",
  action: "new",
  kind: "new",
  binding_status: "unbound",
  confidence: 100,
  trigger: "new_execution",
  reroute_performed: true,
  rationale: "no active candidate task",
}), `session-task routing decision validates: ${ajv.errorsText(routeDecisionValidator.errors)}`);
assert(!routeDecisionValidator({
  schema_version: "1.0",
  project_id: "project.contract",
  interaction_id: "interaction.contract",
  session_id: "session.contract",
  action: "ask",
  kind: "ask",
  binding_status: "invalid",
  confidence: 0,
  trigger: "ambiguous_candidate",
  reroute_performed: true,
  options: [{ task_id: "task.1", title: "One", state: "active", confidence: 50 }, { task_id: "task.2", title: "Two", state: "active", confidence: 49 }, { task_id: "task.3", title: "Three", state: "active", confidence: 48 }, { task_id: "task.4", title: "Four", state: "active", confidence: 47 }],
  rationale: "ambiguous candidates",
}), "session-task routing decision caps confirmation options at three");

const settingsValidator = ajv.getSchema(schemas.get("project-settings.schema.json").$id);
assert(settingsValidator({ expected_revision: 0, output_language: "zh-CN", agent_spec_targets: ["claude_code"] }), "project settings update accepts one language scalar and known targets");
assert(!settingsValidator({ expected_revision: 0, output_language: ["zh-CN"], agent_spec_targets: ["claude_code"] }), "project settings language is a scalar, not an array");
assert(!settingsValidator({ expected_revision: 0, output_language: "zh-CN", agent_spec_targets: ["cursor"] }), "project settings rejects unsupported target");

const agentSpecValidator = ajv.getSchema(schemas.get("agent-spec.schema.json").$id);
assert(agentSpecValidator({ force_managed_region: false, migrate_global: false }), "agent spec sync request validates");
assert(!agentSpecValidator({ force_managed_region: false, adapters: true }), "agent spec sync rejects legacy adapter fields");
const agentSpecArtifact = (status) => ({
  path: "AGENTS.md",
  consumers: ["codex"],
  status,
  reason: "contract sample",
  current_hash: null,
  desired_hash: "desired",
  last_written_hash: null,
});
const hostMcpInspection = (scope = "project", status = "mismatched") => ({
  schema_version: "1.0",
  consumer: "codex",
  scope,
  path: scope === "project" ? "repo/.codex/config.toml" : "/Users/alex/.codex/config.toml",
  status,
  reason: "contract sample",
  server_name: "vibehub",
  binary: "/bin/vibehub",
  configured_project_root: "/other",
  canonical_configured_project_root: null,
  revision: "sha256:revision",
  owned_fields: ["server_name", "command", "args"],
  provenance: "host_config_file",
  repair_action: "run typed host MCP sync",
});
const agentSpecInspection = (status) => ({
  spec_version: "3.0",
  renderer_version: "3",
  settings_revision: 1,
  scope: {
    control_root: "/project",
    execution_root: "/project/repo",
    git_root: "/project/repo",
    host_config_root: "/project/repo",
    source: "detected_git_root",
    nested_repository: true,
    warnings: ["V3_NESTED_GIT_ROOT_DETECTED"],
  },
  effective_declarations: [{ path: "repo/AGENTS.md", consumers: ["codex"], precedence: 1, exists: true, contains_v3_region: true }],
  mcp_hosts: [hostMcpInspection()],
  artifacts: [agentSpecArtifact(status)],
});
assert(agentSpecValidator({
  status: "incomplete",
  inspection: agentSpecInspection("legacy_migratable"),
  written_paths: [],
  skipped_paths: ["AGENTS.md"],
  blocking_paths: ["AGENTS.md"],
  host_mcp: {
    status: "synchronized",
    inspections: [hostMcpInspection()],
    written_paths: [],
    skipped_paths: [],
    blocking_paths: [],
    global_migration: {
      status: "unchanged",
      path: "/Users/alex/.codex/config.toml",
      backup_path: null,
      before_revision: "sha256:revision",
      after_revision: "sha256:revision",
      removed_server_names: [],
      preserved_bytes: true,
      repair_action: null,
    },
  },
}), "agent spec sync result exposes incomplete legacy migration");
assert(!agentSpecValidator({
  status: "synchronized",
  inspection: agentSpecInspection("unsupported"),
  written_paths: [],
  skipped_paths: ["AGENTS.md"],
}), "agent spec sync result requires blocking paths");

const agentProfileValidator = ajv.getSchema(schemas.get("agent-profile.schema.json").$id);
const agentProfileSchemaCapabilityValidator = ajv.compile({
  $ref: `${schemas.get("agent-profile.schema.json").$id}#/$defs/SchemaCapability`,
});
const agentProfileTimestamp = "2026-08-09T00:00:00.000Z";
const agentProfilePath = (native, platform = "macos") => ({
  platform,
  native,
  display: native,
  identity_key: platform + ":" + native,
  path_kind: platform === "windows" ? "drive" : "absolute",
  accessible: true,
});
const agentProfileCredential = {
  kind: "env",
  reference: "OPENAI_API_KEY",
  display: "环境变量 OPENAI_API_KEY",
  secret_state: "configured",
  persisted_in_config: false,
};
const agentProfileProtocol = {
  native_protocol: "openai_responses",
  upstream_protocol: "openai_chat_completions",
  route: "adapter",
  compatibility: "supported",
  adapter_id: "adapter.openai-chat-to-responses",
  adapter_version: "1.0.0",
  limitations: [],
};
const agentProfileThinking = {
  supports_reasoning: true,
  supports_effort: true,
  selected: "medium",
  options: ["low", "medium", "high"],
  custom_allowed: false,
};
const agentProfileProvider = {
  provider_id: "provider.deepseek",
  display_name: "DeepSeek",
  base_url: "https://api.deepseek.com/v1",
  credential: agentProfileCredential,
  protocol: agentProfileProtocol,
  models: [{ model_id: "deepseek-chat", display_name: "DeepSeek Chat", enabled: true, thinking: agentProfileThinking }],
};
const agentProfileTarget = {
  target_id: "runtime.macos.host",
  kind: "host",
  platform: "macos",
  distribution: null,
  display_name: "macOS host",
  home_path: agentProfilePath("/Users/alex"),
  source: "fixture",
  capability_manifest_revision: "capabilities.agent-profile.1",
};
const agentProfileRevision = {
  revision: 2,
  content_sha256: "a".repeat(64),
  observed_at: agentProfileTimestamp,
};
const agentProfileSource = {
  path: agentProfilePath("/Users/alex/.codex/deepseek.config.toml"),
  format: "toml",
  scope: "profile",
  profile_name: "deepseek",
  last_modified: agentProfileTimestamp,
};
const agentProfileManaged = {
  providers: [agentProfileProvider],
  default_provider_id: "provider.deepseek",
  default_model_id: "deepseek-chat",
  small_model_id: null,
};
const agentProfileDefaultState = {
  is_default: true,
  selected_by: "vibehub",
  projection: {
    strategy: "base_config_projection",
    target_path: agentProfilePath("/Users/alex/.codex/config.toml"),
    managed_field_paths: ["model_provider", "model"],
    warning: "项目级配置可能覆盖用户级默认",
  },
};
const agentProfileDocument = {
  profile_id: "profile.codex.deepseek",
  display_name: "DeepSeek 默认",
  agent: "codex",
  runtime_target_id: "runtime.macos.host",
  source: agentProfileSource,
  revision: agentProfileRevision,
  managed: agentProfileManaged,
  default_state: agentProfileDefaultState,
  preservation: {
    unknown_fields_preserved: true,
    comments_preserved: true,
    formatting_preserved: true,
    managed_field_paths: ["model_provider", "model"],
    unmanaged_field_paths: ["approval_policy", "sandbox_mode"],
    backup_path: agentProfilePath("/Users/alex/.codex/config.toml.vibehub-backup"),
    rollback_available: true,
  },
  protocol: agentProfileProtocol,
  schema_capability: {
    schema_id: "codex.config.toml",
    schema_version: "0.134",
    compatibility: "supported",
    supported_fields: ["model_provider", "model"],
    unsupported_fields: [],
    unknown_fields: [],
  },
  launch: {
    executable: "codex",
    profile_argument: "--profile",
    settings_argument: null,
    extra_arguments: [],
  },
};
const agentProfileReadResult = {
  schema_version: "1.0",
  kind: "agent_profile_read_result",
  generated_at: agentProfileTimestamp,
  model_version: "agent-profile.fixture.1",
  freshness: "fresh",
  completeness: "complete",
  evidence_refs: [],
  warnings: [],
  errors: [],
  agent: "codex",
  runtime_target: agentProfileTarget,
  source: agentProfileSource,
  revision: agentProfileRevision,
  managed_fields: agentProfileManaged,
  default_state: agentProfileDefaultState,
  protocol: agentProfileProtocol,
  profile: agentProfileDocument,
  schema_capability: agentProfileDocument.schema_capability,
};
assert(agentProfileValidator(agentProfileReadResult), "agent profile read result validates: " + ajv.errorsText(agentProfileValidator.errors));
assert(agentProfileSchemaCapabilityValidator({
  schema_id: "claude-code.settings",
  schema_version: "1.0",
  compatibility: "partial",
  supported_fields: ["model", "env.CLAUDE_CODE_SUBAGENT_MODEL"],
  unsupported_fields: [],
  unknown_fields: [],
  capability_declaration: {
    status: "declared",
    source: "claude-code.settings.adapter",
    version: "1.0",
    supported_fields: ["model", "env.CLAUDE_CODE_SUBAGENT_MODEL"],
    fallback_priority: ["explicit_override", "managed.default_model_id", "unavailable"],
    message: null,
  },
  custom_model_options: {
    status: "available",
    source: "managed.providers[].models",
    values: ["water18", "water18-mini"],
    allow_custom: true,
    fallback_priority: ["explicit_override", "managed.default_model_id", "unavailable"],
    message: null,
  },
}), "Claude capability declaration and custom model options validate");
assert(agentProfileValidator({
  kind: "agent_profile_command",
  command: "save",
  agent: "codex",
  runtime_target_id: "runtime.macos.host",
  profile_id: "profile.codex.deepseek",
  expected_revision: 2,
  profile: agentProfileDocument,
}), "agent profile save command validates with revision precondition");
const agentProfileCrudCommands = [
  {
    command: "create",
    profile_name: "new-profile",
    template_profile_id: "profile.codex.template",
  },
  {
    command: "clone",
    source_profile_id: "profile.codex.deepseek",
    profile_name: "cloned-profile",
  },
  {
    command: "rename",
    profile_id: "profile.codex.deepseek",
    new_profile_name: "renamed-profile",
  },
  {
    command: "delete",
    profile_id: "profile.codex.deepseek",
    replacement_profile_id: "profile.codex.template",
  },
];
for (const crudCommand of agentProfileCrudCommands) {
  assert(agentProfileValidator({
    kind: "agent_profile_command",
    agent: "codex",
    runtime_target_id: "runtime.macos.host",
    ...crudCommand,
  }), `agent profile ${crudCommand.command} command validates`);
}
assert(!agentProfileValidator({
  kind: "agent_profile_command",
  agent: "codex",
  runtime_target_id: "runtime.macos.host",
  command: "rename",
  profile_id: "profile.codex.deepseek",
  new_profile_name: "",
}), "agent profile rename rejects an empty profile name");
for (const operation of ["create", "clone", "rename", "delete"]) {
  assert(agentProfileValidator({
    schema_version: "1.0",
    kind: "agent_profile_save_result",
    generated_at: agentProfileTimestamp,
    model_version: "agent-profile.fixture.1",
    freshness: "fresh",
    completeness: "complete",
    evidence_refs: [],
    warnings: [],
    errors: [],
    operation,
    profile: agentProfileDocument,
    revision: agentProfileRevision,
    backup_path: agentProfilePath("/Users/alex/.codex/profile.vibehub-backup.toml"),
    rollback_available: true,
  }), `agent profile ${operation} result validates`);
}
assert(!agentProfileValidator({
  ...agentProfileReadResult,
  profile: {
    ...agentProfileDocument,
    managed: {
      ...agentProfileDocument.managed,
      providers: [{
        ...agentProfileProvider,
        credential: { ...agentProfileCredential, persisted_in_config: true },
      }],
    },
  },
}), "agent profile rejects persisted plaintext credentials");
assert(!agentProfileValidator({
  ...agentProfileReadResult,
  protocol: { ...agentProfileProtocol, route: "direct", compatibility: "unsupported" },
}), "agent profile rejects unsupported direct route");

const commandValidator = ajv.getSchema(schemas.get("application-command.schema.json").$id);
assert(commandValidator({
  command: "session_open",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
  actor: "contract-check",
  expected_version: 0,
  idempotency_key: "idem.command.open",
}), `session_open command sample validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(!commandValidator({ command: "session_open", project_id: "project.contract" }), "write command rejects missing scope/version/idempotency");
assert(commandValidator({
  command: "agent_result_record",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
  actor: "contract-check",
  expected_version: 1,
  idempotency_key: "idem.agent.result",
  result_id: "result.contract",
  node_id: "node.contracts",
  kind: "evaluation",
  request_source: "evaluation_instruction",
  instruction: "Evaluate the contract corpus",
  status: "succeeded",
  summary: "All checks passed",
  evaluation: { target: "contracts", rubric: ["schemas validate"], verdict: "passed", findings: [] },
  artifacts: [],
  evidence_refs: [],
}), `agent_result_record command validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(!commandValidator({
  command: "agent_result_record",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
  actor: "contract-check",
  expected_version: 1,
  idempotency_key: "idem.agent.result.invalid",
  result_id: "result.contract",
  kind: "evaluation",
  request_source: "evaluation_instruction",
  status: "succeeded",
  summary: "Missing instruction",
}), "agent result command rejects missing instruction");
const planNodeAdd = {
  command: "plan_node_add",
  project_id: "project.contract",
  task_id: "task.contract",
  actor: "contract-check",
  expected_version: 0,
  idempotency_key: "idem.plan.add",
  node_id: "node.contracts",
  title: "Contracts",
  goal: "Close the wire contract",
  scope: ["contracts/v3"],
  dependencies: [],
};
assert(commandValidator(planNodeAdd), `plan_node_add command validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(commandValidator({
  command: "plan_dependencies_set",
  project_id: "project.contract",
  task_id: "task.contract",
  actor: "contract-check",
  expected_version: 1,
  idempotency_key: "idem.plan.dependencies",
  node_id: "node.contracts",
  dependencies: ["node.foundation"],
}), `plan_dependencies_set command validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(commandValidator({
  command: "plan_node_state_set",
  project_id: "project.contract",
  task_id: "task.contract",
  actor: "contract-check",
  expected_version: 2,
  idempotency_key: "idem.plan.state",
  node_id: "node.contracts",
  state: "active",
}), `plan_node_state_set command validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(!commandValidator({ ...planNodeAdd, session_id: "session.unexpected" }), "task-scoped plan command rejects session scope");
assert(!commandValidator({ ...planNodeAdd, expected_version: -1 }), "plan command rejects negative expected_version");
assert(!commandValidator({ ...planNodeAdd, idempotency_key: "x" }), "plan command rejects invalid idempotency key");
assert(!commandValidator({ ...planNodeAdd, command: "plan_node_state_set", state: "invented" }), "plan state command rejects unknown state");
assert(commandValidator({
  command: "worktree_event",
  event_type: "worktree.create_prepared",
  project_id: "project.contract",
  task_id: "task.contract",
  actor: "contract-check",
  expected_version: 3,
  idempotency_key: "idem.command.worktree-create",
  node_id: "node.contracts",
  worktree_id: "worktree.contracts",
  session_id: "session.main",
  lease_id: "lease.contracts",
  operation_id: "operation.create.contracts",
  eligibility_digest: "a".repeat(64),
}), `worktree_create command sample validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(!commandValidator({
  command: "worktree_event",
  event_type: "worktree.create_prepared",
  project_id: "project.contract",
  task_id: "task.contract",
  actor: "contract-check",
  expected_version: 3,
  idempotency_key: "idem.command.worktree-create-invalid",
  node_id: "node.contracts",
  worktree_id: "worktree.contracts",
  operation_id: "operation.create.contracts",
  eligibility_digest: "not-a-digest",
}), "worktree command rejects an invalid eligibility digest");
for (const scenario of ["FX-HAPPY", "FX-WIN-PATHS", "FX-REWORK", "FX-LARGE"]) {
  const bundle = await repository.loadScenario(scenario);
  assert(Object.keys(bundle).length === Object.keys(CONTRACT_FILES).length, `TypeScript repository loads all views for ${scenario}`);
  assert(Object.values(bundle).every((view) => view.schema_version === CONTRACT_VERSION), `TypeScript repository preserves version for ${scenario}`);
}

const winPaths = await readJson(resolve(fixtureRoot, "FX-WIN-PATHS/project-structure.json"));
const winKinds = new Set(winPaths.nodes.map((node) => node.path.path_kind));
assert(["drive", "unc", "extended"].every((kind) => winKinds.has(kind)), "Windows fixtures cover drive, UNC, and extended paths");
assert(winPaths.nodes.some((node) => node.path.native !== node.path.display), "Windows fixtures preserve distinct native and display paths");
assert(winPaths.nodes.some((node) => node.path.accessible === false), "Windows fixtures cover a locked/inaccessible path");

const emptyOverview = await readJson(resolve(fixtureRoot, "FX-EMPTY/project-overview.json"));
const emptyTimeline = await readJson(resolve(fixtureRoot, "FX-EMPTY/task-timeline.json"));
const emptyGraph = await readJson(resolve(fixtureRoot, "FX-EMPTY/plan-graph.json"));
assert(emptyOverview.active_tasks.length === 0 && emptyTimeline.events.length === 0 && emptyGraph.nodes.length === 0, "empty fixture contains no synthetic active work");
const emptyResults = await readJson(resolve(fixtureRoot, "FX-EMPTY/agent-results.json"));
assert(emptyResults.state === "not_executed" && emptyResults.results.length === 0, "empty fixture distinguishes a task that has not executed");
const happyResults = await readJson(resolve(fixtureRoot, "FX-HAPPY/agent-results.json"));
assert(happyResults.state === "available" && happyResults.results.some((result) => result.kind === "evaluation" && result.evaluation?.verdict === "passed"), "happy fixture exposes a real evaluation result");
const happyStructure = await readJson(resolve(fixtureRoot, "FX-HAPPY/project-structure.json"));
assert(happyStructure.workspace.source === "session_worktree" && happyStructure.architecture_nodes.length > 0, "happy structure separates session worktree scope from semantic architecture");
const parallelOverview = await readJson(resolve(fixtureRoot, "FX-PARALLEL/project-overview.json"));
const parallelTimeline = await readJson(resolve(fixtureRoot, "FX-PARALLEL/task-timeline.json"));
const parallelGraph = await readJson(resolve(fixtureRoot, "FX-PARALLEL/plan-graph.json"));
assert(parallelOverview.active_tasks.length >= 2 && parallelTimeline.lanes.length >= 3, "parallel fixture covers multiple tasks and sessions");
assert(parallelGraph.warnings.some((item) => item.code === "SCOPE_OVERLAP"), "parallel fixture exposes overlap warning");
const parallelOrchestration = await readJson(resolve(fixtureRoot, "FX-PARALLEL/worktree-orchestration.json"));
assert(parallelOrchestration.worktrees.length === 2, "parallel fixture projects two worktrees");
assert(parallelOrchestration.worktrees.some((item) => item.eligibility.decision === "block" && item.lease === null), "blocked parallel scope receives no lease");
assert(parallelOrchestration.entry_gate.state === "closed" && !parallelOrchestration.entry_gate.self_host_writes_allowed, "M5 fixture keeps self-host writes closed without native and owner evidence");
const reworkTimeline = await readJson(resolve(fixtureRoot, "FX-REWORK/task-timeline.json"));
const reworkGraph = await readJson(resolve(fixtureRoot, "FX-REWORK/plan-graph.json"));
assert(reworkTimeline.events.filter((item) => item.kind === "attempt").length >= 2 && reworkGraph.trace_relations.length >= 2, "rework fixture preserves two attempts and causal traces");
const reworkOrchestration = await readJson(resolve(fixtureRoot, "FX-REWORK/worktree-orchestration.json"));
assert(reworkOrchestration.worktrees[0].state === "conflicted" && reworkOrchestration.worktrees[0].conflict.owner_session_id === "session.main", "rework fixture keeps conflict ownership on the original session");
assert(reworkOrchestration.worktrees[0].recovery.operation_id === reworkOrchestration.worktrees[0].integration.operation_id, "rework fixture preserves operation identity across retry");
const winOrchestration = await readJson(resolve(fixtureRoot, "FX-WIN-PATHS/worktree-orchestration.json"));
assert(winOrchestration.worktrees[0].git.dirty && winOrchestration.worktrees[0].recovery.state === "inspect_required", "Windows fixture refuses blind cleanup of a dirty worktree");
assert(winOrchestration.orphan_candidates[0].process_state === "unknown" && winOrchestration.orphan_candidates[0].inspect_required, "Windows fixture does not infer process death from timeout");

const schemaHashSummary = schemaNames.map((name) => `${basename(name)}=${sha256(JSON.stringify(schemas.get(name)))}`).join(" ");
if (failures.length) {
  console.error(`V3 contract check failed (${failures.length}):`);
  for (const failure of failures) {
    console.error(`- ${failure}`);
    if (process.env.GITHUB_ACTIONS === "true") {
      const annotation = failure.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A");
      console.error(`::error file=scripts/v3-contracts/check.mjs,title=V3 contract check::${annotation}`);
    }
  }
  process.exitCode = 1;
} else {
  console.log(`V3 contract check passed: ${passed.length} assertions, ${manifest.scenarios.length} scenarios, 6 views, 5 write contracts.`);
  console.log(`Schema hashes: ${schemaHashSummary}`);
}
