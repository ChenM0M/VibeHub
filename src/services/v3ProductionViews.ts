import { invoke } from "@tauri-apps/api/core";
import type { V3FixtureBundle } from "@/v3/contracts/fixtureRepository";
import type { V3ProductionLoader } from "@/v3/stores/v3Store";
import type { NodeBrief } from "@/v3/contracts/generated/node-brief";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";
import type { ProjectOverviewView } from "@/v3/contracts/generated/project-overview-view";

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

export const loadV3NodeBrief = async (
  projectPath: string,
  taskId: string,
  nodeId: string,
  expectedProjectId: string | null = null,
): Promise<NodeBrief> =>
  invoke<NodeBrief>("v3_load_node_brief", {
    projectPath,
    taskId,
    nodeId,
    expectedProjectId,
  });

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

export interface V3ArchivedTaskPage {
  schema_version: "1.0";
  project_id: string;
  tasks: NonNullable<ProjectOverviewView["archived_tasks"]>;
  page: {
    cursor: string | null;
    next_cursor: string | null;
    limit: number;
    returned: number;
    total_estimate: number;
    truncated: boolean;
    truncation_reason: "none" | "page_limit";
    model_version: string;
  };
}

export async function queryV3ArchivedTasks(
  projectPath: string,
  expectedProjectId: string,
  cursor: string | null = null,
  limit = 50,
): Promise<V3ArchivedTaskPage> {
  return invoke<V3ArchivedTaskPage>("v3_query_archived_tasks", {
    projectPath,
    cursor,
    limit,
    expectedProjectId,
  });
}
