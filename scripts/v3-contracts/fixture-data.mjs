import { createHash } from "node:crypto";

export const CONTRACT_VERSION = "1.0";
export const FIXTURE_SEED = 20260711;
export const GENERATED_AT = "2026-07-11T08:00:00.000Z";

export const CONTRACT_FILES = {
  project_overview: "project-overview.json",
  project_structure: "project-structure.json",
  task_timeline: "task-timeline.json",
  plan_graph: "plan-graph.json",
  node_brief: "node-brief.json",
};

const schemaFor = {
  project_overview: "project-overview-view.schema.json",
  project_structure: "project-structure-view.schema.json",
  task_timeline: "task-timeline-view.schema.json",
  plan_graph: "plan-graph-view.schema.json",
  node_brief: "node-brief.schema.json",
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

function evidence(id = "ev.fixture") {
  return [{
    evidence_id: id,
    kind: "test",
    grade: "hard_observed",
    label_key: "evidence.fixture.generated",
    locator: `fixtures/v3/${id}`,
    captured_at: GENERATED_AT,
  }];
}

function warning(code, severity = "warning", id = "ev.fixture") {
  return { code, severity, message_key: `warning.${code.toLowerCase()}`, evidence_refs: evidence(id) };
}

function error(code, recoverable, category = "internal") {
  return { code, category, recoverable, message_key: `error.${code.toLowerCase()}`, evidence_refs: evidence("ev.error") };
}

function criterion(id = "criterion.m0.c07", status = "passed") {
  return { criterion_id: id, title: "Fixture repository consumer proof", status, required: true, evidence_refs: evidence("ev.criterion") };
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
    { node_id: "module.core", parent_id: "file.root", name: "vibehub-core", kind: "module", path: path("macos", "/Users/alex/Projects/VibeHub/crates/vibehub-core"), git_state: "clean", module_id: "module.core", ide_target: null, evidence_refs: evidence("ev.manifest") },
    { node_id: "file.lib", parent_id: "module.core", name: "lib.rs", kind: "file", path: path("macos", "/Users/alex/Projects/VibeHub/crates/vibehub-core/src/lib.rs"), git_state: "modified", module_id: "module.core", ide_target: null, evidence_refs: evidence("ev.git") },
  ];
  const timelineEvents = [
    { timeline_event_id: "timeline.session.open", kind: "session", occurred_at: GENERATED_AT, recorded_at: GENERATED_AT, order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.session.opened", details: {}, evidence_refs: evidence("ev.session") },
    { timeline_event_id: "timeline.validation.pass", kind: "validation", occurred_at: "2026-07-11T08:10:00.000Z", recorded_at: "2026-07-11T08:10:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "npm", node_id: "node.contracts", session_id: "session.main", commit_sha: "d7ece57", summary_key: "timeline.validation.passed", details: { command: "npm run v3:contracts:check" }, evidence_refs: evidence("ev.validation") },
  ];
  return {
    project_overview: {
      ...shared,
      name: "VibeHub",
      root,
      repository: { state: "available", branch: "feature/v3", head: "d7ece57", dirty: false, worktree_count: 1 },
      model: { state: "ready", last_evidence_at: GENERATED_AT, generator_version: "fixture-1.0", indexed_files: 128 },
      architecture: { declared_docs: 2, modules: 3, relationships: 4, confidence: 0.95, evidence_refs: evidence("ev.architecture") },
      active_tasks: [{ task_id: "task.m0", title: "Freeze V3 contracts", state: "active", risk_level: "low", criteria: [criterion()], active_sessions: 1 }],
      protocol_coverage: { state: "complete", opened_sessions: 1, closed_sessions: 1, gaps: 0 },
    },
    project_structure: {
      ...shared,
      index_state: "ready",
      nodes: structureNodes,
      edges: [
        { edge_id: "edge.root.core", from_node_id: "file.root", to_node_id: "module.core", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.root") },
        { edge_id: "edge.core.lib", from_node_id: "module.core", to_node_id: "file.lib", kind: "contains", source_kind: "filesystem", confidence: 1, evidence_refs: evidence("ev.manifest") },
      ],
      unsupported_analyzers: [],
      page: { cursor: null, next_cursor: null, limit: 100, returned: 3, total_estimate: 3, truncated: false, truncation_reason: "none", model_version: "model.fixture.1" },
    },
    task_timeline: {
      ...shared,
      task_id: "task.m0", title: "Freeze V3 contracts", state: "active", criteria: [criterion()],
      lanes: [{ lane_id: "lane.session.main", kind: "session", label: "Codex main", state: "closed" }],
      events: timelineEvents,
      window: { cursor: null, next_cursor: null, limit: 100, returned: timelineEvents.length, total_estimate: timelineEvents.length, truncated: false, truncation_reason: "none", model_version: "model.fixture.1" },
    },
    plan_graph: {
      ...shared,
      task_id: "task.m0", plan_version: 1, graph_state: "valid",
      nodes: [{ node_id: "node.contracts", title: "Freeze contracts", goal: "Create M1-ready contracts", state: "active", readiness: "ready", block_reasons: [], scope: ["contracts/v3", "fixtures/v3"], criterion_ids: ["criterion.m0.c07"] }],
      scheduling_edges: [], trace_relations: [],
      execution: { planned_sessions: 1, observed_sessions: 1, planned_worktrees: 0, observed_worktrees: 0 },
    },
    node_brief: {
      ...shared,
      task_id: "task.m0", node_id: "node.contracts", goal: "Create M1-ready contracts", scope: ["contracts/v3", "fixtures/v3"], non_scope: ["V3 event store", "M1 UI"], dependencies: [],
      accepted_decisions: ["JSON Schema 2020-12 is canonical"], research_summary: ["Fixture-first isolates M1 from V2"], criteria: [criterion()], files: [root],
      validation_commands: ["npm run v3:contracts:check"], state: "active", next_intent: "Validate generated fixtures",
      budget: { max_tokens: 4000, estimated_tokens: 900, truncated_sections: [] }, source_versions: { contract: CONTRACT_VERSION, model: "model.fixture.1" }, protocol_coverage: "complete",
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
    views.project_structure.index_state = "uninitialized";
    views.project_structure.page.returned = 0;
    views.project_structure.page.total_estimate = 0;
    views.task_timeline.task_id = "task.none";
    views.task_timeline.title = "No task selected";
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
    views.node_brief.task_id = "task.none";
    views.node_brief.node_id = "node.none";
    views.node_brief.goal = "No task selected";
    views.node_brief.scope = [];
    views.node_brief.non_scope = [];
    views.node_brief.criteria = [];
    views.node_brief.files = [];
    views.node_brief.validation_commands = [];
    views.node_brief.next_intent = "Create or select a task";
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
    views.project_overview.active_tasks.push({ task_id: "task.m1", title: "Build fixture UI", state: "active", risk_level: "medium", criteria: [criterion("criterion.m1.visual", "accepted")], active_sessions: 2 });
    views.plan_graph.nodes.push(
      { node_id: "node.ui", title: "Build Project view", goal: "Render fixtures", state: "active", readiness: "ready", block_reasons: [], scope: ["src/v3/project"], criterion_ids: ["criterion.m1.visual"] },
      { node_id: "node.shared", title: "Update shared types", goal: "Resolve overlap", state: "blocked", readiness: "blocked", block_reasons: ["scope_overlap"], scope: ["src/v3/contracts"], criterion_ids: ["criterion.m1.visual"] },
    );
    views.plan_graph.execution = { planned_sessions: 3, observed_sessions: 3, planned_worktrees: 2, observed_worktrees: 2 };
    views.plan_graph.warnings.push(warning("SCOPE_OVERLAP"));
    views.task_timeline.lanes.push(
      { lane_id: "lane.session.ui", kind: "session", label: "UI session", state: "active" },
      { lane_id: "lane.session.types", kind: "session", label: "Types session", state: "idle" },
    );
    views.task_timeline.events.push(
      { timeline_event_id: "timeline.session.ui", kind: "session", occurred_at: "2026-07-11T08:11:00.000Z", recorded_at: "2026-07-11T08:11:00.000Z", order_state: "ordered", lane_id: "lane.session.ui", actor: "codex", tool: "codex", node_id: "node.ui", session_id: "session.ui", commit_sha: null, summary_key: "timeline.session.parallel", details: { scope: "src/v3/project" }, evidence_refs: evidence("ev.session.ui") },
      { timeline_event_id: "timeline.session.types", kind: "session", occurred_at: "2026-07-11T08:12:00.000Z", recorded_at: "2026-07-11T08:12:00.000Z", order_state: "ordered", lane_id: "lane.session.types", actor: "opencode", tool: "opencode", node_id: "node.shared", session_id: "session.types", commit_sha: null, summary_key: "timeline.session.overlap", details: { scope: "src/v3/contracts" }, evidence_refs: evidence("ev.session.types") },
    );
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
  }
  if (kind === "rework") {
    views.task_timeline.events.push(
      { timeline_event_id: "timeline.finding.one", kind: "finding", occurred_at: "2026-07-11T08:20:00.000Z", recorded_at: "2026-07-11T08:20:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "reviewer", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.finding.schema_gap", details: { severity: "high" }, evidence_refs: evidence("ev.finding") },
      { timeline_event_id: "timeline.attempt.one", kind: "attempt", occurred_at: "2026-07-11T08:30:00.000Z", recorded_at: "2026-07-11T08:30:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.attempt.remediation", details: { attempt: 1 }, evidence_refs: evidence("ev.attempt") },
      { timeline_event_id: "timeline.attempt.two", kind: "attempt", occurred_at: "2026-07-11T08:35:00.000Z", recorded_at: "2026-07-11T08:35:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "codex", tool: "codex", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.attempt.remediation", details: { attempt: 2 }, evidence_refs: evidence("ev.attempt.two") },
      { timeline_event_id: "timeline.validation.repass", kind: "validation", occurred_at: "2026-07-11T08:40:00.000Z", recorded_at: "2026-07-11T08:40:00.000Z", order_state: "ordered", lane_id: "lane.session.main", actor: "reviewer", tool: "npm", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.validation.repassed", details: { attempt: 2 }, evidence_refs: evidence("ev.repass") },
    );
    views.task_timeline.window.returned = views.task_timeline.events.length;
    views.task_timeline.window.total_estimate = views.task_timeline.events.length;
    views.plan_graph.trace_relations.push({ relation_id: "trace.attempt.finding", from_id: "timeline.attempt.one", to_id: "timeline.finding.one", kind: "addresses", evidence_refs: evidence("ev.attempt") });
    views.plan_graph.trace_relations.push({ relation_id: "trace.attempt.two.finding", from_id: "timeline.attempt.two", to_id: "timeline.finding.one", kind: "repairs", evidence_refs: evidence("ev.attempt.two") });
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
      view.errors.push(error("PROJECTION_UNAVAILABLE", true), error("CONTRACT_TERMINAL", false, "validation"));
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
    views.task_timeline.events.push({ timeline_event_id: "timeline.gap.recovered", kind: "gap", occurred_at: "2026-07-11T08:15:00.000Z", recorded_at: "2026-07-11T08:45:00.000Z", order_state: "late", lane_id: "lane.session.main", actor: "doctor", tool: "vibehub", node_id: "node.contracts", session_id: "session.main", commit_sha: null, summary_key: "timeline.gap.recovered", details: { missing: ["session_close"], host_capability: "unknown" }, evidence_refs: evidence("ev.gap") });
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
