import type { PlanGraphView } from "@/v3/contracts/generated/plan-graph-view";
import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";

type PlanNode = PlanGraphView["nodes"][number];

/**
 * Pure derivation of everything the plan node drawer shows from the plan graph,
 * so the clicked node - completed or not - always renders its own facts instead
 * of whichever node the projection happened to brief.
 */
export interface PlanNodeDetailView {
  node: PlanNode | null;
  dependencies: string[];
  criteria: CriterionSummary[];
}

export function planNodeDetailView(
  planGraph: Pick<PlanGraphView, "nodes" | "scheduling_edges"> | null | undefined,
  nodeId: string | null,
  taskCriteria: CriterionSummary[] = [],
): PlanNodeDetailView {
  if (!planGraph || !nodeId) return { node: null, dependencies: [], criteria: [] };
  const node = planGraph.nodes.find((candidate) => candidate.node_id === nodeId) ?? null;
  const dependencies = (planGraph.scheduling_edges ?? [])
    .filter((edge) => edge.to_node_id === nodeId)
    .map((edge) => edge.from_node_id);
  const criteria = (node?.criterion_ids ?? [])
    .map((criterionId) => taskCriteria.find((criterion) => criterion.criterion_id === criterionId))
    .filter((criterion): criterion is CriterionSummary => Boolean(criterion));
  return { node, dependencies, criteria };
}
