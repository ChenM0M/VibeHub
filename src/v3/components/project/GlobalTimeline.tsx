import { useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import type { TaskTimelineView } from "@/v3/contracts/generated/task-timeline-view";
import { useTranslation } from "react-i18next";

const eventKindTone: Record<string, string> = {
  decision: "text-purple-600 dark:text-purple-400",
  evidence: "text-blue-600 dark:text-blue-400",
  finding: "text-cyan-600 dark:text-cyan-400",
  attempt: "text-amber-600 dark:text-amber-400",
  validation: "text-emerald-600 dark:text-emerald-400",
  confirmation: "text-teal-600 dark:text-teal-400",
  gap: "text-orange-600 dark:text-orange-400",
  session: "text-indigo-600 dark:text-indigo-400",
  plan: "text-pink-600 dark:text-pink-400",
};

const eventKindDot: Record<string, string> = {
  decision: "bg-purple-500",
  evidence: "bg-blue-500",
  finding: "bg-cyan-500",
  attempt: "bg-amber-500",
  validation: "bg-emerald-500",
  confirmation: "bg-teal-500",
  gap: "bg-orange-500",
  session: "bg-indigo-500",
  plan: "bg-pink-500",
};

const allKinds = ["decision", "evidence", "finding", "attempt", "validation", "confirmation", "gap", "session", "plan"];

interface GlobalTimelineProps {
  data: TaskTimelineView;
}

export function GlobalTimeline({ data }: GlobalTimelineProps) {
  const { t, i18n } = useTranslation();
  const [filterKind, setFilterKind] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const sortedEvents = useMemo(() => {
    let events = [...data.events].sort((a, b) => a.occurred_at.localeCompare(b.occurred_at));
    if (filterKind) events = events.filter((e) => e.kind === filterKind);
    return events;
  }, [data.events, filterKind]);

  const timeRange = useMemo(() => {
    if (data.events.length === 0) return null;
    const times = data.events.map((e) => e.occurred_at).sort();
    return { start: times[0], end: times[times.length - 1] };
  }, [data.events]);

  const virtualizer = useVirtualizer({
    count: sortedEvents.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 52,
    overscan: 8,
  });

  return (
    <div className="space-y-4 pb-8">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      <div className="flex items-center gap-3 flex-wrap text-xs text-muted-foreground">
        <span>{t("v3.timeline.eventCount", { count: data.events.length })}</span>
        {timeRange && (
          <span>
            {new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "medium" }).format(new Date(timeRange.start))} → {new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "medium" }).format(new Date(timeRange.end))}
          </span>
        )}
        <span className="italic">{t("v3.globalTimeline.currentTaskOnly")}</span>
      </div>

      <div className="flex items-center gap-1 flex-wrap">
        <button
          type="button"
          onClick={() => setFilterKind(null)}
          className={cn(
            "border px-2.5 py-0.5 text-xs font-medium transition-colors",
            filterKind === null ? "border-foreground bg-foreground/5 text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
          )}
        >
          {t("v3.globalTimeline.all")}
        </button>
        {allKinds.map((kind) => (
          <button
            key={kind}
            type="button"
            onClick={() => setFilterKind(filterKind === kind ? null : kind)}
            className={cn(
              "border px-2.5 py-0.5 text-xs font-medium transition-colors",
              filterKind === kind ? "border-foreground bg-foreground/5 text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
            )}
          >
            {t(`v3.timeline.eventKind.${kind}`, { defaultValue: kind })}
          </button>
        ))}
      </div>

      <div ref={scrollRef} className="h-[55vh] overflow-auto border border-border/70">
        {sortedEvents.length === 0 ? (
          <div className="px-4 py-8 text-center text-sm text-muted-foreground">{t("v3.globalTimeline.noEvents")}</div>
        ) : (
          <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const event = sortedEvents[virtualItem.index];
              return (
                <div
                  key={virtualItem.key}
                  data-index={virtualItem.index}
                  ref={virtualizer.measureElement}
                  style={{ position: "absolute", top: 0, left: 0, width: "100%", transform: `translateY(${virtualItem.start}px)` }}
                  className="border-b border-border/50 px-3 py-2"
                >
                  <div className="flex items-start gap-3">
                    <span className={cn("mt-1 h-2 w-2 shrink-0 rounded-full", eventKindDot[event.kind])} />
                    <div className="min-w-0 flex-1">
                      <div className={cn("text-sm font-medium", eventKindTone[event.kind])}>{event.summary_key}</div>
                      <div className="mt-0.5 flex items-center gap-2 text-[11px] text-muted-foreground">
                        <span>{new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "medium" }).format(new Date(event.occurred_at))}</span>
                        <span>{t("v3.globalTimeline.actor", { actor: event.actor })}</span>
                        {event.tool && <span>{t("v3.globalTimeline.via", { tool: event.tool })}</span>}
                        {event.commit_sha && <span className="font-mono">{event.commit_sha.slice(0, 7)}</span>}
                        {event.order_state !== "ordered" && <span className="text-orange-600">{t("v3.globalTimeline.late")}</span>}
                      </div>
                      <EvidenceLink evidenceRefs={event.evidence_refs} />
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
