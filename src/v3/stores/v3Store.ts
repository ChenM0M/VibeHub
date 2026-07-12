import { create } from "zustand";
import type {
  V3FixtureBundle,
  V3FixtureScenario,
} from "@/v3/contracts/fixtureRepository";
import { v3Repository } from "@/v3/data/fixtureLoader";

export type V3ProductionLoader = (
  projectPath: string,
  taskId: string | null,
  expectedProjectId: string | null,
) => Promise<V3FixtureBundle>;

let productionLoader: V3ProductionLoader | null = null;
let loadRequestId = 0;

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

  selectScenario: (scenario: V3FixtureScenario) => void;
  selectProject: (projectPath: string, loader: V3ProductionLoader) => void;
  leaveProject: () => void;
  loadCurrentBundle: () => Promise<void>;
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

  selectScenario: (scenario) => {
    set({ currentScenario: scenario, projectPath: null });
    void get().loadCurrentBundle();
  },

  selectProject: (projectPath, loader) => {
    productionLoader = loader;
    set({ projectPath, currentScenario: null, bundle: null, error: null, selectedTaskId: null, selectedNodeId: null });
    void get().loadCurrentBundle();
  },

  leaveProject: () => {
    loadRequestId += 1;
    productionLoader = null;
    set({
      projectPath: null,
      bundle: null,
      loading: false,
      error: null,
      selectedTaskId: null,
      selectedNodeId: null,
      currentView: "project-overview",
      navStack: [],
    });
  },

  loadCurrentBundle: async () => {
    const { currentScenario: scenario, projectPath } = get();
    if (!scenario && !projectPath) return;
    const requestId = ++loadRequestId;
    set({ loading: true, error: null });
    try {
      const previousProjectId = get().bundle?.projectOverview.project_id ?? null;
      const bundle = scenario
        ? await v3Repository.loadScenario(scenario)
        : await productionLoader!(projectPath!, get().selectedTaskId, previousProjectId);
      if (requestId !== loadRequestId || get().projectPath !== projectPath || get().currentScenario !== scenario) return;
      if (previousProjectId && bundle.projectOverview.project_id !== previousProjectId) {
        throw new Error("V3_IDENTITY_MISMATCH: refresh returned a different project");
      }
      const firstTaskId = bundle.projectOverview.active_tasks[0]?.task_id ?? null;
      const firstNodeId = bundle.planGraph.nodes[0]?.node_id ?? null;
      set({
        bundle,
        loading: false,
        selectedTaskId: firstTaskId,
        selectedNodeId: firstNodeId,
        currentView: "project-overview",
        navStack: [],
      });
    } catch (err) {
      if (requestId !== loadRequestId) return;
      set({ loading: false, error: (err as Error).message });
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
