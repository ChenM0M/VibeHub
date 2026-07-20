import { useState } from "react";
import { ChevronRight, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import type { EvidenceRef } from "@/v3/contracts/generated/project-overview-view";
import { useTranslation } from "react-i18next";

const gradeTone: Record<EvidenceRef["grade"], string> = {
  hard_observed: "text-emerald-600 dark:text-emerald-400",
  agent_reported: "text-blue-600 dark:text-blue-400",
  inferred: "text-amber-600 dark:text-amber-400",
  user_confirmed: "text-purple-600 dark:text-purple-400",
};

const kindIcon: Record<EvidenceRef["kind"], string> = {
  event: "E",
  file: "F",
  git: "G",
  command: "C",
  test: "T",
  user: "U",
  external: "X",
};

interface EvidenceLinkProps {
  evidenceRefs: EvidenceRef[];
  className?: string;
}

export function EvidenceLink({ evidenceRefs, className }: EvidenceLinkProps) {
  const [expanded, setExpanded] = useState(false);
  const { t, i18n } = useTranslation();
  if (!evidenceRefs || evidenceRefs.length === 0) return null;

  return (
    <div className={cn("text-xs", className)}>
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1 text-muted-foreground hover:text-foreground transition-colors"
      >
        {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        <span>{t("v3.common.evidenceCount", { count: evidenceRefs.length })}</span>
      </button>
      {expanded && (
        <ul className="mt-1 ml-4 space-y-1">
          {evidenceRefs.map((ref) => (
            <li key={ref.evidence_id} className="flex items-start gap-1.5">
              <span className="inline-flex h-4 w-4 shrink-0 items-center justify-center border border-border text-[10px] font-mono">
                {kindIcon[ref.kind]}
              </span>
              <span className={cn("font-medium", gradeTone[ref.grade])}>{t(`v3.common.evidenceGrade.${ref.grade}`)}</span>
              <span className="text-muted-foreground">{t(ref.label_key, { defaultValue: ref.label_key })}</span>
              <span className="truncate font-mono text-[10px] text-muted-foreground/70">{ref.locator}</span>
              {ref.captured_at && (
                <span className="text-muted-foreground/50">{new Intl.DateTimeFormat(i18n.resolvedLanguage).format(new Date(ref.captured_at))}</span>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
