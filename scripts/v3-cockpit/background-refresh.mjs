import assert from "node:assert/strict";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const entry = `
  import assert from "node:assert/strict";
  import { useV3Store } from "./src/v3/stores/v3Store.ts";
  import { projectSetupSignature, shouldAdoptProjectSetupValues } from "./src/v3/components/project/projectSetupFormSync.ts";

  const lifecycleApi = {
    inspect: async () => ({ state: "v3" }),
    initialize: async () => ({}),
    migrate: async () => ({}),
    recover: async () => ({}),
    inspectRepairCandidates: async () => [],
    repair: async () => ({}),
  };

  // 1) A project without a current task keeps failing to load a bundle.
  //    The initial load must expose the loading state, the 4s background poll must not.
  let loaderCalls = 0;
  const failingLoader = async () => {
    loaderCalls += 1;
    throw new Error("CURRENT_TASK_NOT_FOUND: no current task");
  };

  useV3Store.getState().selectProject("/tmp/uninitialized", failingLoader, undefined, undefined, lifecycleApi);
  await new Promise((resolve) => setTimeout(resolve, 10));
  const callsBeforeForeground = loaderCalls;
  const foregroundStates = [];
  const unsubscribeForeground = useV3Store.subscribe((state) => foregroundStates.push({ loading: state.loading, bundle: state.bundle, error: state.error }));
  await useV3Store.getState().loadCurrentBundle();
  unsubscribeForeground();
  assert.ok(foregroundStates.some((snapshot) => snapshot.loading === true), "the first foreground load must still surface the loading state");
  assert.equal(useV3Store.getState().bundle, null, "a project without a current task has no bundle");
  assert.match(useV3Store.getState().error ?? "", /CURRENT_TASK/, "the not-found error must be kept for the setup empty state");

  const callsBeforeBackground = loaderCalls;
  assert.equal(callsBeforeBackground, callsBeforeForeground + 1, "the foreground load must hit the loader exactly once");
  const backgroundStates = [];
  const unsubscribeBackground = useV3Store.subscribe((state) => backgroundStates.push({ loading: state.loading, bundle: state.bundle, error: state.error }));
  for (let tick = 0; tick < 3; tick += 1) {
    await useV3Store.getState().loadCurrentBundle(undefined, { background: true });
  }
  unsubscribeBackground();
  assert.equal(loaderCalls - callsBeforeBackground, 3, "each background tick must still hit the loader");
  assert.equal(
    backgroundStates.some((snapshot) => snapshot.loading === true && snapshot.bundle === null),
    false,
    "background polling must never enter the (loading && !bundle) full-screen branch that unmounts the setup modal",
  );
  assert.equal(backgroundStates.length, 0, "an unchanged background failure must not churn store state");
  assert.match(useV3Store.getState().error ?? "", /CURRENT_TASK/, "background polling must keep the not-found error stable");

  // Regression witness: a non-background refresh (the old polling behavior) still flips into that branch.
  const foregroundPollStates = [];
  const unsubscribeForegroundPoll = useV3Store.subscribe((state) => foregroundPollStates.push({ loading: state.loading, bundle: state.bundle }));
  await useV3Store.getState().loadCurrentBundle();
  unsubscribeForegroundPoll();
  assert.ok(
    foregroundPollStates.some((snapshot) => snapshot.loading === true && snapshot.bundle === null),
    "a foreground refresh must still show the full-screen loading branch, proving the background flag is what protects the setup modal",
  );

  // 2) A background refresh that finally finds a task must still update the cockpit.
  const bundle = {
    projectOverview: { project_id: "project.test", active_tasks: [{ task_id: "task.new", criteria: [], active_sessions: 0 }], archived_tasks: [] },
    projectStructure: {},
    agentResults: {},
    taskTimeline: { task_id: "task.new", events: [], lanes: [] },
    planGraph: { task_id: "task.new", nodes: [] },
    nodeBrief: { task_id: "task.new" },
  };
  useV3Store.getState().selectProject("/tmp/with-task", async () => bundle, undefined, undefined, lifecycleApi);
  await useV3Store.getState().loadCurrentBundle(undefined, { background: true });
  assert.equal(useV3Store.getState().bundle, bundle, "a successful background refresh must publish the new bundle");
  assert.equal(useV3Store.getState().error, null, "a successful background refresh must clear a stale error");
  assert.equal(useV3Store.getState().loading, false, "a successful background refresh must not leave the loading flag set");

  // 3) The setup form must survive prop refreshes that carry identical server data.
  const serverValues = () => ({ language: "zh-CN", gitUrl: "", tools: ["opencode", "codex"], revision: 3 });
  const first = projectSetupSignature(serverValues());
  const second = projectSetupSignature(serverValues());
  assert.equal(first, second, "identical server settings must produce a stable signature despite new array identities");
  assert.equal(
    shouldAdoptProjectSetupValues({ nextSignature: second, syncedSignature: first, dirty: false }),
    false,
    "a refresh with unchanged settings must not reset the form",
  );
  assert.equal(
    shouldAdoptProjectSetupValues({ nextSignature: projectSetupSignature({ ...serverValues(), revision: 4 }), syncedSignature: first, dirty: true }),
    false,
    "a refresh must not overwrite selections the user has not submitted yet",
  );
  assert.equal(
    shouldAdoptProjectSetupValues({ nextSignature: projectSetupSignature({ ...serverValues(), revision: 4 }), syncedSignature: first, dirty: false }),
    true,
    "a pristine form must still adopt genuinely new server settings",
  );
  assert.equal(
    projectSetupSignature({ language: "zh-CN", gitUrl: "", tools: ["codex", "opencode"], revision: 3 }),
    first,
    "tool order must not affect the signature",
  );
`;

const result = await build({
  absWorkingDir: root,
  stdin: { contents: entry, resolveDir: root, sourcefile: "cockpit-background-refresh.test.ts" },
  alias: { "@": join(root, "src") },
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node20",
  write: false,
});

assert.equal(result.outputFiles.length, 1);
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString("base64")}`);
console.log("V3 cockpit background refresh behavior OK");
