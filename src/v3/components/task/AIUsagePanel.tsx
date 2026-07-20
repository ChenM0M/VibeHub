import { Coins, ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { LocalAgentUsageOverview } from "@/types";
import { formatTokenCount } from "@/v3/usageFormatting";

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

export function AIUsagePanel({ scope, taskId, usage, loading, error, onRefresh }: AIUsagePanelProps) {
  const [expanded, setExpanded] = useState(false);
  const { t } = useTranslation();
  const sources = usage ? [usage.claude_code, usage.claude_app, usage.codex, usage.opencode, usage.cursor] : [];
  const tools = sources.map((source) => ({
    id: source.source,
    name: sourceNames[source.source] ?? source.source,
    tokens: source.total_tokens,
    sessions: source.records,
    color: sourceColors[source.source] ?? "#64748b",
    status: source.status,
  }));
  const sessions = sources.flatMap((source) => source.recent.map((session) => ({ ...session, source: source.source })));
  const totalTokens = usage?.total_tokens ?? 0;
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
  const tokenDisplay = !hasLoadedUsage
    ? "—"
    : taskAttributionState === "no-sessions"
      ? t("v3.usage.none")
      : taskAttributionState === "unlinked"
        ? t("v3.usage.unlinked")
        : taskAttributionState === "unavailable"
          ? t("v3.usage.unavailable")
          : formatTokenCount(totalTokens);
  const sessionDisplay = !usage
    ? "—"
    : scope === "task"
      ? `${usage.matched_session_count}/${usage.requested_session_count}`
      : String(totalSessions);
  let cumulativePct = 0;
  const pieSegments = tools.filter((tool) => tool.tokens > 0).map((tool) => {
    const pct = totalTokens ? (tool.tokens / totalTokens) * 100 : 0;
    const segment = { ...tool, startPct: cumulativePct, endPct: cumulativePct + pct };
    cumulativePct += pct;
    return segment;
  });
  const status = error ? "error" : loading && !usage ? "loading" : usage?.freshness === "stale" ? "stale" : usage?.completeness === "partial" || usage?.warnings.length ? "partial" : totalSessions === 0 ? "empty" : "available";
  const statusLabel = taskAttributionState === "no-sessions"
    ? t("v3.usage.noTaskSessions")
    : taskAttributionState === "unlinked"
      ? t("v3.usage.tokenUnlinked")
      : taskAttributionState === "unavailable"
        ? t("v3.usage.tokenUnavailable")
        : taskAttributionState === "partial"
          ? t("v3.usage.partialSessions")
          : t(`v3.usage.status.${status}`);

  return (
    <div className="bg-card rounded-md shadow-sm p-5 border border-border/30">
      <div className="mb-3 flex items-center gap-2">
        <Coins className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-semibold">{scope === "task" ? t("v3.usage.taskTitle") : t("v3.usage.projectTitle")}</span>
        <span className="ml-auto text-[10px] text-muted-foreground/60">{statusLabel}</span>
      </div>
      {taskAttributionState === "no-sessions" && <div className="mb-3 border border-border/50 bg-muted/30 p-2 text-xs text-muted-foreground">{t("v3.usage.noSessionsHint")}</div>}
      {taskAttributionState === "unlinked" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.unlinkedHint", { count: usage?.requested_session_count ?? 0 })}</div>}
      {taskAttributionState === "unavailable" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.unavailableHint")}</div>}
      {taskAttributionState === "partial" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.partialHint", { matched: usage?.matched_session_count ?? 0, requested: usage?.requested_session_count ?? 0 })}</div>}
      {usage?.freshness === "stale" && <div className="mb-3 border border-amber-500/30 bg-amber-500/5 p-2 text-xs text-amber-700 dark:text-amber-400">{t("v3.usage.staleHint")}<button type="button" className="ml-2 underline" onClick={onRefresh}>{t("v3.common.refresh")}</button></div>}
      {error && <div className="mb-3 border border-destructive/30 bg-destructive/5 p-2 text-xs text-destructive">{error}<button type="button" className="ml-2 underline" onClick={onRefresh}>{t("v3.common.retry")}</button></div>}
      {loading && <div className="mb-3 text-xs text-muted-foreground">{t("v3.usage.refreshing")}</div>}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <div className="border border-border/50 p-2.5 text-center"><div className="text-lg font-semibold">{tokenDisplay}</div><div className="text-[10px] text-muted-foreground">{scope === "task" ? t("v3.usage.taskToken") : t("v3.usage.totalToken")}</div></div>
        <div className="border border-border/50 p-2.5 text-center"><div className="text-lg font-semibold">{t("v3.usage.unavailable")}</div><div className="text-[10px] text-muted-foreground">{t("v3.usage.estimatedCost")}</div></div>
        <div className="border border-border/50 p-2.5 text-center"><div className="text-lg font-semibold">{sessionDisplay}</div><div className="text-[10px] text-muted-foreground">{scope === "task" ? t("v3.usage.linkedSessions") : t("v3.usage.sessionCount")}</div></div>
      </div>
      <div className="flex gap-4 mb-4">
        <div className="relative h-24 w-24 shrink-0">
          <div className="absolute inset-0 rounded-full dark:opacity-80" style={{ background: pieSegments.length ? `conic-gradient(${pieSegments.map((segment) => `${segment.color} ${segment.startPct * 3.6}deg ${segment.endPct * 3.6}deg`).join(", ")})` : "var(--muted)" }} />
          <div className="absolute inset-3 rounded-full bg-background flex items-center justify-center"><span className="text-[10px] font-semibold">{tokenDisplay}</span></div>
        </div>
        <div className="flex-1 space-y-1.5">
          {tools.map((tool) => { const pct = totalTokens ? (tool.tokens / totalTokens) * 100 : 0; return <div key={tool.id} className="flex items-center gap-2"><span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: tool.color }} /><span className="w-20 shrink-0 text-xs">{tool.name}</span><div className="flex-1 h-3 bg-muted relative overflow-hidden rounded-full"><div className="h-full transition-all dark:opacity-85" style={{ width: `${pct}%`, backgroundColor: tool.color }} /></div><span className="w-12 shrink-0 text-right text-[10px] font-mono text-muted-foreground">{tool.tokens ? formatTokenCount(tool.tokens) : tool.status}</span></div>; })}
        </div>
      </div>
      <div className="mb-3"><div className="mb-1.5 text-[10px] font-semibold text-muted-foreground">{t("v3.usage.byPhase")}</div><div className="flex h-4 items-center justify-center rounded-full bg-muted text-[9px] text-muted-foreground">{t("v3.usage.noPhaseMapping")}</div></div>
      <button type="button" onClick={() => setExpanded(!expanded)} className="flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">{expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}{t("v3.usage.sessionDetails", { count: sessions.length })}</button>
      {expanded && <div className="mt-2 space-y-1.5 max-h-48 overflow-y-auto scrollbar-auto-hide pr-1">{sessions.length === 0 ? <div className="p-2 text-xs text-muted-foreground">{scope === "task" && taskAttributionState ? t("v3.usage.noLinkedRecords") : t("v3.usage.noChatRecords")}</div> : sessions.map((session) => <div key={`${session.source}:${session.id}`} className="flex items-center gap-2 border border-border/50 p-2 text-[11px]"><span className="max-w-32 truncate font-mono text-[10px] text-muted-foreground/60" title={session.title}>{session.id}</span><span className="shrink-0">{sourceNames[session.source] ?? session.source}</span><span className="shrink-0 text-muted-foreground">{session.models.join(" / ") || t("v3.usage.unknownModel")}</span><span className="shrink-0 text-muted-foreground">{session.duration_seconds != null ? t("v3.usage.minutes", { count: Math.round(session.duration_seconds / 60) }) : t("v3.usage.unknownDuration")}</span><span className="ml-auto font-mono text-muted-foreground">{formatTokenCount(session.total_tokens)}</span><span className={session.status === "partial" ? "text-amber-600" : "text-emerald-600"}>{session.status === "partial" ? t("v3.usage.partial") : t("v3.usage.recorded")}</span></div>)}</div>}
      {usage?.warnings.length ? <div className="mt-2 max-h-20 overflow-auto border-t border-border/30 pt-2 text-[10px] text-amber-600 dark:text-amber-400">{usage.warnings.slice(0, 4).map((warning, index) => <div key={index}>{warning}</div>)}</div> : null}
      <div className="mt-2 border-t border-border/30 pt-2 text-[10px] text-muted-foreground/50">{scope === "task" ? t("v3.usage.taskFootnote", { taskId: taskId ?? t("v3.usage.notSelected") }) : t("v3.usage.projectFootnote")}</div>
    </div>
  );
}
