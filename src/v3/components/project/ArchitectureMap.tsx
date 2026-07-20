import { useMemo, useState } from "react";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { Box, Folder, Package, File } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ProjectStructureView } from "@/v3/contracts/generated/project-structure-view";
import { useTranslation } from "react-i18next";

const sourceKindColor: Record<string, string> = {
  filesystem: "#9ca3af", git: "#22c55e", manifest: "#3b82f6",
  parser: "#a855f7", documentation: "#f97316", inference: "#eab308",
};
const kindIcon: Record<string, React.ComponentType<{ className?: string }>> = {
  root: Box, directory: Folder, file: File, module: Package, package: Package, symbol: File,
};

interface ArchitectureMapProps {
  data: ProjectStructureView;
  onModuleClick?: (moduleId: string) => void;
}

export function ArchitectureMap({ data, onModuleClick }: ArchitectureMapProps) {
  const { t } = useTranslation();
  const edgeLabel = (kind: string) => t(`v3.structure.edgeKind.${kind}`, { defaultValue: kind });
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null);
  const [hoveredModule, setHoveredModule] = useState<string | null>(null);

  const { modules, moduleFiles, edges } = useMemo(() => {
    const moduleNodes = data.nodes.filter((n) => n.kind === "module" || n.kind === "package" || n.kind === "root");
    const moduleIds = new Set(moduleNodes.map((n) => n.node_id));
    const fileMap = new Map<string, typeof data.nodes>();
    for (const node of data.nodes) {
      if (node.module_id && moduleIds.has(node.module_id) || moduleIds.has(node.node_id)) {
        const key = node.module_id ?? node.node_id;
        if (!fileMap.has(key)) fileMap.set(key, []);
        fileMap.get(key)!.push(node);
      }
    }
    const relevantEdges = data.edges.filter((e) => moduleIds.has(e.from_node_id) && moduleIds.has(e.to_node_id));
    return { modules: moduleNodes, moduleFiles: fileMap, edges: relevantEdges };
  }, [data.nodes, data.edges]);

  const selectedEdge = data.edges.find((e) => e.edge_id === selectedEdgeId) ?? null;

  if (modules.length === 0) {
    return (
      <div className="h-full">
        <WarningList warnings={data.warnings} />
        <ErrorList errors={data.errors} />
        <div className="border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
          {data.nodes.length === 0 ? t("v3.structure.noData") : t("v3.structure.noModules")}
        </div>
      </div>
    );
  }

  return (
    <div className="relative h-full overflow-hidden">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      {/* 画板：模块卡片网格 */}
      <div className="relative h-full overflow-auto p-4">
        <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${Math.min(modules.length, 3)}, 1fr)` }}>
          {modules.map((mod) => {
            const files = moduleFiles.get(mod.module_id ?? mod.node_id) ?? [];
            const incoming = edges.filter((e) => e.to_node_id === mod.node_id);
            const outgoing = edges.filter((e) => e.from_node_id === mod.node_id);
            const isHovered = hoveredModule === mod.node_id;
            return (
              <div
                key={mod.node_id}
                className={cn("border p-3 transition-colors", onModuleClick ? "cursor-pointer hover:border-foreground/30 hover:bg-muted/10" : "", isHovered ? "border-foreground/40 bg-muted/10" : "border-border/70")}
                onClick={() => onModuleClick?.(mod.module_id ?? mod.node_id)}
                onMouseEnter={() => setHoveredModule(mod.node_id)}
                onMouseLeave={() => setHoveredModule(null)}
              >
                <div className="flex items-center gap-2 mb-2">
                  <Package className="h-4 w-4 text-muted-foreground" />
                  <span className="text-sm font-medium">{mod.name}</span>
                  <span className="ml-auto text-[10px] text-muted-foreground">{t("v3.structure.fileCount", { count: files.length })}</span>
                </div>
                {/* 文件列表预览 */}
                <div className="space-y-0.5">
                  {files.slice(0, 5).map((f) => {
                    const Icon = kindIcon[f.kind] ?? File;
                    return (
                      <div key={f.node_id} className="flex items-center gap-1.5 text-[11px] text-muted-foreground truncate">
                        <Icon className="h-3 w-3 shrink-0" />
                        <span className="truncate">{f.name}</span>
                      </div>
                    );
                  })}
                  {files.length > 5 && <div className="text-[10px] text-muted-foreground/60 pl-4.5">{t("v3.structure.moreCount", { count: files.length - 5 })}</div>}
                </div>
                {/* 关联边 */}
                {(incoming.length > 0 || outgoing.length > 0) && (
                  <div className="mt-2 border-t border-border/50 pt-1.5 flex flex-wrap gap-1">
                    {incoming.map((e) => (
                      <button key={e.edge_id} type="button" onClick={(ev) => { ev.stopPropagation(); setSelectedEdgeId(e.edge_id); }} className="border border-border/50 px-1 py-0.5 text-[9px] hover:border-foreground/30" title={`${edgeLabel(e.kind)} ← ${data.nodes.find((n) => n.node_id === e.from_node_id)?.name ?? e.from_node_id}`}>
                        <span style={{ color: sourceKindColor[e.source_kind] }}>●</span> {edgeLabel(e.kind)} ← {data.nodes.find((n) => n.node_id === e.from_node_id)?.name ?? "?"}
                      </button>
                    ))}
                    {outgoing.map((e) => (
                      <button key={e.edge_id} type="button" onClick={(ev) => { ev.stopPropagation(); setSelectedEdgeId(e.edge_id); }} className="border border-border/50 px-1 py-0.5 text-[9px] hover:border-foreground/30" title={`${edgeLabel(e.kind)} → ${data.nodes.find((n) => n.node_id === e.to_node_id)?.name ?? e.to_node_id}`}>
                        <span style={{ color: sourceKindColor[e.source_kind] }}>●</span> {edgeLabel(e.kind)} → {data.nodes.find((n) => n.node_id === e.to_node_id)?.name ?? "?"}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {/* 选中边详情 */}
        {selectedEdge && (
          <div className="mt-4 border border-border/70 p-3">
            <div className="mb-2 text-xs font-semibold">{t("v3.structure.selectedEdge")}</div>
            <div className="grid grid-cols-2 gap-y-1 text-xs">
              <span className="text-muted-foreground">{t("v3.structure.type")}</span><span>{edgeLabel(selectedEdge.kind)}</span>
              <span className="text-muted-foreground">{t("v3.structure.source")}</span><span>{t(`v3.structure.sourceKind.${selectedEdge.source_kind}`, { defaultValue: selectedEdge.source_kind })}</span>
              <span className="text-muted-foreground">{t("v3.structure.confidence")}</span><span>{(selectedEdge.confidence * 100).toFixed(0)}%</span>
              <span className="text-muted-foreground">{t("v3.structure.from")}</span><span className="font-mono">{data.nodes.find((n) => n.node_id === selectedEdge.from_node_id)?.name ?? selectedEdge.from_node_id}</span>
              <span className="text-muted-foreground">{t("v3.structure.to")}</span><span className="font-mono">{data.nodes.find((n) => n.node_id === selectedEdge.to_node_id)?.name ?? selectedEdge.to_node_id}</span>
            </div>
            <EvidenceLink evidenceRefs={selectedEdge.evidence_refs} />
          </div>
        )}
      </div>

      {/* 固定图例 — 叠在画板右上角 */}
      <div className="pointer-events-none absolute right-3 top-3">
        <div className="pointer-events-auto border border-border/70 bg-background/95 p-2 backdrop-blur-sm shadow-sm">
          <div className="mb-1 text-[10px] font-semibold text-muted-foreground">{t("v3.structure.sourceTypes")}</div>
          <div className="space-y-0.5">
            {Object.entries(sourceKindColor).map(([kind, color]) => (
              <div key={kind} className="flex items-center gap-1.5 text-[10px]">
                <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: color }} />
                <span>{t(`v3.structure.sourceKind.${kind}`, { defaultValue: kind })}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
