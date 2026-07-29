import { create } from "zustand";
import type {
  V3FixtureBundle,
  V3FixtureScenario,
} from "@/v3/contracts/fixtureRepository";
import { v3Repository } from "@/v3/data/fixtureLoader";
import type { LegacyV2Archive } from "@/legacy-v2/contracts";
import type {
  V3AgentSpecInspection,
  V3AgentSpecSyncResult,
  V3ProjectSettings,
  V3ProjectSettingsInspection,
  V3ProjectSettingsUpdateRequest,
} from "@/v3/contracts";
import type {
  LocalAgentUsageOverview,
  V3BootstrapResult,
  V3ProjectLayoutStatus,
  V3RepairCandidate,
  V3RepairResult,
} from "@/types";
import type { LegacyV2Loader } from "@/services/legacyV2";

export type V3LifecycleAction = "initialize" | "migrate" | "recover" | "repair";

export interface V3LifecycleApi {
  inspect: (projectPath: string) => Promise<V3ProjectLayoutStatus>;
  initialize: (projectPath: string) => Promise<V3BootstrapResult>;
  migrate: (projectPath: string) => Promise<V3BootstrapResult>;
  recover: (projectPath: string) => Promise<V3BootstrapResult>;
  inspectRepairCandidates: (projectPath: string) => Promise<V3RepairCandidate[]>;
  repair: (projectPath: string, taskId: string) => Promise<V3RepairResult>;
}

export interface V3ProjectSettingsApi {
  get: (projectPath: string) => Promise<V3ProjectSettingsInspection>;
  update: (projectPath: string, request: V3ProjectSettingsUpdateRequest) => Promise<V3ProjectSettings>;
  inspectSpecs: (projectPath: string) => Promise<V3AgentSpecInspection>;
  syncSpecs: (projectPath: string, forceManagedRegion: boolean) => Promise<V3AgentSpecSyncResult>;
}

export type V3UsageLoader = (projectPath: string, taskId: string | null) => Promise<LocalAgentUsageOverview>;

export type V3ProductionLoader = (
  projectPath: string,
  taskId: string | null,
  expectedProjectId: string | null,
) => Promise<V3FixtureBundle>;

let productionLoader: V3ProductionLoader | null = null;
let legacyLoader: LegacyV2Loader | null = null;
let usageLoader: V3UsageLoader | null = null;
let lifecycleApi: V3LifecycleApi | null = null;
let projectSettingsApi: V3ProjectSettingsApi | null = null;
let loadRequestId = 0;
let legacyRequestId = 0;
let usageRequestId = 0;
let taskUsageRequestId = 0;
let layoutRequestId = 0;
let lifecycleRequestId = 0;
let settingsRequestId = 0;
let specsRequestId = 0;

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string") return error;
  try {
    const serialized = JSON.stringify(error);
    if (serialized) return serialized;
  } catch {
    return String(error);
  }
  return String(error);
}

export type V3View =
  | "project-overview"
  | "architecture-map"
  | "structure-explorer"
  | "global-timeline"
  | "task-timeline"
  | "plan-graph"
  | "acceptance-progress"
  | "node-brief";

export interface BreadcrumbCrumb {
  view: V3View;
  label: string;
}

interface V3State {
  currentScenario: V3FixtureScenario | null;
  projectPath: string | null;
  currentView: V3View;
  navStack: BreadcrumbCrumb[];
  bundle: V3FixtureBundle | null;
  loading: boolean;
  error: string | null;
  selectedTaskId: string | null;
  selectedNodeId: string | null;
  legacyArchive: LegacyV2Archive | null;
  legacyLoading: boolean;
  legacyError: string | null;
  usage: LocalAgentUsageOverview | null;
  usageLoading: boolean;
  usageError: string | null;
  taskUsage: LocalAgentUsageOverview | null;
  taskUsageLoading: boolean;
  taskUsageError: string | null;
  layoutStatus: V3ProjectLayoutStatus | null;
  layoutLoading: boolean;
  layoutError: string | null;
  lifecycleAction: V3LifecycleAction | null;
  lifecycleResult: V3BootstrapResult | V3RepairResult | null;
  lifecycleError: string | null;
  repairCandidates: V3RepairCandidate[];
  projectSettings: V3ProjectSettingsInspection | null;
  settingsLoading: boolean;
  settingsError: string | null;
  agentSpecs: V3AgentSpecInspection | null;
  specsLoading: boolean;
  specsError: string | null;

  selectScenario: (scenario: V3FixtureScenario) => void;
  selectProject: (projectPath: string, loader: V3ProductionLoader, archiveLoader: LegacyV2Loader | undefined, localUsageLoader: V3UsageLoader | undefined, projectLifecycleApi: V3LifecycleApi, settingsApi?: V3ProjectSettingsApi) => void;
  leaveProject: () => void;
  inspectProjectLayout: () => Promise<V3ProjectLayoutStatus | null>;
  runLifecycleAction: (action: V3LifecycleAction, taskId?: string) => Promise<void>;
  loadCurrentBundle: (taskIdOverride?: string | null, options?: { background?: boolean }) => Promise<V3FixtureBundle | null>;
  loadLegacyArchive: () => Promise<void>;
  loadUsage: () => Promise<void>;
  loadTaskUsage: (taskId: string | null) => Promise<void>;
  loadProjectSettings: () => Promise<void>;
  loadAgentSpecs: () => Promise<void>;
  updateProjectSettings: (request: V3ProjectSettingsUpdateRequest) => Promise<boolean>;
  syncAgentSpecs: (forceManagedRegion?: boolean) => Promise<boolean>;
  drillIn: (view: V3View, label: string) => void;
  goBack: () => void;
  goHome: () => void;
  selectTask: (taskId: string | null) => void;
  selectNode: (nodeId: string | null) => void;
}

export const useV3Store = create<V3State>((set, get) => ({
  currentScenario: null,
  projectPath: null,
  currentView: "project-overview",
  navStack: [],
  bundle: null,
  loading: false,
  error: null,
  selectedTaskId: null,
  selectedNodeId: null,
  legacyArchive: null,
  legacyLoading: false,
  legacyError: null,
  usage: null,
  usageLoading: false,
  usageError: null,
  taskUsage: null,
  taskUsageLoading: false,
  taskUsageError: null,
  layoutStatus: null,
  layoutLoading: false,
  layoutError: null,
  lifecycleAction: null,
  lifecycleResult: null,
  lifecycleError: null,
  repairCandidates: [],
  projectSettings: null,
  settingsLoading: false,
  settingsError: null,
  agentSpecs: null,
  specsLoading: false,
  specsError: null,

  selectScenario: (scenario) => {
    if (get().currentScenario === scenario && get().loading) return;
    loadRequestId += 1;
    legacyRequestId += 1;
    usageRequestId += 1;
    taskUsageRequestId += 1;
    layoutRequestId += 1;
    lifecycleRequestId += 1;
    settingsRequestId += 1;
    specsRequestId += 1;
    productionLoader = null;
    legacyLoader = null;
    usageLoader = null;
    lifecycleApi = null;
    projectSettingsApi = null;
    set({
      currentScenario: scenario,
      projectPath: null,
      bundle: null,
      loading: false,
      error: null,
      selectedTaskId: null,
      selectedNodeId: null,
      legacyArchive: null, legacyLoading: false, legacyError: null,
      usage: null, usageLoading: false, usageError: null,
      taskUsage: null, taskUsageLoading: false, taskUsageError: null,
      layoutStatus: null, layoutLoading: false, layoutError: null,
      lifecycleAction: null, lifecycleResult: null, lifecycleError: null, repairCandidates: [],
      projectSettings: null,
      settingsLoading: false, settingsError: null,
      agentSpecs: null,
      specsLoading: false, specsError: null,
      currentView: "project-overview",
      navStack: [],
    });
    void get().loadCurrentBundle();
  },

  selectProject: (projectPath, loader, archiveLoader, localUsageLoader, projectLifecycleApi, settingsApi) => {
    loadRequestId += 1;
    legacyRequestId += 1;
    usageRequestId += 1;
    taskUsageRequestId += 1;
    layoutRequestId += 1;
    lifecycleRequestId += 1;
    settingsRequestId += 1;
    specsRequestId += 1;
    productionLoader = loader;
    legacyLoader = archiveLoader ?? null;
    usageLoader = localUsageLoader ?? null;
    lifecycleApi = projectLifecycleApi;
    projectSettingsApi = settingsApi ?? null;
    set({
      projectPath,
      currentScenario: null,
      bundle: null, loading: false, error: null,
      selectedTaskId: null,
      selectedNodeId: null,
      legacyArchive: null, legacyLoading: false, legacyError: null,
      usage: null, usageLoading: false, usageError: null,
      taskUsage: null, taskUsageLoading: false, taskUsageError: null,
      layoutStatus: null, layoutLoading: false, layoutError: null,
      lifecycleAction: null, lifecycleResult: null, lifecycleError: null, repairCandidates: [],
      projectSettings: null, settingsLoading: false, settingsError: null,
      agentSpecs: null, specsLoading: false, specsError: null,
      currentView: "project-overview",
      navStack: [],
    });
    void get().inspectProjectLayout();
  },

  leaveProject: () => {
    loadRequestId += 1;
    legacyRequestId += 1;
    usageRequestId += 1;
    taskUsageRequestId += 1;
    layoutRequestId += 1;
    lifecycleRequestId += 1;
    settingsRequestId += 1;
    specsRequestId += 1;
    productionLoader = null;
    legacyLoader = null;
    usageLoader = null;
    lifecycleApi = null;
    projectSettingsApi = null;
    set({
      projectPath: null,
      bundle: null,
      loading: false,
      error: null,
      selectedTaskId: null,
      selectedNodeId: null,
      legacyArchive: null,
      legacyLoading: false,
      legacyError: null,
      usage: null,
      usageLoading: false,
      usageError: null,
      taskUsage: null, taskUsageLoading: false, taskUsageError: null,
      layoutStatus: null,
      layoutLoading: false,
      layoutError: null,
      lifecycleAction: null,
      lifecycleResult: null,
      lifecycleError: null,
      repairCandidates: [],
      projectSettings: null,
      settingsLoading: false,
      settingsError: null,
      agentSpecs: null,
      specsLoading: false,
      specsError: null,
      currentView: "project-overview",
      navStack: [],
    });
  },

  inspectProjectLayout: async () => {
    const { projectPath, currentScenario } = get();
    const api = lifecycleApi;
    if (!projectPath || currentScenario || !api) return null;
    const requestId = ++layoutRequestId;
    set({ layoutLoading: true, layoutError: null });
    try {
      const status = await api.inspect(projectPath);
      if (requestId !== layoutRequestId || get().projectPath !== projectPath || get().currentScenario) return null;
      const repairCandidates = status.state === "conflict"
        ? await api.inspectRepairCandidates(projectPath)
        : [];
      if (requestId !== layoutRequestId || get().projectPath !== projectPath || get().currentScenario) return null;
      set({ layoutStatus: status, layoutLoading: false, repairCandidates });
      if (status.state === "v3") {
        void get().loadCurrentBundle();
        if (legacyLoader) void get().loadLegacyArchive();
        if (usageLoader) void get().loadUsage();
        if (projectSettingsApi) void get().loadProjectSettings();
      }
      return status;
    } catch (err) {
      if (requestId !== layoutRequestId || get().projectPath !== projectPath || get().currentScenario) return null;
      set({ layoutLoading: false, layoutError: errorMessage(err) });
      return null;
    }
  },

  runLifecycleAction: async (action, taskId) => {
    const { projectPath, currentScenario, layoutStatus, lifecycleAction, repairCandidates } = get();
    const api = lifecycleApi;
    if (!projectPath || currentScenario || !api || lifecycleAction) return;
    const expectedAction: Partial<Record<V3ProjectLayoutStatus["state"], V3LifecycleAction>> = {
      absent: "initialize",
      v2: "migrate",
      migration_interrupted: "recover",
      conflict: repairCandidates.length > 0 ? "repair" : undefined,
    };
    if (!layoutStatus || expectedAction[layoutStatus.state] !== action) {
      set({ lifecycleError: "V3_LIFECYCLE_STATE_MISMATCH: inspect the project again before continuing" });
      return;
    }
    if (action === "repair" && (!taskId || !repairCandidates.some((candidate) => candidate.task_id === taskId))) {
      set({ lifecycleError: "V3_REPAIR_TASK_REQUIRED: select a verified V3 task before repairing" });
      return;
    }
    const requestId = ++lifecycleRequestId;
    set({ lifecycleAction: action, lifecycleError: null, lifecycleResult: null });
    try {
      const result = action === "repair"
        ? await api.repair(projectPath, taskId!)
        : await api[action](projectPath);
      if (requestId !== lifecycleRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ lifecycleAction: null, lifecycleResult: result });
      await get().inspectProjectLayout();
    } catch (err) {
      if (requestId !== lifecycleRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ lifecycleAction: null, lifecycleError: errorMessage(err) });
    }
  },

  loadCurrentBundle: async (taskIdOverride, options) => {
    const { currentScenario: scenario, projectPath } = get();
    if (!scenario && !projectPath) return null;
    const loader = productionLoader;
    if (!scenario && !loader) return null;
    const requestId = ++loadRequestId;
    const background = options?.background === true;
    if (!background) set({ loading: true, error: null });
    try {
      const previousProjectId = get().bundle?.projectOverview.project_id ?? null;
      const requestedTaskId = taskIdOverride === undefined ? get().selectedTaskId : taskIdOverride;
      const bundle = scenario
        ? await v3Repository.loadScenario(scenario)
        : await loader!(projectPath!, requestedTaskId, previousProjectId);
      if (requestId !== loadRequestId || get().projectPath !== projectPath || get().currentScenario !== scenario) return null;
      if (previousProjectId && bundle.projectOverview.project_id !== previousProjectId) {
        throw new Error("V3_IDENTITY_MISMATCH: refresh returned a different project");
      }
      const previousTaskId = get().selectedTaskId;
      const previousNodeId = get().selectedNodeId;
      const selectedTaskId = previousTaskId && bundle.projectOverview.active_tasks.some((task) => task.task_id === previousTaskId)
        ? previousTaskId
        : bundle.projectOverview.active_tasks.some((task) => task.task_id === bundle.taskTimeline.task_id)
          ? bundle.taskTimeline.task_id
          : bundle.projectOverview.active_tasks[0]?.task_id ?? null;
      const selectedNodeId = previousNodeId && bundle.planGraph.nodes.some((node) => node.node_id === previousNodeId)
        ? previousNodeId
        : bundle.planGraph.nodes[0]?.node_id ?? null;
      set({
        bundle,
        loading: false,
        error: null,
        selectedTaskId,
        selectedNodeId,
      });
      return bundle;
    } catch (err) {
      if (requestId !== loadRequestId || get().projectPath !== projectPath || get().currentScenario !== scenario) return null;
      const message = errorMessage(err);
      if (background && get().error === message && get().loading === false) return null;
      set({ loading: false, error: message });
      return null;
    }
  },

  loadLegacyArchive: async () => {
    const { projectPath, currentScenario } = get();
    const loader = legacyLoader;
    if (!projectPath || currentScenario || !loader) return;
    const requestId = ++legacyRequestId;
    set({ legacyLoading: true, legacyError: null });
    try {
      const archive = await loader(projectPath);
      if (requestId !== legacyRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ legacyArchive: archive, legacyLoading: false });
    } catch (err) {
      if (requestId !== legacyRequestId || get().projectPath !== projectPath) return;
      set({ legacyLoading: false, legacyError: errorMessage(err) });
    }
  },

  loadProjectSettings: async () => {
    const { projectPath, currentScenario } = get();
    const api = projectSettingsApi;
    if (!projectPath || currentScenario || !api) return;
    const requestId = ++settingsRequestId;
    set({ settingsLoading: true, settingsError: null });
    try {
      const inspection = await api.get(projectPath);
      if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ projectSettings: inspection, settingsLoading: false });
      if (inspection.status === "present") void get().loadAgentSpecs();
      else set({ agentSpecs: null, specsLoading: false, specsError: null });
    } catch (err) {
      if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ settingsLoading: false, settingsError: errorMessage(err) });
    }
  },

  loadAgentSpecs: async () => {
    const { projectPath, currentScenario, projectSettings } = get();
    const api = projectSettingsApi;
    if (!projectPath || currentScenario || !api || projectSettings?.status !== "present") return;
    const requestId = ++specsRequestId;
    set({ specsLoading: true, specsError: null });
    try {
      const inspection = await api.inspectSpecs(projectPath);
      if (requestId !== specsRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ agentSpecs: inspection, specsLoading: false });
    } catch (err) {
      if (requestId !== specsRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ specsLoading: false, specsError: errorMessage(err) });
    }
  },

  updateProjectSettings: async (request) => {
    const { projectPath, currentScenario } = get();
    const api = projectSettingsApi;
    if (!projectPath || currentScenario || !api) return false;
    const requestId = ++settingsRequestId;
    set({ settingsLoading: true, settingsError: null });
    try {
      const settings = await api.update(projectPath, request);
      if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
      set({ projectSettings: { status: "present", settings, recommended_action: null }, settingsLoading: false });
      return await get().syncAgentSpecs(false);
    } catch (err) {
      if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
      const message = errorMessage(err);
      if (message.includes("V3_PROJECT_SETTINGS_REVISION_CONFLICT")) {
        try {
          const latest = await api.get(projectPath);
          const latestSpecs = latest.status === "present" ? await api.inspectSpecs(projectPath) : null;
          if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
          set({
            projectSettings: latest,
            agentSpecs: latestSpecs,
            settingsLoading: false,
            specsLoading: false,
            settingsError: `${message} Latest project settings were reloaded; review and submit again.`,
          });
          return false;
        } catch (refreshError) {
          if (requestId !== settingsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
          set({
            settingsLoading: false,
            settingsError: `${message} Refresh failed: ${errorMessage(refreshError)}`,
          });
          return false;
        }
      }
      set({ settingsLoading: false, settingsError: message });
      return false;
    }
  },

  syncAgentSpecs: async (forceManagedRegion = false) => {
    const { projectPath, currentScenario } = get();
    const api = projectSettingsApi;
    if (!projectPath || currentScenario || !api) return false;
    const requestId = ++specsRequestId;
    set({ specsLoading: true, specsError: null });
    try {
      const result = await api.syncSpecs(projectPath, forceManagedRegion);
      if (requestId !== specsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
      const synchronized = result.status === "synchronized"
        && result.blocking_paths.length === 0
        && result.inspection.artifacts.every((artifact) => artifact.status === "in_sync");
      set({
        agentSpecs: result.inspection,
        specsLoading: false,
        specsError: synchronized
          ? null
          : `V3_AGENT_SPECS_SYNC_INCOMPLETE: ${result.blocking_paths.join(", ") || "final inspection is not in_sync"}`,
      });
      return synchronized;
    } catch (err) {
      if (requestId !== specsRequestId || get().projectPath !== projectPath || get().currentScenario) return false;
      set({ specsLoading: false, specsError: errorMessage(err) });
      return false;
    }
  },

  loadUsage: async () => {
    const { projectPath, currentScenario } = get();
    const loader = usageLoader;
    if (!projectPath || currentScenario || !loader) return;
    const requestId = ++usageRequestId;
    set({ usageLoading: true, usageError: null });
    try {
      const usage = await loader(projectPath, null);
      if (requestId !== usageRequestId || get().projectPath !== projectPath || get().currentScenario) return;
      set({ usage, usageLoading: false });
    } catch (err) {
      if (requestId !== usageRequestId || get().projectPath !== projectPath) return;
      set({ usageLoading: false, usageError: errorMessage(err) });
    }
  },

  loadTaskUsage: async (taskId) => {
    const { projectPath, currentScenario } = get();
    const loader = usageLoader;
    if (!projectPath || currentScenario || !loader || !taskId) {
      if (!taskId) set({ taskUsage: null, taskUsageLoading: false, taskUsageError: null });
      return;
    }
    const requestId = ++taskUsageRequestId;
    set((state) => ({
      taskUsage: state.taskUsage?.task_id === taskId ? state.taskUsage : null,
      taskUsageLoading: true,
      taskUsageError: null,
    }));
    try {
      const taskUsage = await loader(projectPath, taskId);
      if (requestId !== taskUsageRequestId || get().projectPath !== projectPath || get().currentScenario || get().selectedTaskId !== taskId) return;
      set({ taskUsage, taskUsageLoading: false });
    } catch (err) {
      if (requestId !== taskUsageRequestId || get().projectPath !== projectPath || get().selectedTaskId !== taskId) return;
      set({ taskUsageLoading: false, taskUsageError: errorMessage(err) });
    }
  },

  drillIn: (view, _label) => {
    const { currentView, navStack } = get();
    const currentLabel = viewLabel(currentView);
    set({
      currentView: view,
      navStack: [...navStack, { view: currentView, label: currentLabel }],
    });
  },

  goBack: () => {
    const { navStack } = get();
    if (navStack.length === 0) return;
    const last = navStack[navStack.length - 1];
    set({
      currentView: last.view,
      navStack: navStack.slice(0, -1),
    });
  },

  goHome: () => {
    set({ currentView: "project-overview", navStack: [] });
  },

  selectTask: (taskId) => set({ selectedTaskId: taskId }),
  selectNode: (nodeId) => set({ selectedNodeId: nodeId }),
}));

function viewLabel(view: V3View): string {
  const labels: Record<V3View, string> = {
    "project-overview": "项目总览",
    "architecture-map": "架构地图",
    "structure-explorer": "结构浏览",
    "global-timeline": "全局时间线",
    "task-timeline": "任务时间线",
    "plan-graph": "计划图",
    "acceptance-progress": "验收进度",
    "node-brief": "节点简报",
  };
  return labels[view] ?? view;
}
