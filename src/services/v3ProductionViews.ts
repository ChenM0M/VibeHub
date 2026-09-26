import { invoke } from "@tauri-apps/api/core";
import type { V3FixtureBundle } from "@/v3/contracts/fixtureRepository";
import type { V3ProductionLoader, V3ViewPanel } from "@/v3/stores/v3Store";
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

// Bounded process-local cache. Foreground refresh always loads; background probes
// validate all legacy bundle dependencies and preserve object identity if unchanged.
const panelNames: Record<V3ViewPanel, keyof V3FixtureBundle> = {
  project_overview: "projectOverview", project_structure: "projectStructure", agent_results: "agentResults",
  task_timeline: "taskTimeline", plan_graph: "planGraph", node_brief: "nodeBrief",
};
const allPanels = Object.keys(panelNames) as V3ViewPanel[];
const productionCache = new Map<string, { revisions: Partial<Record<V3ViewPanel, string>>; bundle: V3FixtureBundle }>();

export const loadV3ProductionViews: V3ProductionLoader = async (projectPath, taskId, expectedProjectId, options) => {
  const key = JSON.stringify([projectPath, taskId, expectedProjectId]);
  const probe = () => taskId ? invoke<string>("v3_read_view_revision", { projectPath, taskId, expectedProjectId }) : Promise.resolve(null);
  const revision = await probe();
  const cached = productionCache.get(key);
  const wanted = options?.background && options.panels?.length ? options.panels : allPanels;
  const stale = wanted.filter(panel => cached?.revisions[panel] !== revision);
  if (options?.background && revision && cached && stale.length === 0) return cached.bundle;
  const partial = Boolean(options?.background && cached && revision && options.panels?.length);
  const requested = partial ? stale : allPanels;
  const native = partial
    ? await invoke<Partial<NativeV3ViewBundle>>("v3_load_view_sections", { projectPath, taskId, expectedProjectId, expectedRevision: revision, sections: requested })
    : await invoke<NativeV3ViewBundle>("v3_load_view_bundle", { projectPath, taskId, expectedProjectId });
  for (const panel of requested) if (!native[panel]) throw new Error(`V3_PANEL_MISSING: ${panel}`);
  const updates = Object.fromEntries(requested.map(panel => [panelNames[panel], native[panel]]));
  const bundle = { ...(partial ? cached!.bundle : {}), ...updates } as unknown as V3FixtureBundle;
  // A hidden panel keeps its previous revision and is fetched when selected.
  if (revision && revision === await probe()) {
    const revisions = { ...(partial ? cached!.revisions : {}) };
    for (const panel of requested) revisions[panel] = revision;
    if (productionCache.size >= 4 && !productionCache.has(key)) productionCache.delete(productionCache.keys().next().value!);
    productionCache.set(key, { revisions, bundle });
  }
  return bundle;
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
