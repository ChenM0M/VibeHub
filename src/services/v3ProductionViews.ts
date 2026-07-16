import { invoke } from "@tauri-apps/api/core";
import type { V3FixtureBundle } from "@/v3/contracts/fixtureRepository";
import type { V3ProductionLoader } from "@/v3/stores/v3Store";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";

interface NativeV3ViewBundle {
  project_overview: V3FixtureBundle["projectOverview"];
  project_structure: V3FixtureBundle["projectStructure"];
  agent_results: V3FixtureBundle["agentResults"];
  task_timeline: V3FixtureBundle["taskTimeline"];
  plan_graph: V3FixtureBundle["planGraph"];
  node_brief: V3FixtureBundle["nodeBrief"];
}

export const loadV3ProductionViews: V3ProductionLoader = async (
  projectPath,
  taskId,
  expectedProjectId,
) => {
  const native = await invoke<NativeV3ViewBundle>("v3_load_view_bundle", {
    projectPath,
    taskId,
    expectedProjectId,
  });
  return {
    projectOverview: native.project_overview,
    projectStructure: native.project_structure,
    agentResults: native.agent_results,
    taskTimeline: native.task_timeline,
    planGraph: native.plan_graph,
    nodeBrief: native.node_brief,
  };
};

export async function queryV3ProjectStructure(
  projectPath: string,
  taskId: string,
  relativeDir: string,
  cursor: string | null = null,
  limit = 200,
  query: string | null = null,
): Promise<ProjectStructureView> {
  return invoke<ProjectStructureView>("v3_query_project_structure", {
    projectPath,
    taskId,
    relativeDir,
    cursor,
    limit,
    query,
  });
}
