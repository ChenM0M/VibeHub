import type {
  AgentResultsView,
  NodeBrief,
  PlanGraphView,
  ProjectOverviewView,
  ProjectStructureView,
  TaskTimelineView,
  WorktreeOrchestrationView,
} from "./generated";

export const V3_FIXTURE_SCENARIOS = [
  "FX-EMPTY",
  "FX-HAPPY",
  "FX-NO-DOCS",
  "FX-PARALLEL",
  "FX-REWORK",
  "FX-STALE",
  "FX-PARTIAL",
  "FX-ERROR",
  "FX-WIN-PATHS",
  "FX-MAC-PATHS",
  "FX-LARGE",
  "FX-COVERAGE-GAP",
] as const;

export type V3FixtureScenario = (typeof V3_FIXTURE_SCENARIOS)[number];

export interface V3FixtureBundle {
  projectOverview: ProjectOverviewView;
  projectStructure: ProjectStructureView;
  agentResults: AgentResultsView;
  taskTimeline: TaskTimelineView;
  planGraph: PlanGraphView;
  nodeBrief: NodeBrief;
  // Production bundle exposes the V3 worktree projection when available; fixtures always include it.
  worktreeOrchestration?: WorktreeOrchestrationView;
}

export type JsonLoader = <T>(path: string) => Promise<T>;

export interface V3ViewRepository {
  listScenarios(): readonly V3FixtureScenario[];
  loadScenario(scenario: V3FixtureScenario): Promise<V3FixtureBundle>;
}

const fixtureFiles = {
  projectOverview: "project-overview.json",
  projectStructure: "project-structure.json",
  agentResults: "agent-results.json",
  taskTimeline: "task-timeline.json",
  planGraph: "plan-graph.json",
  nodeBrief: "node-brief.json",
  worktreeOrchestration: "worktree-orchestration.json",
} as const;

export function createV3FixtureRepository(
  loadJson: JsonLoader,
  basePath = "/fixtures/v3",
): V3ViewRepository {
  return {
    listScenarios: () => V3_FIXTURE_SCENARIOS,
    async loadScenario(scenario) {
      const load = <T>(filename: string) =>
        loadJson<T>(`${basePath}/${scenario}/${filename}`);
      const [projectOverview, projectStructure, agentResults, taskTimeline, planGraph, nodeBrief, worktreeOrchestration] =
        await Promise.all([
          load<ProjectOverviewView>(fixtureFiles.projectOverview),
          load<ProjectStructureView>(fixtureFiles.projectStructure),
          load<AgentResultsView>(fixtureFiles.agentResults),
          load<TaskTimelineView>(fixtureFiles.taskTimeline),
          load<PlanGraphView>(fixtureFiles.planGraph),
          load<NodeBrief>(fixtureFiles.nodeBrief),
          load<WorktreeOrchestrationView>(fixtureFiles.worktreeOrchestration),
        ]);
      return { projectOverview, projectStructure, agentResults, taskTimeline, planGraph, nodeBrief, worktreeOrchestration };
    },
  };
}
