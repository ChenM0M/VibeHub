import { cn } from "@/lib/utils";
import { useTranslation } from "react-i18next";

type Freshness = "fresh" | "stale" | "rebuilding" | "unavailable";
type Completeness = "complete" | "partial" | "unsupported" | "unknown";

const freshnessTone: Record<Freshness, string> = {
  fresh: "text-emerald-600 dark:text-emerald-400",
  stale: "text-amber-600 dark:text-amber-400",
  rebuilding: "text-blue-600 dark:text-blue-400",
  unavailable: "text-muted-foreground",
};

const completenessTone: Record<Completeness, string> = {
  complete: "text-emerald-600 dark:text-emerald-400",
  partial: "text-amber-600 dark:text-amber-400",
  unsupported: "text-orange-600 dark:text-orange-400",
  unknown: "text-muted-foreground",
};

interface StateBadgeProps {
  freshness?: Freshness;
  completeness?: Completeness;
  className?: string;
}

export function StateBadge({ freshness, completeness, className }: StateBadgeProps) {
  const { t } = useTranslation();
  return (
    <span className={cn("inline-flex items-center gap-2 text-xs", className)}>
      {freshness && (
        <span className={cn("border border-border/70 px-1.5 py-0.5", freshnessTone[freshness])}>
          {t(`v3.common.freshness.${freshness}`)}
        </span>
      )}
      {completeness && (
        <span className={cn("border border-border/70 px-1.5 py-0.5", completenessTone[completeness])}>
          {t(`v3.common.completeness.${completeness}`)}
        </span>
      )}
    </span>
  );
}
