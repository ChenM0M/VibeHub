import { useEffect, useMemo, useRef, useState } from "react";
import { motion } from "framer-motion";
import { ArrowLeft, RefreshCw, FlaskConical, Circle, GitBranch, Clock, CheckSquare, Network, FolderTree, FileText, X, Coins, Archive, ChevronDown, ChevronUp, Settings, ListTodo, Filter, Copy, Check } from "lucide-react";
import { useV3Store } from "@/v3/stores/v3Store";
import { findArchivedTaskAfterClosure } from "@/v3/app/taskClosure";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatTokenCount } from "@/v3/usageFormatting";
import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { V3PanelErrorBoundary } from "@/v3/components/common/V3PanelErrorBoundary";
import { AcceptanceProgress } from "@/v3/components/task/AcceptanceProgress";
import { NodeBriefPanel } from "@/v3/components/task/NodeBriefPanel";
import { StructureArchitecture } from "@/v3/components/project/StructureArchitecture";
import { PlanGraph } from "@/v3/components/task/PlanGraph";
import { AIUsagePanel } from "@/v3/components/task/AIUsagePanel";
import { AgentResultsPanel } from "@/v3/components/task/AgentResultsPanel";
import { ProjectSetupModal } from "@/v3/components/project/ProjectSetupModal";
import { ProjectLifecycleModal } from "@/v3/components/project/ProjectLifecycleModal";
import type { V3FixtureScenario } from "@/v3/contracts/fixtureRepository";
import type { ProjectOverviewView } from "@/v3/contracts/generated/project-overview-view";
import type { NodeBrief } from "@/v3/contracts/generated/node-brief";
import type { V3LifecycleApi, V3ProductionLoader, V3ProjectSettingsApi, V3UsageLoader } from "@/v3/stores/v3Store";
import type { LegacyV2Card } from "@/legacy-v2/contracts";
import type { V3TaskCreateRequest, V3TaskCreateResult } from "@/v3/contracts";
import type { V3AppendResult, V3PlanAddNodeCommand, V3PlanSetDependenciesCommand, V3PlanSetStateCommand } from "@/types";
import type { LegacyV2Loader } from "@/services/legacyV2";
import { useTranslation } from "react-i18next";

const scenarios: V3FixtureScenario[] = ["FX-EMPTY", "FX-HAPPY", "FX-NO-DOCS", "FX-PARALLEL", "FX-REWORK", "FX-STALE", "FX-PARTIAL", "FX-ERROR", "FX-WIN-PATHS", "FX-MAC-PATHS", "FX-LARGE", "FX-COVERAGE-GAP"];
const taskStateTone: Record<string, string> = {
  planned: "text-muted-foreground", active: "text-blue-600 dark:text-blue-400",
  blocked: "text-orange-600 dark:text-orange-400", review: "text-purple-600 dark:text-purple-400",
  completed: "text-emerald-600 dark:text-emerald-400", cancelled: "text-red-600 dark:text-red-400",
};
const taskStateDot: Record<string, string> = {
  planned: "bg-muted-foreground/60", active: "bg-blue-500", blocked: "bg-orange-500",
  review: "bg-purple-500", completed: "bg-emerald-500", cancelled: "bg-red-500",
};
const eventKindDot: Record<string, string> = {
  decision: "bg-purple-500", evidence: "bg-blue-500", finding: "bg-cyan-500", attempt: "bg-amber-500",
  validation: "bg-emerald-500", confirmation: "bg-teal-500", gap: "bg-orange-500", session: "bg-indigo-500", plan: "bg-pink-500",
};
const freshnessTone: Record<string, string> = {
  fresh: "text-emerald-600 dark:text-emerald-400", stale: "text-amber-600 dark:text-amber-400",
  rebuilding: "text-blue-600 dark:text-blue-400", unavailable: "text-muted-foreground",
};

type Tab = "overview" | "timeline" | "plan" | "structure";
type V3SourceMode = "production" | "fixture";
type V3ArchivedTask = NonNullable<ProjectOverviewView["archived_tasks"]>[number];
const tabs: { id: Tab; labelKey: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: "overview", labelKey: "v3.cockpit.tabs.overview", icon: CheckSquare },
  { id: "timeline", labelKey: "v3.cockpit.tabs.timeline", icon: Clock },
  { id: "plan", labelKey: "v3.cockpit.tabs.plan", icon: Network },
  { id: "structure", labelKey: "v3.cockpit.tabs.structure", icon: FolderTree },
];

interface V3CockpitProps {
  onBack: () => void;
  initialSourceMode: V3SourceMode;
  debugMode?: boolean;
  projectPath?: string;
  productionLoader?: V3ProductionLoader;
  legacyLoader?: LegacyV2Loader;
  usageLoader?: V3UsageLoader;
  lifecycleApi?: V3LifecycleApi;
  projectSettingsApi?: V3ProjectSettingsApi;
  planApi?: {
    addNode: (projectPath: string, command: V3PlanAddNodeCommand) => Promise<V3AppendResult>;
    setDependencies: (projectPath: string, command: V3PlanSetDependenciesCommand) => Promise<V3AppendResult>;
    setState: (projectPath: string, command: V3PlanSetStateCommand) => Promise<V3AppendResult>;
  };
  openLegacyFile?: (projectPath: string, relativePath: string) => Promise<void>;
  revealProjectFile?: (projectPath: string, taskId: string, relativePath: string) => Promise<void>;
  openProjectFile?: (projectPath: string, taskId: string, relativePath: string) => Promise<void>;
  createTask?: (projectPath: string, request: V3TaskCreateRequest) => Promise<V3TaskCreateResult>;
  completeTask?: (projectPath: string, request: { project_id: string; task_id: string; actor: string; confirmed_by: string; channel: string; idempotency_key: string }) => Promise<V3AppendResult>;
  closeTaskWithExceptions?: (projectPath: string, request: { project_id: string; task_id: string; actor: string; confirmed_by: string; channel: string; reason: string; idempotency_key: string }) => Promise<V3AppendResult>;
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

export function V3Cockpit({ onBack, initialSourceMode, debugMode = false, projectPath, productionLoader, legacyLoader, usageLoader, lifecycleApi, projectSettingsApi, planApi, openLegacyFile, revealProjectFile, openProjectFile, createTask, completeTask, closeTaskWithExceptions }: V3CockpitProps) {
  const { t, i18n } = useTranslation();
  const {
    currentScenario,
    projectPath: activeProjectPath,
    selectedTaskId,
    selectScenario,
    selectProject,
    selectTask,
    bundle,
    loading,
    error,
    loadCurrentBundle,
    legacyArchive,
    legacyLoading,
    legacyError,
    loadLegacyArchive,
    usage,
    usageLoading,
    usageError,
    loadUsage,
    taskUsage,
    taskUsageLoading,
    taskUsageError,
    loadTaskUsage,
    layoutStatus,
    layoutLoading,
    layoutError,
    lifecycleAction,
    lifecycleResult,
    lifecycleError,
    repairCandidates,
    inspectProjectLayout,
    runLifecycleAction,
    projectSettings,
    settingsLoading,
    settingsError,
    agentSpecs,
    specsLoading,
    specsError,
    updateProjectSettings,
    syncAgentSpecs,
  } = useV3Store();
  const [sourceMode, setSourceMode] = useState<V3SourceMode>(initialSourceMode);
  const [showScenarioDropdown, setShowScenarioDropdown] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null);
  const [nodeBriefPanel, setNodeBriefPanel] = useState<NodeBrief | null>(null);
  const [highlightModuleId, setHighlightModuleId] = useState<string | null>(null);
  const [showArchived, setShowArchived] = useState(false);
  const [showTokenPanel, setShowTokenPanel] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [archivedDetail, setArchivedDetail] = useState<V3ArchivedTask | LegacyV2Card | null>(null);
  const [legacyFileError, setLegacyFileError] = useState<string | null>(null);
  const [timelineFilter, setTimelineFilter] = useState<string>("all");
  const [showCreateTask, setShowCreateTask] = useState(false);
  const [taskTitle, setTaskTitle] = useState("");
  const [taskIntent, setTaskIntent] = useState("");
  const [taskWorkflowProfile, setTaskWorkflowProfile] = useState<V3TaskCreateRequest["workflow_profile"]>("standard");
  const [taskCriteria, setTaskCriteria] = useState("");
  const [taskCreatePending, setTaskCreatePending] = useState(false);
  const [taskCreateError, setTaskCreateError] = useState<string | null>(null);
  const [planMutationError, setPlanMutationError] = useState<string | null>(null);
  const autoOpenedSettingsFor = useRef<string | null>(null);
  const refreshInFlight = useRef(false);
  const [copiedTaskId, setCopiedTaskId] = useState(false);
  const [completePending, setCompletePending] = useState(false);
  const [completeError, setCompleteError] = useState<string | null>(null);
  const [forceCloseDialogOpen, setForceCloseDialogOpen] = useState(false);
  const [forceCloseReason, setForceCloseReason] = useState("");
  const [forceCloseValidationError, setForceCloseValidationError] = useState<string | null>(null);

  const activateScenario = (scenario: V3FixtureScenario) => {
    setSourceMode("fixture");
    selectScenario(scenario);
  };

  const submitTask = async () => {
    if (!projectPath || !createTask || sourceMode !== "production") return;
    const criteria = taskCriteria.split("\n").map((criterion) => criterion.trim()).filter(Boolean);
    if (!taskTitle.trim() || !taskIntent.trim() || criteria.length === 0) {
      setTaskCreateError(t("v3.cockpit.create.validation"));
      return;
    }
    setTaskCreatePending(true);
    setTaskCreateError(null);
    try {
      await createTask(projectPath, {
        title: taskTitle.trim(),
        intent: taskIntent.trim(),
        acceptance_criteria: criteria,
        workflow_profile: taskWorkflowProfile ?? "standard",
      });
      setShowCreateTask(false);
      setTaskTitle("");
      setTaskIntent("");
      setTaskCriteria("");
      setTaskWorkflowProfile("standard");
      await loadCurrentBundle();
    } catch (err) {
      setTaskCreateError((err as Error).message);
    } finally {
      setTaskCreatePending(false);
    }
  };

  useEffect(() => {
    if (sourceMode === "production") {
      if (projectPath && productionLoader && lifecycleApi && activeProjectPath !== projectPath) {
        selectProject(projectPath, productionLoader, legacyLoader, usageLoader, lifecycleApi, projectSettingsApi);
      } else if (layoutStatus?.state === "v3" && !bundle && !loading && !error && activeProjectPath === projectPath) {
        void loadCurrentBundle();
      }
      return;
    }
    if (!currentScenario) {
      selectScenario("FX-HAPPY");
    } else if (!bundle && !loading && !error) {
      void loadCurrentBundle();
    }
  }, [sourceMode, projectPath, productionLoader, legacyLoader, usageLoader, lifecycleApi, projectSettingsApi, activeProjectPath, currentScenario, bundle, loading, error, layoutStatus, selectScenario, selectProject, loadCurrentBundle]);

  useEffect(() => {
    if (sourceMode !== "production" || !projectPath || !productionLoader || layoutStatus?.state !== "v3") return;
    const interval = window.setInterval(() => {
      if (typeof document !== "undefined" && document.visibilityState === "hidden") return;
      if (refreshInFlight.current) return;
      refreshInFlight.current = true;
      void loadCurrentBundle().finally(() => {
        refreshInFlight.current = false;
      });
    }, 4000);
    return () => window.clearInterval(interval);
  }, [sourceMode, projectPath, productionLoader, layoutStatus?.state, loadCurrentBundle]);

  useEffect(() => {
    setArchivedDetail(null);
    setLegacyFileError(null);
    setShowArchived(false);
  }, [activeProjectPath]);

  useEffect(() => {
    if (sourceMode !== "production" || !activeProjectPath || !selectedTaskId) {
      void loadTaskUsage(null);
      return;
    }
    void loadTaskUsage(selectedTaskId);
  }, [sourceMode, activeProjectPath, selectedTaskId, loadTaskUsage]);

  useEffect(() => {
    if (sourceMode !== "production" || projectSettings?.status !== "missing" || !lifecycleResult || !activeProjectPath) return;
    if (autoOpenedSettingsFor.current === activeProjectPath) return;
    autoOpenedSettingsFor.current = activeProjectPath;
    setShowSettingsModal(true);
  }, [sourceMode, projectSettings, lifecycleResult, activeProjectPath]);

  const overview = bundle?.projectOverview;
  const timeline = bundle?.taskTimeline;
  const planGraph = bundle?.planGraph;
  const nodeBrief = bundle?.nodeBrief;
  const structure = bundle?.projectStructure;
  const agentResults = bundle?.agentResults;
  const archivedTasks = overview?.archived_tasks ?? [];
  const selectedTask = overview?.active_tasks.find((t) => t.task_id === selectedTaskId) ?? null;
  const selectedTaskHasDetail =
    selectedTaskId === timeline?.task_id &&
    selectedTaskId === planGraph?.task_id &&
    selectedTaskId === nodeBrief?.task_id;
  const selectedTaskAllGreen = Boolean(selectedTask && selectedTask.criteria.length > 0 && selectedTask.criteria.every((criterion) => criterion.status === "passed") && selectedTask.active_sessions === 0);

  const refreshAfterTaskClosure = async (taskId: string) => {
    const refreshedBundle = await loadCurrentBundle(null);
    const archivedTask = findArchivedTaskAfterClosure(refreshedBundle, taskId);
    if (!archivedTask) {
      throw new Error(t("v3.cockpit.archive.refreshMissing"));
    }
    setShowArchived(true);
    setArchivedDetail(archivedTask);
  };

  const completeSelectedTask = async () => {
    if (!completeTask || !projectPath || !overview || !selectedTaskId || !selectedTaskAllGreen || completePending) return;
    const taskId = selectedTaskId;
    setCompletePending(true);
    setCompleteError(null);
    try {
      await completeTask(projectPath, { project_id: overview.project_id, task_id: taskId, actor: "desktop_user", confirmed_by: "desktop_user", channel: "desktop_ui", idempotency_key: `complete.${taskId}.${Date.now()}` });
      await refreshAfterTaskClosure(taskId);
    } catch (err) {
      setCompleteError((err as Error).message);
    } finally {
      setCompletePending(false);
    }
  };
  const forceCloseSelectedTask = () => {
    if (!closeTaskWithExceptions || !projectPath || !overview || !selectedTaskId || completePending) return;
    setForceCloseReason("");
    setForceCloseValidationError(null);
    setCompleteError(null);
    setForceCloseDialogOpen(true);
  };

  const submitForceClose = async () => {
    if (!closeTaskWithExceptions || !projectPath || !overview || !selectedTaskId || completePending) return;
    const taskId = selectedTaskId;
    const reason = forceCloseReason.trim();
    if (!reason) {
      setForceCloseValidationError(t("v3.cockpit.archive.reasonRequired"));
      return;
    }
    setCompletePending(true);
    setCompleteError(null);
    try {
      await closeTaskWithExceptions(projectPath, { project_id: overview.project_id, task_id: taskId, actor: "desktop_user", confirmed_by: "desktop_user", channel: "desktop_ui", reason, idempotency_key: `force-close.${taskId}.${Date.now()}` });
      await refreshAfterTaskClosure(taskId);
      setForceCloseDialogOpen(false);
      setForceCloseReason("");
    } catch (err) {
      setCompleteError((err as Error).message);
    } finally {
      setCompletePending(false);
    }
  };

  const sortedEvents = useMemo(() => {
    if (!timeline) return [];
    const filtered = timelineFilter === "all" ? timeline.events : timeline.events.filter((e) => e.kind === timelineFilter);
    return [...filtered].sort((a, b) => b.occurred_at.localeCompare(a.occurred_at));
  }, [timeline, timelineFilter]);
  const activeSessionLane = [...(timeline?.lanes ?? [])].reverse().find((lane) => lane.kind === "session" && lane.state === "active") ?? null;
  const latestProtocolEvent = useMemo(() => {
    if (!timeline) return null;
    return timeline.events
      .filter((event) => event.summary_key === "progress.logged" || event.summary_key === "risk.logged")
      .reduce<typeof timeline.events[number] | null>((latest, event) => {
        if (!latest) return event;
        return Date.parse(event.occurred_at) > Date.parse(latest.occurred_at) ? event : latest;
      }, null);
  }, [timeline]);

  function renderCreateTaskDialog() {
    return <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4" onClick={() => !taskCreatePending && setShowCreateTask(false)}><div role="dialog" aria-modal="true" aria-labelledby="v3-create-task-title" className="w-full max-w-lg space-y-4 rounded-lg border border-border bg-background p-6 shadow-xl" onClick={(event) => event.stopPropagation()}><div><h3 id="v3-create-task-title" className="text-lg font-semibold">{t("v3.cockpit.create.title")}</h3><p className="mt-1 text-sm text-muted-foreground">{t("v3.cockpit.create.description")}</p></div><label className="block space-y-1"><span className="text-sm font-medium">{t("v3.cockpit.create.titleLabel")}</span><input value={taskTitle} onChange={(event) => setTaskTitle(event.target.value)} className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm" placeholder={t("v3.cockpit.create.titlePlaceholder")} /></label><label className="block space-y-1"><span className="text-sm font-medium">{t("v3.cockpit.create.intentLabel")}</span><textarea value={taskIntent} onChange={(event) => setTaskIntent(event.target.value)} className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" placeholder={t("v3.cockpit.create.intentPlaceholder")} /></label><label className="block space-y-1"><span className="text-sm font-medium">{t("v3.cockpit.workflowLabel")}</span><select value={taskWorkflowProfile} onChange={(event) => setTaskWorkflowProfile(event.target.value as V3TaskCreateRequest["workflow_profile"])} className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"><option value="lightweight">{t("v3.cockpit.workflow.lightweight")}</option><option value="standard">{t("v3.cockpit.workflow.standard")}</option><option value="full">{t("v3.cockpit.workflow.full")}</option></select><span className="text-[11px] text-muted-foreground">{t("v3.cockpit.workflowHint")}</span></label><label className="block space-y-1"><span className="text-sm font-medium">{t("v3.cockpit.create.criteriaLabel")}</span><textarea value={taskCriteria} onChange={(event) => setTaskCriteria(event.target.value)} className="min-h-28 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" placeholder={t("v3.cockpit.create.criteriaPlaceholder")} /></label>{taskCreateError && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{taskCreateError}</div>}<div className="flex justify-end gap-2"><Button variant="outline" disabled={taskCreatePending} onClick={() => setShowCreateTask(false)}>{t("v3.common.cancel")}</Button><Button disabled={taskCreatePending} onClick={() => void submitTask()}>{t(taskCreatePending ? "v3.cockpit.create.creating" : "v3.cockpit.create.submit")}</Button></div></div></div>;
  }

  if (sourceMode === "production" && projectPath && lifecycleApi && (layoutLoading || layoutError || layoutStatus?.state !== "v3")) {
    return (
      <div className="relative h-full">
        <ProjectLifecycleModal
          projectPath={projectPath}
          status={layoutStatus}
          loading={layoutLoading}
          error={layoutError}
          action={lifecycleAction}
          actionError={lifecycleError}
          result={lifecycleResult}
          repairCandidates={repairCandidates}
          onInspect={() => void inspectProjectLayout()}
          onRunAction={(action, taskId) => void runLifecycleAction(action, taskId)}
          onBack={onBack}
        />
      </div>
    );
  }
  if (loading && !bundle) { return <div className="flex h-full items-center justify-center"><div className="text-sm text-muted-foreground">{t("v3.cockpit.loading")}</div></div>; }
  if (error && !(sourceMode === "production" && createTask && /CURRENT_TASK|TASK_NOT_FOUND/.test(error))) { return <div className="flex h-full flex-col gap-4 p-8"><Button variant="ghost" size="sm" className="w-fit" onClick={onBack}><ArrowLeft className="mr-2 h-4 w-4" />{t("v3.cockpit.backToProjects")}</Button><div className="border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">{error}</div></div>; }
  if (sourceMode === "production" && createTask && error && /CURRENT_TASK|TASK_NOT_FOUND/.test(error)) {
    return <div className="flex h-full flex-col"><div className="flex items-center border-b border-border/60 px-6 py-3"><Button variant="ghost" size="sm" onClick={onBack}><ArrowLeft className="mr-2 h-4 w-4" /></Button><h2 className="text-xl font-semibold">{t("v3.cockpit.v3Project")}</h2></div><div className="flex flex-1 items-center justify-center p-8"><div className="max-w-md rounded-lg border border-dashed border-border p-8 text-center"><ListTodo className="mx-auto mb-3 h-8 w-8 text-muted-foreground" /><h3 className="font-semibold">{t("v3.cockpit.noTaskTitle")}</h3><p className="mt-2 text-sm text-muted-foreground">{t("v3.cockpit.noTaskDescription")}</p><Button className="mt-5" onClick={() => { setTaskCreateError(null); setShowCreateTask(true); }}>{t("v3.cockpit.create.submit")}</Button></div></div>{showCreateTask && renderCreateTaskDialog()}</div>;
  }
  if (!overview || !timeline || !planGraph || !nodeBrief || !structure || !agentResults) return null;

  const syntheticStructureWarnings = structure.unsupported_analyzers.length > 0 && !structure.warnings.some((warning) => warning.code === "PI_ANALYZER_UNSUPPORTED") ? [{
    code: "PI_ANALYZER_UNSUPPORTED",
    severity: "warning" as const,
    message_key: "v3.warning.analyzer_unsupported",
    details: { analyzers: structure.unsupported_analyzers },
    evidence_refs: structure.evidence_refs,
  }] : [];
  const projectWarnings = Array.from(new Map([
    ...overview.warnings,
    ...timeline.warnings,
    ...planGraph.warnings,
    ...structure.warnings,
    ...syntheticStructureWarnings,
    ...nodeBrief.warnings,
    ...agentResults.warnings,
  ].map((warning) => [`${warning.code}:${warning.message_key}`, warning])).values());
  const projectErrors = Array.from(new Map([
    ...overview.errors,
    ...timeline.errors,
    ...planGraph.errors,
    ...structure.errors,
    ...nodeBrief.errors,
    ...agentResults.errors,
  ].map((item) => [`${item.code}:${item.message_key}`, item])).values());

  const criteria = selectedTask?.criteria ?? timeline.criteria;
  const selectedEvent = timeline.events.find((e) => e.timeline_event_id === selectedEventId) ?? null;
  const eventLabel = (key: string) => t(`v3.cockpit.eventSummary.${key.split(".").join("_")}`, { defaultValue: key });
  const eventKind = (kind: string) => t(`v3.timeline.eventKind.${kind}`, { defaultValue: kind });
  const formatDateTime = (value: string) => new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "medium" }).format(new Date(value));
  const formatDate = (value: string) => new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium" }).format(new Date(value));

  const renderOverview = () => (
    <div className="grid h-full gap-4 lg:grid-cols-2">
      <div className="flex flex-col gap-4 overflow-auto pr-1">
        <BlockerDetailsPanel blockers={selectedTask?.blocker_details ?? timeline.blocker_details} />
        <div className="bg-card rounded-md shadow-sm p-5 border border-border/30">
          <AcceptanceProgress criteria={criteria} />
        </div>
        <AIUsagePanel scope="task" taskId={selectedTask?.task_id ?? null} usage={taskUsage} loading={taskUsageLoading} error={taskUsageError} onRefresh={() => void loadTaskUsage(selectedTask?.task_id ?? null)} />
      </div>
      <div className="bg-card rounded-md shadow-sm p-5 border border-border/30 overflow-auto">
        <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><FileText className="h-4 w-4 text-muted-foreground" />{t("v3.cockpit.agentResults")}</div>
        {selectedTaskId === agentResults.task_id ? (
          <V3PanelErrorBoundary resetKey={`${agentResults.task_id}:${agentResults.generated_at}`} title={t("v3.cockpit.agentResultsUnavailable")}>
            <AgentResultsPanel data={agentResults} />
          </V3PanelErrorBoundary>
        ) : <div className="flex min-h-32 items-center justify-center text-sm text-muted-foreground">{t("v3.cockpit.primaryTaskOnly.agentResults")}</div>}
        {selectedTaskHasDetail && <div className="mt-4 border-t border-border/50 pt-3"><div className="flex items-center gap-2"><div className="min-w-0 flex-1"><div className="text-xs font-semibold">{t("v3.cockpit.currentNodeBrief")}</div><div className="mt-1 truncate text-xs text-muted-foreground">{nodeBrief.goal}{nodeBrief.next_intent ? t("v3.cockpit.nextIntent", { intent: nodeBrief.next_intent }) : ""}</div></div><button type="button" onClick={() => setNodeBriefPanel(nodeBrief)} className="shrink-0 text-xs text-blue-600 hover:underline">{t("v3.cockpit.viewBrief")}</button></div></div>}
      </div>
    </div>
  );

  const renderTimeline = () => (
    <div className="grid h-full gap-4 lg:grid-cols-[minmax(0,1fr)_20rem]">
      <div className="overflow-auto bg-card rounded-md shadow-sm p-5 border border-border/30 flex flex-col">
        <div className="mb-4 space-y-3 shrink-0">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <Clock className="h-4 w-4 text-muted-foreground" />{t("v3.cockpit.eventStream")}
            <span className="text-xs text-muted-foreground">({sortedEvents.length})</span>
          </div>
          
          <div className="flex flex-wrap items-center gap-1.5">
            <Filter className="h-3 w-3 text-muted-foreground/70 mr-0.5 shrink-0" />
            <button
              type="button"
              onClick={() => setTimelineFilter("all")}
              className={cn("px-2.5 py-0.5 text-[11px] font-medium rounded-full border transition-colors", timelineFilter === "all" ? "bg-foreground text-background border-foreground shadow-sm" : "bg-muted/30 text-muted-foreground border-border/60 hover:bg-muted/60 hover:text-foreground")}
            >
              {t("v3.globalTimeline.all")}
            </button>
            {Object.keys(eventKindDot).map((kind) => (
              <button
                key={kind}
                type="button"
                onClick={() => setTimelineFilter(kind)}
                className={cn("flex items-center gap-1.5 px-2.5 py-0.5 text-[11px] font-medium rounded-full border transition-colors", timelineFilter === kind ? "bg-foreground text-background border-foreground shadow-sm" : "bg-muted/30 text-muted-foreground border-border/60 hover:bg-muted/60 hover:text-foreground")}
              >
                <span className={cn("h-1.5 w-1.5 rounded-full shrink-0", timelineFilter === kind ? "bg-background" : eventKindDot[kind])} />
                {eventKind(kind)}
              </button>
            ))}
          </div>
        </div>
        
        {sortedEvents.length === 0 ? <div className="py-8 text-center text-sm text-muted-foreground flex-1">{t("v3.cockpit.noMatchingEvents")}</div> : (
          <div className="space-y-1 overflow-y-auto flex-1 scrollbar-auto-hide pr-1">
            {sortedEvents.map((event) => (
              <button key={event.timeline_event_id} type="button" onClick={() => setSelectedEventId(event.timeline_event_id)} className={cn("flex w-full items-start gap-2.5 border-b border-border/30 py-2 text-left hover:bg-muted/10", selectedEventId === event.timeline_event_id && "bg-muted/20")}>
                <span className={cn("mt-1.5 h-2 w-2 shrink-0 rounded-full", eventKindDot[event.kind])} />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2"><span className="text-xs text-muted-foreground">{eventKind(event.kind)}</span><span className="truncate text-sm font-medium">{eventLabel(event.summary_key)}</span></div>
                  <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground"><span>{formatDateTime(event.occurred_at)}</span><span>· {event.actor}</span>{event.commit_sha && <span className="font-mono">· {event.commit_sha.slice(0, 7)}</span>}</div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="overflow-auto bg-card rounded-md shadow-sm p-5 border border-border/30">
        {selectedEvent ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2"><span className={cn("h-2 w-2 rounded-full", eventKindDot[selectedEvent.kind])} /><span className="text-base font-medium">{eventLabel(selectedEvent.summary_key)}</span></div>
            <div className="space-y-1 text-sm text-muted-foreground">
              <div>{t("v3.timeline.time", { value: formatDateTime(selectedEvent.occurred_at) })}</div>
              <div>{t("v3.timeline.actor", { actor: selectedEvent.actor, tool: selectedEvent.tool ? t("v3.timeline.viaTool", { tool: selectedEvent.tool }) : "" })}</div>
              {selectedEvent.commit_sha && <div className="font-mono">{t("v3.timeline.commit", { value: selectedEvent.commit_sha })}</div>}
              {selectedEvent.order_state !== "ordered" && <div className="text-orange-600">{t("v3.cockpit.orderAnomaly")}</div>}
            </div>
            {Object.keys(selectedEvent.details ?? {}).length > 0 && (
              <div><div className="mb-1 text-[10px] text-muted-foreground">{t("v3.cockpit.details")}</div><EventDetails details={selectedEvent.details} /></div>
            )}
          </div>
        ) : <div className="py-8 text-center text-xs text-muted-foreground">{t("v3.cockpit.selectEventHint")}</div>}
      </div>
    </div>
  );

  const mutatePlan = async (kind: "add" | "dependencies" | "state", input: { nodeId: string; idempotencyKey: string; title?: string; goal?: string; scope?: string[]; dependencies?: string[]; state?: string }) => {
    if (sourceMode !== "production" || !projectPath || !planApi || !planGraph) return false;
    setPlanMutationError(null);
    const identity = { project_id: planGraph.project_id, task_id: planGraph.task_id, actor: "vibehub-ui", expected_version: planGraph.plan_version, idempotency_key: input.idempotencyKey };
    try {
      if (kind === "add") await planApi.addNode(projectPath, { ...identity, node_id: input.nodeId, title: input.title!, goal: input.goal!, scope: input.scope ?? [], dependencies: input.dependencies ?? [] });
      if (kind === "dependencies") await planApi.setDependencies(projectPath, { ...identity, node_id: input.nodeId, dependencies: input.dependencies ?? [] });
      if (kind === "state") await planApi.setState(projectPath, { ...identity, node_id: input.nodeId, state: input.state! });
      await loadCurrentBundle();
      setActiveTab("plan");
      return true;
    } catch (err) {
      setPlanMutationError(err instanceof Error ? err.message : String(err));
      return false;
    }
  };

  const renderTab = () => {
    switch (activeTab) {
      case "overview": return renderOverview();
      case "timeline": return selectedTaskHasDetail ? renderTimeline() : <div className="flex h-full items-center justify-center text-sm text-muted-foreground">{t("v3.cockpit.primaryTaskOnly.timeline")}</div>;
      case "plan": return selectedTaskHasDetail ? <PlanGraph data={planGraph} taskTitle={selectedTask?.title} taskIntent={nodeBrief.goal || selectedTask?.title} mutationError={planMutationError} onAddNode={sourceMode === "production" && planApi ? (input) => mutatePlan("add", input) : undefined} onSetDependencies={sourceMode === "production" && planApi ? (input) => mutatePlan("dependencies", input) : undefined} onSetState={sourceMode === "production" && planApi ? (input) => mutatePlan("state", input) : undefined} onNodeClick={(node) => { if (node.node_id === nodeBrief.node_id) setNodeBriefPanel(nodeBrief); }} /> : <div className="flex h-full items-center justify-center text-sm text-muted-foreground">{t("v3.cockpit.primaryTaskOnly.plan")}</div>;
      case "structure": return <StructureArchitecture data={structure} taskId={timeline.task_id} projectPath={activeProjectPath ?? undefined} onModuleClick={setHighlightModuleId} highlightModuleId={highlightModuleId} onRevealProjectFile={revealProjectFile} onOpenProjectFile={openProjectFile} />;
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
        <div className="flex items-center gap-3">
          <div className="flex items-baseline gap-3">
            <h2 className="text-2xl font-semibold tracking-tight">{overview.name}</h2>
            <span className={cn("text-xs", freshnessTone[overview.freshness])}>{t(`v3.common.freshness.${overview.freshness}`, { defaultValue: overview.freshness })}</span>
          </div>
          {(projectWarnings.length > 0 || projectErrors.length > 0) && (
            <div className="flex items-center gap-1.5 ml-1">
              {projectWarnings.length > 0 && <WarningList warnings={projectWarnings} />}
              {projectErrors.length > 0 && <ErrorList errors={projectErrors} />}
            </div>
          )}
        </div>
        <div className="ml-auto flex items-center gap-4 text-xs text-muted-foreground">
          {overview.repository.branch && <span className="flex items-center gap-1"><GitBranch className="h-3 w-3" /><span className="font-mono">{overview.repository.branch}</span></span>}
          {debugMode && <span className="border border-border/50 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">{t("v3.cockpit.debugMode")}</span>}
          <button type="button" onClick={() => setShowTokenPanel(true)} className="flex items-center gap-1 hover:text-foreground transition-colors"><Coins className="h-3 w-3" />{usageLoading && !usage ? t("v3.cockpit.usageLoading") : usage?.total_tokens ? t("v3.cockpit.usageValue", { value: formatTokenCount(usage.total_tokens) }) : t("v3.cockpit.usageEmpty")}</button>
          <Button variant="ghost" size="sm" onClick={() => { void loadCurrentBundle(); void loadLegacyArchive(); void loadUsage(); void loadTaskUsage(selectedTaskId); }} disabled={loading || legacyLoading || usageLoading || taskUsageLoading}><RefreshCw className={cn("mr-1.5 h-3.5 w-3.5", (loading || legacyLoading || usageLoading || taskUsageLoading) && "animate-spin")} />{t("v3.common.refresh")}</Button>
          {sourceMode === "production" && createTask && <Button size="sm" onClick={() => { setTaskCreateError(null); setShowCreateTask(true); }}><ListTodo className="mr-1.5 h-3.5 w-3.5" />{t("v3.cockpit.create.submit")}</Button>}
          {debugMode && <div className="relative">
            <Button variant="ghost" size="sm" onClick={() => setShowScenarioDropdown(!showScenarioDropdown)}><FlaskConical className="mr-1.5 h-3.5 w-3.5" />{currentScenario ? t(`v3.cockpit.scenarios.${currentScenario}`, { defaultValue: t("v3.cockpit.customScenario") }) : t("v3.cockpit.selectScenario")}</Button>
            {showScenarioDropdown && (
              <div className="absolute right-0 top-full z-50 mt-1 max-h-72 overflow-auto rounded-md border border-border bg-popover p-1 shadow-md">
                {scenarios.map((s) => (
                  <button key={s} type="button" onClick={() => { activateScenario(s); setShowScenarioDropdown(false); }} className={cn("block w-full px-3 py-1.5 text-left text-sm hover:bg-accent", currentScenario === s && "bg-accent font-medium")}>
                    <span>{t(`v3.cockpit.scenarios.${s}`)}</span>
                  </button>
                ))}
              </div>
            )}
          </div>}
          {sourceMode === "production" && projectSettingsApi && <Button variant="ghost" size="sm" aria-label={t("v3.setup.settingsTitle")} onClick={() => setShowSettingsModal(true)}><Settings className="h-3.5 w-3.5" /></Button>}
        </div>
      </div>

      <div className="relative flex-1 flex flex-col min-h-0">
        <div className={cn("flex flex-col flex-1 min-h-0 transition-all", isUninitialized && "opacity-40 pointer-events-none grayscale-[0.5]")}>
          {(projectSettings?.status === "missing" || settingsError || specsError || agentSpecs?.artifacts.some((artifact) => artifact.status !== "in_sync")) && sourceMode === "production" && (
            <div className="shrink-0 px-6 pt-2"><div className="flex items-center justify-between gap-3 rounded-md border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-xs text-amber-800 dark:text-amber-300"><span>{settingsError || specsError || t(projectSettings?.status === "missing" ? "v3.cockpit.settingsMissing" : "v3.cockpit.specsNeedAttention")}</span><button type="button" className="shrink-0 underline" onClick={() => setShowSettingsModal(true)}>{t("v3.cockpit.openSettings")}</button></div></div>
          )}
          {/* 主区域 */}
          <div className="flex flex-1 min-h-0 px-6 py-2 gap-4">
        {/* 左列：任务列表 */}
        <div className="flex w-60 shrink-0 flex-col overflow-hidden">
          <div className="flex items-center gap-2 text-sm font-semibold shrink-0 mb-2">
            <ListTodo className="h-4 w-4 text-muted-foreground" />{t("v3.projectOverview.activeTasks")}<span className="text-xs text-muted-foreground">({overview.active_tasks.length})</span>
          </div>
          <div className="flex-1 overflow-y-auto scrollbar-auto-hide space-y-2 pr-1">
            {overview.active_tasks.length === 0 ? (
              <div className="border border-dashed border-border px-4 py-8 text-center"><Circle className="mx-auto mb-2 h-6 w-6 text-muted-foreground/40" /><div className="text-sm text-muted-foreground">{t("v3.projectOverview.noActiveTasks")}</div></div>
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
                    title={hasDetail ? t("v3.cockpit.viewTask", { title: task.title }) : t("v3.cockpit.primaryTaskOnly.details")}
                    onClick={() => {
                      selectTask(task.task_id);
                      setSelectedEventId(null);
                      setNodeBriefPanel(null);
                      if (sourceMode === "production") void loadCurrentBundle();
                    }}
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
                      <div className="flex items-start gap-2">
                        <div className="line-clamp-2 flex-1 text-sm font-medium leading-snug">{task.title}</div>
                        {overview.current_task_id === task.task_id && <span className="shrink-0 rounded border border-blue-500/30 bg-blue-500/10 px-1.5 py-0.5 text-[10px] text-blue-600 dark:text-blue-400">{t("v3.cockpit.current")}</span>}
                      </div>
                      <div className="mt-1.5 flex items-center gap-2 text-xs">
                        <span className="flex items-center gap-1.5">
                          <span className={cn("h-2 w-2 shrink-0 rounded-full", taskStateDot[task.state] ?? "bg-muted-foreground/60")} aria-hidden="true" />
                          <span className={taskStateTone[task.state]}>{t(`v3.timeline.taskState.${task.state}`, { defaultValue: task.state })}</span>
                        </span>
                        <div className="ml-auto shrink-0 text-[10px] text-muted-foreground bg-muted/40 px-1.5 py-0.5 rounded border border-border/40">
                          {t("v3.projectOverview.sessionCount", { count: task.active_sessions })}
                        </div>
                      </div>
                      {task.criteria.length > 0 && <div className="mt-2.5 flex flex-wrap items-center gap-2">{task.criteria.map((c) => <CriterionBadge key={c.criterion_id} criterion={c} compact />)}</div>}
                    </div>
                  </button>
                );
              })
            )}
          </div>

          {/* 归档任务 (固定在底部，向上展开) */}
          {!currentScenario && <div className="shrink-0 border-t border-border/50 pt-2 mt-2 flex flex-col">
            {showArchived && (
              <div className="mb-2 space-y-1.5 max-h-[40vh] overflow-y-auto scrollbar-auto-hide pr-1">
                {archivedTasks.length > 0 && <>
                  <div className="px-2 text-[10px] font-medium text-muted-foreground">{t("v3.cockpit.archive.v3Ended")}</div>
                  {archivedTasks.map((at) => {
                    const passed = at.criteria.filter((criterion) => criterion.status === "passed").length;
                    return (
                      <button key={at.task_id} type="button" onClick={() => { setLegacyFileError(null); setArchivedDetail(at); }} className="w-full rounded-md border border-border/40 bg-card/30 p-2.5 text-left hover:bg-muted/50 transition-colors">
                        <div className="flex items-start gap-2">
                          <div className="min-w-0 flex-1 truncate text-xs font-medium">{at.title}</div>
                          <span className={cn("shrink-0 text-[10px] font-medium", taskStateTone[at.state])}>{t(`v3.timeline.taskState.${at.state}`, { defaultValue: at.state })}</span>
                        </div>
                        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[10px] text-muted-foreground">
                          {at.terminal_at && <span>{formatDate(at.terminal_at)}</span>}
                          <span>{t("v3.cockpit.archive.acceptanceCount", { passed, total: at.criteria.length })}</span>
                          <span>{t("v3.cockpit.archive.nodeCount", { completed: at.plan.completed, total: at.plan.total })}</span>
                          <span>{t("v3.common.evidenceCount", { count: at.evidence_count })}</span>
                          <span className={cn(at.risk_level !== "none" && "text-orange-600 dark:text-orange-400")}>{t("v3.cockpit.risk", { level: t(`v3.cockpit.riskLevel.${at.risk_level}`, { defaultValue: at.risk_level }) })}</span>
                        </div>
                        <div className="mt-1 line-clamp-2 text-[10px] text-muted-foreground">{at.result.summary ?? at.next_action}</div>
                      </button>
                    );
                  })}
                </>}
                {archivedTasks.length > 0 && legacyArchive?.cards.length ? <div className="my-2 border-t border-border/40" /> : null}
                <div className="px-2 text-[10px] font-medium text-muted-foreground">{t("v3.cockpit.archive.legacySource", { source: legacyArchive?.source_state ?? t("v3.cockpit.unloaded") })}</div>
                {legacyLoading && <div className="px-2 py-2 text-xs text-muted-foreground">{t("v3.cockpit.archive.loading")}</div>}
                {legacyError && <div className="rounded border border-destructive/30 bg-destructive/5 px-2 py-2 text-xs text-destructive">{legacyError}<button type="button" className="ml-2 underline" onClick={() => void loadLegacyArchive()}>{t("v3.common.retry")}</button></div>}
                {legacyArchive?.warnings.map((warning, index) => <div key={index} className="px-2 text-[10px] text-amber-600 dark:text-amber-400">{warning}</div>)}
                {!legacyLoading && !legacyError && legacyArchive?.cards.length === 0 && <div className="px-2 py-2 text-xs text-muted-foreground">{t("v3.cockpit.archive.noLegacy")}</div>}
                {legacyArchive?.cards.map((at) => (
                  <button key={at.task_id} type="button" onClick={() => { setLegacyFileError(null); setArchivedDetail(at); }} className="w-full rounded-md p-2 text-left hover:bg-muted/50 transition-colors">
                    <div className="truncate text-xs font-medium">{at.title}</div>
                    <div className="mt-0.5 flex items-center gap-2 text-[10px] text-muted-foreground">
                      {at.state && <span className={taskStateTone[at.state]}>{at.state}</span>}
                      {at.completed_at && <span>{formatDate(at.completed_at)}</span>}
                      {at.phase && <span className="ml-auto">{at.phase}</span>}
                    </div>
                    {at.warnings.map((warning, index) => <div key={index} className="mt-1 text-[10px] text-amber-600 dark:text-amber-400">{warning}</div>)}
                  </button>
                ))}
              </div>
            )}
            <button type="button" onClick={() => setShowArchived(!showArchived)} className="flex w-full items-center gap-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors">
              {showArchived ? <ChevronDown className="h-3 w-3" /> : <ChevronUp className="h-3 w-3" />}
              <Archive className="h-3.5 w-3.5" />{t("v3.cockpit.archive.taskCount", { count: archivedTasks.length + (legacyArchive?.cards.length ?? 0) })}
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
                  <Icon className="h-3.5 w-3.5" />{t(tab.labelKey)}
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
            <div className="mb-3 shrink-0 pb-1.5 border-b-0">
              <div className="flex flex-wrap items-end gap-3">
                <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                  <div className="flex min-w-0 items-center gap-3">
                    <h3 className="truncate text-lg font-semibold">{selectedTask.title}</h3>
                    <span className="shrink-0 text-xs text-muted-foreground border border-border/50 px-1.5 py-0.5 rounded bg-muted/20">{t("v3.cockpit.risk", { level: t(`v3.cockpit.riskLevel.${selectedTask.risk_level}`, { defaultValue: selectedTask.risk_level }) })}</span>
                  </div>
                  <div className="flex min-w-0 items-center gap-2 text-[11px] text-muted-foreground" title={activeSessionLane ? t("v3.cockpit.activeSession", { id: activeSessionLane.lane_id }) : t("v3.cockpit.noActiveSession")}>
                    <span className={cn("h-1.5 w-1.5 shrink-0 rounded-full", activeSessionLane ? "bg-emerald-500" : "bg-muted-foreground/40")} aria-hidden="true" />
                    <span className="shrink-0 font-medium text-foreground/80">Task ID</span>
                    <button
                      type="button"
                      onClick={() => {
                        const id = selectedTaskId ?? overview.current_task_id;
                        if (id) {
                          navigator.clipboard.writeText(id).catch(() => {});
                          setCopiedTaskId(true);
                          setTimeout(() => setCopiedTaskId(false), 2000);
                        }
                      }}
                      className="flex min-w-0 items-center gap-1.5 rounded bg-muted/30 px-1.5 py-0.5 hover:bg-muted/50 transition-colors group cursor-pointer text-foreground/80"
                      title={t(copiedTaskId ? "v3.cockpit.copied" : "v3.cockpit.copyTaskId")}
                    >
                      <code className="truncate font-mono">{selectedTaskId ?? overview.current_task_id ?? t("v3.cockpit.unresolved")}</code>
                      {copiedTaskId ? <Check className="h-3 w-3 text-emerald-500 shrink-0" /> : <Copy className="h-3 w-3 opacity-0 group-hover:opacity-50 transition-opacity shrink-0" />}
                    </button>
                    {latestProtocolEvent && <span className="sr-only">{t("v3.cockpit.latestProtocolEvent", { event: latestProtocolEvent.summary_key })}</span>}
                  </div>
                </div>
                <div className="ml-auto flex shrink-0 items-center gap-2">
                  {sourceMode === "production" && completeTask && selectedTaskAllGreen && <Button size="sm" variant="outline" onClick={() => void completeSelectedTask()} disabled={completePending}><CheckSquare className="mr-1.5 h-3.5 w-3.5" />{t(completePending ? "v3.cockpit.archive.archiving" : "v3.cockpit.archive.complete")}</Button>}
                  {sourceMode === "production" && closeTaskWithExceptions && !selectedTaskAllGreen && <Button size="sm" variant="outline" onClick={() => void forceCloseSelectedTask()} disabled={completePending}><Archive className="mr-1.5 h-3.5 w-3.5" />{t("v3.cockpit.archive.force")}</Button>}
                </div>
              </div>
              {completeError && <div role="alert" className="mt-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">{completeError}</div>}
            </div>
          )}
          <div className="min-h-0 flex-1 overflow-hidden">{renderTab()}</div>
        </div>
      </div>

      {/* 项目总 Token 用量面板 */}
      {showTokenPanel && (
        <div className="fixed inset-0 z-50 flex justify-end" onClick={() => setShowTokenPanel(false)}>
          <div className="absolute inset-0 bg-black/20" />
          <div className="relative h-full w-[40rem] max-w-[90vw] overflow-auto border-l border-border bg-background shadow-xl animate-in slide-in-from-right-8 fade-in duration-200 ease-out" onClick={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => setShowTokenPanel(false)} className="absolute right-3 top-3 z-10 rounded p-1 hover:bg-muted"><X className="h-4 w-4" /></button>
            <div className="p-6">
              <h3 className="mb-4 text-lg font-semibold">{t("v3.cockpit.projectUsage")}</h3>
              <AIUsagePanel scope="project" taskId={null} usage={usage} loading={usageLoading} error={usageError} onRefresh={() => void loadUsage()} />
            </div>
          </div>
        </div>
      )}

      {/* 归档任务详情 */}
      {archivedDetail && (
        <div className="fixed inset-0 z-50 flex justify-end" onClick={() => setArchivedDetail(null)}>
          <div className="absolute inset-0 bg-black/20" />
          <div className="relative h-full w-[40rem] max-w-[90vw] overflow-auto border-l border-border bg-background shadow-xl animate-in slide-in-from-right-8 fade-in duration-200 ease-out" onClick={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => setArchivedDetail(null)} className="absolute right-3 top-3 z-10 rounded p-1 hover:bg-muted"><X className="h-4 w-4" /></button>
            <div className="p-6 space-y-5">
              {"terminal_at" in archivedDetail ? (
                <>
                  <div className="pb-4 border-b border-border/60">
                    <div className="mb-2 flex flex-wrap items-center gap-2 text-[10px] text-muted-foreground">
                      <span className="rounded border border-blue-500/30 bg-blue-500/10 px-1.5 py-0.5 text-blue-700 dark:text-blue-300">V3 projection</span>
                      <span>{t("v3.cockpit.archive.notCompletion")}</span>
                    </div>
                    <h3 className="text-xl font-semibold text-foreground tracking-tight">{archivedDetail.title}</h3>
                    <div className="mt-2.5 flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                      <span className={cn("font-medium", taskStateTone[archivedDetail.state])}>{t(`v3.timeline.taskState.${archivedDetail.state}`, { defaultValue: archivedDetail.state })}</span>
                      {archivedDetail.terminal_at && <span>{t("v3.cockpit.archive.archivedAt", { date: formatDateTime(archivedDetail.terminal_at) })}</span>}
                      <span>{archivedDetail.task_id}</span>
                    </div>
                  </div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.taskIntent")}</h4><p className="whitespace-pre-wrap text-sm leading-relaxed text-muted-foreground">{archivedDetail.intent}</p></div>
                  <div className="grid gap-2 sm:grid-cols-3">
                    <div className="rounded-lg border border-border/40 bg-card/30 p-3"><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.acceptance")}</div><div className="mt-1 text-lg font-semibold">{archivedDetail.criteria.filter((criterion) => criterion.status === "passed").length}/{archivedDetail.criteria.length}</div><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.criteriaPassed")}</div></div>
                    <div className="rounded-lg border border-border/40 bg-card/30 p-3"><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.planNodes")}</div><div className="mt-1 text-lg font-semibold">{archivedDetail.plan.completed}/{archivedDetail.plan.total}</div><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.completed")}</div></div>
                    <div className="rounded-lg border border-border/40 bg-card/30 p-3"><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.evidence")}</div><div className="mt-1 text-lg font-semibold">{archivedDetail.evidence_count}</div><div className="text-[10px] text-muted-foreground">{t("v3.cockpit.archive.evidenceRefs")}</div></div>
                  </div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.finalResult")}</h4><div className="flex flex-wrap gap-2 text-xs text-muted-foreground"><span>{t("v3.cockpit.archive.result", { value: archivedDetail.result.status })}</span><span>{t("v3.cockpit.archive.artifacts", { count: archivedDetail.result.artifact_count })}</span><span>{t("v3.cockpit.archive.sessionsClosed", { closed: archivedDetail.sessions.closed, total: archivedDetail.sessions.total })}</span><span>{t("v3.cockpit.archive.openFindings", { count: archivedDetail.findings.open })}</span></div>{archivedDetail.result.summary ? <p className="mt-3 whitespace-pre-wrap text-sm leading-relaxed text-muted-foreground">{archivedDetail.result.summary}</p> : <p className="mt-3 text-sm text-muted-foreground">{t("v3.cockpit.archive.noAgentSummary")}</p>}</div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><div className="flex items-center justify-between gap-3"><h4 className="text-sm font-semibold">{t("v3.cockpit.archive.completionConfirmation")}</h4><span className={cn("text-xs font-medium", archivedDetail.completion.confirmed ? "text-emerald-600 dark:text-emerald-400" : "text-orange-600 dark:text-orange-400")}>{t(archivedDetail.completion.confirmed ? "v3.cockpit.archive.confirmed" : "v3.cockpit.archive.unconfirmed")}</span></div><p className="mt-2 text-xs leading-relaxed text-muted-foreground">{archivedDetail.next_action}</p>{(archivedDetail.completion.confirmed_by || archivedDetail.completion.channel) && <div className="mt-2 text-[10px] text-muted-foreground">{t("v3.cockpit.archive.confirmationAudit", { user: archivedDetail.completion.confirmed_by ?? t("v3.cockpit.notRecorded"), channel: archivedDetail.completion.channel ?? t("v3.cockpit.notRecorded") })}</div>}</div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.closureAudit")}</h4><div className="space-y-1 text-xs text-muted-foreground"><div>{t("v3.cockpit.archive.closureMethod", { method: archivedDetail.closure.method ? t(`v3.cockpit.archive.method.${archivedDetail.closure.method}`) : t("v3.cockpit.notRecorded") })}</div><div>{t("v3.cockpit.archive.closureActor", { actor: archivedDetail.closure.actor ?? t("v3.cockpit.notRecorded") })}</div>{archivedDetail.closure.confirmed_at && <div>{t("v3.cockpit.archive.closureTime", { date: formatDateTime(archivedDetail.closure.confirmed_at) })}</div>}{archivedDetail.closure.reason && <div>{t("v3.cockpit.archive.closureReason", { reason: archivedDetail.closure.reason })}</div>}<div>{t("v3.cockpit.archive.unresolvedCount", { count: archivedDetail.closure.unresolved_items.length })}</div></div></div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><AcceptanceProgress criteria={archivedDetail.criteria} /></div>
                  <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.evidenceAndSource")}</h4><EvidenceLink evidenceRefs={archivedDetail.evidence_refs} /></div>
                  {archivedDetail.blocker_details.length > 0 && <div className="rounded-lg border border-orange-500/30 bg-orange-500/5 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.blockers")}</h4><ul className="space-y-2 text-xs text-orange-700 dark:text-orange-400">{archivedDetail.blocker_details.map((blocker) => <li key={blocker.blocker_id}><div className="font-medium">{blocker.summary}</div><div className="mt-0.5">{t("v3.cockpit.archive.nextStep", { action: blocker.resume_action })}</div></li>)}</ul></div>}
                </>
              ) : (
                <>
                  <div className="pb-4 border-b border-border/60">
                    <div className="mb-2 text-[10px] text-muted-foreground">{t("v3.cockpit.archive.legacyReadonly")}</div>
                    <h3 className="text-xl font-semibold text-foreground tracking-tight">{archivedDetail.title}</h3>
                    <div className="mt-2.5 flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                      {archivedDetail.state && <span className={cn("font-medium", taskStateTone[archivedDetail.state])}>{archivedDetail.state}</span>}
                      {archivedDetail.completed_at && <span>{t("v3.cockpit.archive.completedAt", { date: formatDateTime(archivedDetail.completed_at) })}</span>}
                      {archivedDetail.phase && <span>{t("v3.cockpit.archive.phase", { phase: archivedDetail.phase })}</span>}
                    </div>
                  </div>
                  {archivedDetail.final_summary && <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.finalSummary")}</h4><p className="whitespace-pre-wrap text-sm leading-relaxed text-muted-foreground">{archivedDetail.final_summary}</p></div>}
                  {archivedDetail.file_links.length > 0 && <div className="rounded-lg border border-border/40 bg-card/30 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.files")}</h4><div className="space-y-1">{archivedDetail.file_links.map((relativePath) => <button key={relativePath} type="button" className="block break-all text-left text-xs text-blue-600 hover:underline" onClick={async () => { if (!activeProjectPath) return; setLegacyFileError(null); try { await openLegacyFile?.(activeProjectPath, relativePath); } catch (err) { setLegacyFileError(err instanceof Error ? err.message : String(err)); } }}>{relativePath}</button>)}</div>{legacyFileError && <div className="mt-2 text-xs text-destructive">{legacyFileError}</div>}</div>}
                  {archivedDetail.warnings.length > 0 && <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-4"><h4 className="mb-2 text-sm font-semibold">{t("v3.cockpit.archive.warnings")}</h4><ul className="list-disc space-y-1 pl-4 text-xs text-amber-700 dark:text-amber-400">{archivedDetail.warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul></div>}
                </>
              )}
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

      {showCreateTask && renderCreateTaskDialog()}

      {forceCloseDialogOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4" onClick={() => !completePending && setForceCloseDialogOpen(false)}>
          <div role="dialog" aria-modal="true" aria-labelledby="v3-force-close-title" className="w-full max-w-lg space-y-4 rounded-lg border border-border bg-background p-6 shadow-xl" onClick={(event) => event.stopPropagation()}>
            <div>
              <h3 id="v3-force-close-title" className="text-lg font-semibold">{t("v3.cockpit.archive.forceTitle")}</h3>
              <p className="mt-1 text-sm text-muted-foreground">{t("v3.cockpit.archive.forceDetail")}</p>
            </div>
            <label className="block space-y-1">
              <span className="text-sm font-medium">{t("v3.cockpit.archive.reasonLabel")}</span>
              <textarea autoFocus value={forceCloseReason} onChange={(event) => { setForceCloseReason(event.target.value); setForceCloseValidationError(null); }} className="min-h-24 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" placeholder={t("v3.cockpit.archive.reasonPlaceholder")} disabled={completePending} />
            </label>
            {forceCloseValidationError && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{forceCloseValidationError}</div>}
            {completeError && <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">{completeError}</div>}
            <div className="flex justify-end gap-2">
              <Button variant="outline" disabled={completePending} onClick={() => setForceCloseDialogOpen(false)}>{t("v3.cockpit.archive.forceCancel")}</Button>
              <Button variant="destructive" disabled={completePending} onClick={() => void submitForceClose()}><Archive className="mr-1.5 h-3.5 w-3.5" />{t(completePending ? "v3.cockpit.archive.archiving" : "v3.cockpit.archive.forceConfirmAction")}</Button>
            </div>
          </div>
        </div>
      )}

      {/* Setup / Settings Modal */}
      {sourceMode === "production" && projectSettingsApi && showSettingsModal && (
        <ProjectSetupModal
          mode={projectSettings?.status === "missing" ? "setup" : "settings"}
          onClose={() => setShowSettingsModal(false)}
          onSubmit={updateProjectSettings}
          onSync={() => syncAgentSpecs(false)}
          onForceSync={() => syncAgentSpecs(true)}
          initialLanguage={projectSettings?.settings?.output_language}
          initialTools={projectSettings?.settings?.agent_spec_targets}
          initialGitUrl={projectSettings?.settings?.repository_remote_url ?? ""}
          expectedRevision={projectSettings?.settings?.revision ?? 0}
          agentSpecs={agentSpecs}
          pending={settingsLoading || specsLoading}
          error={settingsError || specsError}
        />
      )}
      </div>
    </div>
    </>
  );
}
