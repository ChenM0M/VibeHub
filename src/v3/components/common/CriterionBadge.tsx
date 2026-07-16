import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";
import { useTranslation } from "react-i18next";

const statusColor: Record<CriterionSummary["status"], string> = {
  proposed: "bg-muted-foreground/40",
  accepted: "bg-blue-500",
  passed: "bg-emerald-500",
  failed: "bg-red-500",
  blocked: "bg-orange-500",
  not_applicable: "bg-muted-foreground/20",
};
interface CriterionBadgeProps {
  criterion: CriterionSummary;
  className?: string;
  compact?: boolean;
}

export function CriterionBadge({ criterion, className, compact = false }: CriterionBadgeProps) {
  const { t } = useTranslation();
  const statusLabel = t(`v3.criteria.status.${criterion.status}`, {
    defaultValue: t("v3.criteria.status.unknown"),
  });
  const requiredSuffix = criterion.required ? t("v3.criteria.requiredSuffix") : "";
  const accessibleLabel = t("v3.criteria.ariaLabel", { status: statusLabel });
  const title = `${criterion.title}${requiredSuffix}\n${statusLabel}`;

  if (compact) {
    return (
      <span
        className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${statusColor[criterion.status]} ${className ?? ""}`}
        role="img"
        aria-label={accessibleLabel}
        title={title}
      />
    );
  }

  return (
    <span className={`inline-flex items-center gap-1 rounded border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground ${className ?? ""}`} title={title} aria-label={accessibleLabel}>
      <span className={`h-2 w-2 rounded-full ${statusColor[criterion.status]}`} />
      <span>{statusLabel}</span>
    </span>
  );
}
