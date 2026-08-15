import { useMemo, useState, useCallback } from "react";
import { Button } from "@/components/ui/button";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  type NodeProps,
  BackgroundVariant,
  Panel,
  MarkerType,
  Handle,
  Position,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { cn } from "@/lib/utils";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import type { PlanGraphView } from "@/v3/contracts/generated/plan-graph-view";
import { useTranslation } from "react-i18next";

const nodeStyle: Record<string, string> = {
  completed: "bg-card border-border text-muted-foreground",
  active: "bg-card border-blue-500 text-foreground ring-1 ring-blue-500 shadow-md shadow-blue-500/10",
  blocked: "bg-card border-orange-500/50 text-foreground",
  review: "bg-card border-purple-500/50 text-foreground",
  planned: "bg-muted/30 border-border text-muted-foreground",
  ready: "bg-card border-cyan-500/30 text-foreground",
  cancelled: "bg-muted/30 border-border text-muted-foreground line-through opacity-70",
  superseded: "bg-muted/10 border-border/40 text-muted-foreground/50",
};
const stateTransitions: Record<string, string[]> = {
  planned: ["ready", "active", "blocked", "cancelled"],
  ready: ["active", "blocked", "cancelled"],
  active: ["completed", "failed", "blocked", "cancelled"],
  blocked: ["ready", "active", "cancelled"],
  failed: ["active", "cancelled"],
};

// Theme-aware graph colors.
const isDark = () => typeof document !== "undefined" && document.documentElement.classList.contains("dark");
const edgeColor = () => (isDark() ? "#64748b" : "#94a3b8");
const edgeDoneColor = "#10b981";
const edgeActiveColor = "#3b82f6";
const minimapNodeColor: Record<string, string> = {
  completed: "#d1fae5",
  active: "#3b82f6",
  blocked: "#fb923c",
  review: "#c084fc",
  ready: "#67e8f9",
  planned: "#cbd5e1",
  cancelled: "#94a3b8",
  superseded: "#cbd5e1",
};
const minimapNodeStrokeColor: Record<string, string> = {
  completed: "#10b981",
  active: "#2563eb",
  blocked: "#f97316",
  review: "#a855f7",
  ready: "#0891b2",
  planned: "#64748b",
  cancelled: "#64748b",
  superseded: "#94a3b8",
};

type PlanNodeData = { title: string; state: string; readiness: string; block_reasons: string[]; sequence: number; session_ids: string[]; agent_result_ids: string[]; parallel_layer?: number; parallel_candidate?: boolean; execution_state?: string; showSessions?: boolean; noSessionsLabel: string; listSeparator: string };

function PlanNode({ data, selected }: NodeProps) {
  const d = data as unknown as PlanNodeData;
  const isDone = d.state === "completed";
  const isActive = d.state === "active";

  return (
    <div className={cn("relative flex items-center gap-3 border px-3 py-2 min-w-[180px] max-w-[240px] rounded-lg cursor-pointer transition-all", nodeStyle[d.state] ?? nodeStyle.planned, selected && "ring-2 ring-foreground/50 ring-offset-2 ring-offset-background")}>
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-muted-foreground !border-background" />
      <div className={cn("flex shrink-0 h-5 w-5 items-center justify-center rounded-full text-[10px] font-bold", isDone ? "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400" : isActive ? "bg-blue-500 text-white" : "bg-muted-foreground/20 text-muted-foreground")}>
        {d.sequence}
      </div>
      <div className="flex-1 min-w-0">
        <div className="truncate text-sm font-medium">{d.title}</div>
        {d.block_reasons.length > 0 && <div className="mt-0.5 text-[10px] text-orange-600 truncate">{d.block_reasons.join(d.listSeparator)}</div>}
        {d.showSessions && d.session_ids.length > 0 && (
          <div className="mt-1.5 max-w-full truncate rounded border border-emerald-500/30 bg-emerald-500/5 px-1.5 py-0.5 text-[9px] font-medium text-emerald-700 dark:text-emerald-300" title={d.session_ids.join("\n")}>
            Session × {d.session_ids.length}
          </div>
        )}
        {d.showSessions && d.session_ids.length === 0 && (
          <div className="mt-1.5 w-fit rounded border border-border/50 bg-muted/50 px-1.5 py-0.5 text-[9px] text-muted-foreground">
            {d.noSessionsLabel}
          </div>
        )}
      </div>
      <Handle type="source" position={Position.Right} className="!w-2 !h-2 !bg-muted-foreground !border-background" />
    </div>
  );
}
const nodeTypes = { planNode: PlanNode };

function computeLayout(nodes: PlanGraphView["nodes"], edges: { from: string; to: string }[]) {
  const adjacency = new Map<string, string[]>();
  const inDegree = new Map<string, number>();
  for (const node of nodes) { adjacency.set(node.node_id, []); inDegree.set(node.node_id, 0); }
  for (const edge of edges) {
    if (adjacency.has(edge.from) && adjacency.has(edge.to)) { adjacency.get(edge.from)!.push(edge.to); inDegree.set(edge.to, (inDegree.get(edge.to) ?? 0) + 1); }
  }
  const levels = new Map<string, number>();
  const queue: string[] = [];
  for (const [id, deg] of inDegree) { if (deg === 0) { levels.set(id, 0); queue.push(id); } }
  const sequence = new Map<string, number>();
  let seq = 1;
  while (queue.length > 0) {
    const id = queue.shift()!;
    sequence.set(id, seq++);
    const level = levels.get(id) ?? 0;
    for (const next of adjacency.get(id) ?? []) {
      const nl = Math.max(levels.get(next) ?? 0, level + 1);
      levels.set(next, nl);
      inDegree.set(next, (inDegree.get(next) ?? 1) - 1);
      if (inDegree.get(next) === 0) queue.push(next);
    }
  }
  for (const node of nodes) { if (!levels.has(node.node_id)) { levels.set(node.node_id, 0); sequence.set(node.node_id, seq++); } }
  const byLevel = new Map<number, string[]>();
  for (const [id, level] of levels) { if (!byLevel.has(level)) byLevel.set(level, []); byLevel.get(level)!.push(id); }
  const positions = new Map<string, { x: number; y: number }>();
  const xSpacing = 300, ySpacing = 100;
  for (const [level, ids] of byLevel) { ids.forEach((id, i) => { positions.set(id, { x: level * xSpacing, y: i * ySpacing - ((ids.length - 1) * ySpacing) / 2 }); }); }
  return { positions, sequence };
}

interface PlanGraphProps {
  data: PlanGraphView;
  taskTitle?: string;
  taskIntent?: string;
  onNodeClick?: (node: PlanGraphView["nodes"][number]) => void;
  onAddNode?: (input: { nodeId: string; title: string; goal: string; scope: string[]; dependencies: string[]; idempotencyKey: string }) => Promise<boolean>;
  onSetDependencies?: (input: { nodeId: string; dependencies: string[]; idempotencyKey: string }) => Promise<boolean>;
  onSetState?: (input: { nodeId: string; state: string; idempotencyKey: string }) => Promise<boolean>;
  mutationError?: string | null;
}

function submissionKey(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto ? crypto.randomUUID() : `plan-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function PlanGraph({ data, taskTitle = "", taskIntent = "", onNodeClick, onAddNode, onSetDependencies, onSetState, mutationError }: PlanGraphProps) {
  const { t } = useTranslation();
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showSessions, setShowSessions] = useState(false);
  const [nodeMeasurements, setNodeMeasurements] = useState<Record<string, { width: number; height: number }>>({});
  const [editor, setEditor] = useState<"add" | "dependencies" | "state" | null>(null);
  const [title, setTitle] = useState("");
  const [goal, setGoal] = useState("");
  const [scope, setScope] = useState("");
  const [dependencies, setDependencies] = useState<string[]>([]);
  const [state, setState] = useState("planned");
  const [pending, setPending] = useState(false);
  const [idempotencyKey, setIdempotencyKey] = useState(submissionKey);
  const [draftNodeId, setDraftNodeId] = useState(submissionKey);
  const [isMoving, setIsMoving] = useState(false);

  const handleNodesChange = useCallback((changes: NodeChange[]) => {
    const dimensionChanges = changes.filter((change) => change.type === "dimensions" && change.dimensions) as Extract<NodeChange, { type: "dimensions" }>[];
    if (dimensionChanges.length === 0) return;

    setNodeMeasurements((previous) => {
      let next = previous;
      for (const change of dimensionChanges) {
        const width = change.dimensions?.width;
        const height = change.dimensions?.height;
        if (typeof width !== "number" || typeof height !== "number") continue;
        const current = previous[change.id];
        if (current?.width === width && current.height === height) continue;
        if (next === previous) next = { ...previous };
        next[change.id] = { width, height };
      }
      return next;
    });
  }, []);

  const { nodes: flowNodes, edges: flowEdges, completedCount, activeCount, blockedCount } = useMemo(() => {
    const activeEdges = data.scheduling_edges.map((e) => ({ from: e.from_node_id, to: e.to_node_id }));
    const { positions, sequence } = computeLayout(data.nodes, activeEdges);
    const baseEdgeColor = edgeColor();
    const nodes: Node[] = data.nodes.map((node) => {
      const pos = positions.get(node.node_id) ?? { x: 0, y: 0 };
      return {
        id: node.node_id, type: "planNode", position: pos,
        measured: nodeMeasurements[node.node_id],
        data: { title: node.title, state: node.state, readiness: node.readiness, block_reasons: node.block_reasons, sequence: sequence.get(node.node_id) ?? 0, session_ids: node.session_ids ?? [], agent_result_ids: node.agent_result_ids ?? [], parallel_layer: node.parallel_layer, parallel_candidate: node.parallel_candidate, execution_state: node.execution_state, showSessions, noSessionsLabel: t("v3.plan.noSessions"), listSeparator: t("v3.common.listSeparator") } as unknown as Record<string, unknown>,
        selected: selectedNodeId === node.node_id,
      };
    });
    const edges: Edge[] = data.scheduling_edges.map((e) => {
      const toNode = data.nodes.find((n) => n.node_id === e.to_node_id);
      const isTargetActive = toNode?.state === "active";
      const isTargetDone = toNode?.state === "completed";
      
      // Active edge (currently running path)
      if (isTargetActive) {
        return {
          id: e.edge_id, source: e.from_node_id, target: e.to_node_id,
          type: "smoothstep",
          style: { stroke: edgeActiveColor, strokeWidth: 2, strokeDasharray: "4 4" },
          animated: true,
          markerEnd: { type: MarkerType.ArrowClosed, width: 24, height: 24, color: edgeActiveColor },
        };
      }
      
      // Past edge (completed path)
      if (isTargetDone) {
        return {
          id: e.edge_id, source: e.from_node_id, target: e.to_node_id,
          type: "smoothstep",
          style: { stroke: edgeDoneColor, strokeWidth: 2 },
          animated: false,
          markerEnd: { type: MarkerType.ArrowClosed, width: 24, height: 24, color: edgeDoneColor },
        };
      }

      // Future edge (planned/ready path) - faint and dotted to reduce visual clutter
      return {
        id: e.edge_id, source: e.from_node_id, target: e.to_node_id,
        type: "smoothstep",
        style: { stroke: baseEdgeColor, strokeWidth: 1.5, strokeDasharray: "2 4", opacity: 0.4 },
        animated: false,
      };
    });
    const completed = data.nodes.filter((n) => n.state === "completed").length;
    const active = data.nodes.filter((n) => n.state === "active").length;
    const blocked = data.nodes.filter((n) => n.state === "blocked").length;
    return { nodes, edges, completedCount: completed, activeCount: active, blockedCount: blocked };
  }, [data, nodeMeasurements, selectedNodeId, showSessions, t]);

  const handleNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNodeId(node.id);
    const fullNode = data.nodes.find((n) => n.node_id === node.id);
    if (fullNode && onNodeClick) onNodeClick(fullNode);
  }, [data, onNodeClick]);

  const selectedNode = data.nodes.find((n) => n.node_id === selectedNodeId) ?? null;
  const totalNodes = data.nodes.length;
  const editable = Boolean(onAddNode || onSetDependencies || onSetState);
  const graphBlockers = useMemo(() => {
    const seen = new Set<string>();
    return data.nodes.flatMap((node) => node.blocker_details ?? []).filter((blocker) => {
      if (seen.has(blocker.blocker_id)) return false;
      seen.add(blocker.blocker_id);
      return true;
    });
  }, [data.nodes]);

  const openEditor = (mode: "add" | "dependencies" | "state") => {
    if (mode === "dependencies" && selectedNode && ["active", "completed"].includes(selectedNode.state)) return;
    setEditor(mode);
    setIdempotencyKey(submissionKey());
    if (mode === "add") {
      setDraftNodeId(submissionKey());
      setTitle(data.nodes.length === 0 ? taskTitle : "");
      setGoal(data.nodes.length === 0 ? taskIntent : "");
      setScope("");
      setDependencies([]);
    } else if (selectedNode) {
      setDependencies(data.scheduling_edges.filter((edge) => edge.to_node_id === selectedNode.node_id).map((edge) => edge.from_node_id));
      setState(stateTransitions[selectedNode.state]?.[0] ?? "");
    }
  };

  const submitEdit = async () => {
    setPending(true);
    try {
      let succeeded = false;
      if (editor === "add" && onAddNode && title.trim() && goal.trim()) succeeded = await onAddNode({ nodeId: draftNodeId, title: title.trim(), goal: goal.trim(), scope: scope.split("\n").map((item) => item.trim()).filter(Boolean), dependencies, idempotencyKey });
      if (editor === "dependencies" && onSetDependencies && selectedNode) succeeded = await onSetDependencies({ nodeId: selectedNode.node_id, dependencies, idempotencyKey });
      if (editor === "state" && onSetState && selectedNode) succeeded = await onSetState({ nodeId: selectedNode.node_id, state, idempotencyKey });
      if (succeeded) setEditor(null);
    } finally { setPending(false); }
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="shrink-0 flex items-center gap-3 px-1 py-2">
        <div className="min-w-0">
          <div className="text-sm font-semibold">{t("v3.plan.title")}</div>
          <div className="mt-0.5 text-[11px] text-muted-foreground">{t("v3.plan.description")}</div>
        </div>
        <div className="flex-1 h-2 bg-muted overflow-hidden flex">
          <div className="h-full bg-emerald-500" style={{ width: `${(completedCount / Math.max(totalNodes, 1)) * 100}%` }} />
          <div className="h-full bg-blue-500" style={{ width: `${(activeCount / Math.max(totalNodes, 1)) * 100}%` }} />
          <div className="h-full bg-orange-500" style={{ width: `${(blockedCount / Math.max(totalNodes, 1)) * 100}%` }} />
        </div>
        <span className="text-xs text-muted-foreground">{t("v3.plan.completedCount", { completed: completedCount, total: totalNodes })}</span>
        {activeCount > 0 && <span className="text-xs text-blue-600">{t("v3.plan.activeCount", { count: activeCount })}</span>}
        {blockedCount > 0 && <span className="text-xs text-orange-600">{t("v3.plan.blockedCount", { count: blockedCount })}</span>}
        {editable && data.planning_required && <div className="ml-auto flex gap-2"><Button size="sm" variant="outline" onClick={() => openEditor("add")}>{totalNodes === 0 ? t("v3.plan.createInitial") : t("v3.plan.addNode")}</Button>{selectedNode && <><Button size="sm" variant="outline" onClick={() => openEditor("dependencies")}>{t("v3.plan.editDependencies")}</Button><Button size="sm" variant="outline" onClick={() => openEditor("state")}>{t("v3.plan.updateState")}</Button></>}</div>}
      </div>

      {graphBlockers.length > 0 && (
        <div className="max-h-[min(32vh,18rem)] shrink-0 overflow-y-auto overscroll-contain pr-1 scrollbar-auto-hide">
          <BlockerDetailsPanel blockers={graphBlockers} compact />
        </div>
      )}

      {data.nodes.length === 0 ? <div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/30 p-8 text-center"><div><div className="font-medium">{t(data.planning_required ? "v3.plan.emptyTitle" : "v3.plan.lightweightTitle")}</div><p className="mt-1 text-sm text-muted-foreground">{t(data.planning_required ? "v3.plan.emptyDescription" : "v3.plan.lightweightDescription")}</p>{data.planning_required && onAddNode && <Button className="mt-4" onClick={() => openEditor("add")}>{t("v3.plan.createInitial")}</Button>}</div></div> : <div className="relative min-h-0 flex-1 rounded-md bg-card/50 overflow-hidden shadow-sm border border-border/30 [&_.react-flow\_\_controls]:!bg-background [&_.react-flow\_\_controls-button]:!bg-background [&_.react-flow\_\_controls-button]:!border-border [&_.react-flow\_\_controls-button]:!text-foreground [&_.react-flow\_\_controls-button:hover]:!bg-accent [&_.react-flow\_\_controls-button_svg]:!fill-foreground [&_.react-flow\_\_minimap]:!bg-background [&_.react-flow\_\_minimap]:!border [&_.react-flow\_\_minimap]:!border-border">
        <ReactFlow
          nodes={flowNodes}
          edges={flowEdges}
          nodeTypes={nodeTypes}
          onNodesChange={handleNodesChange}
          onNodeClick={handleNodeClick}
          onMoveStart={() => setIsMoving(true)}
          onMoveEnd={() => setIsMoving(false)}
          fitView
          fitViewOptions={{
            nodes: flowNodes.filter(n => (n.data as unknown as PlanNodeData).state === "active").length > 0 
              ? flowNodes.filter(n => (n.data as unknown as PlanNodeData).state === "active")
              : undefined,
            padding: 1, 
            minZoom: 0.8, 
            maxZoom: 1 
          }}
          minZoom={0.1}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={20} size={1} className="!bg-background" />
          <Controls showInteractive={false} />

          <Panel position="top-right" className="!m-3 overflow-y-auto overflow-x-hidden" style={{ maxHeight: "calc(100% - 12rem)" }}>
            <div className="border border-border/70 bg-background/95 p-3.5 backdrop-blur-sm shadow-sm space-y-3 min-w-[220px] max-w-[260px] rounded-lg">
              <div className="flex items-center justify-between gap-3 border-b border-border/50 pb-3">
                <label className="text-xs font-semibold text-foreground cursor-pointer select-none" onClick={() => setShowSessions(!showSessions)}>
                  {t("v3.plan.agentDistribution")}
                </label>
                <button type="button" onClick={() => setShowSessions(!showSessions)} className={cn("relative inline-flex h-4 w-8 shrink-0 cursor-pointer items-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2", showSessions ? "bg-blue-500" : "bg-muted-foreground/30")} role="switch" aria-checked={showSessions}>
                  <span className={cn("pointer-events-none block h-3 w-3 rounded-full bg-white shadow-sm ring-0 transition-transform", showSessions ? "translate-x-4.5" : "translate-x-0.5")} style={{ transform: showSessions ? "translateX(18px)" : "translateX(2px)" }} />
                </button>
              </div>
              <div>
                <div className="mb-2 text-[10px] font-semibold text-muted-foreground">{t("v3.plan.legend.title")}</div>
                <div className="grid grid-cols-2 gap-x-3 gap-y-1.5 text-[10px] text-muted-foreground">
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-border bg-emerald-500/15 shrink-0" /><span>{t("v3.plan.legend.completed")}</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-emerald-500 shrink-0" /><span>{t("v3.plan.legend.completedPath")}</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-blue-500 bg-blue-500 shrink-0" /><span>{t("v3.plan.legend.active")}</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-blue-500 shrink-0" /><span>{t("v3.plan.legend.currentPath")}</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-orange-500/50 bg-card shrink-0" /><span>{t("v3.plan.legend.blocked")}</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-muted-foreground/50 shrink-0" /><span>{t("v3.plan.legend.plannedPath")}</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-border bg-muted/30 shrink-0" /><span>{t("v3.plan.legend.planned")}</span></div>
                </div>
              </div>
              {selectedNode && (
                <div className="border-t border-border/50 pt-1.5">
                  <div className="text-xs font-medium">{selectedNode.title}</div>
                  <div className="mt-0.5 text-[10px] text-muted-foreground">{t(`v3.common.nodeState.${selectedNode.state}`, { defaultValue: selectedNode.state })}</div>
                  <div className="mt-1 text-[10px] text-muted-foreground">{selectedNode.goal}</div>
                  {selectedNode.parallel_candidate && <div className="mt-1 text-[10px] text-violet-600">{t("v3.plan.parallelCandidate", { layer: (selectedNode.parallel_layer ?? 0) + 1 })}</div>}
                  <div className="mt-1 text-[10px] text-muted-foreground">{t("v3.plan.executionStateLabel", { value: t(`v3.plan.executionState.${selectedNode.execution_state ?? "not_started"}`, { defaultValue: selectedNode.execution_state ?? "not_started" }) })}</div>
                  <div className="mt-1 text-[10px] text-muted-foreground">{t("v3.plan.executionEvidence", { sessions: selectedNode.session_ids?.length ?? 0, results: selectedNode.agent_result_ids?.length ?? 0 })}</div>
                  {selectedNode.scope.length > 0 && <div className="mt-1 text-[10px]"><span className="text-muted-foreground">{t("v3.plan.scope")}</span>{selectedNode.scope.join(t("v3.common.listSeparator"))}</div>}
                  {selectedNode.block_reasons.length > 0 && <div className="mt-1 text-[10px] text-orange-600">{t("v3.plan.blocked", { value: selectedNode.block_reasons.join(t("v3.common.listSeparator")) })}</div>}
                  <div className="mt-1 text-[10px] text-blue-600">{t("v3.plan.nodeHint")}</div>
                </div>
              )}
            </div>
          </Panel>

          {/* The overview stays available while the graph has nodes: it is a navigation control, not a transient tooltip. */}
          <MiniMap
            pannable 
            zoomable
            style={{ width: 180, height: 120 }}
            className={cn(
              "!bg-background/80 !border-border/50 shadow-md rounded-md backdrop-blur-sm transition-all duration-300 max-sm:!w-[100px] max-sm:!h-[70px]",
              isMoving ? "opacity-60" : "opacity-0 hover:opacity-100"
            )}
            nodeColor={(node) => minimapNodeColor[(node.data as unknown as PlanNodeData)?.state] ?? minimapNodeColor.planned}
            nodeStrokeColor={(node) => minimapNodeStrokeColor[(node.data as unknown as PlanNodeData)?.state] ?? minimapNodeStrokeColor.planned}
            nodeStrokeWidth={1.5}
            nodeBorderRadius={3}
            ariaLabel={t("v3.plan.minimapLabel")}
          />
        </ReactFlow>
      </div>}
      {editor && <div className="fixed inset-0 z-[110] flex items-center justify-center bg-black/30 p-4" onClick={() => !pending && setEditor(null)}><div role="dialog" aria-modal="true" className="w-full max-w-lg space-y-4 rounded-lg border border-border bg-background p-5 shadow-xl" onClick={(event) => event.stopPropagation()}><div><h3 className="font-semibold">{editor === "add" ? (totalNodes === 0 ? t("v3.plan.createInitial") : t("v3.plan.addNode")) : editor === "dependencies" ? t("v3.plan.editDependencies") : t("v3.plan.updateState")}</h3>{selectedNode && editor !== "add" && <p className="mt-1 text-sm text-muted-foreground">{selectedNode.title}</p>}</div>{editor === "add" && <><label className="block space-y-1"><span className="text-sm">{t("v3.plan.editor.nodeTitle")}</span><input value={title} onChange={(event) => setTitle(event.target.value)} className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm" /></label><label className="block space-y-1"><span className="text-sm">{t("v3.plan.editor.goal")}</span><textarea value={goal} onChange={(event) => setGoal(event.target.value)} className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" /></label><label className="block space-y-1"><span className="text-sm">{t("v3.plan.editor.scope")}</span><textarea value={scope} onChange={(event) => setScope(event.target.value)} className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" /></label></>}{(editor === "add" || editor === "dependencies") && data.nodes.length > 0 && <fieldset className="space-y-2"><legend className="text-sm">{t("v3.plan.editor.dependencies")}</legend>{data.nodes.filter((node) => editor === "add" || node.node_id !== selectedNode?.node_id).map((node) => <label key={node.node_id} className="flex items-center gap-2 text-sm"><input type="checkbox" checked={dependencies.includes(node.node_id)} onChange={(event) => setDependencies((current) => event.target.checked ? [...current, node.node_id] : current.filter((id) => id !== node.node_id))} />{node.title}</label>)}</fieldset>}{editor === "state" && selectedNode && <label className="block space-y-1"><span className="text-sm">{t("v3.plan.editor.state")}</span><select value={state} onChange={(event) => setState(event.target.value)} className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm">{(stateTransitions[selectedNode.state] ?? []).map((value) => <option key={value} value={value} disabled={(value === "active" || value === "ready") && selectedNode.readiness === "blocked"} title={selectedNode.block_reasons.join(", ")}>{t(`v3.common.nodeState.${value}`, { defaultValue: value })}</option>)}</select>{selectedNode.readiness === "blocked" && <span className="text-xs text-orange-600">{selectedNode.block_reasons.join(", ") || "Server readiness blocks activation."}</span>}{(stateTransitions[selectedNode.state] ?? []).length === 0 && <span className="text-xs text-muted-foreground">{t("v3.plan.editor.noTransitions")}</span>}</label>}{mutationError && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{mutationError}</div>}<div className="flex justify-end gap-2"><Button variant="outline" disabled={pending} onClick={() => setEditor(null)}>{t("v3.common.cancel")}</Button><Button disabled={pending || (editor === "add" && (!title.trim() || !goal.trim())) || (editor === "state" && (!state || ((state === "active" || state === "ready") && selectedNode?.readiness === "blocked")))} onClick={() => void submitEdit()}>{pending ? t("v3.plan.editor.submitting") : t("v3.plan.editor.submit")}</Button></div></div></div>}
    </div>
  );
}
