import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { ArrowLeft, RefreshCw, FlaskConical, Circle, GitBranch, Clock, CheckSquare, Network, FolderTree, FileText, X, Coins, Archive, ChevronDown, ChevronUp, Settings, ListTodo, Filter } from "lucide-react";
import { useV3Store } from "@/v3/stores/v3Store";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { AcceptanceProgress } from "@/v3/components/task/AcceptanceProgress";
import { NodeBriefPanel } from "@/v3/components/task/NodeBriefPanel";
import { StructureArchitecture } from "@/v3/components/project/StructureArchitecture";
import { PlanGraph } from "@/v3/components/task/PlanGraph";
import { AIUsagePanel } from "@/v3/components/task/AIUsagePanel";
import { ProjectSetupModal } from "@/v3/components/project/ProjectSetupModal";
import type { V3FixtureScenario } from "@/v3/contracts/fixtureRepository";
import type { NodeBrief } from "@/v3/contracts/generated/node-brief";
import type { V3ProductionLoader } from "@/v3/stores/v3Store";

const scenarioLabels: Record<string, string> = {
  "FX-EMPTY": "空项目", "FX-HAPPY": "正常流程", "FX-NO-DOCS": "无架构文档",
  "FX-PARALLEL": "多会话并行", "FX-REWORK": "返工流程", "FX-STALE": "数据过期",
  "FX-PARTIAL": "部分支持", "FX-ERROR": "错误状态", "FX-WIN-PATHS": "Windows 长路径",
  "FX-MAC-PATHS": "macOS 路径", "FX-LARGE": "大数据量", "FX-COVERAGE-GAP": "协议覆盖缺口",
};
const taskStateTone: Record<string, string> = {
  planned: "text-muted-foreground", active: "text-blue-600 dark:text-blue-400",
  blocked: "text-orange-600 dark:text-orange-400", review: "text-purple-600 dark:text-purple-400",
  completed: "text-emerald-600 dark:text-emerald-400", cancelled: "text-red-600 dark:text-red-400",
};
const riskLabel: Record<string, string> = { none: "无", low: "低", medium: "中", high: "高", critical: "严重" };
const eventKindDot: Record<string, string> = {
  decision: "bg-purple-500", evidence: "bg-blue-500", finding: "bg-cyan-500", attempt: "bg-amber-500",
  validation: "bg-emerald-500", confirmation: "bg-teal-500", gap: "bg-orange-500", session: "bg-indigo-500", plan: "bg-pink-500",
};
const eventKindLabel: Record<string, string> = {
  decision: "决策", evidence: "证据", finding: "发现", attempt: "尝试", validation: "验证", confirmation: "确认", gap: "缺口", session: "会话", plan: "计划",
};
const eventSummaryZh: Record<string, string> = {
  "timeline.session.opened": "会话已开启", "timeline.session.closed": "会话已关闭", "timeline.session.parallel": "并行会话开启", "timeline.session.overlap": "会话范围重叠",
  "timeline.plan.created": "计划已创建", "timeline.decision.schema2020": "决策：采用 JSON Schema 2020-12", "timeline.evidence.contractDrafted": "契约草稿已产出",
  "timeline.attempt.generateFixtures": "尝试生成测试数据", "timeline.attempt.remediation": "修复尝试", "timeline.validation.passed": "验证通过", "timeline.validation.repassed": "验证重新通过",
  "timeline.confirmation.committed": "已提交确认", "timeline.finding.schemaGap": "发现：Schema 缺口", "timeline.finding.scopeOverlap": "发现：范围重叠",
  "timeline.gap.recovered": "协议缺口已恢复", "timeline.large.item": "批量事件项",
};
const freshnessTone: Record<string, string> = {
  fresh: "text-emerald-600 dark:text-emerald-400", stale: "text-amber-600 dark:text-amber-400",
  rebuilding: "text-blue-600 dark:text-blue-400", unavailable: "text-muted-foreground",
};
const freshnessLabel: Record<string, string> = { fresh: "最新", stale: "过期", rebuilding: "重建中", unavailable: "不可用" };

type Tab = "overview" | "timeline" | "plan" | "structure";
const tabs: { id: Tab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: "overview", label: "概要", icon: CheckSquare },
  { id: "timeline", label: "时间线", icon: Clock },
  { id: "plan", label: "计划图", icon: Network },
  { id: "structure", label: "结构架构", icon: FolderTree },
];

// 假数据：归档任务
const mockArchivedTasks = [
  { task_id: "task.m0.old", title: "初版契约草稿", state: "completed", completed_at: "2026-07-08T12:00:00Z", risk_level: "low", sessions: 1, tokens: 8500, goal: "生成 VibeHub 核心协议草案，包括状态模型、事件模型等。", criteria: ["状态模型通过结构验证", "协议文档完成起草"], steps: [{ summary: "计划创建", time: "10:00" }, { summary: "草案编写完毕", time: "11:30" }, { summary: "验证通过，自动归档", time: "12:00" }] },
  { task_id: "task.pre.m0", title: "技术选型调研", state: "completed", completed_at: "2026-07-07T15:00:00Z", risk_level: "low", sessions: 2, tokens: 12300, goal: "调研适合的多智能体协同框架与前端技术栈，输出选型报告。", criteria: ["完成 React Flow 渲染性能测试", "确认状态管理架构(Zustand)"], steps: [{ summary: "调研开始", time: "09:00" }, { summary: "输出核心调研报告", time: "14:00" }, { summary: "架构师确认", time: "15:00" }] },
  { task_id: "task.pre.m1", title: "UI 框架对比", state: "cancelled", completed_at: "2026-07-06T10:00:00Z", risk_level: "medium", sessions: 1, tokens: 4200, goal: "对比 Tailwind 和普通 CSS in JS 的开发效率。", criteria: ["评估开发效率与开发体验", "评估运行时性能损耗"], steps: [{ summary: "计划创建", time: "09:00" }, { summary: "因业务调整主动取消任务", time: "10:00" }] },
];

// 假数据：项目总 Token
const mockProjectTokens = { total: 91000, cost: "1.37", sessions: 7 };

interface V3CockpitProps {
  onBack: () => void;
  debugMode?: boolean;
  projectPath?: string;
  productionLoader?: V3ProductionLoader;
}

function EventDetails({ details }: { details: Record<string, unknown> | undefined }) {
  if (!details) return null;
  const entries = Object.entries(details);
  if (entries.length === 0) return null;
  return (
    <div className="space-y-1 bg-muted/30 p-2 rounded-md border border-border/30 mt-2">
      {entries.map(([key, value]) => (
        <div key={key} className="flex gap-2 text-[11px]">
          <span className="shrink-0 text-muted-foreground">{key.replace(/_/g, ' ')}：</span>
          <span className="break-all">{String(value)}</span>
        </div>
      ))}
    </div>
  );
}

export function V3Cockpit({ onBack, debugMode = false, projectPath, productionLoader }: V3CockpitProps) {
  const { currentScenario, projectPath: activeProjectPath, selectScenario, selectProject, bundle, loading, error, loadCurrentBundle } = useV3Store();
  const [showScenarioDropdown, setShowScenarioDropdown] = useState(false);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [nodeBriefPanel, setNodeBriefPanel] = useState<NodeBrief | null>(null);
  const [highlightModuleId, setHighlightModuleId] = useState<string | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [showTokenPanel, setShowTokenPanel] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [archivedDetail, setArchivedDetail] = useState<typeof mockArchivedTasks[0] | null>(null);
  const [timelineFilter, setTimelineFilter] = useState<string>("all");

  useEffect(() => {
    if (projectPath && productionLoader && activeProjectPath !== projectPath) { selectProject(projectPath, productionLoader); }
    else if (!projectPath && !currentScenario) { selectScenario("FX-HAPPY"); }
    else if (!bundle && !loading) { void loadCurrentBundle(); }
  }, [projectPath, productionLoader, activeProjectPath, currentScenario, bundle, loading, selectScenario, selectProject, loadCurrentBundle]);

  useEffect(() => {
    if (bundle) { setSelectedTaskId(bundle.taskTimeline.task_id); }
  }, [bundle]);

  const overview = bundle?.projectOverview;
  const timeline = bundle?.taskTimeline;
  const planGraph = bundle?.planGraph;
  const nodeBrief = bundle?.nodeBrief;
  const structure = bundle?.projectStructure;
  const selectedTask = overview?.active_tasks.find((t) => t.task_id === selectedTaskId) ?? null;
  const selectedTaskHasDetail =
    selectedTaskId === timeline?.task_id &&
    selectedTaskId === planGraph?.task_id &&
    selectedTaskId === nodeBrief?.task_id;

  const sortedEvents = useMemo(() => {
    if (!timeline) return [];
    const filtered = timelineFilter === "all" ? timeline.events : timeline.events.filter((e) => e.kind === timelineFilter);
    return [...filtered].sort((a, b) => b.occurred_at.localeCompare(a.occurred_at));
  }, [timeline, timelineFilter]);

  if (loading && !bundle) { return <div className="flex h-full items-center justify-center"><div className="text-sm text-muted-foreground">正在加载…</div></div>; }
  if (error) { return <div className="flex h-full flex-col gap-4 p-8"><Button variant="ghost" size="sm" className="w-fit" onClick={onBack}><ArrowLeft className="mr-2 h-4 w-4" />返回项目列表</Button><div className="border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">{error}</div></div>; }
  if (!overview || !timeline || !planGraph || !nodeBrief || !structure) return null;

  const criteria = selectedTask?.criteria ?? timeline.criteria;
  const selectedEvent = timeline.events.find((e) => e.timeline_event_id === selectedEventId) ?? null;
  const eventZh = (key: string) => eventSummaryZh[key] ?? key;

  const renderOverview = () => (
    <div className="grid h-full gap-4 lg:grid-cols-2">
      <div className="flex flex-col gap-4 overflow-auto pr-1">
        <div className="bg-card rounded-md shadow-sm p-5 border border-border/30">
          <AcceptanceProgress criteria={criteria} />
        </div>
        {debugMode && <AIUsagePanel taskId={selectedTask?.task_id ?? null} />}
      </div>
      <div className="bg-card rounded-md shadow-sm p-5 border border-border/30 overflow-auto">
        {selectedTaskHasDetail ? (
          <>
            <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><FileText className="h-4 w-4 text-muted-foreground" />当前节点简报</div>
            <div className="space-y-3">
              <h4 className="text-sm font-semibold">{nodeBrief.goal}</h4>
              <div className="flex flex-wrap gap-1.5">
                <span className="border border-border/70 px-1.5 py-0.5 text-[10px]">{nodeBrief.state}</span>
              </div>
              {nodeBrief.next_intent && <div className="text-xs text-muted-foreground">下一步：{nodeBrief.next_intent}</div>}
              {nodeBrief.accepted_decisions.length > 0 && (
                <div className="text-xs"><span className="text-muted-foreground">已接受决策：</span><ul className="ml-4 list-disc list-inside space-y-0.5 mt-1">{nodeBrief.accepted_decisions.slice(0, 3).map((d, i) => <li key={i}>{d}</li>)}</ul></div>
              )}
              {nodeBrief.research_summary.length > 0 && (
                <div className="text-xs"><span className="text-muted-foreground">研究摘要：</span><ul className="ml-4 list-disc list-inside space-y-0.5 mt-1">{nodeBrief.research_summary.slice(0, 3).map((r, i) => <li key={i}>{r}</li>)}</ul></div>
              )}
              <button type="button" onClick={() => setNodeBriefPanel(nodeBrief)} className="text-xs text-blue-600 hover:underline">查看完整简报 →</button>
            </div>
          </>
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground">该任务详情待 M2 后端接入</div>
        )}
      </div>
    </div>
  );

  const renderTimeline = () => (
    <div className="grid h-full gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
      <div className="overflow-auto bg-card rounded-md shadow-sm p-5 border border-border/30 flex flex-col">
        <div className="mb-4 space-y-3 shrink-0">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <Clock className="h-4 w-4 text-muted-foreground" />事件流
            <span className="text-xs text-muted-foreground">({sortedEvents.length})</span>
          </div>
          
          <div className="flex flex-wrap items-center gap-1.5">
            <Filter className="h-3 w-3 text-muted-foreground/70 mr-0.5 shrink-0" />
            <button
              type="button"
              onClick={() => setTimelineFilter("all")}
              className={cn("px-2.5 py-0.5 text-[11px] font-medium rounded-full border transition-colors", timelineFilter === "all" ? "bg-foreground text-background border-foreground shadow-sm" : "bg-muted/30 text-muted-foreground border-border/60 hover:bg-muted/60 hover:text-foreground")}
            >
              全部
            </button>
            {Object.entries(eventKindLabel).map(([kind, label]) => (
              <button
                key={kind}
                type="button"
                onClick={() => setTimelineFilter(kind)}
                className={cn("flex items-center gap-1.5 px-2.5 py-0.5 text-[11px] font-medium rounded-full border transition-colors", timelineFilter === kind ? "bg-foreground text-background border-foreground shadow-sm" : "bg-muted/30 text-muted-foreground border-border/60 hover:bg-muted/60 hover:text-foreground")}
              >
                <span className={cn("h-1.5 w-1.5 rounded-full shrink-0", timelineFilter === kind ? "bg-background" : eventKindDot[kind])} />
                {label}
              </button>
            ))}
          </div>
        </div>
        
        {sortedEvents.length === 0 ? <div className="py-8 text-center text-sm text-muted-foreground flex-1">无对应事件</div> : (
          <div className="space-y-1 overflow-y-auto flex-1 scrollbar-auto-hide pr-1">
            {sortedEvents.map((event) => (
              <button key={event.timeline_event_id} type="button" onClick={() => setSelectedEventId(event.timeline_event_id)} className={cn("flex w-full items-start gap-2.5 border-b border-border/30 py-2 text-left hover:bg-muted/10", selectedEventId === event.timeline_event_id && "bg-muted/20")}>
                <span className={cn("mt-1.5 h-2 w-2 shrink-0 rounded-full", eventKindDot[event.kind])} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2"><span className="text-xs text-muted-foreground">{eventKindLabel[event.kind] ?? event.kind}</span><span className="truncate text-sm font-medium">{eventZh(event.summary_key)}</span></div>
                  <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground"><span>{new Date(event.occurred_at).toLocaleString()}</span><span>· {event.actor}</span>{event.commit_sha && <span className="font-mono">· {event.commit_sha.slice(0, 7)}</span>}</div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="overflow-auto bg-card rounded-md shadow-sm p-5 border border-border/30">
        {selectedEvent ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2"><span className={cn("h-2 w-2 rounded-full", eventKindDot[selectedEvent.kind])} /><span className="text-base font-medium">{eventZh(selectedEvent.summary_key)}</span></div>
            <div className="space-y-1 text-sm text-muted-foreground">
              <div>时间：{new Date(selectedEvent.occurred_at).toLocaleString()}</div>
              <div>执行者：{selectedEvent.actor}{selectedEvent.tool ? `（通过 ${selectedEvent.tool}）` : ""}</div>
              {selectedEvent.commit_sha && <div className="font-mono">提交：{selectedEvent.commit_sha}</div>}
              {selectedEvent.order_state !== "ordered" && <div className="text-orange-600">顺序异常</div>}
            </div>
            {Object.keys(selectedEvent.details ?? {}).length > 0 && (
              <div><div className="mb-1 text-[10px] text-muted-foreground">详情</div><EventDetails details={selectedEvent.details} /></div>
            )}
          </div>
        ) : <div className="py-8 text-center text-xs text-muted-foreground">点击左侧事件查看详情</div>}
      </div>
    </div>
  );

  const renderTab = () => {
    switch (activeTab) {
      case "overview": return renderOverview();
      case "timeline": return selectedTaskHasDetail ? renderTimeline() : <div className="flex h-full items-center justify-center text-sm text-muted-foreground">该任务时间线待 M2 后端接入</div>;
      case "plan": return selectedTaskHasDetail ? <PlanGraph data={planGraph} onNodeClick={(node) => { if (node.node_id === nodeBrief.node_id) setNodeBriefPanel(nodeBrief); }} /> : <div className="flex h-full items-center justify-center text-sm text-muted-foreground">该任务计划图待 M2 后端接入</div>;
      case "structure": return <StructureArchitecture data={structure} projectPath={activeProjectPath ?? undefined} onModuleClick={setHighlightModuleId} highlightModuleId={highlightModuleId} />;
      default: return null;
    }
  };

  const isUninitialized = overview.warnings.some(w => w.code === "PROJECT_UNINITIALIZED");

  return (
    <>
    <div className="flex h-full flex-col animate-in fade-in slide-in-from-bottom-2 duration-200 ease-out transition-all">
      {/* 顶部栏：返回 + 标题 + 指标 + 操作 */}
      <div className="relative z-40 flex shrink-0 items-center gap-4 border-b border-border/60 px-6 py-3 bg-background">
        <Button variant="ghost" size="sm" onClick={onBack}><ArrowLeft className="mr-2 h-4 w-4" /></Button>
        <div className="flex items-baseline gap-3">
          <h2 className="text-2xl font-semibold tracking-tight">{overview.name}</h2>
          <span className={cn("text-xs", freshnessTone[overview.freshness])}>{freshnessLabel[overview.freshness] ?? overview.freshness}</span>
        </div>
        <div className="ml-auto flex items-center gap-4 text-xs text-muted-foreground">
          {overview.repository.branch && <span className="flex items-center gap-1"><GitBranch className="h-3 w-3" /><span className="font-mono">{overview.repository.branch}</span></span>}
          {debugMode && <span className="border border-border/50 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">调试模式</span>}
          {debugMode && <button type="button" onClick={() => setShowTokenPanel(true)} className="flex items-center gap-1 hover:text-foreground transition-colors"><Coins className="h-3 w-3" />{mockProjectTokens.total.toLocaleString()} Token · ${mockProjectTokens.cost}</button>}
          <Button variant="ghost" size="sm" onClick={() => void loadCurrentBundle()} disabled={loading}><RefreshCw className={cn("mr-1.5 h-3.5 w-3.5", loading && "animate-spin")} />刷新</Button>
          {debugMode && <div className="relative">
            <Button variant="ghost" size="sm" onClick={() => setShowScenarioDropdown(!showScenarioDropdown)}><FlaskConical className="mr-1.5 h-3.5 w-3.5" />{currentScenario ? (scenarioLabels[currentScenario] ?? "自定义场景") : "选择场景"}</Button>
            {showScenarioDropdown && (
              <div className="absolute right-0 top-full z-50 mt-1 max-h-72 overflow-auto rounded-md border border-border bg-popover p-1 shadow-md">
                {(Object.keys(scenarioLabels) as V3FixtureScenario[]).map((s) => (
                  <button key={s} type="button" onClick={() => { selectScenario(s); setShowScenarioDropdown(false); }} className={cn("block w-full px-3 py-1.5 text-left text-sm hover:bg-accent", currentScenario === s && "bg-accent font-medium")}>
                    <span>{scenarioLabels[s]}</span>
                  </button>
                ))}
              </div>
            )}
          </div>}
          {debugMode && <Button variant="ghost" size="sm" onClick={() => setShowSettingsModal(true)}><Settings className="h-3.5 w-3.5" /></Button>}
        </div>
      </div>

      <div className="relative flex-1 flex flex-col min-h-0">
        <div className={cn("flex flex-col flex-1 min-h-0 transition-all", isUninitialized && "opacity-40 pointer-events-none grayscale-[0.5]")}>
          {(overview.warnings.length > 0 || overview.errors.length > 0) && (
            <div className="shrink-0 px-6 pt-2"><WarningList warnings={overview.warnings} /><ErrorList errors={overview.errors} /></div>
          )}

          {/* 主区域 */}
          <div className="flex flex-1 min-h-0 px-6 py-2 gap-4">
        {/* 左列：任务列表 */}
        <div className="flex w-60 shrink-0 flex-col overflow-hidden">
          <div className="flex items-center gap-2 text-sm font-semibold shrink-0 mb-2">
            <ListTodo className="h-4 w-4 text-muted-foreground" />活跃任务<span className="text-xs text-muted-foreground">({overview.active_tasks.length})</span>
          </div>
          <div className="flex-1 overflow-y-auto scrollbar-auto-hide space-y-2 pr-1">
            {overview.active_tasks.length === 0 ? (
              <div className="border border-dashed border-border px-4 py-8 text-center"><Circle className="mx-auto mb-2 h-6 w-6 text-muted-foreground/40" /><div className="text-sm text-muted-foreground">暂无活跃任务</div></div>
            ) : (
              overview.active_tasks.map((task) => {
                const active = selectedTaskId === task.task_id;
                const hasDetail =
                  task.task_id === timeline.task_id &&
                  task.task_id === planGraph.task_id &&
                  task.task_id === nodeBrief.task_id;
                return (
                  <button
                    key={task.task_id}
                    type="button"
                    title={hasDetail ? `查看 ${task.title}` : "该任务的真实详情将在 M2 接入"}
                    onClick={() => setSelectedTaskId(task.task_id)}
                    className={cn(
                      "relative block w-full rounded-md p-3 text-left transition-colors",
                      active ? "text-foreground" : "hover:bg-muted/50",
                      !hasDetail && "text-muted-foreground",
                    )}
                  >
                    {active && (
                      <motion.div
                        layoutId="active-task-background"
                        className="absolute inset-0 rounded-md border border-primary/30 bg-primary/10 shadow-sm"
                        initial={false}
                        transition={{ type: "spring", bounce: 0.15, duration: 0.35 }}
                      />
                    )}
                    <div className="relative z-10">
                      <div className="line-clamp-2 text-sm font-medium leading-snug pr-1">{task.title}</div>
                      <div className="mt-1.5 flex items-center gap-2 text-xs">
                        <span className={taskStateTone[task.state]}>{task.state === "active" ? "实施" : task.state === "planned" ? "计划" : task.state === "blocked" ? "阻塞" : task.state === "review" ? "审查" : task.state === "completed" ? "已完成" : task.state === "cancelled" ? "已取消" : task.state}</span>
                        <div className="ml-auto shrink-0 text-[10px] text-muted-foreground bg-muted/40 px-1.5 py-0.5 rounded border border-border/40">
                          {task.active_sessions} 会话
                        </div>
                      </div>
                      {task.criteria.length > 0 && <div className="mt-2.5 flex flex-wrap gap-1">{task.criteria.map((c) => <CriterionBadge key={c.criterion_id} criterion={c} />)}</div>}
                      {!hasDetail && <div className="mt-2 text-[10px] text-muted-foreground">详情待 M2 后端接入</div>}
                    </div>
                  </button>
                );
              })
            )}
          </div>

          {/* 归档任务 (固定在底部，向上展开) */}
          {debugMode && <div className="shrink-0 border-t border-border/50 pt-2 mt-2 flex flex-col">
            {showArchived && (
              <div className="mb-2 space-y-1.5 max-h-[40vh] overflow-y-auto scrollbar-auto-hide pr-1">
                {mockArchivedTasks.map((at) => (
                  <button key={at.task_id} type="button" onClick={() => setArchivedDetail(at)} className="w-full rounded-md p-2 text-left hover:bg-muted/50 transition-colors">
                    <div className="truncate text-xs font-medium">{at.title}</div>
                    <div className="mt-0.5 flex items-center gap-2 text-[10px] text-muted-foreground">
                      <span className={taskStateTone[at.state]}>{at.state === "completed" ? "已完成" : "已取消"}</span>
                      <span>{new Date(at.completed_at).toLocaleDateString()}</span>
                      <span className="ml-auto">{at.tokens.toLocaleString()} Token</span>
                    </div>
                  </button>
                ))}
              </div>
            )}
            <button type="button" onClick={() => setShowArchived(!showArchived)} className="flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
              {showArchived ? <ChevronDown className="h-3 w-3" /> : <ChevronUp className="h-3 w-3" />}
              <Archive className="h-3.5 w-3.5" />归档任务（{mockArchivedTasks.length}）
            </button>
          </div>}
        </div>

        {/* 右列：标签面板 */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="mb-2 flex shrink-0 items-center gap-1 border-b border-border/60">
            {tabs.map((tab) => {
              const Icon = tab.icon;
              const active = activeTab === tab.id;
              return (
                <button key={tab.id} type="button" onClick={() => setActiveTab(tab.id)} className={cn("relative flex items-center gap-1.5 px-3 py-1.5 text-sm transition-colors", active ? "text-foreground font-medium" : "text-muted-foreground hover:text-foreground")}>
                  <Icon className="h-3.5 w-3.5" />{tab.label}
                  {active && (
                    <motion.div
                      layoutId="active-tab-indicator"
                      className="absolute bottom-[-1px] left-0 right-0 h-[2px] bg-foreground"
                      transition={{ type: "spring", bounce: 0.15, duration: 0.35 }}
                    />
                  )}
                </button>
              );
            })}
          </div>
          {selectedTask && (activeTab === "overview" || activeTab === "timeline" || activeTab === "plan") && (
            <div className="mb-3 flex shrink-0 items-center gap-3 pb-1.5 border-b-0">
              <h3 className="truncate text-lg font-semibold">{selectedTask.title}</h3>
              <span className="text-xs text-muted-foreground border border-border/50 px-1.5 py-0.5 rounded bg-muted/20">{riskLabel[selectedTask.risk_level] ?? selectedTask.risk_level}风险</span>
            </div>
          )}
          <div className="flex-1 overflow-hidden">{renderTab()}</div>
        </div>
      </div>

      {/* 项目总 Token 用量面板 */}
      {debugMode && showTokenPanel && (
        <div className="fixed inset-0 z-50 flex justify-end" onClick={() => setShowTokenPanel(false)}>
          <div className="absolute inset-0 bg-black/20" />
          <div className="relative h-full w-[40rem] max-w-[90vw] overflow-auto border-l border-border bg-background shadow-xl animate-in slide-in-from-right-8 fade-in duration-200 ease-out" onClick={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => setShowTokenPanel(false)} className="absolute right-3 top-3 z-10 rounded p-1 hover:bg-muted"><X className="h-4 w-4" /></button>
            <div className="p-6">
              <h3 className="mb-4 text-lg font-semibold">项目 AI 用量总览</h3>
              <AIUsagePanel taskId={null} />
              <div className="mt-4 border border-border/70 p-4">
                <div className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wide">历史任务 Token</div>
                <div className="space-y-1.5">
                  {mockArchivedTasks.map((at) => (
                    <div key={at.task_id} className="flex items-center gap-2 text-xs">
                      <span className="truncate flex-1">{at.title}</span>
                      <span className={taskStateTone[at.state]}>{at.state === "completed" ? "已完成" : "已取消"}</span>
                      <span className="font-mono text-muted-foreground">{at.tokens.toLocaleString()}</span>
                    </div>
                  ))}
                  <div className="flex items-center gap-2 text-xs border-t border-border/50 pt-1.5 mt-1.5">
                    <span className="font-medium flex-1">合计</span>
                    <span className="font-mono font-semibold">{mockArchivedTasks.reduce((s, t) => s + t.tokens, 0).toLocaleString()}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 归档任务详情 */}
      {debugMode && archivedDetail && (
        <div className="fixed inset-0 z-50 flex justify-end" onClick={() => setArchivedDetail(null)}>
          <div className="absolute inset-0 bg-black/20" />
          <div className="relative h-full w-[40rem] max-w-[90vw] overflow-auto border-l border-border bg-background shadow-xl animate-in slide-in-from-right-8 fade-in duration-200 ease-out" onClick={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => setArchivedDetail(null)} className="absolute right-3 top-3 z-10 rounded p-1 hover:bg-muted"><X className="h-4 w-4" /></button>
            <div className="p-6 space-y-6">
              <div className="pb-4 border-b border-border/60">
                <h3 className="text-xl font-semibold text-foreground tracking-tight">{archivedDetail.title}</h3>
                <div className="mt-2.5 flex items-center gap-3 text-sm">
                  <span className={cn("font-medium", taskStateTone[archivedDetail.state])}>{archivedDetail.state === "completed" ? "已完成" : "已取消"}</span>
                  <span className="text-muted-foreground border border-border/50 px-2 py-0.5 rounded-md bg-muted/20">{riskLabel[archivedDetail.risk_level] ?? archivedDetail.risk_level}风险</span>
                  <span className="text-muted-foreground">完成于 {new Date(archivedDetail.completed_at).toLocaleDateString()}</span>
                  <span className="ml-auto font-mono text-xs text-muted-foreground">ID: {archivedDetail.task_id}</span>
                </div>
              </div>
              
              <div className="space-y-4">
                <div className="bg-card/30 rounded-lg p-4 border border-border/40">
                  <h4 className="text-sm font-semibold text-foreground/90 mb-2 flex items-center gap-2"><CheckSquare className="h-4 w-4 text-muted-foreground" />任务目标</h4>
                  <p className="text-sm text-muted-foreground/90 leading-relaxed">{archivedDetail.goal}</p>
                </div>

                <div className="bg-card/30 rounded-lg p-4 border border-border/40">
                  <h4 className="text-sm font-semibold text-foreground/90 mb-2.5 flex items-center gap-2"><ListTodo className="h-4 w-4 text-muted-foreground" />验收结果</h4>
                  <ul className="space-y-2">
                    {archivedDetail.criteria.map((c, i) => (
                      <li key={i} className="flex items-start gap-2.5 text-sm text-muted-foreground/90">
                        <CheckSquare className={cn("h-4 w-4 shrink-0 mt-0.5", archivedDetail.state === "completed" ? "text-emerald-500" : "text-muted-foreground/40")} />
                        <span>{c}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                <div className="bg-card/30 rounded-lg p-4 border border-border/40">
                  <h4 className="text-sm font-semibold text-foreground/90 mb-3 flex items-center gap-2"><Clock className="h-4 w-4 text-muted-foreground" />执行快照</h4>
                  <div className="space-y-3">
                    {archivedDetail.steps.map((s, i) => (
                      <div key={i} className="flex items-center gap-3 text-sm">
                        <span className="text-xs font-mono text-muted-foreground/70 w-12">{s.time}</span>
                        <div className="h-1.5 w-1.5 rounded-full bg-muted-foreground/40" />
                        <span className="text-muted-foreground/90">{s.summary}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>

              <div className="pt-2">
                <AIUsagePanel taskId={archivedDetail.task_id} />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 节点简报侧滑面板 — 加宽到 40rem */}
      {nodeBriefPanel && (
        <div className="fixed inset-0 z-50 flex justify-end" onClick={() => setNodeBriefPanel(null)}>
          <div className="absolute inset-0 bg-black/20" />
          <div className="relative h-full w-[40rem] max-w-[90vw] overflow-auto border-l border-border bg-background shadow-xl animate-in slide-in-from-right-8 fade-in duration-200 ease-out" onClick={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => setNodeBriefPanel(null)} className="absolute right-3 top-3 z-10 rounded p-1 hover:bg-muted"><X className="h-4 w-4" /></button>
            <NodeBriefPanel data={nodeBriefPanel} />
          </div>
        </div>
      )}
      </div>

      {/* Setup / Settings Modal */}
      {debugMode && (isUninitialized || showSettingsModal) && (
        <ProjectSetupModal 
          mode={isUninitialized ? "setup" : "settings"}
          onClose={() => setShowSettingsModal(false)}
          onInitialize={() => {
            setShowSettingsModal(false);
            if (isUninitialized) {
              selectScenario("FX-HAPPY");
            }
          }}
        />
      )}
      </div>
    </div>
    </>
  );
}
