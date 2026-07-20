import { useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { GitGraph, CheckSquare, ChevronRight } from "lucide-react";
import type { V3View } from "@/v3/stores/v3Store";
import type { TaskTimelineView } from "@/v3/contracts/generated/task-timeline-view";
import { useTranslation } from "react-i18next";

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

const laneStateTone: Record<string, string> = {
  open: "border-blue-500/30 bg-blue-500/5",
  active: "border-emerald-500/30 bg-emerald-500/5",
  idle: "border-border/70",
  closed: "border-border/40 bg-transparent",
  gapped: "border-orange-500/30 bg-orange-500/5",
  abandoned: "border-red-500/20 bg-red-500/5",
  repaired: "border-teal-500/30 bg-teal-500/5",
};

const taskStateTone: Record<string, string> = {
  planned: "text-muted-foreground",
  active: "text-blue-600 dark:text-blue-400",
  blocked: "text-orange-600 dark:text-orange-400",
  review: "text-purple-600 dark:text-purple-400",
  completed: "text-emerald-600 dark:text-emerald-400",
  cancelled: "text-red-600 dark:text-red-400",
};

interface TaskTimelineProps {
  data: TaskTimelineView;
  onDrillIn: (view: V3View, label: string) => void;
}

export function TaskTimeline({ data, onDrillIn }: TaskTimelineProps) {
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const { t, i18n } = useTranslation();
  const formatDateTime = (value: string | number) => new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));

  const { timeRange, positionedEvents } = useMemo(() => {
    if (data.events.length === 0) return { timeRange: null, positionedEvents: [] };
    const times = data.events.map((e) => new Date(e.occurred_at).getTime());
    const min = Math.min(...times);
    const max = Math.max(...times);
    const range = max - min || 1;
    return {
      timeRange: { min, max },
      positionedEvents: data.events.map((event) => ({
        event,
        pct: ((new Date(event.occurred_at).getTime() - min) / range) * 100,
      })),
    };
  }, [data.events]);

  const lanesById = useMemo(() => {
    const map = new Map<string, TaskTimelineView["lanes"][number]>();
    for (const lane of data.lanes) map.set(lane.lane_id, lane);
    return map;
  }, [data.lanes]);

  const selectedEvent = data.events.find((e) => e.timeline_event_id === selectedEventId) ?? null;

  return (
    <div className="space-y-6 pb-8">
      <div className="flex items-center gap-3">
        <h3 className="text-lg font-semibold">{data.title}</h3>
        <span className={cn("text-xs font-medium", taskStateTone[data.state])}>
          {t(`v3.timeline.taskState.${data.state}`, { defaultValue: data.state })}
        </span>
        <span className="font-mono text-[11px] text-muted-foreground">{data.task_id}</span>
      </div>

      {timeRange && (
        <div className="text-xs text-muted-foreground">
          {formatDateTime(timeRange.min)} → {formatDateTime(timeRange.max)}
        </div>
      )}

      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />
      <BlockerDetailsPanel blockers={data.blocker_details} />

      {/* Lanes */}
      <div className="space-y-2">
        {data.lanes.length === 0 ? (
          <div className="border border-dashed border-border px-6 py-8 text-center text-sm text-muted-foreground">{t("v3.timeline.noLanes")}</div>
        ) : (
          data.lanes.map((lane) => {
            const laneEvents = positionedEvents.filter((pe) => pe.event.lane_id === lane.lane_id);
            return (
              <div key={lane.lane_id} className={cn("border p-2", laneStateTone[lane.state])}>
                <div className="mb-1 flex items-center gap-2">
                  <span className="text-sm font-medium">{lane.label}</span>
                  <span className="text-[10px] text-muted-foreground">{t(`v3.timeline.laneState.${lane.state}`, { defaultValue: lane.state })}</span>
                  <span className="ml-auto text-[10px] text-muted-foreground">{t("v3.timeline.eventCount", { count: laneEvents.length })}</span>
                </div>
                <div className="relative h-7 bg-muted/30">
                  {laneEvents.map(({ event, pct }) => (
                    <button
                      key={event.timeline_event_id}
                      type="button"
                      onClick={() => setSelectedEventId(event.timeline_event_id)}
                      title={event.summary_key}
                      className={cn(
                        "absolute top-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full transition-transform hover:scale-150",
                        eventKindDot[event.kind] ?? "bg-muted-foreground",
                        selectedEventId === event.timeline_event_id && "scale-150 ring-2 ring-foreground",
                      )}
                      style={{ left: `${pct}%` }}
                    />
                  ))}
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Selected event details */}
      {selectedEvent && (() => {
        const lane = lanesById.get(selectedEvent.lane_id);
        return (
          <div className="border border-border/70 p-3">
            <div className="flex items-center gap-2">
              <span className={cn("h-2 w-2 rounded-full", eventKindDot[selectedEvent.kind] ?? "bg-muted-foreground")} />
              <span className="text-sm font-medium">{t(selectedEvent.summary_key, { defaultValue: selectedEvent.summary_key })}</span>
              <span className="text-[10px] text-muted-foreground">
                {t(`v3.timeline.eventKind.${selectedEvent.kind}`, { defaultValue: selectedEvent.kind })}
              </span>
            </div>
            <div className="mt-2 space-y-0.5 text-xs text-muted-foreground">
              <div>{t("v3.timeline.time", { value: formatDateTime(selectedEvent.occurred_at) })}</div>
              <div>{t("v3.timeline.actor", { actor: selectedEvent.actor, tool: selectedEvent.tool ? t("v3.timeline.viaTool", { tool: selectedEvent.tool }) : "" })}</div>
              {lane && <div>{t("v3.timeline.lane", { value: lane.label })}</div>}
              {selectedEvent.node_id && <div>{t("v3.timeline.node", { value: selectedEvent.node_id })}</div>}
              {selectedEvent.session_id && <div>{t("v3.timeline.session", { value: selectedEvent.session_id })}</div>}
              {selectedEvent.commit_sha && <div className="font-mono">{t("v3.timeline.commit", { value: selectedEvent.commit_sha })}</div>}
              {selectedEvent.order_state !== "ordered" && (
                <div className="text-orange-600">{t("v3.timeline.order", { value: t(`v3.timeline.orderState.${selectedEvent.order_state}`, { defaultValue: selectedEvent.order_state }) })}</div>
              )}
            </div>
            <EvidenceLink evidenceRefs={selectedEvent.evidence_refs} />
          </div>
        );
      })()}

      {/* Drill-in actions */}
      <div className="grid gap-4 lg:grid-cols-2">
        <button
          type="button"
          onClick={() => onDrillIn("plan-graph", t("v3.timeline.plan.title"))}
          className="group flex items-center gap-3 border border-border/70 p-4 text-left transition-colors hover:border-foreground/30 hover:bg-muted/20"
        >
          <GitGraph className="h-5 w-5 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="text-sm font-semibold">{t("v3.timeline.plan.title")}</div>
            <div className="mt-0.5 text-xs text-muted-foreground">{t("v3.timeline.plan.description")}</div>
          </div>
          <ChevronRight className="h-4 w-4 text-muted-foreground/50 transition group-hover:text-foreground" />
        </button>

        <button
          type="button"
          onClick={() => onDrillIn("acceptance-progress", t("v3.timeline.acceptance.title"))}
          className="group flex items-center gap-3 border border-border/70 p-4 text-left transition-colors hover:border-foreground/30 hover:bg-muted/20"
        >
          <CheckSquare className="h-5 w-5 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="text-sm font-semibold">{t("v3.timeline.acceptance.title")}</div>
            <div className="mt-0.5 text-xs text-muted-foreground">{t("v3.timeline.acceptance.description")}</div>
          </div>
          <ChevronRight className="h-4 w-4 text-muted-foreground/50 transition group-hover:text-foreground" />
        </button>
      </div>
    </div>
  );
}
