import { cn } from "@/lib/utils";

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

const freshnessLabelMap: Record<Freshness, string> = {
  fresh: "最新",
  stale: "过期",
  rebuilding: "重建中",
  unavailable: "不可用",
};

const completenessLabelMap: Record<Completeness, string> = {
  complete: "完整",
  partial: "部分",
  unsupported: "不支持",
  unknown: "未知",
};

interface StateBadgeProps {
  freshness?: Freshness;
  completeness?: Completeness;
  className?: string;
}

export function StateBadge({ freshness, completeness, className }: StateBadgeProps) {
  return (
    <span className={cn("inline-flex items-center gap-2 text-xs", className)}>
      {freshness && (
        <span className={cn("border border-border/70 px-1.5 py-0.5", freshnessTone[freshness])}>
          {freshnessLabelMap[freshness]}
        </span>
      )}
      {completeness && (
        <span className={cn("border border-border/70 px-1.5 py-0.5", completenessTone[completeness])}>
          {completenessLabelMap[completeness]}
        </span>
      )}
    </span>
  );
}
