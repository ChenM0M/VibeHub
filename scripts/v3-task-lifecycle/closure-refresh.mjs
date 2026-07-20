import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const entry = `
  import assert from "node:assert/strict";
  import { useV3Store } from "./src/v3/stores/v3Store.ts";
  import { findArchivedTaskAfterClosure } from "./src/v3/app/taskClosure.ts";

  const oldTaskId = "task.old";
  const nextTaskId = "task.next";
  const archivedTask = { task_id: oldTaskId, title: "Archived task" };
  const bundle = (activeTaskId, archivedTasks = []) => ({
    projectOverview: {
      project_id: "project.test",
      active_tasks: activeTaskId ? [{ task_id: activeTaskId, criteria: [], active_sessions: 0 }] : [],
      archived_tasks: archivedTasks,
    },
    projectStructure: {},
    agentResults: {},
    taskTimeline: { task_id: activeTaskId, events: [], lanes: [] },
    planGraph: { task_id: activeTaskId, nodes: [] },
    nodeBrief: { task_id: activeTaskId },
  });

  const loaderCalls = [];
  let closed = false;
  const loader = async (_projectPath, taskId, expectedProjectId) => {
    loaderCalls.push({ taskId, expectedProjectId });
    return closed ? bundle(nextTaskId, [archivedTask]) : bundle(oldTaskId);
  };
  const lifecycleApi = {
    inspect: async () => ({ state: "v3" }),
    initialize: async () => ({}),
    migrate: async () => ({}),
    recover: async () => ({}),
    inspectRepairCandidates: async () => [],
    repair: async () => ({}),
  };

  const store = useV3Store.getState();
  store.selectProject("/tmp/project", loader, undefined, undefined, lifecycleApi);
  await useV3Store.getState().loadCurrentBundle();
  assert.equal(useV3Store.getState().selectedTaskId, oldTaskId, "initial active task must be selected");

  closed = true;
  const refreshed = await useV3Store.getState().loadCurrentBundle(null);
  assert.equal(loaderCalls.at(-1).taskId, null, "post-closure refresh must not request the archived task id");
  assert.equal(loaderCalls.at(-1).expectedProjectId, "project.test", "project identity guard must remain active");
  assert.equal(useV3Store.getState().selectedTaskId, nextTaskId, "selection must move to the new current active task");
  assert.equal(refreshed.projectOverview.active_tasks.some((task) => task.task_id === oldTaskId), false, "closed task must leave active tasks");
  assert.equal(findArchivedTaskAfterClosure(refreshed, oldTaskId), archivedTask, "closed task must be found in the archived projection");
  assert.equal(findArchivedTaskAfterClosure(refreshed, "task.missing"), null, "a missing archived projection must be treated as an error condition");
`;

const result = await build({
  absWorkingDir: root,
  stdin: { contents: entry, resolveDir: root, sourcefile: "closure-refresh.test.ts" },
  alias: { "@": join(root, "src") },
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node20",
  write: false,
});

assert.equal(result.outputFiles.length, 1);
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString("base64")}`);
console.log("V3 task closure refresh behavior OK");
