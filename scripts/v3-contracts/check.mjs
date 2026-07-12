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
  "project-overview-view.schema.json",
  "project-structure-view.schema.json",
  "task-timeline-view.schema.json",
  "plan-graph-view.schema.json",
  "node-brief.schema.json",
  "worktree-orchestration-view.schema.json",
];
const requiredScenarios = [
  "FX-EMPTY", "FX-HAPPY", "FX-NO-DOCS", "FX-PARALLEL", "FX-REWORK", "FX-STALE",
  "FX-PARTIAL", "FX-ERROR", "FX-WIN-PATHS", "FX-MAC-PATHS", "FX-LARGE", "FX-COVERAGE-GAP",
];

const failures = [];
const passed = [];
const assert = (condition, message) => condition ? passed.push(message) : failures.push(message);
const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));

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
}

for (const sentinelPath of manifest.invalid_sentinels) {
  const sentinel = await readJson(resolve(fixtureRoot, sentinelPath));
  const validator = ajv.getSchema(schemas.get(sentinel.expected_schema).$id);
  const valid = validator(sentinel.instance);
  const matchingError = validator.errors?.some((item) => item.keyword === sentinel.expected_keyword && item.instancePath === sentinel.expected_instance_path);
  assert(!valid && matchingError, `${sentinelPath} fails at expected keyword/path`);
}

const expectedTree = buildFixtureTree();
const diskFiles = (await listFiles(fixtureRoot)).map((path) => relative(fixtureRoot, path));
assert(JSON.stringify(diskFiles) === JSON.stringify([...expectedTree.keys()].sort()), "fixture tree contains only deterministic generator output");
for (const [relativePath, expected] of expectedTree) {
  const actual = await readFile(resolve(fixtureRoot, relativePath), "utf8");
  assert(actual === expected, `${relativePath} is byte-identical to fixed-seed generation`);
}

const tempRoot = await mkdtemp(resolve(tmpdir(), "vibehub-v3-types-"));
try {
  await execFileAsync(process.execPath, [resolve(scriptDir, "generate-types.mjs"), tempRoot], { cwd: projectRoot });
  const expectedFiles = (await listFiles(tempRoot)).map((path) => relative(tempRoot, path));
  const generatedFiles = (await listFiles(generatedRoot)).map((path) => relative(generatedRoot, path));
  assert(JSON.stringify(expectedFiles) === JSON.stringify(generatedFiles), "generated TypeScript file set has no drift");
  for (const relativePath of expectedFiles) {
    assert(await readFile(resolve(tempRoot, relativePath), "utf8") === await readFile(resolve(generatedRoot, relativePath), "utf8"), `${relativePath} generated TypeScript has no drift`);
  }
} finally {
  await rm(tempRoot, { recursive: true, force: true });
}

const v3Sources = (await listFiles(resolve(projectRoot, "src/v3"))).filter((path) => path.endsWith(".ts") || path.endsWith(".tsx"));
const forbidden = [
  [/\.ya?ml\b/i, "V2 YAML"],
  [/@tauri-apps|services\/tauri|\binvoke\s*\(/, "Tauri command"],
  [/vibehub-core|\.vibehub\//, "V2/core state"],
];
for (const sourcePath of v3Sources) {
  const source = await readFile(sourcePath, "utf8");
  for (const [pattern, label] of forbidden) assert(!pattern.test(source), `${relative(projectRoot, sourcePath)} has no ${label} dependency`);
}
const repositorySource = await readFile(resolve(projectRoot, "src/v3/contracts/fixtureRepository.ts"), "utf8");
for (const filename of Object.values(CONTRACT_FILES)) assert(repositorySource.includes(filename), `fixture repository loads ${filename}`);
assert(repositorySource.includes("Promise.all"), "fixture repository exposes one view-bundle load boundary");
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
  command: "worktree_create",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
  actor: "contract-check",
  expected_version: 3,
  idempotency_key: "idem.command.worktree-create",
  node_id: "node.contracts",
  worktree_id: "worktree.contracts",
  lease_id: "lease.contracts",
  operation_id: "operation.create.contracts",
  eligibility_digest: "a".repeat(64),
}), `worktree_create command sample validates: ${ajv.errorsText(commandValidator.errors)}`);
assert(!commandValidator({
  command: "worktree_create",
  project_id: "project.contract",
  task_id: "task.contract",
  session_id: "session.main",
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
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`V3 contract check passed: ${passed.length} assertions, ${manifest.scenarios.length} scenarios, 6 views, 2 write contracts.`);
  console.log(`Schema hashes: ${schemaHashSummary}`);
}
