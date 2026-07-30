import { Coins, ChevronDown, ChevronRight, AlertTriangle, Clock, Database, Info, Shield } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LocalAgentUsageOverview, UsageNotice } from "@/types";
import { formatTokenCount, formatTokenCountExact, formatCost, formatCostExact } from "@/v3/usageFormatting";

interface AIUsagePanelProps {
  scope: "project" | "task";
  taskId: string | null;
  usage: LocalAgentUsageOverview | null;
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
}

const sourceColors: Record<string, string> = { claude_code: "#c084fc", claude_app: "#a78bfa", codex: "#60a5fa", opencode: "#4ade80", cursor: "#f59e0b" };
const sourceNames: Record<string, string> = { claude_code: "Claude Code", claude_app: "Claude App", codex: "Codex", opencode: "OpenCode", cursor: "Cursor" };

/**
 * Collapses identical notices (same code and message) into one counted line and
 * orders them by weight, so an expected transcript shape that occurs thousands
 * of times never floods the panel.
 */
function aggregateNotices(notices: UsageNotice[]): UsageNotice[] {
  const merged = new Map<string, UsageNotice>();
  for (const notice of notices) {
    const key = `${notice.code}::${notice.message}`;
    const existing = merged.get(key);
    merged.set(key, existing ? { ...existing, count: existing.count + notice.count } : { ...notice });
  }
  return [...merged.values()].sort((a, b) => b.count - a.count);
}

export function AIUsagePanel({ scope, taskId, usage, loading, error, onRefresh }: AIUsagePanelProps) {
  const [expanded, setExpanded] = useState(false);
  const [showExact, setShowExact] = useState(false);
  const [showBreakdown, setShowBreakdown] = useState(false);
  const { t } = useTranslation();
  const sources = usage ? [usage.claude_code, usage.claude_app, usage.codex, usage.opencode, usage.cursor] : [];
  const tools = sources.map((source) => ({
    id: source.source,
    name: sourceNames[source.source] ?? source.source,
    tokens: source.non_cached_total_tokens,
    cachedTokens: source.total_tokens,
    sessions: source.records,
    color: sourceColors[source.source] ?? "#64748b",
    status: source.status,
    cost: source.cost,
    freshness: source.freshness,
  }));
  const sessions = sources.flatMap((source) => source.recent.map((session) => ({ ...session, source: source.source })));
  // `total_tokens` includes cache reads/writes, which are billed at a fraction
  // of the base input price and dominate long-lived projects; it stays as a
  // secondary detail while the headline uses the non-cached primary metric.
  const totalTokens = usage?.total_tokens ?? 0;
  const primaryTokens = usage?.primary_metric.tokens ?? usage?.non_cached_total_tokens ?? 0;
  const totalSessions = tools.reduce((sum, tool) => sum + tool.sessions, 0);
  const hasLoadedUsage = usage !== null;
  const taskAttributionState = scope !== "task" || !usage
    ? null
    : usage.requested_session_count === 0
      ? "no-sessions"
      : usage.matched_session_count === 0
        ? "unlinked"
        : usage.primary_metric.tokens == null
          ? "unavailable"
          : usage.matched_session_count < usage.requested_session_count
            ? "partial"
            : "available";

  const formatTokens = (value: number) => showExact ? formatTokenCountExact(value) : formatTokenCount(value);
  const tokenDisplay = !hasLoadedUsage
    ? "—"
    : taskAttributionState === "no-sessions"
      ? t("v3.usage.none")
      : taskAttributionState === "unlinked"
        ? t("v3.usage.unlinked")
        : taskAttributionState === "unavailable"
          ? t("v3.usage.unavailable")
          : formatTokens(primaryTokens);

  // Cost display with state awareness
  const costSummary = usage?.audit?.cost;
  const costDisplay = !hasLoadedUsage
    ? "—"
    : costSummary?.state === "complete" && costSummary.known_cost != null
      ? formatCost(costSummary.known_cost, costSummary.currency)
      : costSummary?.state === "partial" && costSummary.known_cost != null
        ? `${formatCost(costSummary.known_cost, costSummary.currency)} (${t("v3.usage.partial")})`
        : t("v3.usage.unavailable");
  const costTitle = costSummary
    ? `${t("v3.usage.costState")}: ${costSummary.state} | ${t("v3.usage.pricedTokens")}: ${formatTokenCountExact(costSummary.priced_tokens)} | ${t("v3.usage.unpricedTokens")}: ${formatTokenCountExact(costSummary.unpriced_tokens)} | ${t("v3.usage.pricingVersion")}: ${costSummary.pricing_version}`
    : undefined;

  const sessionDisplay = !usage
    ? "—"
    : scope === "task"
      ? `${usage.matched_session_count}/${usage.requested_session_count}`
      : String(totalSessions);

  let cumulativePct = 0;
  const pieSegments = tools.filter((tool) => tool.tokens > 0).map((tool) => {
    const pct = primaryTokens ? (tool.tokens / primaryTokens) * 100 : 0;
    const segment = { ...tool, startPct: cumulativePct, endPct: cumulativePct + pct };
    cumulativePct += pct;
    return segment;
  });

  // Unsupported sources and expected transcript shapes arrive as notices, so
  // only real defects (`warnings`) may degrade the panel status.
  const warnings = usage?.warnings ?? [];
  const notices = aggregateNotices(usage?.notices ?? []);
  const status = error ? "error" : loading && !usage ? "loading" : usage?.freshness === "stale" ? "stale" : usage?.completeness === "partial" || warnings.length ? "partial" : totalSessions === 0 ? "empty" : "available";
  const statusLabel = taskAttributionState === "no-sessions"
    ? t("v3.usage.noTaskSessions")
    : taskAttributionState === "unlinked"
      ? t("v3.usage.tokenUnlinked")
      : taskAttributionState === "unavailable"
        ? t("v3.usage.tokenUnavailable")
        : taskAttributionState === "partial"
          ? t("v3.usage.partialSessions")
          : t(`v3.usage.status.${status}`);

  // Anomaly detection
  const anomaly = usage?.audit?.anomaly;
  const hasAnomaly = anomaly != null;

  // Excluded / ambiguous records
  const excludedRecords = usage?.audit?.excluded_records ?? 0;
  const ambiguousRecords = usage?.audit?.ambiguous_records ?? 0;
  const hasExcluded = excludedRecords > 0 || ambiguousRecords > 0;

  // Last refresh time
  const lastRefreshTime = usage?.generated_at ? new Date(usage.generated_at).toLocaleString() : null;
  // Stale means "these are the newest real records and they are old", so the panel
  // states the observed-through time and offers a refresh instead of alarming.
  const observedThrough = usage?.observed_through_ms != null ? new Date(usage.observed_through_ms) : null;
  const observedThroughAgeMinutes = observedThrough ? Math.max(0, Math.round((Date.now() - observedThrough.getTime()) / 60000)) : null;

  return (
    <div className="bg-card rounded-md shadow-sm p-5 border border-border/30">
      <div className="mb-3 flex items-center gap-2">
        <Coins className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-semibold">{scope === "task" ? t("v3.usage.taskTitle") : t("v3.usage.projectTitle")}</span>
        <span className="ml-auto text-[10px] text-muted-foreground/60">{statusLabel}</span>
      </div>

      {/* Status hints */}
      {taskAttributionState === "no-sessions" && <div className="mb-3 border border-border/50 bg-muted/30 p-2 text-xs text-muted-foreground">{t("v3.usage.noSessionsHint")}</div>}
      {taskAttributionState === "unlinked" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.unlinkedHint", { count: usage?.requested_session_count ?? 0 })}</div>}
      {taskAttributionState === "unavailable" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.unavailableHint")}</div>}
      {taskAttributionState === "partial" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.partialHint", { matched: usage?.matched_session_count ?? 0, requested: usage?.requested_session_count ?? 0 })}</div>}
      {usage?.freshness === "stale" && (
        <div className="mb-3 flex items-center gap-1.5 border border-border/50 bg-muted/30 p-2 text-xs text-muted-foreground">
          <Clock className="h-3 w-3" />
          <span>{observedThrough ? t("v3.usage.observedThrough", { time: observedThrough.toLocaleString(), minutes: observedThroughAgeMinutes ?? 0 }) : t("v3.usage.staleHint")}</span>
          <button type="button" className="ml-auto underline" onClick={onRefresh}>{t("v3.common.refresh")}</button>
        </div>
      )}
      {error && <div className="mb-3 border border-destructive/30 bg-destructive/5 p-2 text-xs text-destructive">{error}<button type="button" className="ml-2 underline" onClick={onRefresh}>{t("v3.common.retry")}</button></div>}
      {loading && <div className="mb-3 text-xs text-muted-foreground">{t("v3.usage.refreshing")}</div>}

      {/* Anomaly warning */}
      {hasAnomaly && (
        <div className="mb-3 border border-red-500/30 bg-red-500/5 p-2 text-xs text-red-700 dark:text-red-400">
          <div className="flex items-center gap-1.5 font-semibold"><AlertTriangle className="h-3 w-3" />{t("v3.usage.anomalyDetected")}</div>
          <div className="mt-1">{anomaly.explanation}</div>
          <div className="mt-1 text-[10px] opacity-80">{t("v3.usage.repairAction")}: {anomaly.repair_action}</div>
        </div>
      )}

      {/* Excluded / ambiguous records */}
      {hasExcluded && (
        <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">
          <div className="flex items-center gap-1.5"><Shield className="h-3 w-3" />{t("v3.usage.excludedRecords", { excluded: excludedRecords, ambiguous: ambiguousRecords })}</div>
        </div>
      )}

      {/* Main metrics */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <button type="button" onClick={() => setShowExact(!showExact)} className="border border-border/50 p-2.5 text-center hover:bg-muted/50 transition-colors" title={`${showExact ? t("v3.usage.showScaled") : t("v3.usage.showExact")} · ${t("v3.usage.cachedTotalHint", { total: formatTokenCountExact(totalTokens) })}`}>
          <div className="text-lg font-semibold">{tokenDisplay}</div>
          <div className="text-[10px] text-muted-foreground">{scope === "task" ? t("v3.usage.taskToken") : t("v3.usage.totalToken")}</div>
        </button>
        <div className="border border-border/50 p-2.5 text-center" title={costTitle}>
          <div className="text-lg font-semibold">{costDisplay}</div>
          <div className="text-[10px] text-muted-foreground">{t("v3.usage.estimatedCost")}</div>
        </div>
        <div className="border border-border/50 p-2.5 text-center">
          <div className="text-lg font-semibold">{sessionDisplay}</div>
          <div className="text-[10px] text-muted-foreground">{scope === "task" ? t("v3.usage.linkedSessions") : t("v3.usage.sessionCount")}</div>
        </div>
      </div>

      {/* Provider breakdown */}
      <div className="flex gap-4 mb-4">
        <div className="relative h-24 w-24 shrink-0">
          <div className="absolute inset-0 rounded-full dark:opacity-80" style={{ background: pieSegments.length ? `conic-gradient(${pieSegments.map((segment) => `${segment.color} ${segment.startPct * 3.6}deg ${segment.endPct * 3.6}deg`).join(", ")})` : "var(--muted)" }} />
          <div className="absolute inset-3 rounded-full bg-background flex items-center justify-center"><span className="text-[10px] font-semibold">{tokenDisplay}</span></div>
        </div>
        <div className="flex-1 space-y-1.5">
          {tools.map((tool) => { const pct = primaryTokens ? (tool.tokens / primaryTokens) * 100 : 0; return <div key={tool.id} className="flex items-center gap-2"><span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: tool.color }} /><span className="w-20 shrink-0 text-xs">{tool.name}</span><div className="flex-1 h-3 bg-muted relative overflow-hidden rounded-full"><div className="h-full transition-all dark:opacity-85" style={{ width: `${pct}%`, backgroundColor: tool.color }} /></div><span className="w-12 shrink-0 text-right text-[10px] font-mono text-muted-foreground" title={[tool.cost != null ? formatCostExact(tool.cost) : null, tool.cachedTokens ? t("v3.usage.cachedTotalHint", { total: formatTokenCountExact(tool.cachedTokens) }) : null].filter(Boolean).join(" · ") || undefined}>{tool.tokens ? formatTokens(tool.tokens) : tool.status}</span></div>; })}
        </div>
      </div>

      {/* Refresh info */}
      {usage?.refresh && (
        <div className="mb-3 flex items-center gap-2 text-[10px] text-muted-foreground/70">
          <Clock className="h-3 w-3" />
          <span>{t("v3.usage.lastRefresh")}: {lastRefreshTime ?? t("v3.usage.unknown")}</span>
          <span className="mx-1">·</span>
          <span>{t("v3.usage.refreshDuration")}: {usage.refresh.duration_ms}ms</span>
          <span className="mx-1">·</span>
          <span className={usage.refresh.within_budget ? "text-emerald-600" : "text-amber-600"}>{usage.refresh.within_budget ? t("v3.usage.withinBudget") : t("v3.usage.overBudget")}</span>
          {usage.freshness && <><span className="mx-1">·</span><span className={usage.freshness === "fresh" ? "text-emerald-600" : usage.freshness === "stale" ? "text-amber-600" : "text-muted-foreground"}>{t(`v3.usage.freshness.${usage.freshness}`)}</span></>}
        </div>
      )}

      {/* Cost missing reasons */}
      {costSummary && costSummary.missing_reasons.length > 0 && (
        <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">
          <div className="font-semibold">{t("v3.usage.costMissingReasons")}</div>
          <ul className="mt-1 list-disc pl-4">{costSummary.missing_reasons.map((reason, i) => <li key={i}>{reason}</li>)}</ul>
          {costSummary.repair_actions.length > 0 && <div className="mt-1 text-[10px] opacity-80">{t("v3.usage.repairAction")}: {costSummary.repair_actions[0]}</div>}
        </div>
      )}

      {/* Audit breakdown toggle */}
      {usage?.audit && usage.audit.breakdowns.length > 0 && (
        <div className="mb-3">
          <button type="button" onClick={() => setShowBreakdown(!showBreakdown)} className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
            {showBreakdown ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
            {t("v3.usage.auditBreakdown", { count: usage.audit.breakdowns.length })}
          </button>
          {showBreakdown && (
            <div className="mt-2 space-y-1 max-h-40 overflow-y-auto scrollbar-auto-hide pr-1">
              {usage.audit.breakdowns.map((item, i) => (
                <div key={`${item.kind}-${item.id}-${i}`} className="flex items-center gap-2 border border-border/50 p-2 text-[11px]">
                  <span className="w-16 shrink-0 text-[10px] font-mono text-muted-foreground">{item.kind}</span>
                  <span className="max-w-24 truncate font-mono text-[10px] text-muted-foreground/60" title={item.id}>{item.id}</span>
                  <span className="shrink-0">{item.provider}</span>
                  {item.model && <span className="shrink-0 text-muted-foreground">{item.model}</span>}
                  <span className="ml-auto font-mono text-muted-foreground">{item.total_tokens != null ? formatTokens(item.total_tokens) : "—"}</span>
                  {item.cost != null && <span className="font-mono text-muted-foreground">{formatCost(item.cost)}</span>}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Session details */}
      <button type="button" onClick={() => setExpanded(!expanded)} className="flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
        {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        {t("v3.usage.sessionDetails", { count: sessions.length })}
      </button>
      {expanded && (
        <div className="mt-2 space-y-1.5 max-h-48 overflow-y-auto scrollbar-auto-hide pr-1">
          {sessions.length === 0 ? (
            <div className="p-2 text-xs text-muted-foreground">{scope === "task" && taskAttributionState ? t("v3.usage.noLinkedRecords") : t("v3.usage.noChatRecords")}</div>
          ) : sessions.map((session) => (
            <div key={`${session.source}:${session.id}`} className="flex items-center gap-2 border border-border/50 p-2 text-[11px]">
              <span className="max-w-32 truncate font-mono text-[10px] text-muted-foreground/60" title={session.title}>{session.id}</span>
              <span className="shrink-0">{sourceNames[session.source] ?? session.source}</span>
              <span className="shrink-0 text-muted-foreground">{session.models.join(" / ") || t("v3.usage.unknownModel")}</span>
              <span className="shrink-0 text-muted-foreground">{session.duration_seconds != null ? t("v3.usage.minutes", { count: Math.round(session.duration_seconds / 60) }) : t("v3.usage.unknownDuration")}</span>
              <span className="ml-auto font-mono text-muted-foreground" title={session.cost != null ? formatCostExact(session.cost) : undefined}>{formatTokens(session.total_tokens)}</span>
              {session.cost != null && <span className="font-mono text-muted-foreground">{formatCost(session.cost)}</span>}
              <span className={session.status === "partial" ? "text-amber-600" : "text-emerald-600"}>{session.status === "partial" ? t("v3.usage.partial") : t("v3.usage.recorded")}</span>
            </div>
          ))}
        </div>
      )}

      {/* Warnings: real defects only */}
      {warnings.length ? (
        <div className="mt-2 max-h-20 overflow-auto border-t border-border/30 pt-2 text-[10px] text-amber-600 dark:text-amber-400">
          {warnings.slice(0, 4).map((warning, index) => <div key={index}>{warning}</div>)}
          {warnings.length > 4 ? <div className="opacity-70">{t("v3.usage.moreWarnings", { count: warnings.length - 4 })}</div> : null}
        </div>
      ) : null}

      {/* Notices: expected observations, aggregated by code with counts */}
      {notices.length ? (
        <div className="mt-2 border-t border-border/30 pt-2 text-[10px] text-muted-foreground/60">
          <div className="flex items-center gap-1"><Info className="h-3 w-3" />{t("v3.usage.notices")}</div>
          {notices.slice(0, 5).map((notice) => (
            <div key={`${notice.code}::${notice.message}`} className="mt-0.5">{notice.message}</div>
          ))}
          {notices.length > 5 ? <div className="mt-0.5 opacity-70">{t("v3.usage.moreNotices", { count: notices.length - 5 })}</div> : null}
        </div>
      ) : null}

      {/* Evidence provenance */}
      {usage?.audit?.evidence_provenance && usage.audit.evidence_provenance.length > 0 && (
        <div className="mt-2 border-t border-border/30 pt-2 text-[10px] text-muted-foreground/50">
          <div className="flex items-center gap-1"><Database className="h-3 w-3" />{t("v3.usage.evidenceProvenance")}</div>
          {usage.audit.evidence_provenance.map((prov, i) => (
            <div key={i} className="mt-0.5">{prov.provider} · {prov.source_kind} · {prov.records} {t("v3.usage.records")} · {prov.freshness}</div>
          ))}
        </div>
      )}

      <div className="mt-2 border-t border-border/30 pt-2 text-[10px] text-muted-foreground/50">
        {scope === "task" ? t("v3.usage.taskFootnote", { taskId: taskId ?? t("v3.usage.notSelected") }) : t("v3.usage.projectFootnote")}
      </div>
    </div>
  );
}
