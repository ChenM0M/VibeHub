import { useMemo, useRef, useState, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, ChevronDown, Search, Folder, File, Package, Box, Braces, ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { NativePathDisplay } from "@/v3/components/common/NativePathDisplay";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";
import { queryV3ProjectStructure } from "@/services/v3ProductionViews";

const sourceKindColor: Record<string, string> = {
  filesystem: "#9ca3af", git: "#22c55e", manifest: "#3b82f6",
  parser: "#a855f7", documentation: "#f97316", inference: "#eab308",
};
const sourceKindLabel: Record<string, string> = {
  filesystem: "文件系统", git: "Git", manifest: "清单", parser: "解析器", documentation: "文档", inference: "推断",
};
const edgeKindLabel: Record<string, string> = { contains: "包含", imports: "导入", depends_on: "依赖", declares: "声明", generates: "生成" };
const kindIcons: Record<string, React.ComponentType<{ className?: string }>> = { root: Box, directory: Folder, file: File, module: Package, package: Package, symbol: Braces };
const kindLabel: Record<string, string> = { root: "根", directory: "目录", file: "文件", module: "模块", package: "包", symbol: "符号" };
const gitStateTone: Record<string, string> = { clean: "", modified: "text-amber-600 dark:text-amber-400", added: "text-blue-600 dark:text-blue-400", deleted: "text-red-600 dark:text-red-400", ignored: "text-muted-foreground/50", unknown: "text-muted-foreground/50" };
const gitStateLabel: Record<string, string> = { clean: "干净", modified: "已修改", added: "已添加", deleted: "已删除", ignored: "已忽略", unknown: "未知" };
const indexStateLabel: Record<string, string> = { uninitialized: "未初始化", indexing: "索引中", ready: "就绪", interrupted: "已中断", error: "错误" };

interface TreeNode { node: ProjectStructureView["nodes"][number]; children: TreeNode[]; depth: number; }

function buildTree(nodes: ProjectStructureView["nodes"]): TreeNode[] {
  const map = new Map<string, TreeNode>();
  const roots: TreeNode[] = [];
  for (const node of nodes) map.set(node.node_id, { node, children: [], depth: 0 });
  for (const node of nodes) {
    const tn = map.get(node.node_id)!;
    if (node.parent_id && map.has(node.parent_id)) { const p = map.get(node.parent_id)!; tn.depth = p.depth + 1; p.children.push(tn); }
    else roots.push(tn);
  }
  return roots;
}
function flattenTree(roots: TreeNode[], expanded: Set<string>): TreeNode[] {
  const result: TreeNode[] = [];
  const walk = (nodes: TreeNode[]) => { for (const n of nodes) { result.push(n); if (expanded.has(n.node.node_id) && n.children.length > 0) walk(n.children); } };
  walk(roots);
  return result;
}

interface StructureArchitectureProps {
  data: ProjectStructureView;
  projectPath?: string;
  highlightModuleId?: string | null;
  onModuleClick?: (moduleId: string | null) => void;
}

export function StructureArchitecture({ data, projectPath, highlightModuleId, onModuleClick }: StructureArchitectureProps) {
  const [viewData, setViewData] = useState(data);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [remoteSearch, setRemoteSearch] = useState<ProjectStructureView | null>(null);
  const [queryLoading, setQueryLoading] = useState(false);
  const [queryError, setQueryError] = useState<string | null>(null);
  const [pageDirectory, setPageDirectory] = useState(".");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const searchRequestRef = useRef(0);

  useEffect(() => {
    searchRequestRef.current += 1;
    setViewData(data);
    setRemoteSearch(null);
    setPageDirectory(".");
  }, [data]);

  const treeRoots = useMemo(() => buildTree(viewData.nodes), [viewData.nodes]);
  const flatNodes = useMemo(() => (search ? null : flattenTree(treeRoots, expanded)), [treeRoots, expanded, search]);

  const searchResults = useMemo(() => {
    if (!search) return null;
    const q = search.toLowerCase();
    const source = remoteSearch?.nodes ?? viewData.nodes;
    const byId = new Map(source.map((n) => [n.node_id, n]));
    const result: { node: ProjectStructureView["nodes"][number]; parents: string[] }[] = [];
    for (const node of source) {
      if (node.name.toLowerCase().includes(q) || node.path.display.toLowerCase().includes(q)) {
        const parents: string[] = [];
        let cur = node.parent_id;
        while (cur && byId.has(cur)) { const p = byId.get(cur)!; parents.unshift(p.name); cur = p.parent_id; }
        result.push({ node, parents });
      }
    }
    return result;
  }, [viewData.nodes, remoteSearch, search]);

  useEffect(() => {
    const requestId = ++searchRequestRef.current;
    if (!projectPath || search.trim().length < 2) {
      setRemoteSearch(null);
      setQueryError(null);
      setQueryLoading(false);
      return;
    }
    const request = window.setTimeout(() => {
      setQueryLoading(true);
      setQueryError(null);
      void queryV3ProjectStructure(projectPath, ".", null, 200, search)
        .then((result) => {
          if (searchRequestRef.current === requestId) setRemoteSearch(result);
        })
        .catch((error: Error) => {
          if (searchRequestRef.current === requestId) setQueryError(error.message);
        })
        .finally(() => {
          if (searchRequestRef.current === requestId) setQueryLoading(false);
        });
    }, 180);
    return () => {
      window.clearTimeout(request);
      if (searchRequestRef.current === requestId) searchRequestRef.current += 1;
    };
  }, [projectPath, search]);

  const virtualizer = useVirtualizer({ count: flatNodes?.length ?? 0, getScrollElement: () => scrollRef.current, estimateSize: () => 32, overscan: 10 });

  useEffect(() => {
    if (highlightModuleId && viewData.nodes.length > 0) {
      const allTargets = viewData.nodes.filter((n) => n.module_id === highlightModuleId || n.node_id === highlightModuleId);
      if (allTargets.length > 0) {
        const parents = new Set<string>();
        const byId = new Map(viewData.nodes.map((n) => [n.node_id, n]));
        for (const target of allTargets) {
          let cur = target.parent_id;
          while (cur && byId.has(cur)) { parents.add(cur); cur = byId.get(cur)!.parent_id; }
        }
        setExpanded((prev) => new Set([...prev, ...parents]));
      }
    }
  }, [highlightModuleId, viewData.nodes]);

  const mergePage = (page: ProjectStructureView) => setViewData((current) => {
    const nodes = new Map(current.nodes.map((node) => [node.node_id, node]));
    const edges = new Map(current.edges.map((edge) => [edge.edge_id, edge]));
    page.nodes.forEach((node) => {
      const existing = nodes.get(node.node_id);
      nodes.set(node.node_id, existing && node.parent_id === null ? { ...node, parent_id: existing.parent_id } : node);
    });
    page.edges.forEach((edge) => edges.set(edge.edge_id, edge));
    return { ...current, nodes: [...nodes.values()], edges: [...edges.values()], page: page.page, warnings: [...current.warnings, ...page.warnings] };
  });
  const relativePath = (node: ProjectStructureView["nodes"][number]) => {
    const root = viewData.nodes.find((candidate) => candidate.kind === "root")?.path.display.replace(/[/\\]+$/, "") ?? projectPath ?? "";
    return node.path.display.slice(root.length).replace(/^[/\\]+/, "") || ".";
  };
  const toggleExpand = async (node: ProjectStructureView["nodes"][number], hasChildren: boolean) => {
    if (expanded.has(node.node_id)) { setExpanded((value) => { const next = new Set(value); next.delete(node.node_id); return next; }); return; }
    if (projectPath && node.kind === "directory" && !hasChildren) {
      setQueryLoading(true); setQueryError(null);
      const directory = relativePath(node);
      try {
        mergePage(await queryV3ProjectStructure(projectPath, directory));
        setPageDirectory(directory);
      }
      catch (error) { setQueryError((error as Error).message); }
      finally { setQueryLoading(false); }
    }
    setExpanded((value) => new Set(value).add(node.node_id));
  };
  const loadNextPage = async () => {
    if (!projectPath || !viewData.page.next_cursor || queryLoading) return;
    setQueryLoading(true); setQueryError(null);
    try {
      mergePage(await queryV3ProjectStructure(projectPath, pageDirectory, viewData.page.next_cursor));
    } catch (error) {
      setQueryError((error as Error).message);
    } finally {
      setQueryLoading(false);
    }
  };
  const selectedNode = viewData.nodes.find((n) => n.node_id === selectedNodeId) ?? null;

  // 架构侧：模块列表
  const modules = useMemo(() => {
    const mods = viewData.nodes.filter((n) => n.kind === "module" || n.kind === "package" || n.kind === "root");
    const moduleIds = new Set(mods.map((n) => n.node_id));
    const fileMap = new Map<string, typeof viewData.nodes>();
    for (const node of viewData.nodes) {
      const key = node.module_id ?? (moduleIds.has(node.node_id) ? node.node_id : null);
      if (key) { if (!fileMap.has(key)) fileMap.set(key, []); fileMap.get(key)!.push(node); }
    }
    const edges = viewData.edges.filter((e) => moduleIds.has(e.from_node_id) && moduleIds.has(e.to_node_id));
    return { mods, fileMap, edges };
  }, [viewData.nodes, viewData.edges]);

  return (
    <div className="flex h-full flex-col gap-2 min-h-0">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />
      {queryError && <div className="text-xs text-red-600" role="alert">{queryError}</div>}

      <div className="flex items-center gap-3 shrink-0">
        <span className="text-xs text-muted-foreground">{viewData.nodes.length} 节点 · {viewData.edges.length} 边 · 索引：{indexStateLabel[viewData.index_state] ?? viewData.index_state}{viewData.page.truncated && "（已截断）"}{queryLoading && " · 查询中"}</span>
        {data.unsupported_analyzers.length > 0 && <span className="text-xs text-orange-600">不支持：{data.unsupported_analyzers.join("、")}</span>}
      </div>

      {/* 左右对照 */}
      <div className="grid flex-1 min-h-0 gap-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
        {/* 左：架构模块列表 */}
        <div className="flex flex-col overflow-hidden">
          <div className="shrink-0 pb-3 flex flex-col gap-2">
            <span className="text-sm font-semibold text-foreground/80">架构模块</span>
            {/* 固定图例 */}
            <div className="flex flex-wrap gap-x-3 gap-y-1.5">
              {Object.entries(sourceKindColor).map(([kind, color]) => (
                <div key={kind} className="flex items-center gap-1 text-[11px]"><span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: color }} /><span className="text-muted-foreground/80">{sourceKindLabel[kind]}</span></div>
              ))}
            </div>
          </div>
          <div className="flex-1 overflow-y-auto scrollbar-auto-hide space-y-1.5 pr-2">
            {modules.mods.map((mod) => {
              const files = modules.fileMap.get(mod.module_id ?? mod.node_id) ?? [];
              const incoming = modules.edges.filter((e) => e.to_node_id === mod.node_id);
              const outgoing = modules.edges.filter((e) => e.from_node_id === mod.node_id);
              const isHighlighted = highlightModuleId === (mod.module_id ?? mod.node_id);
              return (
                <div key={mod.node_id} className={cn("p-3 rounded-xl transition-all border border-transparent", isHighlighted ? "bg-card shadow-sm border-border/50" : "hover:bg-muted/40", onModuleClick && "cursor-pointer")} onClick={() => onModuleClick?.(mod.module_id ?? mod.node_id)}>
                  <div className="flex items-center gap-2 mb-1.5">
                    <Package className={cn("h-4 w-4", isHighlighted ? "text-primary" : "text-muted-foreground")} />
                    <span className={cn("text-sm font-medium truncate", isHighlighted ? "text-foreground" : "text-foreground/80")}>{mod.name}</span>
                    <span className="ml-auto text-xs text-muted-foreground/70 bg-muted px-1.5 py-0.5 rounded-md">{files.length} 文件</span>
                  </div>
                  <div className="space-y-1 mt-2 pl-6">
                    {files.slice(0, 4).map((f) => { const Icon = kindIcons[f.kind] ?? File; return <div key={f.node_id} className="flex items-center gap-2 text-[13px] text-muted-foreground truncate"><Icon className="h-3.5 w-3.5 shrink-0 opacity-70" /><span className="truncate">{f.name}</span></div>; })}
                    {files.length > 4 && <div className="text-xs text-muted-foreground/50 pt-0.5">还有 {files.length - 4} 个文件…</div>}
                  </div>
                  {(incoming.length > 0 || outgoing.length > 0) && (
                    <div className="mt-3 pt-2 flex flex-wrap gap-1.5 pl-6">
                      {outgoing.map((e) => { const target = data.nodes.find((n) => n.node_id === e.to_node_id); return <div key={e.edge_id} className="flex items-center gap-1 rounded bg-accent/50 px-1.5 py-0.5 text-[11px] text-muted-foreground" title={edgeKindLabel[e.kind]}><span style={{ color: sourceKindColor[e.source_kind] }}>●</span>{edgeKindLabel[e.kind]} <ArrowRight className="h-3 w-3" /> <span className="font-medium">{target?.name ?? "?"}</span></div>; })}
                      {incoming.map((e) => { const src = data.nodes.find((n) => n.node_id === e.from_node_id); return <div key={e.edge_id} className="flex items-center gap-1 rounded bg-accent/50 px-1.5 py-0.5 text-[11px] text-muted-foreground" title={edgeKindLabel[e.kind]}><span style={{ color: sourceKindColor[e.source_kind] }}>●</span><span className="font-medium">{src?.name ?? "?"}</span> <ArrowRight className="h-3 w-3" /> {edgeKindLabel[e.kind]}</div>; })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* 右：文件树 */}
        <div className="flex flex-col overflow-hidden bg-card/20 rounded-xl ring-1 ring-border/30">
          <div className="shrink-0 pb-3 flex items-center gap-3 px-1 mt-[-6px]">
            <span className="text-sm font-semibold text-foreground/80">文件树</span>
            <div className="relative ml-auto w-48">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground/70" />
              <input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder="搜索文件或节点…" className="h-8 w-full rounded-md border border-input/50 bg-background/50 pl-8 pr-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-all" />
            </div>
          </div>
          <div ref={scrollRef} className="flex-1 overflow-auto rounded-lg bg-card/30 border border-border/30">
            {search ? (
              <div className="divide-y divide-border/30">
                {searchResults!.length === 0 ? <div className="px-3 py-6 text-center text-sm text-muted-foreground">无匹配项</div> : searchResults!.map(({ node, parents }) => { const Icon = kindIcons[node.kind] ?? File; return <button key={node.node_id} type="button" onClick={() => setSelectedNodeId(node.node_id)} className={cn("flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-muted/20", selectedNodeId === node.node_id && "bg-muted/30")}><Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" /><span className="truncate">{node.name}</span>{parents.length > 0 && <span className="ml-auto truncate text-xs text-muted-foreground/60">{"< " + parents.join(" < ")}</span>}</button>; })}
              </div>
            ) : (
              <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
                {virtualizer.getVirtualItems().map((vi) => {
                  const tn = flatNodes![vi.index]; const node = tn.node; const Icon = kindIcons[node.kind] ?? File; const hasChildren = tn.children.length > 0; const isExpanded = expanded.has(node.node_id);
                  const isHighlighted = highlightModuleId && (node.module_id === highlightModuleId || node.node_id === highlightModuleId);
                  const expandable = node.kind === "directory" || node.kind === "root" || hasChildren;
                  return <div key={vi.key} data-index={vi.index} ref={virtualizer.measureElement} style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${vi.start}px)` }}><button type="button" onClick={() => expandable ? void toggleExpand(node, hasChildren) : setSelectedNodeId(node.node_id)} onDoubleClick={() => setSelectedNodeId(node.node_id)} className={cn("flex w-full items-center gap-2 px-2 py-1 text-left text-sm transition-all duration-500", selectedNodeId === node.node_id ? "bg-muted/30" : "hover:bg-muted/20", isHighlighted && "bg-blue-500/10 text-blue-600 dark:text-blue-400 font-medium")} style={{ paddingLeft: `${tn.depth * 16 + 8}px` }}>{expandable ? (isExpanded ? <ChevronDown className={cn("h-3.5 w-3.5 shrink-0 transition-transform", isHighlighted && "text-blue-500")} /> : <ChevronRight className={cn("h-3.5 w-3.5 shrink-0 transition-transform", isHighlighted && "text-blue-500")} />) : <span className="w-3.5 shrink-0" />}<Icon className={cn("h-3.5 w-3.5 shrink-0", isHighlighted ? "text-blue-500" : "text-muted-foreground")} /><span className="truncate">{node.name}</span><span className={cn("ml-auto text-[10px]", gitStateTone[node.git_state])}>{node.git_state !== "clean" && node.git_state !== "unknown" ? gitStateLabel[node.git_state] ?? node.git_state : ""}</span></button></div>;
                })}
              </div>
            )}
          </div>
          {!search && viewData.page.next_cursor && (
            <button type="button" onClick={() => void loadNextPage()} disabled={queryLoading} className="mt-2 h-8 shrink-0 rounded-md border border-border/50 text-xs text-muted-foreground hover:bg-muted/30 disabled:opacity-50">
              {queryLoading ? "加载中" : "加载更多"}
            </button>
          )}
          {selectedNode && (
            <div className="shrink-0 mt-3 rounded-lg bg-card/60 border border-border/40 p-3.5 shadow-sm">
              <div className="mb-2 text-sm font-medium text-foreground">{selectedNode.name}</div>
              <div className="flex flex-wrap gap-x-4 gap-y-1.5 text-xs text-muted-foreground/90">
                <span className="flex items-center gap-1.5"><Box className="h-3.5 w-3.5 opacity-70" />{kindLabel[selectedNode.kind] ?? selectedNode.kind}</span>
                <span className={cn("flex items-center gap-1.5", gitStateTone[selectedNode.git_state])}><File className="h-3.5 w-3.5 opacity-70" />{gitStateLabel[selectedNode.git_state] ?? selectedNode.git_state}</span>
                {selectedNode.module_id && <span className="flex items-center gap-1.5"><Package className="h-3.5 w-3.5 opacity-70" />{selectedNode.module_id}</span>}
              </div>
              <div className="mt-3"><NativePathDisplay path={selectedNode.path} showPlatform /></div>
              <div className="mt-2"><EvidenceLink evidenceRefs={selectedNode.evidence_refs} /></div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
