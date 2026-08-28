import { useMemo, useRef, useState, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, ChevronDown, Search, Folder, File, Package, Box, Braces, ArrowRight, ExternalLink, FolderOpen } from "lucide-react";
import { cn } from "@/lib/utils";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { NativePathDisplay } from "@/v3/components/common/NativePathDisplay";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";
import { queryV3ProjectStructure } from "@/services/v3ProductionViews";
import { useTranslation } from "react-i18next";

const sourceKindColor: Record<string, string> = {
  filesystem: "#9ca3af", git: "#22c55e", manifest: "#3b82f6",
  parser: "#a855f7", documentation: "#f97316", inference: "#eab308",
};
const kindIcons: Record<string, React.ComponentType<{ className?: string }>> = { root: Box, directory: Folder, file: File, module: Package, package: Package, symbol: Braces };
const gitStateTone: Record<string, string> = { clean: "", modified: "text-amber-600 dark:text-amber-400", added: "text-blue-600 dark:text-blue-400", deleted: "text-red-600 dark:text-red-400", ignored: "text-muted-foreground/50", unknown: "text-muted-foreground/50" };

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
  taskId: string;
  projectPath?: string;
  highlightModuleId?: string | null;
  onModuleClick?: (moduleId: string | null) => void;
  onRevealProjectFile?: (projectPath: string, taskId: string, relativePath: string) => Promise<void>;
  onOpenProjectFile?: (projectPath: string, taskId: string, relativePath: string) => Promise<void>;
}

export function StructureArchitecture({ data, taskId, projectPath, highlightModuleId, onModuleClick, onRevealProjectFile, onOpenProjectFile }: StructureArchitectureProps) {
  const { t } = useTranslation();
  const sourceLabel = (kind: string) => t(`v3.structure.sourceKind.${kind}`, { defaultValue: kind });
  const edgeLabel = (kind: string) => t(`v3.structure.edgeKind.${kind}`, { defaultValue: kind });
  const [viewData, setViewData] = useState(data);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [remoteSearch, setRemoteSearch] = useState<ProjectStructureView | null>(null);
  const [queryLoading, setQueryLoading] = useState(false);
  const [queryError, setQueryError] = useState<string | null>(null);
  const [pageDirectory, setPageDirectory] = useState(".");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedArchitectureEdgeId, setSelectedArchitectureEdgeId] = useState<string | null>(null);
  const [fileActionLoading, setFileActionLoading] = useState<"reveal" | "open" | null>(null);
  const [fileActionError, setFileActionError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const searchRequestRef = useRef(0);

  useEffect(() => {
    searchRequestRef.current += 1;
    setViewData(data);
    setRemoteSearch(null);
    setPageDirectory(".");
    setSelectedNodeId(null);
    setSelectedArchitectureEdgeId(null);
    setFileActionLoading(null);
    setFileActionError(null);
  }, [data]);

  useEffect(() => {
    if (!projectPath || data.index_state !== "ready" || data.nodes.length > 0 || !data.page.truncated) return;
    let cancelled = false;
    setQueryLoading(true);
    setQueryError(null);
    void queryV3ProjectStructure(projectPath, taskId, ".")
      .then((result) => {
        if (!cancelled) setViewData(result);
      })
      .catch((error: Error) => {
        if (!cancelled) setQueryError(error.message);
      })
      .finally(() => {
        if (!cancelled) setQueryLoading(false);
      });
    return () => { cancelled = true; };
  }, [data, projectPath, taskId]);

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
      void queryV3ProjectStructure(projectPath, taskId, ".", null, 200, search)
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
  }, [projectPath, taskId, search]);

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
        mergePage(await queryV3ProjectStructure(projectPath, taskId, directory));
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
      mergePage(await queryV3ProjectStructure(projectPath, taskId, pageDirectory, viewData.page.next_cursor));
    } catch (error) {
      setQueryError((error as Error).message);
    } finally {
      setQueryLoading(false);
    }
  };
  const selectedNode = viewData.nodes.find((n) => n.node_id === selectedNodeId) ?? null;
  const runFileAction = async (mode: "reveal" | "open") => {
    if (!projectPath || !selectedNode) return;
    const action = mode === "reveal" ? onRevealProjectFile : onOpenProjectFile;
    if (!action) return;
    setFileActionLoading(mode);
    setFileActionError(null);
    try {
      await action(projectPath, taskId, relativePath(selectedNode));
    } catch (error) {
      setFileActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setFileActionLoading(null);
    }
  };

  // 架构侧：模块列表
  const modules = useMemo(() => {
    const mods = viewData.architecture_nodes;
    const edges = viewData.architecture_edges;
    return { mods, edges };
  }, [viewData.architecture_nodes, viewData.architecture_edges]);
  const selectedArchitectureEdge = modules.edges.find((edge) => edge.edge_id === selectedArchitectureEdgeId) ?? null;
  return (
    <div className="relative flex h-full flex-col gap-2 min-h-0">
      {queryError && <div className="text-xs text-red-600" role="alert">{queryError}</div>}

      <div className="flex flex-col gap-2 shrink-0 mb-1">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          <div className="flex items-center gap-1 min-w-0 max-w-[300px] xl:max-w-md">
            <span className="text-xs font-medium whitespace-nowrap text-foreground/90">{t("v3.structure.workingDirectory")}</span>
            <span className="text-xs text-muted-foreground truncate" title={viewData.workspace.root.display}>{viewData.workspace.root.display}</span>
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-[11px] text-muted-foreground whitespace-nowrap bg-muted/50 px-2 py-1 rounded-md border border-border/50">
              {t("v3.structure.workspaceSource", { source: t(`v3.structure.workspaceSourceValue.${viewData.workspace.source}`, { defaultValue: viewData.workspace.source }) })}
            </span>
            <span className="text-[11px] text-muted-foreground whitespace-nowrap bg-muted/50 px-2 py-1 rounded-md border border-border/50">
              {t("v3.structure.architectureSummary", { nodes: viewData.nodes.length, edges: viewData.edges.length, modules: viewData.architecture_nodes.length })}
            </span>
            <span className="text-[11px] text-muted-foreground whitespace-nowrap bg-muted/50 px-2 py-1 rounded-md border border-border/50">
              {t("v3.structure.index", { state: t(`v3.structure.indexState.${viewData.index_state}`, { defaultValue: viewData.index_state }) })}{viewData.page.truncated && t("v3.structure.truncatedReason", { reason: viewData.page.truncation_reason })}{queryLoading && t("v3.structure.querying")}
            </span>
          </div>
        </div>
        {viewData.workspace.fallback_reason && (
          <div className="text-[11px] text-amber-600 bg-amber-500/10 px-2 py-1 rounded w-fit border border-amber-500/20">
            {viewData.workspace.fallback_reason}
          </div>
        )}
      </div>

      {/* 左右对照 */}
      <div className="grid flex-1 min-h-0 gap-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)]">
        {/* 左：架构模块列表 */}
        <div className="flex flex-col overflow-hidden">
          <div className="shrink-0 pb-3 flex flex-col gap-2">
            <span className="text-sm font-semibold text-foreground/80">{t("v3.structure.architectureModules")}</span>
            {/* 固定图例 */}
            <div className="flex flex-wrap gap-x-3 gap-y-1.5">
              {Object.entries(sourceKindColor).map(([kind, color]) => (
                <div key={kind} className="flex items-center gap-1 text-[11px]"><span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: color }} /><span className="text-muted-foreground/80">{sourceLabel(kind)}</span></div>
              ))}
            </div>
          </div>
          <div className="flex-1 overflow-y-auto scrollbar-auto-hide space-y-1.5 pr-2">
            {modules.mods.map((mod) => {
              const files = mod.file_count;
              const incoming = modules.edges.filter((e) => e.to_node_id === mod.node_id);
              const outgoing = modules.edges.filter((e) => e.from_node_id === mod.node_id);
              const isHighlighted = highlightModuleId === mod.node_id;
              return (
                <div key={mod.node_id} className={cn("p-3 rounded-xl transition-all border border-transparent", isHighlighted ? "bg-card shadow-sm border-border/50" : "hover:bg-muted/40", onModuleClick && "cursor-pointer")} onClick={() => onModuleClick?.(mod.node_id)}>
                  <div className="flex items-center gap-2 mb-1.5">
                    <Package className={cn("h-4 w-4", isHighlighted ? "text-primary" : "text-muted-foreground")} />
                    <span className={cn("text-sm font-medium truncate", isHighlighted ? "text-foreground" : "text-foreground/80")}>{mod.name}</span>
                    <span className="ml-auto text-xs text-muted-foreground/70 bg-muted px-1.5 py-0.5 rounded-md">{t("v3.structure.fileCount", { count: files })}</span>
                  </div>
                  <div className="mt-2 pl-6 text-[11px] text-muted-foreground">
                    <NativePathDisplay path={mod.path} />
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-1.5 pl-6 text-[10px] text-muted-foreground">
                    <span className="rounded bg-muted px-1.5 py-0.5" style={{ color: sourceKindColor[mod.source_kind] }}>{sourceLabel(mod.source_kind)}</span>
                    <span>{t("v3.structure.confidenceValue", { value: (mod.confidence * 100).toFixed(0) })}</span>
                    <span className="font-mono" title={mod.generator_version}>{mod.generator_version}</span>
                  </div>
                  <EvidenceLink evidenceRefs={mod.evidence_refs} className="mt-2 pl-6" />
                  {(incoming.length > 0 || outgoing.length > 0) && (
                    <div className="mt-3 pt-2 flex flex-wrap gap-1.5 pl-6">
                      {outgoing.map((e) => { const target = viewData.architecture_nodes.find((n) => n.node_id === e.to_node_id); return <button key={e.edge_id} type="button" onClick={(event) => { event.stopPropagation(); setSelectedArchitectureEdgeId(e.edge_id); }} className={cn("flex items-center gap-1 rounded bg-accent/50 px-1.5 py-0.5 text-[11px] text-muted-foreground", selectedArchitectureEdgeId === e.edge_id && "ring-1 ring-primary/40")} title={`${sourceLabel(e.source_kind)} · ${(e.confidence * 100).toFixed(0)}%`}><span style={{ color: sourceKindColor[e.source_kind] }}>●</span>{edgeLabel(e.kind)} <ArrowRight className="h-3 w-3" /> <span className="font-medium">{target?.name ?? "?"}</span></button>; })}
                      {incoming.map((e) => { const src = viewData.architecture_nodes.find((n) => n.node_id === e.from_node_id); return <button key={e.edge_id} type="button" onClick={(event) => { event.stopPropagation(); setSelectedArchitectureEdgeId(e.edge_id); }} className={cn("flex items-center gap-1 rounded bg-accent/50 px-1.5 py-0.5 text-[11px] text-muted-foreground", selectedArchitectureEdgeId === e.edge_id && "ring-1 ring-primary/40")} title={`${sourceLabel(e.source_kind)} · ${(e.confidence * 100).toFixed(0)}%`}><span style={{ color: sourceKindColor[e.source_kind] }}>●</span><span className="font-medium">{src?.name ?? "?"}</span> <ArrowRight className="h-3 w-3" /> {edgeLabel(e.kind)}</button>; })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
          {selectedArchitectureEdge && (
            <div className="mt-2 shrink-0 rounded-lg border border-border/50 bg-card/60 p-3 text-xs">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{modules.mods.find((node) => node.node_id === selectedArchitectureEdge.from_node_id)?.name ?? selectedArchitectureEdge.from_node_id}</span>
                <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="font-medium">{modules.mods.find((node) => node.node_id === selectedArchitectureEdge.to_node_id)?.name ?? selectedArchitectureEdge.to_node_id}</span>
                <span className="ml-auto text-muted-foreground">{edgeLabel(selectedArchitectureEdge.kind)}</span>
              </div>
              <div className="mt-2 flex flex-wrap gap-2 text-[10px] text-muted-foreground">
                <span style={{ color: sourceKindColor[selectedArchitectureEdge.source_kind] }}>{sourceLabel(selectedArchitectureEdge.source_kind)}</span>
                <span>{t("v3.structure.confidenceValue", { value: (selectedArchitectureEdge.confidence * 100).toFixed(0) })}</span>
                <span className="font-mono">{selectedArchitectureEdge.generator_version}</span>
              </div>
              <EvidenceLink evidenceRefs={selectedArchitectureEdge.evidence_refs} className="mt-2" />
            </div>
          )}
        </div>

        {/* 右：文件树 */}
        <div className="flex flex-col overflow-hidden bg-card/20 rounded-xl ring-1 ring-border/30">
          <div className="shrink-0 pb-3 flex items-center gap-3 px-1 mt-[-6px]">
            <span className="text-sm font-semibold text-foreground/80">{t("v3.structure.fileTree")}</span>
            <div className="relative ml-auto w-48">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground/70" />
              <input type="text" value={search} onChange={(e) => setSearch(e.target.value)} placeholder={t("v3.structure.searchFiles")} className="h-8 w-full rounded-md border border-input/50 bg-background/50 pl-8 pr-3 text-sm focus:outline-none focus:ring-1 focus:ring-ring transition-all" />
            </div>
          </div>
          <div ref={scrollRef} className="flex-1 overflow-auto rounded-lg bg-card/30 border border-border/30">
            {search ? (
              <div className="divide-y divide-border/30">
                {searchResults!.length === 0 ? <div className="px-3 py-6 text-center text-sm text-muted-foreground">{t("v3.structure.noMatches")}</div> : searchResults!.map(({ node, parents }) => { const Icon = kindIcons[node.kind] ?? File; return <button key={node.node_id} type="button" onClick={() => setSelectedNodeId(node.node_id)} className={cn("flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-muted/20", selectedNodeId === node.node_id && "bg-muted/30")}><Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" /><span className="truncate">{node.name}</span>{parents.length > 0 && <span className="ml-auto truncate text-xs text-muted-foreground/60">{"< " + parents.join(" < ")}</span>}</button>; })}
              </div>
            ) : (
              <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
                {virtualizer.getVirtualItems().map((vi) => {
                  const tn = flatNodes![vi.index]; const node = tn.node; const Icon = kindIcons[node.kind] ?? File; const hasChildren = tn.children.length > 0; const isExpanded = expanded.has(node.node_id);
                  const isHighlighted = highlightModuleId && (node.module_id === highlightModuleId || node.node_id === highlightModuleId);
                  const expandable = node.kind === "directory" || node.kind === "root" || hasChildren;
                  return <div key={vi.key} data-index={vi.index} ref={virtualizer.measureElement} style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${vi.start}px)` }}><button type="button" onClick={() => expandable ? void toggleExpand(node, hasChildren) : setSelectedNodeId(node.node_id)} onDoubleClick={() => setSelectedNodeId(node.node_id)} className={cn("flex w-full items-center gap-2 px-2 py-1 text-left text-sm transition-all duration-500", selectedNodeId === node.node_id ? "bg-muted/30" : "hover:bg-muted/20", isHighlighted && "bg-blue-500/10 text-blue-600 dark:text-blue-400 font-medium")} style={{ paddingLeft: `${tn.depth * 16 + 8}px` }}>{expandable ? (isExpanded ? <ChevronDown className={cn("h-3.5 w-3.5 shrink-0 transition-transform", isHighlighted && "text-blue-500")} /> : <ChevronRight className={cn("h-3.5 w-3.5 shrink-0 transition-transform", isHighlighted && "text-blue-500")} />) : <span className="w-3.5 shrink-0" />}<Icon className={cn("h-3.5 w-3.5 shrink-0", isHighlighted ? "text-blue-500" : "text-muted-foreground")} /><span className="truncate">{node.name}</span><span className={cn("ml-auto text-[10px]", gitStateTone[node.git_state])}>{node.git_state !== "clean" && node.git_state !== "unknown" ? t(`v3.structure.gitState.${node.git_state}`, { defaultValue: node.git_state }) : ""}</span></button></div>;
                })}
              </div>
            )}
          </div>
          {!search && viewData.page.next_cursor && (
            <button type="button" onClick={() => void loadNextPage()} disabled={queryLoading} className="mt-2 h-8 shrink-0 rounded-md border border-border/50 text-xs text-muted-foreground hover:bg-muted/30 disabled:opacity-50">
              {t(queryLoading ? "v3.structure.loading" : "v3.structure.loadMore")}
            </button>
          )}
          {selectedNode && (
            <div className="shrink-0 mt-3 rounded-lg bg-card/60 border border-border/40 p-3.5 shadow-sm">
              <div className="mb-2 text-sm font-medium text-foreground">{selectedNode.name}</div>
              <div className="flex flex-wrap gap-x-4 gap-y-1.5 text-xs text-muted-foreground/90">
                <span className="flex items-center gap-1.5"><Box className="h-3.5 w-3.5 opacity-70" />{t(`v3.structure.kind.${selectedNode.kind}`, { defaultValue: selectedNode.kind })}</span>
                <span className={cn("flex items-center gap-1.5", gitStateTone[selectedNode.git_state])}><File className="h-3.5 w-3.5 opacity-70" />{t(`v3.structure.gitState.${selectedNode.git_state}`, { defaultValue: selectedNode.git_state })}</span>
                {selectedNode.module_id && <span className="flex items-center gap-1.5"><Package className="h-3.5 w-3.5 opacity-70" />{selectedNode.module_id}</span>}
              </div>
              <div className="mt-3"><NativePathDisplay path={selectedNode.path} showPlatform /></div>
              <div className="mt-2"><EvidenceLink evidenceRefs={selectedNode.evidence_refs} /></div>
              {projectPath && (onRevealProjectFile || (selectedNode.kind === "file" && onOpenProjectFile)) && (
                <div className="mt-3 flex flex-wrap gap-2 border-t border-border/40 pt-3">
                  {onRevealProjectFile && (
                    <button type="button" onClick={() => void runFileAction("reveal")} disabled={fileActionLoading !== null} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-xs hover:bg-muted/40 disabled:opacity-50">
                      <FolderOpen className="h-3.5 w-3.5" />{t(fileActionLoading === "reveal" ? "v3.structure.revealing" : "v3.structure.reveal")}
                    </button>
                  )}
                  {selectedNode.kind === "file" && onOpenProjectFile && (
                    <button type="button" onClick={() => void runFileAction("open")} disabled={fileActionLoading !== null} className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border/60 px-2.5 text-xs hover:bg-muted/40 disabled:opacity-50">
                      <ExternalLink className="h-3.5 w-3.5" />{t(fileActionLoading === "open" ? "v3.structure.opening" : "v3.structure.open")}
                    </button>
                  )}
                </div>
              )}
              {fileActionError && <div className="mt-2 text-xs text-red-600" role="alert">{fileActionError}</div>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
