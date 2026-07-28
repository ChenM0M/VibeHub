import { createHash } from "node:crypto";

export const CONTRACT_VERSION = "1.0";
export const FIXTURE_SEED = 20260711;
export const GENERATED_AT = "2026-07-11T08:00:00.000Z";

export const CONTRACT_FILES = {
  project_overview: "project-overview.json",
  project_structure: "project-structure.json",
  agent_results: "agent-results.json",
  task_timeline: "task-timeline.json",
  plan_graph: "plan-graph.json",
  node_brief: "node-brief.json",
  worktree_orchestration: "worktree-orchestration.json",
};

const schemaFor = {
  project_overview: "project-overview-view.schema.json",
  project_structure: "project-structure-view.schema.json",
  agent_results: "agent-results-view.schema.json",
  task_timeline: "task-timeline-view.schema.json",
  plan_graph: "plan-graph-view.schema.json",
  node_brief: "node-brief.schema.json",
  worktree_orchestration: "worktree-orchestration-view.schema.json",
};

const scenarioSpecs = [
  ["FX-EMPTY", ["M0-C02", "M0-C03", "M0-C06", "M0-C07"], ["macos", "windows"], "empty"],
  ["FX-HAPPY", ["M0-C01", "M0-C02", "M0-C07", "M0-C11"], ["neutral"], "happy"],
  ["FX-NO-DOCS", ["M0-C03", "M0-C06", "M0-C11"], ["neutral"], "no_docs"],
  ["FX-PARALLEL", ["M0-C03", "M0-C07", "M0-C11"], ["neutral"], "parallel"],
  ["FX-REWORK", ["M0-C03", "M0-C07", "M0-C11"], ["neutral"], "rework"],
  ["FX-STALE", ["M0-C03", "M0-C06", "M0-C07"], ["neutral"], "stale"],
  ["FX-PARTIAL", ["M0-C03", "M0-C06", "M0-C07"], ["neutral"], "partial"],
  ["FX-ERROR", ["M0-C02", "M0-C03", "M0-C06", "M0-C07"], ["macos", "windows"], "error"],
  ["FX-WIN-PATHS", ["M0-C03", "M0-C05", "M0-C07"], ["windows"], "windows_paths"],
  ["FX-MAC-PATHS", ["M0-C03", "M0-C07"], ["macos"], "macos_paths"],
  ["FX-LARGE", ["M0-C03", "M0-C04", "M0-C07"], ["macos", "windows"], "large"],
  ["FX-COVERAGE-GAP", ["M0-C03", "M0-C06", "M0-C07"], ["all_hosts"], "coverage_gap"],
];

const clone = (value) => structuredClone(value);

export function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function prettyJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

export function sha256(content) {
  return createHash("sha256").update(content).digest("hex");
}

function path(platform, native, display = native, extras = {}) {
  return {
    platform,
    native,
    display,
    identity_key: `${platform}:${native}`,
    path_kind: platform === "windows" ? "drive" : "absolute",
    accessible: true,
    ...extras,
  };
}

function evidence(id = "ev.fixture", kind = "test", grade = "hard_observed", label = "evidence.fixture.generated", locator) {
  return [{
    evidence_id: id,
    kind,
    grade,
    label_key: label,
    locator: locator ?? `fixtures/v3/${id}`,
    captured_at: GENERATED_AT,
  }];
}

function warning(code, severity = "warning", id = "ev.fixture") {
  return { code, severity, message_key: `warning.${code.toLowerCase()}`, evidence_refs: evidence(id) };
}

function err(code, recoverable, category = "internal") {
  return { code, category, recoverable, message_key: `error.${code.toLowerCase()}`, evidence_refs: evidence("ev.error") };
}

function criterion(id, title, status = "passed", required = true) {
  return { criterion_id: id, title, status, required, evidence_refs: evidence(`ev.${id}`) };
}

function orchestrationWorktree(overrides = {}) {
  const worktreeId = overrides.worktree_id ?? "worktree.contracts";
  const nodeId = overrides.node_id ?? "node.contracts";
  const sessionIds = overrides.session_ids ?? ["session.main"];
  const leaseId = overrides.lease_id ?? "lease.contracts";
  const nativePath = overrides.native_path ?? path("macos", "/Users/alex/.vibehub/worktrees/task-m0/node-contracts");
  return {
    worktree_id: worktreeId,
    node_id: nodeId,
    session_ids: sessionIds,
    display_name: overrides.display_name ?? "M0 / contracts",
    branch: overrides.branch ?? "vibehub/m0/contracts-a1b2c3d",
    base_sha: overrides.base_sha ?? "d7ece57",
    head_sha: overrides.head_sha ?? "e3a1b90",
    native_path: nativePath,
    state: overrides.state ?? "active",
    read_only: overrides.read_only ?? false,
    eligibility: overrides.eligibility ?? {
      decision: "allow",
      digest: "a".repeat(64),
      reason_codes: [],
      declared_scope: ["contracts/v3", "scripts/v3-contracts"],
      observed_delta: ["contracts/v3/common.schema.json"],
      override: false,
      evidence_refs: evidence(`ev.${worktreeId}.eligibility`, "event", "hard_observed", "evidence.worktree.eligibility", `events/${worktreeId}/eligibility`),
    },
    lease: overrides.read_only ? null : (Object.hasOwn(overrides, "lease") ? overrides.lease : {
      lease_id: leaseId,
      owner_session_id: sessionIds[0],
      owner_host: "macbook-pro",
      owner_tool: "codex",
      state: "active",
      generation: 1,
      acquired_at: "2026-07-11T08:20:00.000Z",
      heartbeat_at: "2026-07-11T08:25:00.000Z",
      expires_at: "2026-07-11T08:35:00.000Z",
      reclaim_challenge: null,
      evidence_refs: evidence(`ev.${leaseId}`, "event", "hard_observed", "evidence.lease.active", `events/${leaseId}`),
    }),
    git: overrides.git ?? {
      presence: "present", locked: false, dirty: false, detached: false, unborn: false, base_drift: false,
      changed_files: [], observed_at: "2026-07-11T08:25:00.000Z",
      evidence_refs: evidence(`ev.${worktreeId}.git`, "git", "hard_observed", "evidence.worktree.git", `git/${worktreeId}`),
    },
    integration: overrides.integration ?? {
      operation_id: null, policy: "merge_no_ff", state: "not_requested", queue_position: null, target_branch: "main", evidence_refs: [],
    },
    conflict: overrides.conflict ?? null,
    recovery: overrides.recovery ?? {
      state: "none", operation_id: null, attempts: 0, last_error: null, owner_process_state: "alive", evidence_refs: [],
    },
    next_action: overrides.next_action ?? "Continue the assigned node",
    evidence_refs: evidence(`ev.${worktreeId}`, "event", "hard_observed", "evidence.worktree.projected", `events/${worktreeId}`),
  };
}

function baseViews() {
  const root = path("macos", "/Users/alex/Projects/VibeHub");
  const shared = {
    schema_version: CONTRACT_VERSION,
    project_id: "project.vibehub",
    generated_at: GENERATED_AT,
    model_version: "model.fixture.1",
    freshness: "fresh",
    completeness: "complete",
    evidence_refs: evidence(),
    warnings: [],
    errors: [],
  };

  const structureNodes = [
    { node_id: "file.root", parent_id: null, name: "VibeHub", kind: "root", path: root, git_state: "clean", module_id: null, ide_target: "vscode://file/VibeHub", evidence_refs: evidence("ev.root") },
    { node_id: "dir.src", parent_id: "file.root", name: "src", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/src"), git_state: "clean", module_id: null, ide_target: null, evidence_refs: evidence("ev.src") },
    { node_id: "dir.contracts", parent_id: "dir.src", name: "v3", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/src/v3"), git_state: "added", module_id: null, ide_target: null, evidence_refs: evidence("ev.v3") },
    { node_id: "file.contracts.index", parent_id: "dir.contracts", name: "index.ts", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/index.ts"), git_state: "added", module_id: "module.contracts", ide_target: null, evidence_refs: evidence("ev.contracts.index") },
    { node_id: "file.fixtureRepo", parent_id: "dir.contracts", name: "fixtureRepository.ts", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/fixtureRepository.ts"), git_state: "added", module_id: "module.contracts", ide_target: null, evidence_refs: evidence("ev.fixtureRepo") },
    { node_id: "module.core", parent_id: "file.root", name: "vibehub-core", kind: "module", path: path("macos", "/Users/alex/Projects/VibeHub/crates/vibehub-core"), git_state: "clean", module_id: "module.core", ide_target: null, evidence_refs: evidence("ev.manifest") },
    { node_id: "file.lib", parent_id: "module.core", name: "lib.rs", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/crates/vibehub-core/src/lib.rs"), git_state: "modified", module_id: "module.core", ide_target: null, evidence_refs: evidence("ev.git") },
    { node_id: "module.ui", parent_id: "dir.src", name: "ui", kind: "module", path: path("macos", "/Users/alex/Projects/VibeHub/src/components/ui"), git_state: "clean", module_id: "module.ui", ide_target: null, evidence_refs: evidence("ev.ui") },
    { node_id: "file.card", parent_id: "module.ui", name: "card.tsx", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/src/components/ui/card.tsx"), git_state: "clean", module_id: "module.ui", ide_target: null, evidence_refs: evidence("ev.card") },
    { node_id: "dir.fixtures", parent_id: "file.root", name: "fixtures", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/fixtures"), git_state: "added", module_id: null, ide_target: null, evidence_refs: evidence("ev.fixtures") },
    { node_id: "dir.fixtures.v3", parent_id: "dir.fixtures", name: "v3", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/fixtures/v3"), git_state: "added", module_id: null, ide_target: null, evidence_refs: evidence("ev.fxv3") },
    { node_id: "file.fx.happy", parent_id: "dir.fixtures.v3", name: "FX-HAPPY", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/fixtures/v3/FX-HAPPY"), git_state: "added", module_id: null, ide_target: null, evidence_refs: evidence("ev.fxhappy") },
    { node_id: "file.fx.overview", parent_id: "file.fx.happy", name: "project-overview.json", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/fixtures/v3/FX-HAPPY/project-overview.json"), git_state: "added", module_id: null, ide_target: null, evidence_refs: evidence("ev.fxoverview") },
    { node_id: "dir.scripts", parent_id: "file.root", name: "scripts", kind: "directory", path: path("macos", "/Users/alex/Projects/VibeHub/scripts"), git_state: "clean", module_id: null, ide_target: null, evidence_refs: evidence("ev.scripts") },
    { node_id: "file.check.mjs", parent_id: "dir.scripts", name: "check.mjs", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/scripts/v3-contracts/check.mjs"), git_state: "modified", module_id: null, ide_target: null, evidence_refs: evidence("ev.check") },
  ];

  const structureEdges = [
    { edge_id: "edge.root.src", from_node_id: "file.root", to_node_id: "dir.src", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.root") },
    { edge_id: "edge.src.v3", from_node_id: "dir.src", to_node_id: "dir.contracts", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.src") },
    { edge_id: "edge.v3.index", from_node_id: "dir.contracts", to_node_id: "file.contracts.index", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.v3") },
    { edge_id: "edge.v3.fixtureRepo", from_node_id: "dir.contracts", to_node_id: "file.fixtureRepo", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.v3") },
    { edge_id: "edge.root.core", from_node_id: "file.root", to_node_id: "module.core", kind: "contains", source_kind: "manifest", confidence: 1, evidence_refs: evidence("ev.manifest") },
    { edge_id: "edge.core.lib", from_node_id: "module.core", to_node_id: "file.lib", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.manifest") },
    { edge_id: "edge.src.ui", from_node_id: "dir.src", to_node_id: "module.ui", kind: "contains", source_kind: "filesystem", confidence: 0.9, evidence_refs: evidence("ev.ui") },
    { edge_id: "edge.ui.card", from_node_id: "module.ui", to_node_id: "file.card", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.ui") },
    { edge_id: "edge.root.fixtures", from_node_id: "file.root", to_node_id: "dir.fixtures", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.root") },
    { edge_id: "edge.fixtures.v3", from_node_id: "dir.fixtures", to_node_id: "dir.fixtures.v3", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.fixtures") },
    { edge_id: "edge.fxv3.happy", from_node_id: "dir.fixtures.v3", to_node_id: "file.fx.happy", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.fxv3") },
    { edge_id: "edge.happy.overview", from_node_id: "file.fx.happy", to_node_id: "file.fx.overview", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.fxhappy") },
    { edge_id: "edge.root.scripts", from_node_id: "file.root", to_node_id: "dir.scripts", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.root") },
    { edge_id: "edge.scripts.check", from_node_id: "dir.scripts", to_node_id: "file.check.mjs", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.scripts") },
    { edge_id: "edge.ui.depends.contracts", from_node_id: "module.ui", to_node_id: "module.contracts", kind: "depends_on", source_kind: "parser", confidence: 0.85, evidence_refs: evidence("ev.parser", "file", "hard_observed", "evidence.parser.depends", "src/components/ui/card.tsx") },
    { edge_id: "edge.core.depends.contracts", from_node_id: "module.core", to_node_id: "module.contracts", kind: "imports", source_kind: "parser", confidence: 0.7, evidence_refs: evidence("ev.parser2", "file", "inferred", "evidence.parser.imports", "crates/vibehub-core/src/lib.rs") },
  ];

  const architectureNodes = [
    {
      node_id: "architecture.workspace", name: root.display.split("/").at(-1), kind: "workspace", path: root,
      file_count: structureNodes.filter((node) => node.kind === "file").length,
      source_kind: "manifest", confidence: 1, generator_version: shared.model_version,
      evidence_refs: evidence("ev.architecture.workspace", "file", "hard_observed", "evidence.architecture.workspace", "package.json"),
    },
    ...[...new Set(structureNodes.map((node) => node.module_id).filter(Boolean))].map((moduleId) => {
      const explicitNode = structureNodes.find((node) => node.node_id === moduleId);
      const representative = explicitNode ?? structureNodes.find((node) => node.module_id === moduleId);
      return {
        node_id: moduleId,
        name: explicitNode?.name ?? moduleId.split(".").at(-1),
        kind: explicitNode?.kind === "package" ? "package" : "module",
        path: representative.path,
        file_count: structureNodes.filter((node) => node.kind === "file" && node.module_id === moduleId).length,
        source_kind: explicitNode ? "manifest" : "parser",
        confidence: explicitNode ? 1 : 0.85,
        generator_version: shared.model_version,
        evidence_refs: representative.evidence_refs,
      };
    }),
  ];
  const architectureEdges = structureEdges
    .filter((edge) => edge.source_kind !== "filesystem")
    .map((edge) => ({
      ...edge,
      from_node_id: edge.from_node_id.replace("file.root", "architecture.workspace"),
      to_node_id: edge.to_node_id.replace("file.root", "architecture.workspace"),
      generator_version: shared.model_version,
    }));

  const timelineEvents = [
    { timeline_event_id: "evt.session.open", kind: "session", occurred_at: "2026-07-11T07:30:00.000Z", recorded_at: "2026-07-11T07:30:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.session.opened", details: {}, evidence_refs: evidence("ev.session.open", "event", "hard_observed", "evidence.session.opened", "events/session.open") },
    { timeline_event_id: "evt.plan.created", kind: "plan", occurred_at: "2026-07-11T07:35:00.000Z", recorded_at: "2026-07-11T07:35:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.plan.created", details: { nodes: 3, edges: 2 }, evidence_refs: evidence("ev.plan", "event", "agent_reported", "evidence.plan.created", "events/plan.created") },
    { timeline_event_id: "evt.decision.schema", kind: "decision", occurred_at: "2026-07-11T07:42:00.000Z", recorded_at: "2026-07-11T07:42:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: null, node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.decision.schema2020", details: { decision: "JSON Schema 2020-12 as canonical" }, evidence_refs: evidence("ev.decision", "event", "user_confirmed", "evidence.decision.schema", "events/decision.schema") },
    { timeline_event_id: "evt.evidence.contract", kind: "evidence", occurred_at: "2026-07-11T07:50:00.000Z", recorded_at: "2026-07-11T07:50:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.evidence.contractDrafted", details: { files: ["contracts/v3/common.schema.json", "contracts/v3/project-overview-view.schema.json"] }, evidence_refs: evidence("ev.evidence", "file", "hard_observed", "evidence.evidence.drafted", "contracts/v3/common.schema.json") },
    { timeline_event_id: "evt.attempt.generate", kind: "attempt", occurred_at: "2026-07-11T07:58:00.000Z", recorded_at: "2026-07-11T07:58:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "node", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.attempt.generateFixtures", details: { command: "node scripts/v3-contracts/generate-fixtures.mjs" }, evidence_refs: evidence("ev.attempt", "command", "hard_observed", "evidence.attempt.generate", "commands/generate-fixtures") },
    { timeline_event_id: "evt.validation.check", kind: "validation", occurred_at: "2026-07-11T08:05:00.000Z", recorded_at: "2026-07-11T08:05:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "npm", node_id: "node.contracts", session_id: "session.main", commit_sha: "d7ece57", summary_key: "timeline.validation.passed", details: { command: "npm run v3:contracts:check", assertions: 342 }, evidence_refs: evidence("ev.validation", "command", "hard_observed", "evidence.validation.passed", "commands/v3-contracts-check") },
    { timeline_event_id: "evt.confirmation.commit", kind: "confirmation", occurred_at: "2026-07-11T08:10:00.000Z", recorded_at: "2026-07-11T08:10:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "git", node_id: "node.contracts", session_id: "session.main", commit_sha: "d7ece57", summary_key: "timeline.confirmation.committed", details: { sha: "d7ece57", message: "M0: freeze V3 contracts and fixtures" }, evidence_refs: evidence("ev.commit", "git", "hard_observed", "evidence.confirmation.commit", "git/d7ece57") },
    { timeline_event_id: "evt.session.close", kind: "session", occurred_at: "2026-07-11T08:15:00.000Z", recorded_at: "2026-07-11T08:15:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.session.closed", details: {}, evidence_refs: evidence("ev.session.close", "event", "hard_observed", "evidence.session.closed", "events/session.close") },
  ];

  const c0 = criterion("criterion.m0.c01", "Schema hash stability", "passed");
  const c1 = criterion("criterion.m0.c02", "All views validate against schema", "passed");
  const c2 = criterion("criterion.m0.c07", "Fixture repository consumer proof", "passed");

  return {
    project_overview: {
      ...shared,
      name: "VibeHub",
      root,
      scopes: {
        control_root: "/Users/alex/Projects/VibeHub",
        execution_root: "/Users/alex/Projects/VibeHub",
        git_root: "/Users/alex/Projects/VibeHub",
        host_config_root: "/Users/alex/Projects/VibeHub",
        source: "control_root",
        nested_repository: false,
        warnings: [],
      },
      repository: { state: "available", branch: "feature/v3", head: "d7ece57", dirty: false, worktree_count: 1 },
      model: { state: "ready", last_evidence_at: "2026-07-11T08:15:00.000Z", generator_version: "fixture-1.0", indexed_files: 128 },
      architecture: { declared_docs: 2, modules: architectureNodes.filter((node) => node.kind !== "workspace").length, relationships: architectureEdges.length, confidence: 0.95, evidence_refs: evidence("ev.architecture", "file", "hard_observed", "evidence.architecture.generated", "contracts/v3/README.md") },
      active_tasks: [
        {
          task_id: "task.m0", title: "Freeze V3 contracts and fixtures", intent: "Keep V3 contracts and fixtures stable", workflow_profile: "full", state: "active", risk_level: "low",
          criteria: [c0, c1, c2], active_sessions: 1,
        },
        {
          task_id: "task.m1", title: "Build V3 high-fidelity frontend from fixtures", intent: "Build the V3 frontend from fixtures", workflow_profile: "standard", state: "active", risk_level: "medium",
          criteria: [
            criterion("criterion.m1.c01", "State coverage across 12 scenarios", "accepted"),
            criterion("criterion.m1.c02", "Field traceability to contracts", "proposed"),
            criterion("criterion.m1.c03", "Cross-platform no-overflow", "proposed"),
          ], active_sessions: 2,
        },
        {
          task_id: "task.m2", title: "Implement V3 event core and MCP control plane", intent: "Implement the V3 event core and MCP control plane", workflow_profile: "full", state: "planned", risk_level: "medium",
          criteria: [
            criterion("criterion.m2.c01", "Event store persistence", "proposed"),
          ], active_sessions: 0,
        },
      ],
      protocol_coverage: { state: "complete", opened_sessions: 3, closed_sessions: 2, gaps: 0 },
    },
    project_structure: {
      ...shared,
      index_state: "ready",
      workspace: { root: path("macos", "/Users/alex/.vibehub/worktrees/task-m0/node-contracts"), source: "session_worktree", session_id: "session.main", worktree_id: "worktree.contracts", fallback_reason: null, ignored_directories: [".git", "dist", "node_modules", "target"] },
      nodes: structureNodes,
      edges: structureEdges,
      architecture_nodes: architectureNodes,
      architecture_edges: architectureEdges,
      unsupported_analyzers: [],
      page: { cursor: null, next_cursor: null, limit: 100, returned: structureNodes.length, total_estimate: structureNodes.length, truncated: false, truncation_reason: "none", model_version: "model.fixture.1" },
    },
    agent_results: {
      ...shared,
      task_id: "task.m0",
      state: "available",
      review_required: false,
      next_action: null,
      results: [{
        result_id: "result.m0.evaluation", kind: "evaluation", session_id: "session.main", node_id: "node.contracts",
        request: { source: "evaluation_instruction", instruction: "按 V3 契约验证全部 fixture 并报告不一致" }, status: "succeeded",
        summary: "全部契约与 fixture 校验通过", body: "契约、引用、生成类型和场景覆盖均通过。",
        evaluation: { target: "V3 contract corpus", rubric: ["JSON Schema 合法", "Fixture 全覆盖", "生成类型无漂移"], verdict: "passed", findings: [{ title: "契约一致", detail: "所有场景均通过对应 schema。", severity: "info", evidence_refs: evidence("ev.validation") }] },
        artifacts: [{ label: "契约检查报告", path: path("macos", "/Users/alex/Projects/VibeHub/contracts/v3/README.md"), uri: null }],
        started_at: "2026-07-11T07:58:00.000Z", completed_at: "2026-07-11T08:05:00.000Z", evidence_refs: evidence("ev.validation")
      }]
    },
    task_timeline: {
      ...shared,
      task_id: "task.m0", title: "Freeze V3 contracts and fixtures", state: "active", criteria: [c0, c1, c2],
      completion: { proposal_event_id: null, proposed_at_version: null, digest: null, valid: false, confirmed: false, confirmed_at_version: null, confirmed_by: null, channel: null },
      lanes: [
        { lane_id: "lane.session.main", kind: "session", label: "Codex 主会话", state: "closed" },
        { lane_id: "lane.node.contracts", kind: "node", label: "节点：冻结契约", state: "repaired" },
      ],
      events: timelineEvents,
      window: { cursor: null, next_cursor: null, limit: 100, returned: timelineEvents.length, total_estimate: timelineEvents.length, truncated: false, truncation_reason: "none", model_version: "model.fixture.1" },
    },
    plan_graph: {
      ...shared,
      task_id: "task.m0", plan_version: 2, workflow_profile: "full", planning_required: true, graph_state: "valid",
      nodes: [
        { node_id: "node.schema", title: "定义 JSON Schema", goal: "编写 5 个视图的 JSON Schema 2020-12 定义", state: "completed", readiness: "ready", block_reasons: [], scope: ["contracts/v3/*.schema.json"], criterion_ids: ["criterion.m0.c01"] },
        { node_id: "node.fixtures", title: "生成 Fixture 数据", goal: "生成 12 场景 × 5 视图的确定性 fixture", state: "completed", readiness: "ready", block_reasons: [], scope: ["fixtures/v3", "scripts/v3-contracts/fixture-data.mjs"], criterion_ids: ["criterion.m0.c02"] },
        { node_id: "node.contracts", title: "冻结契约与 TS 类型", goal: "生成 TypeScript 类型并冻结契约版本", state: "active", readiness: "ready", block_reasons: [], scope: ["src/v3/contracts/generated", "src/v3/contracts/fixtureRepository.ts"], criterion_ids: ["criterion.m0.c07"] },
        { node_id: "node.check", title: "验证管线", goal: "342 项断言全通过", state: "completed", readiness: "ready", block_reasons: [], scope: ["scripts/v3-contracts/check.mjs"], criterion_ids: ["criterion.m0.c02"] },
        { node_id: "node.repo", title: "Fixture 仓库接口", goal: "实现 createV3FixtureRepository 和 JsonLoader", state: "completed", readiness: "ready", block_reasons: [], scope: ["src/v3/contracts/fixtureRepository.ts"], criterion_ids: ["criterion.m0.c07"] },
        { node_id: "node.ui.common", title: "通用 UI 组件", goal: "StateBadge / EvidenceLink / WarningList 等 6 个通用组件", state: "active", readiness: "ready", block_reasons: [], scope: ["src/v3/components/common"], criterion_ids: ["criterion.m1.c01"] },
        { node_id: "node.ui.project", title: "Project 视图", goal: "ProjectOverview / StructureExplorer / ArchitectureMap / GlobalTimeline", state: "active", readiness: "ready", block_reasons: [], scope: ["src/v3/components/project"], criterion_ids: ["criterion.m1.c01", "criterion.m1.c03"] },
        { node_id: "node.ui.task", title: "Task 视图", goal: "TaskTimeline / PlanGraph / AcceptanceProgress / NodeBriefPanel", state: "blocked", readiness: "blocked", block_reasons: ["scope_overlap"], scope: ["src/v3/components/task"], criterion_ids: ["criterion.m1.c01"] },
        { node_id: "node.ui.layout", title: "Cockpit 布局整合", goal: "V3Cockpit 单页面 + 标签面板 + 侧滑详情", state: "planned", readiness: "unknown", block_reasons: [], scope: ["src/v3/app/V3Cockpit.tsx"], criterion_ids: ["criterion.m1.c02"] },
        { node_id: "node.ui.coverage", title: "12 场景全覆盖", goal: "逐场景验证降级状态渲染", state: "planned", readiness: "blocked", block_reasons: ["depends_on_ui_task"], scope: ["fixtures/v3/*"], criterion_ids: ["criterion.m1.c01"] },
      ],
      scheduling_edges: [
        { edge_id: "edge.schema.fixtures", from_node_id: "node.schema", to_node_id: "node.fixtures", kind: "depends_on" },
        { edge_id: "edge.fixtures.contracts", from_node_id: "node.fixtures", to_node_id: "node.contracts", kind: "depends_on" },
        { edge_id: "edge.fixtures.repo", from_node_id: "node.fixtures", to_node_id: "node.repo", kind: "depends_on" },
        { edge_id: "edge.contracts.check", from_node_id: "node.contracts", to_node_id: "node.check", kind: "depends_on" },
        { edge_id: "edge.repo.ui.common", from_node_id: "node.repo", to_node_id: "node.ui.common", kind: "depends_on" },
        { edge_id: "edge.ui.common.ui.project", from_node_id: "node.ui.common", to_node_id: "node.ui.project", kind: "depends_on" },
        { edge_id: "edge.ui.common.ui.task", from_node_id: "node.ui.common", to_node_id: "node.ui.task", kind: "depends_on" },
        { edge_id: "edge.ui.project.ui.layout", from_node_id: "node.ui.project", to_node_id: "node.ui.layout", kind: "depends_on" },
        { edge_id: "edge.ui.task.ui.layout", from_node_id: "node.ui.task", to_node_id: "node.ui.layout", kind: "depends_on" },
        { edge_id: "edge.ui.layout.ui.coverage", from_node_id: "node.ui.layout", to_node_id: "node.ui.coverage", kind: "depends_on" },
      ],
      trace_relations: [
        { relation_id: "trace.check.validates.fixtures", from_id: "node.check", to_id: "node.fixtures", kind: "validates", evidence_refs: evidence("ev.trace1", "test", "hard_observed", "evidence.trace.validates", "scripts/v3-contracts/check.mjs") },
        { relation_id: "trace.contracts.addresses.schema", from_id: "node.contracts", to_id: "node.schema", kind: "addresses", evidence_refs: evidence("ev.trace2", "test", "hard_observed", "evidence.trace.addresses", "src/v3/contracts/generated") },
        { relation_id: "trace.repo.validates.fixtures", from_id: "node.repo", to_id: "node.fixtures", kind: "validates", evidence_refs: evidence("ev.trace3", "test", "hard_observed", "evidence.trace.repo", "src/v3/contracts/fixtureRepository.ts") },
      ],
      execution: { planned_sessions: 3, observed_sessions: 2, planned_worktrees: 1, observed_worktrees: 0 },
    },
    worktree_orchestration: {
      ...shared,
      task_id: "task.m0",
      entry_gate: {
        state: "closed", m4_gate_digest: null, windows_native_evidence: false, owner_approved: false,
        self_host_writes_allowed: false,
        evidence_refs: evidence("ev.m5.entry-gate", "file", "hard_observed", "evidence.m5.entryGate", "docs/v3/m4-stability-gate.md"),
      },
      policy: {
        integration_policy: "merge_no_ff", unknown_scope_decision: "warn",
        denylist: [".vibehub/state.yaml", ".vibehub/events/**", "migrations/**", "src/v3/contracts/generated/**"],
        case_sensitive: true, retention_seconds: 86400,
      },
      worktrees: [orchestrationWorktree()],
      integration_queue: [],
      orphan_candidates: [],
    },
    node_brief: {
      ...shared,
      task_id: "task.m0", node_id: "node.contracts", workflow_profile: "full",
      execution_policy: { recommended_profile: "full", effective_profile: "full", policy_version: 1, enforcement_epoch: "v3.1-hard-closure", trigger_reasons: ["fixture:full"], override_record: null, upgrade_history: [], milestone_policy: "full", planning_required: true, review_required: true, required_records: ["plan", "session", "progress", "result", "review"] },
      goal: "冻结契约与 TypeScript 类型",
      scope: ["src/v3/contracts/generated/*.ts", "src/v3/contracts/fixtureRepository.ts", "src/v3/contracts/index.ts"],
      non_scope: ["V3 event store", "M1 UI 组件", "M2 MCP 控制面"],
      dependencies: ["node.schema", "node.fixtures"],
      accepted_decisions: [
        "JSON Schema 2020-12 作为规范格式",
        "Fixture-first 策略隔离 M1 与 V2",
        "确定性种子 20260711 保证可复现",
      ],
      project_memory: [],
      protocol_records: ["plan", "session", "progress", "result", "review"].map((record) => ({ record, status: "complete", repair_action: null })),
      coverage_mode: "enforced",
      completion_gate: { items: [{ gate: "plan_terminal", passed: true }, { gate: "sessions_settled", passed: true }, { gate: "results_terminal", passed: true }, { gate: "criteria_green", passed: true }, { gate: "findings_closed", passed: true }] },
      research_summary: [
        "调查了 JSON Schema 2020-12 vs OpenAPI 3.1，选择前者因为 AJV2020 支持更好",
        "Fixture-first 隔离 M1 前端开发与 V2 后端耦合",
        "12 场景覆盖正常/异常/边界/大数据/跨平台路径",
      ],
      criteria: [c0, c1, c2],
      files: [
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/index.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/fixtureRepository.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/index.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/project-overview-view.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/project-structure-view.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/task-timeline-view.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/plan-graph-view.ts"),
        path("macos", "/Users/alex/Projects/VibeHub/src/v3/contracts/generated/node-brief.ts"),
      ],
      validation_commands: [
        "npm run v3:contracts:check",
        "npm run v3:contracts:generate",
      ],
      state: "active", next_intent: "完成 M0 交付物冻结，进入 M1 fixture-driven 前端开发",
      budget: { max_tokens: 8000, estimated_tokens: 3200, truncated_sections: [] },
      source_versions: { contract: CONTRACT_VERSION, model: "model.fixture.1", "json-schema": "2020-12" },
      protocol_coverage: "complete",
    },
  };
}

function applyScenario(kind, views) {
  const allViews = Object.values(views);
  if (kind === "empty") {
    views.project_overview.repository = { state: "not_repository", branch: null, head: null, dirty: null, worktree_count: 0 };
    views.project_overview.model = { state: "uninitialized", last_evidence_at: null, generator_version: "fixture-1.0", indexed_files: 0 };
    views.project_overview.architecture = { declared_docs: 0, modules: 0, relationships: 0, confidence: 0, evidence_refs: [] };
    views.project_overview.active_tasks = [];
    views.project_structure.nodes = [];
    views.project_structure.edges = [];
    views.project_structure.architecture_nodes = [];
    views.project_structure.architecture_edges = [];
    views.project_structure.index_state = "uninitialized";
    views.agent_results.task_id = "task.none";
    views.agent_results.state = "not_executed";
    views.agent_results.results = [];
    views.project_structure.page.returned = 0;
    views.project_structure.page.total_estimate = 0;
    views.task_timeline.task_id = "task.none";
    views.task_timeline.title = "未选择任务";
    views.task_timeline.state = "planned";
    views.task_timeline.criteria = [];
    views.task_timeline.lanes = [];
    views.task_timeline.events = [];
    views.task_timeline.window.returned = 0;
    views.task_timeline.window.total_estimate = 0;
    views.plan_graph.task_id = "task.none";
    views.plan_graph.nodes = [];
    views.plan_graph.scheduling_edges = [];
    views.plan_graph.trace_relations = [];
    views.plan_graph.execution = { planned_sessions: 0, observed_sessions: 0, planned_worktrees: 0, observed_worktrees: 0 };
    views.worktree_orchestration.task_id = "task.none";
    views.worktree_orchestration.worktrees = [];
    views.worktree_orchestration.integration_queue = [];
    views.worktree_orchestration.orphan_candidates = [];
    views.node_brief.task_id = "task.none";
    views.node_brief.node_id = "node.none";
    views.node_brief.goal = "未选择任务";
    views.node_brief.scope = [];
    views.node_brief.non_scope = [];
    views.node_brief.criteria = [];
    views.node_brief.files = [];
    views.node_brief.validation_commands = [];
    views.node_brief.next_intent = "创建或选择一个任务";
    views.node_brief.protocol_coverage = "unknown";
    for (const view of allViews) {
      view.completeness = "unknown";
      view.warnings.push(warning("PROJECT_UNINITIALIZED", "info"));
    }
  }
  if (kind === "no_docs") {
    views.project_overview.architecture.declared_docs = 0;
    views.project_overview.architecture.confidence = 0.7;
    views.project_overview.warnings.push(warning("NO_DECLARED_ARCHITECTURE"));
  }
  if (kind === "parallel") {
    views.task_timeline.lanes.push(
      { lane_id: "lane.session.ui", kind: "session", label: "UI 会话", state: "active" },
      { lane_id: "lane.session.types", kind: "session", label: "类型会话", state: "idle" },
    );
    views.task_timeline.events.push(
      { timeline_event_id: "evt.parallel.ui.open", kind: "session", occurred_at: "2026-07-11T08:20:00.000Z", recorded_at: "2026-07-11T08:20:00.000Z", order_state: "ordered", lane_id: "lane.session.ui", actor: "codex", tool: "codex", node_id: "node.ui", session_id: "session.ui", commit_sha: null, summary_key: "timeline.session.parallel", details: { scope: "src/v3/components" }, evidence_refs: evidence("ev.session.ui", "event", "hard_observed", "evidence.session.ui", "events/session.ui") },
      { timeline_event_id: "evt.parallel.types.open", kind: "session", occurred_at: "2026-07-11T08:22:00.000Z", recorded_at: "2026-07-11T08:22:00.000Z", order_state: "ordered", lane_id: "lane.session.types", actor: "opencode", tool: "opencode", node_id: "node.shared", session_id: "session.types", commit_sha: null, summary_key: "timeline.session.overlap", details: { scope: "src/v3/contracts" }, evidence_refs: evidence("ev.session.types", "event", "hard_observed", "evidence.session.types", "events/session.types") },
      { timeline_event_id: "evt.parallel.finding", kind: "finding", occurred_at: "2026-07-11T08:25:00.000Z", recorded_at: "2026-07-11T08:25:00.000Z", order_state: "ordered", lane_id: "lane.session.ui", actor: "reviewer", tool: null, node_id: "node.ui", session_id: "session.ui", commit_sha: null, summary_key: "timeline.finding.scopeOverlap", details: { severity: "medium", overlap: "src/v3/contracts" }, evidence_refs: evidence("ev.finding.parallel", "event", "agent_reported", "evidence.finding.overlap", "events/finding.scope") },
    );
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
    views.plan_graph.nodes.push(
      { node_id: "node.ui", title: "构建 Project 视图", goal: "渲染 fixture 数据到 UI 组件", state: "active", readiness: "ready", block_reasons: [], scope: ["src/v3/components/project"], criterion_ids: ["criterion.m1.c01"] },
      { node_id: "node.shared", title: "解决类型重叠", goal: "消除 UI 与契约模块的 scope 重叠", state: "blocked", readiness: "blocked", block_reasons: ["scope_overlap"], scope: ["src/v3/contracts"], criterion_ids: ["criterion.m1.c01"] },
    );
    views.plan_graph.scheduling_edges.push(
      { edge_id: "edge.contracts.ui", from_node_id: "node.contracts", to_node_id: "node.ui", kind: "depends_on" },
      { edge_id: "edge.contracts.shared", from_node_id: "node.contracts", to_node_id: "node.shared", kind: "depends_on" },
    );
    views.plan_graph.execution = { planned_sessions: 3, observed_sessions: 3, planned_worktrees: 2, observed_worktrees: 2 };
    views.plan_graph.warnings.push(warning("SCOPE_OVERLAP"));
    views.worktree_orchestration.worktrees = [
      orchestrationWorktree({
        worktree_id: "worktree.ui", node_id: "node.ui", session_ids: ["session.ui"], lease_id: "lease.ui",
        display_name: "M1 / project UI", branch: "vibehub/m1/project-ui-3f91a2c",
        native_path: path("macos", "/Users/alex/.vibehub/worktrees/task-m1/node-ui"),
        eligibility: {
          decision: "allow", digest: "b".repeat(64), reason_codes: [], declared_scope: ["src/v3/components/project"], observed_delta: [], override: false,
          evidence_refs: evidence("ev.worktree.ui.eligibility", "event", "hard_observed", "evidence.worktree.eligibility", "events/worktree.ui/eligibility"),
        },
      }),
      orchestrationWorktree({
        worktree_id: "worktree.shared", node_id: "node.shared", session_ids: ["session.types"], lease_id: "lease.shared",
        display_name: "M1 / shared types", branch: "vibehub/m1/shared-types-905c2be",
        native_path: path("macos", "/Users/alex/.vibehub/worktrees/task-m1/node-shared"), state: "planned",
        eligibility: {
          decision: "block", digest: "c".repeat(64), reason_codes: ["SCOPE_OVERLAP", "GENERATED_ENTRYPOINT"],
          declared_scope: ["src/v3/contracts"], observed_delta: ["src/v3/contracts/generated/index.ts"], override: false,
          evidence_refs: evidence("ev.worktree.shared.eligibility", "event", "hard_observed", "evidence.worktree.blocked", "events/worktree.shared/eligibility"),
        },
        lease: null,
        recovery: { state: "blocked", operation_id: null, attempts: 0, last_error: "eligibility_blocked", owner_process_state: "not_applicable", evidence_refs: [] },
        next_action: "Resolve scope overlap or record an owner override with evidence",
      }),
    ];
    views.worktree_orchestration.warnings.push(warning("SCOPE_OVERLAP"));
    for (const node of views.plan_graph.nodes) {
      const worktree = views.worktree_orchestration.worktrees.find((item) => item.node_id === node.node_id);
      if (worktree) {
        node.session_ids = worktree.session_ids;
        node.worktree = { worktree_id: worktree.worktree_id, lease_id: worktree.lease?.lease_id ?? null, branch: worktree.branch, state: worktree.state };
      }
    }
  }
  if (kind === "rework") {
    views.task_timeline.events.push(
      { timeline_event_id: "evt.rework.finding", kind: "finding", occurred_at: "2026-07-11T08:30:00.000Z", recorded_at: "2026-07-11T08:30:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "reviewer", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.finding.schemaGap", details: { severity: "high", gap: "common.schema.json 缺少 EvidenceGrade 枚举" }, evidence_refs: evidence("ev.finding.rework", "event", "agent_reported", "evidence.finding.schema", "events/finding.schema") },
      { timeline_event_id: "evt.rework.attempt1", kind: "attempt", occurred_at: "2026-07-11T08:35:00.000Z", recorded_at: "2026-07-11T08:35:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.attempt.remediation", details: { attempt: 1, action: "添加 EvidenceGrade 枚举到 common.schema.json" }, evidence_refs: evidence("ev.attempt1", "event", "hard_observed", "evidence.attempt.first", "events/attempt.1") },
      { timeline_event_id: "evt.rework.attempt2", kind: "attempt", occurred_at: "2026-07-11T08:42:00.000Z", recorded_at: "2026-07-11T08:42:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: "e3a1b90", summary_key: "timeline.attempt.remediation", details: { attempt: 2, action: "修正 EvidenceRef.grade 引用为 $ref" }, evidence_refs: evidence("ev.attempt2", "event", "hard_observed", "evidence.attempt.second", "events/attempt.2") },
      { timeline_event_id: "evt.rework.revalidation", kind: "validation", occurred_at: "2026-07-11T08:48:00.000Z", recorded_at: "2026-07-11T08:48:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "reviewer", tool: "npm", node_id: "node.contracts", session_id: "session.main", commit_sha: "e3a1b90", summary_key: "timeline.validation.repassed", details: { attempt: 2, assertions: 342 }, evidence_refs: evidence("ev.revalidation", "command", "hard_observed", "evidence.validation.repass", "commands/v3-contracts-check") },
    );
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
    views.plan_graph.trace_relations.push(
      { relation_id: "trace.attempt1.addresses.finding", from_id: "evt.rework.attempt1", to_id: "evt.rework.finding", kind: "addresses", evidence_refs: evidence("ev.trace.attempt1", "event", "hard_observed", "evidence.trace.addresses", "events/attempt.1") },
      { relation_id: "trace.attempt2.repairs.finding", from_id: "evt.rework.attempt2", to_id: "evt.rework.finding", kind: "repairs", evidence_refs: evidence("ev.trace.attempt2", "event", "hard_observed", "evidence.trace.repairs", "events/attempt.2") },
    );
    const conflicted = orchestrationWorktree({
      state: "conflicted",
      git: {
        presence: "present", locked: false, dirty: true, detached: false, unborn: false, base_drift: true,
        changed_files: ["contracts/v3/common.schema.json"], observed_at: "2026-07-11T08:42:00.000Z",
        evidence_refs: evidence("ev.worktree.contracts.conflict-git", "git", "hard_observed", "evidence.worktree.conflict", "git/worktree.contracts/conflict"),
      },
      integration: {
        operation_id: "operation.integrate.contracts", policy: "merge_no_ff", state: "conflicted", queue_position: null, target_branch: "main",
        evidence_refs: evidence("ev.integration.contracts", "event", "hard_observed", "evidence.integration.conflicted", "events/integration.contracts"),
      },
      conflict: {
        owner_session_id: "session.main", files: ["contracts/v3/common.schema.json"], base_sha: "d7ece57", head_sha: "e3a1b90", target_sha: "f81a2c3",
        next_action: "Return conflict resolution to session.main",
        evidence_refs: evidence("ev.conflict.contracts", "git", "hard_observed", "evidence.conflict.files", "git/conflicts/contracts"),
      },
      recovery: {
        state: "retry_ready", operation_id: "operation.integrate.contracts", attempts: 2, last_error: "merge_conflict", owner_process_state: "alive",
        evidence_refs: evidence("ev.recovery.contracts", "event", "hard_observed", "evidence.recovery.retry", "events/recovery.contracts"),
      },
      next_action: "Resolve the recorded conflict in session.main, then retry with the same operation id",
    });
    views.worktree_orchestration.worktrees = [conflicted];
    views.worktree_orchestration.integration_queue = [{
      operation_id: "operation.integrate.contracts", worktree_id: conflicted.worktree_id, node_id: conflicted.node_id,
      topology_rank: 0, ready_at: "2026-07-11T08:40:00.000Z", state: "blocked",
    }];
    views.task_timeline.lanes.push({ lane_id: "lane.worktree.contracts", kind: "worktree", label: "contracts worktree", state: "active", worktree_id: "worktree.contracts" });
    views.task_timeline.events.push({
      timeline_event_id: "evt.rework.conflict", kind: "conflict", occurred_at: "2026-07-11T08:42:00.000Z", recorded_at: "2026-07-11T08:42:00.000Z",
      order_state: "ordered", lane_id: "lane.worktree.contracts", actor: "vibehub", tool: "git", node_id: "node.contracts", session_id: "session.main", commit_sha: "e3a1b90",
      worktree_id: "worktree.contracts", lease_id: "lease.contracts", summary_key: "timeline.integration.conflicted", details: { operation_id: "operation.integrate.contracts" },
      evidence_refs: evidence("ev.timeline.conflict", "event", "hard_observed", "evidence.timeline.conflict", "events/integration.contracts"),
    });
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
  }
  if (kind === "stale") {
    for (const view of allViews) {
      view.freshness = "stale";
      view.warnings.push(warning("MODEL_STALE"));
    }
    views.project_overview.model.last_evidence_at = "2026-07-10T08:00:00.000Z";
  }
  if (kind === "partial") {
    for (const view of allViews) view.completeness = "partial";
    views.project_structure.unsupported_analyzers = ["cobol"];
    views.project_structure.page = { cursor: "page:1", next_cursor: "page:2", limit: 2, returned: 2, total_estimate: null, truncated: true, truncation_reason: "permission", model_version: "model.fixture.1" };
    views.project_structure.warnings.push(warning("SUBTREE_INACCESSIBLE"), warning("ANALYZER_UNSUPPORTED"));
  }
  if (kind === "error") {
    for (const view of allViews) {
      view.freshness = "unavailable";
      view.completeness = "unknown";
      view.errors.push(err("PROJECTION_UNAVAILABLE", true), err("CONTRACT_TERMINAL", false, "validation"));
    }
  }
  if (kind === "windows_paths") {
    const longPart = "very-long-component-".repeat(12);
    const paths = [
      path("windows", "C:\\Users\\Alex\\VibeHub", "C:/Users/Alex/VibeHub", { path_kind: "drive" }),
      path("windows", "\\\\server\\share\\VibeHub", "//server/share/VibeHub", { path_kind: "unc" }),
      path("windows", "\\\\?\\C:\\Users\\Alex\\VibeHub", "C:/Users/Alex/VibeHub", { path_kind: "extended" }),
      path("windows", `C:\\Users\\Alex\\${longPart}\\file.ts`, `C:/Users/Alex/${longPart}/file.ts`, { path_kind: "drive" }),
      path("windows", "C:\\Repo\\Readme.md", "C:/Repo/Readme.md", { path_kind: "drive" }),
      path("windows", "C:\\Repo\\README.md", "C:/Repo/README.md", { path_kind: "drive" }),
      path("windows", "C:\\Repo\\locked.db", "C:/Repo/locked.db", { path_kind: "drive", accessible: false }),
    ];
    views.project_overview.root = clone(paths[0]);
    views.project_structure.nodes = paths.map((item, index) => ({ node_id: `file.win.${index}`, parent_id: index === 0 ? null : "file.win.0", name: item.display.split("/").at(-1), kind: index === 0 ? "root" : "file", path: item, git_state: "unknown", module_id: null, ide_target: null, evidence_refs: evidence(`ev.win.${index}`) }));
    views.project_structure.edges = [];
    views.project_structure.page.returned = paths.length;
    views.project_structure.page.total_estimate = paths.length;
    views.project_structure.warnings.push(warning("PATH_CASE_COLLISION"), warning("PATH_LOCKED"));
    views.node_brief.files = paths;
    views.worktree_orchestration.policy.case_sensitive = false;
    views.worktree_orchestration.worktrees = [orchestrationWorktree({
      worktree_id: "worktree.windows", node_id: "node.contracts", session_ids: ["session.windows"], lease_id: "lease.windows",
      native_path: clone(paths[2]), state: "repairing",
      eligibility: {
        decision: "warn", digest: "d".repeat(64), reason_codes: ["PATH_BUDGET_UNKNOWN", "CASE_COLLISION"], declared_scope: ["C:/Repo/Readme.md"], observed_delta: ["C:/Repo/README.md"], override: false,
        evidence_refs: evidence("ev.worktree.windows.eligibility", "test", "hard_observed", "evidence.windows.contract", "fixtures/v3/FX-WIN-PATHS"),
      },
      git: {
        presence: "present", locked: true, dirty: true, detached: false, unborn: false, base_drift: false, changed_files: ["README.md"],
        observed_at: "2026-07-11T08:25:00.000Z", evidence_refs: evidence("ev.worktree.windows.git", "test", "hard_observed", "evidence.windows.locked", "fixtures/v3/FX-WIN-PATHS"),
      },
      recovery: { state: "inspect_required", operation_id: "operation.windows.inspect", attempts: 1, last_error: "path_locked", owner_process_state: "unknown", evidence_refs: evidence("ev.windows.recovery", "test", "hard_observed", "evidence.windows.recovery", "fixtures/v3/FX-WIN-PATHS") },
      next_action: "Inspect the owning process on a Windows native host; do not remove the dirty worktree",
    })];
    views.worktree_orchestration.orphan_candidates = [{
      worktree_id: "worktree.windows", session_id: "session.windows", reason_code: "OWNER_PROCESS_UNKNOWN", process_state: "unknown", inspect_required: true,
      evidence_refs: evidence("ev.windows.orphan", "test", "hard_observed", "evidence.windows.orphan", "fixtures/v3/FX-WIN-PATHS"),
    }];
    views.worktree_orchestration.warnings.push(warning("PATH_BUDGET_UNKNOWN"), warning("DIRTY_CLEANUP_REFUSED"));
  }
  if (kind === "macos_paths") {
    const paths = [
      path("macos", "/Users/alex/My Project/VibeHub"),
      path("macos", "/Users/alex/Projects/维贝中心/README.md"),
      path("macos", "/Users/alex/Projects/current", "/Users/alex/Projects/current -> VibeHub", { symlink_target: "/Users/alex/Projects/VibeHub" }),
      path("macos", "/Applications/VibeHub.app/Contents/MacOS/vibehub"),
      path("macos", "/Users/alex/Repo/Readme.md"),
      path("macos", "/Users/alex/Repo/README.md"),
    ];
    views.project_overview.root = clone(paths[0]);
    views.project_structure.nodes = paths.map((item, index) => ({ node_id: `file.mac.${index}`, parent_id: index === 0 ? null : "file.mac.0", name: item.display.split("/").at(-1), kind: index === 0 ? "root" : "file", path: item, git_state: "clean", module_id: null, ide_target: null, evidence_refs: evidence(`ev.mac.${index}`) }));
    views.project_structure.edges = [];
    views.project_structure.page.returned = paths.length;
    views.project_structure.page.total_estimate = paths.length;
    views.node_brief.files = paths;
  }
  if (kind === "large") {
    views.project_structure.nodes = Array.from({ length: 250 }, (_, index) => ({
      node_id: `file.large.${String(index).padStart(4, "0")}`, parent_id: index === 0 ? null : "file.large.0000", name: index === 0 ? "large-root" : `file-${String(index).padStart(4, "0")}.ts`, kind: index === 0 ? "root" : "file",
      path: path(index % 2 ? "windows" : "macos", index % 2 ? `C:\\Large\\file-${index}.ts` : `/large/file-${index}.ts`, `/large/file-${index}.ts`, { path_kind: index % 2 ? "drive" : "absolute" }),
      git_state: index % 17 === 0 ? "modified" : "clean", module_id: null, ide_target: null, evidence_refs: evidence(`ev.large.${index}`),
    }));
    views.project_structure.edges = [];
    views.project_structure.page = { cursor: null, next_cursor: "large:250", limit: 250, returned: 250, total_estimate: 10000, truncated: true, truncation_reason: "page_limit", model_version: "model.fixture.large.1" };
    views.task_timeline.events = Array.from({ length: 300 }, (_, index) => ({ timeline_event_id: `timeline.large.${String(index).padStart(4, "0")}`, kind: index % 5 === 0 ? "validation" : "evidence", occurred_at: new Date(Date.parse(GENERATED_AT) + index * 1000).toISOString(), recorded_at: new Date(Date.parse(GENERATED_AT) + index * 1000).toISOString(), order_state: "ordered", lane_id: "lane.session.main", actor: "generator", tool: "fixture-generator", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.large.item", details: { index, seed: FIXTURE_SEED }, evidence_refs: evidence(`ev.large.timeline.${index}`) }));
    views.task_timeline.window = { cursor: null, next_cursor: "large:300", limit: 300, returned: 300, total_estimate: 5000, truncated: true, truncation_reason: "page_limit", model_version: "model.fixture.large.1" };
    views.plan_graph.nodes = Array.from({ length: 80 }, (_, index) => ({ node_id: `node.large.${String(index).padStart(3, "0")}`, title: `Large node ${index}`, goal: `Exercise graph node ${index}`, state: index === 0 ? "active" : "planned", readiness: index < 2 ? "ready" : "blocked", block_reasons: index < 2 ? [] : ["dependency"], scope: [`src/large/${index}`], criterion_ids: ["criterion.m0.c07"] }));
    views.plan_graph.scheduling_edges = Array.from({ length: 79 }, (_, index) => ({ edge_id: `edge.large.${String(index).padStart(3, "0")}`, from_node_id: `node.large.${String(index).padStart(3, "0")}`, to_node_id: `node.large.${String(index + 1).padStart(3, "0")}`, kind: "depends_on" }));
    for (const view of allViews) view.model_version = "model.fixture.large.1";
  }
  if (kind === "coverage_gap") {
    views.project_overview.protocol_coverage = { state: "gapped", opened_sessions: 2, closed_sessions: 1, gaps: 1 };
    views.node_brief.protocol_coverage = "gapped";
    views.task_timeline.lanes[0].state = "repaired";
    views.task_timeline.events.push({ timeline_event_id: "evt.gap.recovered", kind: "gap", occurred_at: "2026-07-11T08:15:00.000Z", recorded_at: "2026-07-11T08:45:00.000Z", order_state: "late", lane_id: "lane.session.main", actor: "doctor", tool: "vibehub", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.gap.recovered", details: { missing: ["session_close"], host_capability: "unknown" }, evidence_refs: evidence("ev.gap", "event", "inferred", "evidence.gap.recovered", "events/gap.recovered") });
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
    views.task_timeline.warnings.push(warning("PROTOCOL_COVERAGE_GAP"));
  }
  return views;
}

export function buildFixtureTree() {
  const files = new Map();
  const globalScenarios = [];
  for (const [scenarioId, criteria, platforms, kind] of scenarioSpecs) {
    const views = applyScenario(kind, baseViews());
    const contracts = [];
    for (const [contract, filename] of Object.entries(CONTRACT_FILES)) {
      const relativePath = `${scenarioId}/${filename}`;
      const content = prettyJson(views[contract]);
      files.set(relativePath, content);
      contracts.push({ contract, schema: schemaFor[contract], file: filename, sha256: sha256(content) });
    }
    const expectedWarnings = [...new Set(Object.values(views).flatMap((view) => view.warnings.map((item) => item.code)))].sort();
    const expectedErrors = [...new Set(Object.values(views).flatMap((view) => view.errors.map((item) => item.code)))].sort();
    const expectedStates = [...new Set(Object.values(views).map((view) => `${view.freshness}/${view.completeness}`))].sort();
    const scenarioManifest = { scenario_id: scenarioId, seed: FIXTURE_SEED, platforms, criteria, expected_states: expectedStates, expected_warnings: expectedWarnings, expected_errors: expectedErrors, contracts };
    files.set(`${scenarioId}/manifest.json`, prettyJson(scenarioManifest));
    globalScenarios.push({ scenario_id: scenarioId, manifest: `${scenarioId}/manifest.json`, platforms, criteria });
  }

  const invalid = {
    "invalid/invalid-version.json": {
      expected_schema: "project-overview-view.schema.json",
      expected_keyword: "const",
      expected_instance_path: "/schema_version",
      instance: { ...baseViews().project_overview, schema_version: "2.0" },
    },
    "invalid/invalid-path.json": {
      expected_schema: "project-structure-view.schema.json",
      expected_keyword: "required",
      expected_instance_path: "/nodes/0/path",
      instance: (() => { const value = baseViews().project_structure; delete value.nodes[0].path.display; return value; })(),
    },
    "invalid/invalid-windows-mixed-separators.json": {
      expected_schema: "project-structure-view.schema.json",
      expected_keyword: "pattern",
      expected_instance_path: "/nodes/0/path/native",
      instance: (() => {
        const value = baseViews().project_structure;
        value.nodes[0].path = path("windows", "C:\\Repo/mixed\\file.ts", "C:/Repo/mixed/file.ts", { path_kind: "drive" });
        return value;
      })(),
    },
    "invalid/invalid-worktree-lease-generation.json": {
      expected_schema: "worktree-orchestration-view.schema.json",
      expected_keyword: "minimum",
      expected_instance_path: "/worktrees/0/lease/generation",
      instance: (() => { const value = baseViews().worktree_orchestration; value.worktrees[0].lease.generation = 0; return value; })(),
    },
    "invalid/invalid-worktree-eligibility-digest.json": {
      expected_schema: "worktree-orchestration-view.schema.json",
      expected_keyword: "pattern",
      expected_instance_path: "/worktrees/0/eligibility/digest",
      instance: (() => { const value = baseViews().worktree_orchestration; value.worktrees[0].eligibility.digest = "not-a-digest"; return value; })(),
    },
  };
  for (const [relativePath, value] of Object.entries(invalid)) files.set(relativePath, prettyJson(value));

  const manifest = {
    schema_version: CONTRACT_VERSION,
    fixture_version: "1.0.0",
    generator: "scripts/v3-contracts/fixture-data.mjs",
    seed: FIXTURE_SEED,
    scenarios: globalScenarios,
    invalid_sentinels: Object.keys(invalid),
  };
  files.set("manifest.json", prettyJson(manifest));
  return files;
}
