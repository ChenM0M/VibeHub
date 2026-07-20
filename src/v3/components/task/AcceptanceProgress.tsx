import { cn } from "@/lib/utils";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";
import { useTranslation } from "react-i18next";

interface AcceptanceProgressProps {
  criteria: CriterionSummary[];
  className?: string;
}

export function AcceptanceProgress({ criteria, className }: AcceptanceProgressProps) {
  const { t } = useTranslation();
  const total = criteria.length;
  const passed = criteria.filter((c) => c.status === "passed").length;
  const failed = criteria.filter((c) => c.status === "failed").length;
  const blocked = criteria.filter((c) => c.status === "blocked").length;
  const ratio = total > 0 ? (passed / total) * 100 : 0;

  return (
    <div className={cn("space-y-3", className)}>
      <div className="flex items-baseline justify-between gap-3">
        <div>
          <div className="text-sm font-semibold">{t("v3.acceptance.title")}</div>
          <div className="mt-0.5 text-xs text-muted-foreground">{t("v3.acceptance.description")}</div>
        </div>
        <span className="shrink-0 text-[10px] text-muted-foreground">{t("v3.acceptance.eyebrow")}</span>
      </div>
      <div className="space-y-1.5">
        <div className="flex items-center justify-between text-sm">
          <span className="font-medium">{t("v3.acceptance.passed", { passed, total })}</span>
          {(failed > 0 || blocked > 0) && (
            <span className="text-xs">
              {failed > 0 && <span className="text-red-600">{t("v3.acceptance.failed", { count: failed })} </span>}
              {blocked > 0 && <span className="text-orange-600">{t("v3.acceptance.blocked", { count: blocked })}</span>}
            </span>
          )}
        </div>
        <div className="h-1.5 bg-muted rounded-full overflow-hidden">
          <div className="h-full bg-emerald-500 dark:bg-emerald-500/80 transition-all" style={{ width: `${ratio}%` }} />
        </div>
      </div>

      {total === 0 ? (
        <div className="border border-dashed border-border px-6 py-8 text-center text-sm text-muted-foreground">{t("v3.acceptance.empty")}</div>
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
