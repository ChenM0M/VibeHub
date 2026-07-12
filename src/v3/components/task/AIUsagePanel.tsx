import { Coins, ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";

interface AIUsagePanelProps { taskId: string | null; }

const mockTools = [
  { id: "codex", name: "Codex", tokens: 45200, cost: 0.68, sessions: 3, color: "#60a5fa" },
  { id: "claude", name: "Claude Code", tokens: 28100, cost: 0.42, sessions: 2, color: "#c084fc" },
  { id: "opencode", name: "OpenCode", tokens: 12400, cost: 0.19, sessions: 1, color: "#4ade80" },
  { id: "cursor", name: "Cursor", tokens: 5300, cost: 0.08, sessions: 1, color: "#fb923c" },
];

const mockSessions = [
  { id: "session.main", tool: "Codex", tokens: 38200, phase: "实施", duration: "45 分钟", status: "已关闭" },
  { id: "session.review", tool: "Claude Code", tokens: 28100, phase: "审查", duration: "30 分钟", status: "已关闭" },
  { id: "session.ui", tool: "Codex", tokens: 7000, phase: "实施", duration: "15 分钟", status: "活跃" },
  { id: "session.types", tool: "OpenCode", tokens: 12400, phase: "计划", duration: "20 分钟", status: "空闲" },
  { id: "session.refactor", tool: "Cursor", tokens: 5300, phase: "实施", duration: "10 分钟", status: "已关闭" },
];

export function AIUsagePanel({ taskId }: AIUsagePanelProps) {
  const [expanded, setExpanded] = useState(false);
  const totalTokens = mockTools.reduce((s, t) => s + t.tokens, 0);
  const totalCost = mockTools.reduce((s, t) => s + t.cost, 0);
  const totalSessions = mockTools.reduce((s, t) => s + t.sessions, 0);

  // CSS 饼图
  let cumulativePct = 0;
  const pieSegments = mockTools.map((t) => {
    const pct = (t.tokens / totalTokens) * 100;
    const seg = { ...t, startPct: cumulativePct, endPct: cumulativePct + pct };
    cumulativePct += pct;
    return seg;
  });

  // 按阶段汇总
  const phaseBreakdown = mockSessions.reduce((acc, s) => {
    acc[s.phase] = (acc[s.phase] ?? 0) + s.tokens;
    return acc;
  }, {} as Record<string, number>);

  return (
    <div className="bg-card rounded-md shadow-sm p-5 border border-border/30">
      <div className="mb-3 flex items-center gap-2">
        <Coins className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-semibold">AI 工具用量</span>
        <span className="ml-auto text-[10px] text-muted-foreground/60">示例数据</span>
      </div>

      {/* 总览 */}
      <div className="grid grid-cols-3 gap-2 mb-4">
        <div className="border border-border/50 p-2.5 text-center">
          <div className="text-lg font-semibold">{(totalTokens / 1000).toFixed(1)}k</div>
          <div className="text-[10px] text-muted-foreground">总 Token</div>
        </div>
        <div className="border border-border/50 p-2.5 text-center">
          <div className="text-lg font-semibold">${totalCost.toFixed(2)}</div>
          <div className="text-[10px] text-muted-foreground">预估成本</div>
        </div>
        <div className="border border-border/50 p-2.5 text-center">
          <div className="text-lg font-semibold">{totalSessions}</div>
          <div className="text-[10px] text-muted-foreground">会话数</div>
        </div>
      </div>

      {/* 饼图 + 工具用量条 */}
      <div className="flex gap-4 mb-4">
        <div className="relative h-24 w-24 shrink-0">
          <div className="absolute inset-0 rounded-full dark:opacity-80" style={{ background: `conic-gradient(${pieSegments.map((s) => `${s.color} ${s.startPct * 3.6}deg ${s.endPct * 3.6}deg`).join(", ")})` }} />
          <div className="absolute inset-3 rounded-full bg-background flex items-center justify-center">
            <span className="text-[10px] font-semibold">{(totalTokens / 1000).toFixed(0)}k</span>
          </div>
        </div>
        <div className="flex-1 space-y-1.5">
          {mockTools.map((tool) => {
            const pct = (tool.tokens / totalTokens) * 100;
            return (
              <div key={tool.id} className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: tool.color }} />
                <span className="w-16 shrink-0 text-xs">{tool.name}</span>
                <div className="flex-1 h-3 bg-muted relative overflow-hidden rounded-full">
                  <div className="h-full transition-all dark:opacity-85" style={{ width: `${pct}%`, backgroundColor: tool.color }} />
                </div>
                <span className="w-12 shrink-0 text-right text-[10px] font-mono text-muted-foreground">{(tool.tokens / 1000).toFixed(1)}k</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* 按阶段分布 */}
      <div className="mb-3">
        <div className="mb-1.5 text-[10px] font-semibold text-muted-foreground">按阶段分布</div>
        <div className="flex gap-1 h-4 rounded-full overflow-hidden">
          {Object.entries(phaseBreakdown).map(([phase, tokens]) => {
            const pct = (tokens / totalTokens) * 100;
            const colors: Record<string, string> = { 对齐: "#6366f1", 调研: "#06b6d4", 计划: "#f59e0b", 实施: "#10b981", 审查: "#a855f7" };
            return <div key={phase} className="flex items-center justify-center text-[9px] text-white dark:text-white/90 dark:opacity-85" style={{ width: `${pct}%`, backgroundColor: colors[phase] ?? "#64748b" }}>{phase}</div>;
          })}
        </div>
      </div>

      {/* 会话明细（可展开） */}
      <button type="button" onClick={() => setExpanded(!expanded)} className="flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
        {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        会话明细（{mockSessions.length}）
      </button>
      {expanded && (
        <div className="mt-2 space-y-1.5 max-h-48 overflow-y-auto scrollbar-auto-hide pr-1">
          {mockSessions.map((s) => (
            <div key={s.id} className="flex items-center gap-2 border border-border/50 p-2 text-[11px]">
              <span className="font-mono text-[10px] text-muted-foreground/60">{s.id}</span>
              <span className="shrink-0">{s.tool}</span>
              <span className="shrink-0 text-muted-foreground">{s.phase}</span>
              <span className="shrink-0 text-muted-foreground">{s.duration}</span>
              <span className="ml-auto font-mono text-muted-foreground">{(s.tokens / 1000).toFixed(1)}k</span>
              <span className={s.status === "活跃" ? "text-blue-600" : s.status === "空闲" ? "text-muted-foreground" : "text-emerald-600"}>{s.status}</span>
            </div>
          ))}
        </div>
      )}

      <div className="mt-2 border-t border-border/30 pt-2 text-[10px] text-muted-foreground/50">
        {taskId ? `任务 ${taskId} · ` : ""}示例数据，M2 将接入真实后端
      </div>
    </div>
  );
}
