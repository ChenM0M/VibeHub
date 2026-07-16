import { useState } from "react";
import { AlertTriangle, ChevronDown, ChevronRight, CircleHelp, ExternalLink, ShieldAlert } from "lucide-react";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { BlockerDetail } from "@/v3/contracts/generated/project-overview-view";

const kindLabel: Record<BlockerDetail["kind"], string> = {
  external_precondition: "外部前置",
  permission: "权限阻塞",
  evidence_gap: "证据缺口",
  dependency: "依赖未满足",
  conflict: "冲突",
  workflow: "工作流阻塞",
  unknown: "未分类",
};

interface BlockerDetailsPanelProps {
  blockers?: BlockerDetail[];
  compact?: boolean;
}

export function BlockerDetailsPanel({ blockers = [], compact = false }: BlockerDetailsPanelProps) {
  const [expanded, setExpanded] = useState(false);
  if (blockers.length === 0) return null;

  return (
    <section
      className={compact ? "space-y-2" : "rounded-md border border-orange-500/30 bg-orange-500/5"}
      aria-label="阻塞详情"
    >
      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
        className={compact ? "flex w-full min-w-0 items-center gap-2 text-left" : "flex w-full min-w-0 items-center gap-2 p-3 text-left hover:bg-orange-500/5"}
      >
        <ShieldAlert className="h-4 w-4 text-orange-600 dark:text-orange-400" />
        <h3 className="text-sm font-semibold text-orange-800 dark:text-orange-300">阻塞详情</h3>
        <span className="rounded border border-orange-500/30 px-1.5 py-0.5 text-[10px] text-orange-700 dark:text-orange-300">
          {blockers.length} 项
        </span>
        {!expanded && <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{blockers[0].summary}</span>}
        {expanded ? <ChevronDown className="ml-auto h-4 w-4 shrink-0 text-muted-foreground" /> : <ChevronRight className="ml-auto h-4 w-4 shrink-0 text-muted-foreground" />}
      </button>

      {expanded && <div className={compact ? "space-y-2" : "space-y-2 px-3 pb-3"}>
        {blockers.map((blocker) => (
          <article key={blocker.blocker_id} className="rounded border border-orange-500/20 bg-background/70 p-3">
            <div className="flex items-start gap-2">
              <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-orange-600 dark:text-orange-400" />
              <div className="min-w-0 flex-1">
                <div className="text-sm font-medium leading-snug">{blocker.summary}</div>
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                  <span>类型：{kindLabel[blocker.kind] ?? blocker.kind}</span>
                  <span>责任方：{blocker.owner}</span>
                  <span className="font-mono text-[10px]">{blocker.reason_code}</span>
                </div>
              </div>
            </div>

            <div className="mt-2 grid gap-2 text-xs sm:grid-cols-2">
              <div className="border-l-2 border-orange-500/40 pl-2">
                <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">解除条件</div>
                <div className="mt-0.5 leading-relaxed">{blocker.precondition}</div>
              </div>
              <div className="border-l-2 border-blue-500/40 pl-2">
                <div className="text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">恢复动作</div>
                <div className="mt-0.5 leading-relaxed">{blocker.resume_action}</div>
              </div>
            </div>

            {blocker.criterion_id && (
              <div className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground">
                <CircleHelp className="h-3 w-3" />验收项：<span className="font-mono">{blocker.criterion_id}</span>
              </div>
            )}
            {blocker.node_id && (
              <div className="mt-1 flex items-center gap-1 text-[10px] text-muted-foreground">
                <ExternalLink className="h-3 w-3" />计划节点：<span className="font-mono">{blocker.node_id}</span>
              </div>
            )}
            <EvidenceLink evidenceRefs={blocker.evidence_refs} className="mt-2" />
          </article>
        ))}
      </div>}
    </section>
  );
}
