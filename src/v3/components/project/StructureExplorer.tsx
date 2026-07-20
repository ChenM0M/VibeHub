import { useMemo, useRef, useState, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronRight, ChevronDown, Search, Folder, File, Package, Box, Braces } from "lucide-react";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { NativePathDisplay } from "@/v3/components/common/NativePathDisplay";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";
import { useTranslation } from "react-i18next";

const kindIcons: Record<string, React.ComponentType<{ className?: string }>> = {
  root: Box,
  directory: Folder,
  file: File,
  module: Package,
  package: Package,
  symbol: Braces,
};

const gitStateTone: Record<string, string> = {
  clean: "",
  modified: "text-amber-600 dark:text-amber-400",
  added: "text-blue-600 dark:text-blue-400",
  deleted: "text-red-600 dark:text-red-400",
  ignored: "text-muted-foreground/50",
  unknown: "text-muted-foreground/50",
};

interface TreeNode {
  node: ProjectStructureView["nodes"][number];
  children: TreeNode[];
  depth: number;
}

function buildTree(nodes: ProjectStructureView["nodes"]): TreeNode[] {
  const map = new Map<string, TreeNode>();
  const roots: TreeNode[] = [];
  for (const node of nodes) map.set(node.node_id, { node, children: [], depth: 0 });
  for (const node of nodes) {
    const treeNode = map.get(node.node_id)!;
    if (node.parent_id && map.has(node.parent_id)) {
      const parent = map.get(node.parent_id)!;
      treeNode.depth = parent.depth + 1;
      parent.children.push(treeNode);
    } else {
      roots.push(treeNode);
    }
  }
  return roots;
}

function flattenTree(roots: TreeNode[], expanded: Set<string>): TreeNode[] {
  const result: TreeNode[] = [];
  const walk = (nodes: TreeNode[]) => {
    for (const n of nodes) {
      result.push(n);
      if (expanded.has(n.node.node_id) && n.children.length > 0) walk(n.children);
    }
  };
  walk(roots);
  return result;
}

interface StructureExplorerProps {
  data: ProjectStructureView;
  highlightModuleId?: string | null;
}

export function StructureExplorer({ data, highlightModuleId }: StructureExplorerProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  // 当 highlightModuleId 变化时，自动展开并选中该模块的第一个文件
  useEffect(() => {
    if (highlightModuleId && data.nodes.length > 0) {
      const moduleFile = data.nodes.find((n) => n.module_id === highlightModuleId && n.kind !== "module" && n.kind !== "package");
      const target = moduleFile ?? data.nodes.find((n) => n.node_id === highlightModuleId);
      if (target) {
        setSelectedNodeId(target.node_id);
        // 展开所有父节点
        const parents = new Set<string>();
        let current = target.parent_id;
        const byId = new Map(data.nodes.map((n) => [n.node_id, n]));
        while (current && byId.has(current)) {
          parents.add(current);
          current = byId.get(current)!.parent_id;
        }
        setExpanded(parents);
      }
    }
  }, [highlightModuleId, data.nodes]);

  const treeRoots = useMemo(() => buildTree(data.nodes), [data.nodes]);
  const flatNodes = useMemo(() => (search ? null : flattenTree(treeRoots, expanded)), [treeRoots, expanded, search]);

  const searchResults = useMemo(() => {
    if (!search) return null;
    const lowerQuery = search.toLowerCase();
    const byId = new Map(data.nodes.map((n) => [n.node_id, n]));
    const result: { node: ProjectStructureView["nodes"][number]; parents: string[] }[] = [];
    for (const node of data.nodes) {
      if (node.name.toLowerCase().includes(lowerQuery) || node.path.display.toLowerCase().includes(lowerQuery)) {
        const parents: string[] = [];
        let current = node.parent_id;
        while (current && byId.has(current)) {
          const parent = byId.get(current)!;
          parents.unshift(parent.name);
          current = parent.parent_id;
        }
        result.push({ node, parents });
      }
    }
    return result;
  }, [data.nodes, search]);

  const virtualizer = useVirtualizer({
    count: flatNodes?.length ?? 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 30,
    overscan: 10,
  });

  const toggleExpand = (nodeId: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) next.delete(nodeId);
      else next.add(nodeId);
      return next;
    });
  };

  const selectedNode = data.nodes.find((n) => n.node_id === selectedNodeId) ?? null;

  return (
    <div className="space-y-4 pb-8">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      <div className="flex items-center gap-3">
        <span className="text-xs text-muted-foreground">
          {t("v3.structure.summary", { nodes: data.nodes.length, edges: data.edges.length })}
          {data.page.truncated && t("v3.structure.truncated")}
        </span>
        <span className="text-xs text-muted-foreground">{t("v3.structure.index", { state: t(`v3.structure.indexState.${data.index_state}`, { defaultValue: data.index_state }) })}</span>
        {data.unsupported_analyzers.length > 0 && (
          <span className="text-xs text-orange-600">{t("v3.structure.unsupported", { value: data.unsupported_analyzers.join(t("v3.common.listSeparator")) })}</span>
        )}
        <div className="relative ml-auto">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder={t("v3.structure.searchNodes")}
            className="h-8 w-56 rounded-md border border-input bg-transparent pl-8 pr-3 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
          />
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem]">
        <div ref={scrollRef} className="h-[60vh] overflow-auto border border-border/70">
          {search ? (
            <div className="divide-y divide-border/50">
              {searchResults!.length === 0 ? (
                <div className="px-4 py-8 text-center text-sm text-muted-foreground">{t("v3.structure.noMatches")}</div>
              ) : (
                searchResults!.map(({ node, parents }) => {
                  const Icon = kindIcons[node.kind] ?? File;
                  return (
                    <button
                      key={node.node_id}
                      type="button"
                      onClick={() => setSelectedNodeId(node.node_id)}
                      className={cn("flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-muted/20", selectedNodeId === node.node_id && "bg-muted/30")}
                    >
                      <Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate">{node.name}</span>
                      {parents.length > 0 && (
                        <span className="ml-auto truncate text-[11px] text-muted-foreground/60">
                          {"< " + parents.join(" < ")}
                        </span>
                      )}
                    </button>
                  );
                })
              )}
            </div>
          ) : (
            <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
              {virtualizer.getVirtualItems().map((virtualItem) => {
                const treeNode = flatNodes![virtualItem.index];
                const node = treeNode.node;
                const Icon = kindIcons[node.kind] ?? File;
                const hasChildren = treeNode.children.length > 0;
                const isExpanded = expanded.has(node.node_id);
                return (
                  <div
                    key={virtualItem.key}
                    data-index={virtualItem.index}
                    ref={virtualizer.measureElement}
                    style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <button
                      type="button"
                      onClick={() => (hasChildren ? toggleExpand(node.node_id) : setSelectedNodeId(node.node_id))}
                      onDoubleClick={() => setSelectedNodeId(node.node_id)}
                      className={cn("flex w-full items-center gap-1.5 px-2 py-1 text-left text-sm hover:bg-muted/20", selectedNodeId === node.node_id && "bg-muted/30")}
                      style={{ paddingLeft: `${treeNode.depth * 18 + 8}px` }}
                    >
                      {hasChildren ? (
                        isExpanded ? <ChevronDown className="h-3 w-3 shrink-0" /> : <ChevronRight className="h-3 w-3 shrink-0" />
                      ) : (
                        <span className="w-3 shrink-0" />
                      )}
                      <Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                      <span className="truncate">{node.name}</span>
                      <span className={cn("ml-auto text-[10px]", gitStateTone[node.git_state])}>
                        {node.git_state !== "clean" && node.git_state !== "unknown" ? t(`v3.structure.gitState.${node.git_state}`, { defaultValue: node.git_state }) : ""}
                      </span>
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {selectedNode && (
          <aside className="border border-border/70 p-4">
            <div className="mb-2 text-sm font-medium">{selectedNode.name}</div>
            <div className="space-y-1.5 text-xs">
              <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.structure.type")}</span><span>{t(`v3.structure.kind.${selectedNode.kind}`, { defaultValue: selectedNode.kind })}</span></div>
              <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.structure.gitStatus")}</span><span className={gitStateTone[selectedNode.git_state]}>{t(`v3.structure.gitState.${selectedNode.git_state}`, { defaultValue: selectedNode.git_state })}</span></div>
              {selectedNode.module_id && <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.structure.module")}</span><span>{selectedNode.module_id}</span></div>}
              {selectedNode.ide_target && <div className="flex justify-between"><span className="text-muted-foreground">{t("v3.structure.ideTarget")}</span><span>{selectedNode.ide_target}</span></div>}
              <div className="pt-1">
                <span className="text-muted-foreground">{t("v3.structure.path")} </span>
                <NativePathDisplay path={selectedNode.path} showPlatform />
              </div>
              <EvidenceLink evidenceRefs={selectedNode.evidence_refs} />
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}
