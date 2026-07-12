import type { CriterionSummary } from "@/v3/contracts/generated/project-overview-view";

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
}

export function CriterionBadge({ criterion, className }: CriterionBadgeProps) {
  return (
    <span className={`inline-flex items-center gap-0.5 ${className ?? ""}`} title={`${criterion.title}${criterion.required ? "（必需）" : ""}`}>
      <span className={`h-2 w-2 rounded-full ${statusColor[criterion.status]}`} />
    </span>
  );
}
