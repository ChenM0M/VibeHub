import { useMemo, useState, useCallback } from "react";
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
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import type { PlanGraphView } from "@/v3/contracts/generated/plan-graph-view";

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
const stateLabel: Record<string, string> = {
  completed: "已完成", active: "进行中", blocked: "阻塞", review: "审查中", planned: "待执行", ready: "就绪", cancelled: "已取消", superseded: "已替代",
};

// 深色模式颜色检测
const isDark = () => typeof document !== "undefined" && document.documentElement.classList.contains("dark");
const edgeColor = () => (isDark() ? "#64748b" : "#94a3b8");
const edgeDoneColor = "#10b981";
const edgeActiveColor = "#3b82f6";

type PlanNodeData = { title: string; state: string; readiness: string; block_reasons: string[]; sequence: number; showSessions?: boolean };

function PlanNode({ data, selected }: NodeProps) {
  const d = data as unknown as PlanNodeData;
  const isDone = d.state === "completed";
  const isActive = d.state === "active";
  
  const sessionIdx = d.sequence % 3;
  const agentNames = ["Codex (代码生成)", "Reviewer (审查)", "Planner (规划)"];
  const agentColors = ["text-purple-600 dark:text-purple-400 bg-purple-500/15 border-purple-500/30", "text-amber-600 dark:text-amber-400 bg-amber-500/15 border-amber-500/30", "text-pink-600 dark:text-pink-400 bg-pink-500/15 border-pink-500/30"];
  const hasAgent = isDone || isActive || d.state === "review";

  return (
    <div className={cn("relative flex items-center gap-3 border px-3 py-2 min-w-[180px] max-w-[240px] rounded-lg cursor-pointer transition-all", nodeStyle[d.state] ?? nodeStyle.planned, selected && "ring-2 ring-foreground/50 ring-offset-2 ring-offset-background")}>
      <Handle type="target" position={Position.Left} className="!w-2 !h-2 !bg-muted-foreground !border-background" />
      <div className={cn("flex shrink-0 h-5 w-5 items-center justify-center rounded-full text-[10px] font-bold", isDone ? "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400" : isActive ? "bg-blue-500 text-white" : "bg-muted-foreground/20 text-muted-foreground")}>
        {d.sequence}
      </div>
      <div className="flex-1 min-w-0">
        <div className="truncate text-sm font-medium">{d.title}</div>
        {d.block_reasons.length > 0 && <div className="mt-0.5 text-[10px] text-orange-600 truncate">{d.block_reasons.join("、")}</div>}
        {d.showSessions && (
          <div className={cn("mt-1.5 w-fit rounded border px-1.5 py-0.5 text-[9px] font-medium shadow-sm", hasAgent ? agentColors[sessionIdx] : "text-muted-foreground bg-muted/50 border-border/50")}>
            {hasAgent ? agentNames[sessionIdx] : "等待调度"}
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
  onNodeClick?: (node: PlanGraphView["nodes"][number]) => void;
}

export function PlanGraph({ data, onNodeClick }: PlanGraphProps) {
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showSessions, setShowSessions] = useState(false);

  const { nodes: flowNodes, edges: flowEdges, completedCount, activeCount, blockedCount } = useMemo(() => {
    const activeEdges = data.scheduling_edges.map((e) => ({ from: e.from_node_id, to: e.to_node_id }));
    const { positions, sequence } = computeLayout(data.nodes, activeEdges);
    const baseEdgeColor = edgeColor();
    const nodes: Node[] = data.nodes.map((node) => {
      const pos = positions.get(node.node_id) ?? { x: 0, y: 0 };
      return {
        id: node.node_id, type: "planNode", position: pos,
        data: { title: node.title, state: node.state, readiness: node.readiness, block_reasons: node.block_reasons, sequence: sequence.get(node.node_id) ?? 0, showSessions } as unknown as Record<string, unknown>,
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
  }, [data, selectedNodeId, showSessions]);

  const handleNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNodeId(node.id);
    const fullNode = data.nodes.find((n) => n.node_id === node.id);
    if (fullNode && onNodeClick) onNodeClick(fullNode);
  }, [data, onNodeClick]);

  const selectedNode = data.nodes.find((n) => n.node_id === selectedNodeId) ?? null;
  const totalNodes = data.nodes.length;

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 flex items-center gap-3 px-1 py-2">
        <span className="text-sm font-semibold">执行进度</span>
        <div className="flex-1 h-2 bg-muted overflow-hidden flex">
          <div className="h-full bg-emerald-500" style={{ width: `${(completedCount / Math.max(totalNodes, 1)) * 100}%` }} />
          <div className="h-full bg-blue-500" style={{ width: `${(activeCount / Math.max(totalNodes, 1)) * 100}%` }} />
          <div className="h-full bg-orange-500" style={{ width: `${(blockedCount / Math.max(totalNodes, 1)) * 100}%` }} />
        </div>
        <span className="text-xs text-muted-foreground">{completedCount}/{totalNodes} 完成</span>
        {activeCount > 0 && <span className="text-xs text-blue-600">{activeCount} 进行中</span>}
        {blockedCount > 0 && <span className="text-xs text-orange-600">{blockedCount} 阻塞</span>}
      </div>

      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      <div className="relative flex-1 rounded-md bg-card/50 overflow-hidden shadow-sm border border-border/30 [&_.react-flow\_\_controls]:!bg-background [&_.react-flow\_\_controls-button]:!bg-background [&_.react-flow\_\_controls-button]:!border-border [&_.react-flow\_\_controls-button]:!text-foreground [&_.react-flow\_\_controls-button:hover]:!bg-accent [&_.react-flow\_\_controls-button_svg]:!fill-foreground [&_.react-flow\_\_minimap]:!bg-background [&_.react-flow\_\_minimap]:!border [&_.react-flow\_\_minimap]:!border-border">
        <ReactFlow
          nodes={flowNodes}
          edges={flowEdges}
          nodeTypes={nodeTypes}
          onNodeClick={handleNodeClick}
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

          <Panel position="top-right" className="!m-3">
            <div className="border border-border/70 bg-background/95 p-3.5 backdrop-blur-sm shadow-sm space-y-3 min-w-[220px] max-w-[260px] rounded-lg">
              <div className="flex items-center justify-between gap-3 border-b border-border/50 pb-3">
                <label className="text-xs font-semibold text-foreground cursor-pointer select-none" onClick={() => setShowSessions(!showSessions)}>
                  Agent 调度与分布
                </label>
                <button type="button" onClick={() => setShowSessions(!showSessions)} className={cn("relative inline-flex h-4 w-8 shrink-0 cursor-pointer items-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2", showSessions ? "bg-blue-500" : "bg-muted-foreground/30")} role="switch" aria-checked={showSessions}>
                  <span className={cn("pointer-events-none block h-3 w-3 rounded-full bg-white shadow-sm ring-0 transition-transform", showSessions ? "translate-x-4.5" : "translate-x-0.5")} style={{ transform: showSessions ? "translateX(18px)" : "translateX(2px)" }} />
                </button>
              </div>
              <div>
                <div className="mb-2 text-[10px] font-semibold text-muted-foreground">图例</div>
                <div className="grid grid-cols-2 gap-x-3 gap-y-1.5 text-[10px] text-muted-foreground">
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-border bg-emerald-500/15 shrink-0" /><span>已完成</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-emerald-500 shrink-0" /><span>已完成路径</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-blue-500 bg-blue-500 shrink-0" /><span>进行中</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-blue-500 shrink-0" /><span>当前路径</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-orange-500/50 bg-card shrink-0" /><span>阻塞</span></div>
                  <div className="flex items-center gap-1.5"><span className="h-0.5 w-4 bg-muted-foreground/50 shrink-0" /><span>待执行路径</span></div>
                  
                  <div className="flex items-center gap-1.5"><span className="h-2.5 w-2.5 rounded-full border border-border bg-muted/30 shrink-0" /><span>待执行</span></div>
                </div>
              </div>
              {selectedNode && (
                <div className="border-t border-border/50 pt-1.5">
                  <div className="text-xs font-medium">{selectedNode.title}</div>
                  <div className="mt-0.5 text-[10px] text-muted-foreground">{stateLabel[selectedNode.state] ?? selectedNode.state}</div>
                  <div className="mt-1 text-[10px] text-muted-foreground">{selectedNode.goal}</div>
                  {selectedNode.scope.length > 0 && <div className="mt-1 text-[10px]"><span className="text-muted-foreground">范围：</span>{selectedNode.scope.join("、")}</div>}
                  {selectedNode.block_reasons.length > 0 && <div className="mt-1 text-[10px] text-orange-600">阻塞：{selectedNode.block_reasons.join("、")}</div>}
                  <div className="mt-1 text-[10px] text-blue-600">点击节点查看详细简报</div>
                </div>
              )}
            </div>
          </Panel>

          <MiniMap 
            pannable 
            zoomable
            className="!bg-card !border-border/50 shadow-md rounded-md"
            nodeClassName={(node) => {
              const d = node.data as unknown as PlanNodeData;
              if (d?.state === "completed") return "!fill-emerald-500 !stroke-emerald-600";
              if (d?.state === "active") return "!fill-blue-500 !stroke-blue-600";
              if (d?.state === "blocked") return "!fill-orange-500 !stroke-orange-600";
              return "!fill-muted !stroke-border";
            }}
          />
        </ReactFlow>
      </div>
    </div>
  );
}
