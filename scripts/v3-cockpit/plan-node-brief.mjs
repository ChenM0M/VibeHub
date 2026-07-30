import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const entry = `
  import assert from "node:assert/strict";
  import { useV3Store } from "./src/v3/stores/v3Store.ts";
  import { planNodeDetailView } from "./src/v3/components/task/planNodeDetail.ts";

  const lifecycleApi = {
    inspect: async () => ({ state: "v3" }),
    initialize: async () => ({}),
    migrate: async () => ({}),
    recover: async () => ({}),
    inspectRepairCandidates: async () => [],
    repair: async () => ({}),
  };

  // The projection only ever carries the brief of one node (the "current" one).
  // Every other node in the plan graph - completed ones in particular - used to
  // be a dead click in the plan view.
  const bundle = {
    projectOverview: { project_id: "project.test", active_tasks: [{ task_id: "task.plan", criteria: [], active_sessions: 0 }], archived_tasks: [] },
    projectStructure: {},
    agentResults: {},
    taskTimeline: { task_id: "task.plan", events: [], lanes: [] },
    planGraph: {
      task_id: "task.plan",
      nodes: [
        { node_id: "node.done", title: "Done", goal: "Finished work", state: "completed", scope: ["src/done.ts"], criterion_ids: ["criterion.c01"], session_ids: [], readiness: "ready", block_reasons: [], blocker_details: [] },
        { node_id: "node.current", title: "Current", goal: "Ongoing work", state: "active", scope: [], criterion_ids: [], session_ids: [], readiness: "ready", block_reasons: [], blocker_details: [] },
      ],
    },
    nodeBrief: { task_id: "task.plan", node_id: "node.current", goal: "Ongoing work", state: "active" },
  };
  const loadBundle = async () => bundle;

  const briefCalls = [];
  const briefLoader = async (projectPath, taskId, nodeId, expectedProjectId) => {
    briefCalls.push({ projectPath, taskId, nodeId, expectedProjectId });
    return { task_id: taskId, node_id: nodeId, goal: "Finished work", state: "completed", scope: ["src/done.ts"] };
  };

  useV3Store.getState().selectProject("/tmp/plan", loadBundle, undefined, undefined, lifecycleApi, undefined, briefLoader);
  await useV3Store.getState().loadCurrentBundle();
  assert.equal(useV3Store.getState().bundle, bundle, "the bundle must be loaded before any node is inspected");

  // 1) The node the projection already covers must resolve without a round trip.
  const current = await useV3Store.getState().loadNodeBrief("node.current");
  assert.equal(current, bundle.nodeBrief, "the current node must reuse the brief already in the bundle");
  assert.equal(briefCalls.length, 0, "the current node must not cost an extra IPC round trip");
  assert.deepEqual(useV3Store.getState().nodeBriefDetail, { nodeId: "node.current", status: "ready", brief: bundle.nodeBrief, error: null });

  // 2) Regression witness: a completed node must now open, with its OWN data.
  const done = await useV3Store.getState().loadNodeBrief("node.done");
  assert.equal(done.node_id, "node.done", "a completed node must resolve to its own brief");
  assert.equal(done.state, "completed", "the completed node brief must not be the current node brief");
  assert.equal(briefCalls.length, 1, "a non-current node must be fetched by node_id");
  assert.deepEqual(briefCalls[0], { projectPath: "/tmp/plan", taskId: "task.plan", nodeId: "node.done", expectedProjectId: "project.test" }, "the fetch must be scoped to the project, task and node");
  const detail = useV3Store.getState().nodeBriefDetail;
  assert.equal(detail.status, "ready");
  assert.equal(detail.nodeId, "node.done");
  assert.equal(useV3Store.getState().selectedNodeId, "node.done", "inspecting a node must track it as the selected node");

  // 3) A node outside the projected plan must fail loudly, never silently.
  await useV3Store.getState().loadNodeBrief("node.absent");
  assert.match(useV3Store.getState().nodeBriefDetail.error ?? "", /V3_NODE_NOT_FOUND/, "an unknown node must report why it cannot be opened");
  assert.equal(useV3Store.getState().nodeBriefDetail.status, "unavailable");

  // 4) A backend failure must surface as an unavailable state with the reason.
  useV3Store.getState().clearNodeBrief();
  assert.equal(useV3Store.getState().nodeBriefDetail, null, "clearing must reset the drawer state");
  useV3Store.getState().selectProject("/tmp/plan-failing", loadBundle, undefined, undefined, lifecycleApi, undefined, async () => {
    throw new Error("V3_VIEW_TASK_FAILED: boom");
  });
  await useV3Store.getState().loadCurrentBundle();
  await useV3Store.getState().loadNodeBrief("node.done");
  assert.equal(useV3Store.getState().nodeBriefDetail.status, "unavailable");
  assert.match(useV3Store.getState().nodeBriefDetail.error ?? "", /V3_VIEW_TASK_FAILED/, "a failed fetch must keep the real backend reason");

  // 5) A source without a per-node loader (fixtures/demo) must say so explicitly.
  useV3Store.getState().selectProject("/tmp/plan-no-loader", loadBundle, undefined, undefined, lifecycleApi);
  await useV3Store.getState().loadCurrentBundle();
  await useV3Store.getState().loadNodeBrief("node.done");
  assert.equal(useV3Store.getState().nodeBriefDetail.status, "unavailable");
  assert.match(useV3Store.getState().nodeBriefDetail.error ?? "", /V3_NODE_BRIEF_UNAVAILABLE/, "a source without the per-node pathway must state that, not stay silent");

  // 6) Whatever node was clicked, the drawer must derive that node's own facts.
  const taskCriteria = [
    { criterion_id: "criterion.c01", title: "Done criterion", status: "passed", required: true, evidence_refs: [] },
    { criterion_id: "criterion.c02", title: "Other criterion", status: "accepted", required: true, evidence_refs: [] },
  ];
  const planGraph = {
    nodes: bundle.planGraph.nodes,
    scheduling_edges: [{ edge_id: "schedule.node.done.node.current", from_node_id: "node.done", to_node_id: "node.current", kind: "depends_on" }],
  };
  const doneView = planNodeDetailView(planGraph, "node.done", taskCriteria);
  assert.equal(doneView.node.node_id, "node.done", "the drawer must resolve the clicked node, not the current one");
  assert.equal(doneView.node.state, "completed", "a completed node must still resolve its own facts");
  assert.deepEqual(doneView.dependencies, [], "a node without incoming edges has no dependencies");
  assert.deepEqual(doneView.criteria.map((criterion) => criterion.criterion_id), ["criterion.c01"], "only the criteria linked to the node may be shown as its criteria");
  const currentView = planNodeDetailView(planGraph, "node.current", taskCriteria);
  assert.deepEqual(currentView.dependencies, ["node.done"], "dependencies must come from the scheduling edges pointing at the node");
  assert.deepEqual(currentView.criteria, [], "a node without linked criteria must not inherit task-level criteria");
  const strayView = planNodeDetailView(planGraph, "node.absent", taskCriteria);
  assert.equal(strayView.node, null, "a node outside the plan graph must resolve to no node instead of throwing");
  assert.deepEqual(strayView.criteria, []);
  assert.deepEqual(planNodeDetailView(planGraph, null, taskCriteria), { node: null, dependencies: [], criteria: [] }, "no selection means no derived detail");

  // 7) A loader that answers with a different node must never be trusted.
  useV3Store.getState().selectProject("/tmp/plan-mismatch", loadBundle, undefined, undefined, lifecycleApi, undefined, async () => ({ task_id: "task.plan", node_id: "node.other", goal: "Wrong", state: "completed" }));
  await useV3Store.getState().loadCurrentBundle();
  await useV3Store.getState().loadNodeBrief("node.done");
  assert.equal(useV3Store.getState().nodeBriefDetail.status, "unavailable");
  assert.match(useV3Store.getState().nodeBriefDetail.error ?? "", /V3_IDENTITY_MISMATCH/, "a mismatched brief must be rejected instead of shown under the wrong node");
`;

const result = await build({
  absWorkingDir: root,
  stdin: { contents: entry, resolveDir: root, sourcefile: "cockpit-plan-node-brief.test.ts" },
  alias: { "@": join(root, "src") },
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node20",
  write: false,
});

assert.equal(result.outputFiles.length, 1);
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString("base64")}`);
console.log("V3 cockpit plan node brief behavior OK");
