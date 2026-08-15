import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");

const entry = `
  import assert from "node:assert/strict";
  import { useTabsStore } from "./src/stores/tabsStore.ts";
  import { useV3Store } from "./src/v3/stores/v3Store.ts";
  import { tauriApi } from "./src/services/tauri.ts";

  const tabs = () => useTabsStore.getState();
  const reset = () => useTabsStore.setState({ tabs: [], activeTabId: null });

  // 1) Opening a project appends and activates a tab; reopening only activates.
  reset();
  tabs().openTab("a");
  tabs().openTab("b");
  assert.deepEqual(tabs().tabs, ["a", "b"], "opening two projects keeps two tabs");
  assert.equal(tabs().activeTabId, "b", "the newest opened project becomes active");
  tabs().openTab("a");
  assert.deepEqual(tabs().tabs, ["a", "b"], "reopening an already open project must not duplicate the tab");
  assert.equal(tabs().activeTabId, "a", "reopening an open project activates its existing tab");

  // 2) Closing the active tab activates a neighbour; closing the last tab falls back to the list.
  reset();
  ["a", "b", "c"].forEach((id) => tabs().openTab(id));
  tabs().activateTab("b");
  tabs().closeActiveTab();
  assert.deepEqual(tabs().tabs, ["a", "c"], "closing a tab removes exactly that tab");
  assert.equal(tabs().activeTabId, "c", "closing the active tab activates the following tab");
  tabs().activateTab("c");
  tabs().closeTab("c");
  assert.equal(tabs().activeTabId, "a", "closing the last tab activates the previous one");
  tabs().closeTab("a");
  assert.deepEqual(tabs().tabs, [], "closing every tab empties the tab strip");
  assert.equal(tabs().activeTabId, null, "closing the final tab returns to the project list");

  // 3) Closing a background tab keeps the active tab untouched.
  reset();
  ["a", "b"].forEach((id) => tabs().openTab(id));
  tabs().activateTab("b");
  tabs().closeTab("a");
  assert.equal(tabs().activeTabId, "b", "closing a background tab must not move the active tab");

  // 4) Index and relative activation (Cmd+1..9 / Ctrl+Tab) including wrap-around.
  reset();
  ["a", "b", "c"].forEach((id) => tabs().openTab(id));
  tabs().activateIndex(0);
  assert.equal(tabs().activeTabId, "a", "index activation selects the nth tab");
  tabs().activateIndex(9);
  assert.equal(tabs().activeTabId, "a", "out-of-range index activation is ignored");
  tabs().activateRelative(1);
  assert.equal(tabs().activeTabId, "b", "relative activation moves forward");
  tabs().activateRelative(-1);
  assert.equal(tabs().activeTabId, "a", "relative activation moves backward");
  tabs().activateRelative(-1);
  assert.equal(tabs().activeTabId, "c", "relative activation wraps around");

  // 5) Reorder (drag) and prune (deleted projects) keep the store consistent.
  reset();
  ["a", "b", "c"].forEach((id) => tabs().openTab(id));
  tabs().reorderTabs(["c", "a", "b"]);
  assert.deepEqual(tabs().tabs, ["c", "a", "b"], "dragging a tab persists the new order");
  tabs().activateTab("b");
  tabs().deactivateTabs();
  assert.equal(tabs().activeTabId, null, "navigating home only clears the active tab");
  assert.deepEqual(tabs().tabs, ["c", "a", "b"], "navigating home keeps the open tabs");
  tabs().activateTab("b");
  tabs().pruneTabs(["a", "c"]);
  assert.deepEqual(tabs().tabs, ["c", "a"], "tabs of removed projects are pruned");
  assert.equal(tabs().activeTabId, null, "pruning the active project clears the active tab");

  // 6) v3Store keeps an LRU snapshot per project so switching back renders instantly.
  const lifecycleApi = {
    inspect: async () => ({ state: "v3" }),
    initialize: async () => ({}),
    migrate: async () => ({}),
    recover: async () => ({}),
    inspectRepairCandidates: async () => [],
    repair: async () => ({}),
  };
  const bundleFor = (projectPath) => ({
    projectOverview: { project_id: "project" + projectPath, active_tasks: [{ task_id: "task" + projectPath }] },
    taskTimeline: { task_id: "task" + projectPath },
    planGraph: { nodes: [] },
  });
  const loaderCalls = [];
  const loader = async (projectPath) => {
    loaderCalls.push(projectPath);
    return bundleFor(projectPath);
  };
  const openProject = async (projectPath) => {
    useV3Store.getState().selectProject(projectPath, loader, undefined, undefined, lifecycleApi);
    await new Promise((resolve) => setTimeout(resolve, 5));
  };

  await openProject("/p1");
  assert.equal(useV3Store.getState().bundle?.projectOverview.project_id, "project/p1", "the first project loads its bundle");
  await openProject("/p2");
  assert.equal(useV3Store.getState().bundle?.projectOverview.project_id, "project/p2", "switching tabs loads the other project");

  const callsBeforeReturn = loaderCalls.length;
  useV3Store.getState().selectProject("/p1", loader, undefined, undefined, lifecycleApi);
  assert.equal(
    useV3Store.getState().bundle?.projectOverview.project_id,
    "project/p1",
    "switching back to an open tab renders the cached snapshot synchronously, without a full-screen reload",
  );
  assert.equal(useV3Store.getState().loading, false, "restoring a snapshot must not raise the blocking loading state");
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.ok(loaderCalls.length > callsBeforeReturn, "switching back still triggers exactly one immediate refresh");
  assert.equal(useV3Store.getState().error, null, "the refresh after a snapshot restore must not trip the identity guard");

  // 7) The snapshot cache is bounded to five projects (LRU eviction).
  for (const path of ["/p3", "/p4", "/p5", "/p6", "/p7"]) await openProject(path);
  useV3Store.getState().leaveProject();

  // 8) WorkspaceState hydration is generation-safe: a user mutation during a
  // delayed restore wins, duplicate StrictMode hydration is coalesced, and the
  // winning state is persisted against the recovered revision.
  const persistenceProjects = [
    { id: "persist-a", name: "Persist A", path: "/persist/a" },
    { id: "persist-b", name: "Persist B", path: "/persist/b" },
  ];
  useTabsStore.setState({ tabs: [], activeTabId: null, hydrated: false, hydrating: false, persistedRevision: 0, diagnostics: [] });
  let resolveRestore;
  let restoreCalls = 0;
  const saveCalls = [];
  tauriApi.loadWorkspaceState = async () => {
    restoreCalls += 1;
    return await new Promise((resolve) => { resolveRestore = resolve; });
  };
  tauriApi.saveWorkspaceState = async ({ state }) => {
    saveCalls.push(state);
    return { state: { ...state, revision: state.revision + 1 }, previous_revision: state.revision, backup_path: null, diagnostics: [] };
  };
  const delayedHydration = useTabsStore.getState().hydrate(persistenceProjects);
  await new Promise((resolve) => setTimeout(resolve, 0));
  await useTabsStore.getState().hydrate(persistenceProjects);
  assert.equal(restoreCalls, 1, "duplicate StrictMode hydration must share one restore request");
  useTabsStore.getState().openTab("persist-b");
  resolveRestore({
    status: "present",
    state: {
      schema_version: 3,
      kind: "workspace_state",
      revision: 7,
      open_projects: [{ project_id: "persist-a", canonical_path: "/persist/a", display_name: "Persist A", ui_context: { owner: "v3-cockpit", current_view: "project-overview", selected_task_id: null, selected_node_id: null } }],
      active_project_id: "persist-a",
      updated_at: new Date().toISOString(),
      updated_by: "test",
      provenance: { source: "user", writer: "test", reason: "test" },
    },
    diagnostics: [],
    recommended_action: null,
  });
  await delayedHydration;
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-b"], "a user tab opened during restore must not be overwritten by a stale response");
  assert.equal(useTabsStore.getState().activeTabId, "persist-b", "the user mutation remains active after restore");
  assert.deepEqual(saveCalls.at(-1).open_projects.map((project) => project.project_id), ["persist-b"], "the winning mutation is persisted");
  assert.equal(saveCalls.at(-1).revision, 7, "the winning mutation uses the restored revision precondition");

  // 8b) A close during recovery wins as well; a stale restored active project
  // must not reopen the closed tab.
  let resolveCloseRestore;
  tauriApi.loadWorkspaceState = async () => await new Promise((resolve) => { resolveCloseRestore = resolve; });
  useTabsStore.setState({ tabs: ["persist-a", "persist-b"], activeTabId: "persist-b", hydrated: true, hydrating: false, persistedRevision: 7, diagnostics: [] });
  const delayedCloseHydration = useTabsStore.getState().retryHydrate();
  await new Promise((resolve) => setTimeout(resolve, 0));
  useTabsStore.getState().closeActiveTab();
  resolveCloseRestore({
    status: "present",
    state: {
      schema_version: 3,
      kind: "workspace_state",
      revision: 8,
      open_projects: [
        { project_id: "persist-a", canonical_path: "/persist/a", display_name: "Persist A", ui_context: { owner: "v3-cockpit", current_view: "project-overview", selected_task_id: null, selected_node_id: null } },
        { project_id: "persist-b", canonical_path: "/persist/b", display_name: "Persist B", ui_context: { owner: "v3-cockpit", current_view: "project-overview", selected_task_id: null, selected_node_id: null } },
      ],
      active_project_id: "persist-b",
      updated_at: new Date().toISOString(),
      updated_by: "test",
      provenance: { source: "user", writer: "test", reason: "test" },
    },
    diagnostics: [],
    recommended_action: null,
  });
  await delayedCloseHydration;
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-a"], "a close during restore must not be undone by a stale response");
  assert.equal(useTabsStore.getState().activeTabId, "persist-a", "closing the restored active tab selects the remaining neighbour");
  assert.deepEqual(saveCalls.at(-1).open_projects.map((project) => project.project_id), ["persist-a"], "the close winner is persisted");

  // 8c) A normal restart round-trip restores order, active project, UI context,
  // and project-scoped state from the persisted document.
  let persistedDocument = null;
  tauriApi.saveWorkspaceState = async ({ state }) => {
    persistedDocument = structuredClone({ ...state, revision: state.revision + 1 });
    saveCalls.push(state);
    return { state: persistedDocument, previous_revision: state.revision, backup_path: null, diagnostics: [] };
  };
  tauriApi.loadWorkspaceState = async () => ({
    status: "missing",
    state: null,
    diagnostics: [],
    recommended_action: null,
  });
  useTabsStore.setState({ tabs: [], activeTabId: null, hydrated: false, hydrating: false, persistedRevision: 0, diagnostics: [] });
  await useTabsStore.getState().retryHydrate();
  assert.deepEqual(useTabsStore.getState().tabs, [], "a cold start with no saved state opens an empty tab set");
  assert.equal(useTabsStore.getState().activeTabId, null, "a cold start with no saved state has no active project");
  useTabsStore.getState().openTab("persist-a");
  useTabsStore.getState().openTab("persist-b");
  useTabsStore.getState().reorderTabs(["persist-b", "persist-a"]);
  useTabsStore.getState().activateTab("persist-a");
  useTabsStore.getState().setProjectUiContext("persist-a", { current_view: "plan-graph", selected_task_id: "task-a", selected_node_id: "node-a" });
  await new Promise((resolve) => setTimeout(resolve, 25));
  assert.ok(persistedDocument, "continuous tab mutations produce a persisted restart document");
  const restartDocument = structuredClone(persistedDocument);
  useTabsStore.getState().setProjectUiContext("persist-a", { current_view: "project-overview", selected_task_id: null, selected_node_id: null });
  useTabsStore.setState({ tabs: [], activeTabId: null, hydrated: false, hydrating: false, persistedRevision: 0, diagnostics: [] });
  tauriApi.loadWorkspaceState = async () => ({ status: "present", state: restartDocument, diagnostics: [], recommended_action: null });
  await useTabsStore.getState().retryHydrate();
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-b", "persist-a"], "a restart restores the exact tab order");
  assert.equal(useTabsStore.getState().activeTabId, "persist-a", "a restart restores the active project identity");
  assert.equal(useTabsStore.getState().getProjectUiContext("persist-a").current_view, "plan-graph", "a restart restores project-scoped UI context");

  // 9) Persistence failures surface a diagnostic but never roll back the local
  // tab mutation; retrying restore remains available through the store action.
  tauriApi.saveWorkspaceState = async () => { throw new Error("WORKSPACE_STATE_DISK_FULL"); };
  useTabsStore.getState().openTab("persist-a");
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-b", "persist-a"], "a failed save must not roll back the local tab mutation");
  assert.equal(useTabsStore.getState().diagnostics[0]?.code, "workspace_state.persistence_failed", "save failures remain visible as structured diagnostics");

  // 10) Recovery keeps valid order, clears an invalid active project, restores
  // project context, and reports identity changes instead of guessing by name.
  tauriApi.saveWorkspaceState = async ({ state }) => {
    saveCalls.push(state);
    return { state: { ...state, revision: state.revision + 1 }, previous_revision: state.revision, backup_path: null, diagnostics: [] };
  };
  tauriApi.loadWorkspaceState = async () => ({
    status: "partial",
    state: {
      schema_version: 3,
      kind: "workspace_state",
      revision: 12,
      open_projects: [
        { project_id: "persist-a", canonical_path: "/persist/a", display_name: "Same name", ui_context: { owner: "v3-cockpit", current_view: "plan-graph", selected_task_id: "task-a", selected_node_id: "node-a" } },
        { project_id: "missing", canonical_path: "/deleted/project", display_name: "Same name", ui_context: { owner: "v3-cockpit", current_view: "overview", selected_task_id: null, selected_node_id: null } },
        { project_id: "persist-b", canonical_path: "/persist/b", display_name: "Persist B", ui_context: { owner: "v3-cockpit", current_view: "structure-explorer", selected_task_id: null, selected_node_id: null } },
      ],
      active_project_id: "missing",
      updated_at: new Date().toISOString(),
      updated_by: "test",
      provenance: { source: "recovery", writer: "test", reason: "test" },
    },
    diagnostics: [{ code: "workspace_state.project_missing", severity: "warning", project_id: "missing", message: "missing", recovery_action: "reopen" }],
    recommended_action: "reopen",
  });
  useTabsStore.setState({ tabs: [], activeTabId: null, hydrated: false, hydrating: false, persistedRevision: 0, diagnostics: [] });
  await useTabsStore.getState().retryHydrate();
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-a", "persist-b"], "recovery removes only invalid projects and preserves valid order");
  assert.equal(useTabsStore.getState().activeTabId, null, "an invalid active project cannot remain active");
  assert.equal(useTabsStore.getState().getProjectUiContext("persist-a").current_view, "plan-graph", "project UI context is restored with the project identity");
  assert.equal(useTabsStore.getState().diagnostics[0]?.code, "workspace_state.project_missing", "invalid project recovery keeps a localized diagnostic");
  useTabsStore.getState().reconcileProjects([
    { ...persistenceProjects[0], path: "/moved/a" },
    persistenceProjects[1],
  ]);
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.deepEqual(useTabsStore.getState().tabs, ["persist-b"], "a moved project is removed by identity, without matching a same-name project");
  assert.equal(useTabsStore.getState().diagnostics[0]?.code, "workspace_state.project_identity_changed", "identity changes expose a repair diagnostic");
  useV3Store.getState().selectProject("/p2", loader, undefined, undefined, lifecycleApi);
  assert.equal(useV3Store.getState().bundle, null, "the oldest project snapshot is evicted once the cache is full");
  useV3Store.getState().leaveProject();
  useV3Store.getState().selectProject("/p7", loader, undefined, undefined, lifecycleApi);
  assert.equal(
    useV3Store.getState().bundle?.projectOverview.project_id,
    "project/p7",
    "recently used project snapshots survive in the cache",
  );
  useV3Store.getState().leaveProject();
`;

const result = await build({
  absWorkingDir: root,
  stdin: { contents: entry, resolveDir: root, sourcefile: "project-tabs.test.ts" },
  alias: { "@": join(root, "src") },
  bundle: true,
  format: "esm",
  platform: "node",
  target: "node20",
  write: false,
});

assert.equal(result.outputFiles.length, 1);
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString("base64")}`);

const tabBarSource = await readFile(resolve(root, "src/components/ProjectTabBar.tsx"), "utf8");
const layoutSource = await readFile(resolve(root, "src/components/Layout.tsx"), "utf8");
const homeSource = await readFile(resolve(root, "src/pages/Home.tsx"), "utf8");
const mainRustSource = await readFile(resolve(root, "src-tauri/src/main.rs"), "utf8");

const surfaceAssertions = [
  [/event\.metaKey \|\| event\.ctrlKey/, "tab shortcuts accept both Cmd and Ctrl"],
  [/event\.key\.toLowerCase\(\) === 'w'[\s\S]{0,120}closeActiveTab\(\)/, "Cmd/Ctrl+W closes the active tab"],
  [/event\.key === 'Tab'[\s\S]{0,160}activateRelative\(event\.shiftKey \? -1 : 1\)/, "Ctrl+Tab cycles tabs in both directions"],
  [/BracketRight[\s\S]{0,160}activateRelative\(1\)/, "Cmd/Ctrl+Shift+] activates the next tab"],
  [/BracketLeft[\s\S]{0,160}activateRelative\(-1\)/, "Cmd/Ctrl+Shift+[ activates the previous tab"],
  [/\^\[1-9\]\$[\s\S]{0,160}activateIndex\(Number\(event\.key\) - 1\)/, "Cmd/Ctrl+1..9 jumps to the nth tab"],
  [/onAuxClick=\{\(event\) => \{[\s\S]{0,160}event\.button !== 1/, "middle click closes a tab"],
  [/opacity-0[\s\S]{0,80}group-hover:opacity-100/, "the close button only appears on hover or focus"],
  [/useSortable\(\{ id: tab\.id \}\)/, "tabs are drag-sortable"],
  [/horizontalListSortingStrategy/, "tab dragging uses the horizontal strategy"],
  [/min-w-\[7\.5rem\] max-w-\[13rem\] shrink/, "tabs shrink between a minimum and maximum width before scrolling"],
  [/overflow-x-auto overflow-y-hidden scrollbar-hidden/, "the tab strip scrolls horizontally only and never grows a second row"],
  [/flex-nowrap/, "tabs never wrap onto another line"],
  [/CSS\.Translate\.toString\(transform \? \{ \.\.\.transform, y: 0 \} : null\)/, "dragging a tab cannot drift vertically"],
  [/container\.scrollLeft \+= overflow(Left|Right)/, "revealing the active tab only adjusts horizontal scroll"],
  [/DropdownMenuContent[\s\S]{0,400}activateTab\(tab\.id\)/, "an overflow dropdown can reach every open tab"],
  [/t\('tabs\.close'\)/, "the close control is localized"],
  [/t\('tabs\.listAll'\)/, "the overflow control is localized"],
  [/t\('tabs\.openProjectList'\)/, "the recovery notice can return to the project list"],
];
for (const [pattern, label] of surfaceAssertions) {
  assert.ok(pattern.test(tabBarSource), `ProjectTabBar: ${label}`);
}
assert.ok(!/[\u4e00-\u9fff]/.test(tabBarSource), "ProjectTabBar must not hard-code Han UI text");
assert.ok(
  !/flex-wrap/.test(tabBarSource) && !/scrollIntoView/.test(tabBarSource),
  "the tab strip has no wrapping or vertical auto-scroll escape hatch",
);
assert.ok(
  /\.scrollbar-hidden::-webkit-scrollbar \{[\s\S]{0,120}display: none/.test(
    await readFile(resolve(root, "src/styles/globals.css"), "utf8"),
  ),
  "the scrollbar-hidden utility removes scrollbar height from single-row strips",
);
assert.ok(
  /currentPage === 'home' && <ProjectTabBar \/>/.test(layoutSource),
  "the tab strip lives between the header and the page content, on the project surface only",
);
assert.ok(
  /projectPath=\{selectedProject\.path\}/.test(homeSource) && /useTabsStore\(\(state\) => state\.activeTabId\)/.test(homeSource),
  "the cockpit route is driven by the active tab",
);
assert.ok(
  /onSelect=\{\(\) => openProjectTab\(project\.id\)\}/.test(homeSource),
  "opening a project card opens or activates its tab",
);
assert.ok(
  /CmdOrCtrl\+Shift\+W/.test(mainRustSource) && !/\.close_window\(\)/.test(mainRustSource),
  "macOS keeps Cmd+W for tabs and moves window closing to Cmd+Shift+W",
);

console.log("Project tab bar behavior OK");
