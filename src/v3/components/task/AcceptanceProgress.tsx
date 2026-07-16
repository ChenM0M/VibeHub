import { cn } from "@/lib/utils";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";

interface AcceptanceProgressProps {
  criteria: CriterionSummary[];
  className?: string;
}

export function AcceptanceProgress({ criteria, className }: AcceptanceProgressProps) {
  const total = criteria.length;
  const passed = criteria.filter((c) => c.status === "passed").length;
  const failed = criteria.filter((c) => c.status === "failed").length;
  const blocked = criteria.filter((c) => c.status === "blocked").length;
  const ratio = total > 0 ? (passed / total) * 100 : 0;

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex items-baseline justify-between gap-3">
        <div>
          <div className="text-sm font-semibold">验收门槛</div>
          <div className="mt-0.5 text-xs text-muted-foreground">这些是完成条件，不是实现步骤。</div>
        </div>
        <span className="shrink-0 text-[10px] text-muted-foreground">Acceptance criteria</span>
      </div>
      <div className="space-y-1.5">
        <div className="flex items-center justify-between text-sm">
          <span className="font-medium">{passed}/{total} 已通过</span>
          {(failed > 0 || blocked > 0) && (
            <span className="text-xs">
              {failed > 0 && <span className="text-red-600">{failed} 失败 </span>}
              {blocked > 0 && <span className="text-orange-600">{blocked} 阻塞</span>}
            </span>
          )}
        </div>
        <div className="h-1.5 bg-muted rounded-full overflow-hidden">
          <div className="h-full bg-emerald-500 dark:bg-emerald-500/80 transition-all" style={{ width: `${ratio}%` }} />
        </div>
      </div>

      {total === 0 ? (
        <div className="border border-dashed border-border px-6 py-8 text-center text-sm text-muted-foreground">未定义验收标准</div>
      ) : (
        <div className="divide-y divide-border/30 border-t border-border/30 mt-2">
          {criteria.map((criterion) => (
            <div key={criterion.criterion_id} className="py-2.5">
              <div className="flex items-center gap-2">
                <CriterionBadge criterion={criterion} />
                <span className="min-w-0 flex-1 text-sm font-medium">{criterion.title}</span>
              </div>
              <EvidenceLink evidenceRefs={criterion.evidence_refs} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
