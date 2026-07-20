import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { BlockerDetailsPanel } from "@/v3/components/common/BlockerDetailsPanel";
import { GitBranch, Database, Layers, ShieldCheck, ListTodo, Clock, FolderTree, ChevronRight } from "lucide-react";
import type { V3View } from "@/v3/stores/v3Store";
import type { ProjectOverviewView } from "@/v3/contracts/generated/project-overview-view";
import { useTranslation } from "react-i18next";

const riskTone: Record<string, string> = {
  none: "",
  low: "text-emerald-600 dark:text-emerald-400",
  medium: "text-amber-600 dark:text-amber-400",
  high: "text-orange-600 dark:text-orange-400",
  critical: "text-red-600 dark:text-red-400",
};

const taskStateTone: Record<string, string> = {
  planned: "text-muted-foreground",
  active: "text-blue-600 dark:text-blue-400",
  blocked: "text-orange-600 dark:text-orange-400",
  review: "text-purple-600 dark:text-purple-400",
  completed: "text-emerald-600 dark:text-emerald-400",
  cancelled: "text-red-600 dark:text-red-400",
};

function DrillCard({ icon, title, detail, onClick, children }: {
  icon: React.ReactNode;
  title: string;
  detail?: string;
  onClick?: () => void;
  children?: React.ReactNode;
}) {
  const Tag = onClick ? "button" : "div";
  return (
    <Tag
      type={onClick ? "button" : undefined}
      onClick={onClick}
      className={
        "block w-full border border-border/70 p-4 text-left transition-colors " +
        (onClick ? "hover:border-foreground/30 hover:bg-muted/20" : "")
      }
    >
      <div className="mb-3 flex items-center gap-2">
        <span className="text-muted-foreground">{icon}</span>
        <span className="text-sm font-semibold">{title}</span>
        {detail && <span className="ml-auto text-xs text-muted-foreground">{detail}</span>}
        {onClick && <ChevronRight className="h-4 w-4 text-muted-foreground/50" />}
      </div>
      {children}
    </Tag>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-2 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="min-w-0 truncate text-right">{children}</span>
    </div>
  );
}

interface ProjectOverviewProps {
  data: ProjectOverviewView;
  onDrillIn: (view: V3View, label: string) => void;
}

export function ProjectOverview({ data, onDrillIn }: ProjectOverviewProps) {
  const { t, i18n } = useTranslation();
  const stateLabel = (group: string, value: string) => t(`v3.projectOverview.${group}.${value}`, { defaultValue: value });
  return (
    <div className="space-y-8 pb-8">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      {/* 项目信息卡片网格 — 每个可点击进入详细视图 */}
      <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-4">
        <DrillCard icon={<GitBranch className="h-4 w-4" />} title={t("v3.projectOverview.repository")} >
          <div className="space-y-2">
            <Field label={t("v3.projectOverview.status")}>{stateLabel("repoState", data.repository.state)}</Field>
            {data.repository.branch && <Field label={t("v3.projectOverview.branch")}><span className="font-mono">{data.repository.branch}</span></Field>}
            {data.repository.head && <Field label="HEAD"><span className="font-mono">{data.repository.head}</span></Field>}
            <Field label={t("v3.projectOverview.worktrees")}>{data.repository.worktree_count}</Field>
            <Field label={t("v3.projectOverview.changes")}>{data.repository.dirty === null ? "—" : data.repository.dirty ? t("v3.projectOverview.yes") : t("v3.projectOverview.no")}</Field>
            <Field label={t("v3.projectOverview.controlRoot")}><span className="font-mono" title={data.scopes.control_root}>{data.scopes.control_root}</span></Field>
            <Field label={t("v3.projectOverview.executionRoot")}><span className="font-mono" title={data.scopes.execution_root}>{data.scopes.execution_root}</span></Field>
          </div>
        </DrillCard>

        <DrillCard icon={<Database className="h-4 w-4" />} title={t("v3.projectOverview.model")}>
          <div className="space-y-2">
            <Field label={t("v3.projectOverview.status")}>{stateLabel("modelState", data.model.state)}</Field>
            <Field label={t("v3.projectOverview.indexedFiles")}>{data.model.indexed_files}</Field>
            {data.model.last_evidence_at && <Field label={t("v3.projectOverview.lastEvidence")}>{new Intl.DateTimeFormat(i18n.resolvedLanguage, { dateStyle: "medium", timeStyle: "medium" }).format(new Date(data.model.last_evidence_at))}</Field>}
          </div>
        </DrillCard>

        <DrillCard
          icon={<Layers className="h-4 w-4" />}
          title={t("v3.projectOverview.architecture")}
          detail={t("v3.projectOverview.moduleCount", { count: data.architecture.modules })}
          onClick={() => onDrillIn("architecture-map", t("v3.projectOverview.architectureMap"))}
        >
          <div className="space-y-2">
            <Field label={t("v3.projectOverview.declaredDocs")}>{data.architecture.declared_docs}</Field>
            <Field label={t("v3.projectOverview.relationships")}>{data.architecture.relationships}</Field>
            <Field label={t("v3.projectOverview.confidence")}>{(data.architecture.confidence * 100).toFixed(0)}%</Field>
            <EvidenceLink evidenceRefs={data.architecture.evidence_refs} />
          </div>
        </DrillCard>

        <DrillCard icon={<ShieldCheck className="h-4 w-4" />} title={t("v3.projectOverview.protocolCoverage")}>
          <div className="space-y-2">
            <Field label={t("v3.projectOverview.status")}>{stateLabel("protocolState", data.protocol_coverage.state)}</Field>
            <Field label={t("v3.projectOverview.openedSessions")}>{data.protocol_coverage.opened_sessions}</Field>
            <Field label={t("v3.projectOverview.closedSessions")}>{data.protocol_coverage.closed_sessions}</Field>
            <Field label={t("v3.projectOverview.gaps")}>{data.protocol_coverage.gaps}</Field>
          </div>
        </DrillCard>
      </div>

      {/* 结构浏览入口 */}
      <DrillCard
        icon={<FolderTree className="h-4 w-4" />}
        title={t("v3.projectOverview.structureExplorer")}
        detail={t("v3.projectOverview.structureDetail")}
        onClick={() => onDrillIn("structure-explorer", t("v3.projectOverview.structureExplorer"))}
      >
        <p className="text-sm text-muted-foreground">{t("v3.projectOverview.structureDescription")}</p>
      </DrillCard>

      {/* 活跃任务 — 点击进入任务时间线 */}
      <section>
        <div className="mb-3 flex items-center gap-2">
          <ListTodo className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-semibold">{t("v3.projectOverview.activeTasks")}</span>
          <span className="text-xs text-muted-foreground">({data.active_tasks.length})</span>
        </div>
        {data.active_tasks.length === 0 ? (
          <div className="border border-dashed border-border px-6 py-12 text-center">
            <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center border border-border text-muted-foreground">
              <ListTodo className="h-4 w-4" />
            </div>
            <div className="text-sm font-medium">{t("v3.projectOverview.noActiveTasks")}</div>
          </div>
        ) : (
          <div className="divide-y divide-border/70 border-y border-border/70">
            {data.active_tasks.map((task) => (
              <button
                key={task.task_id}
                type="button"
                onClick={() => onDrillIn("task-timeline", t("v3.projectOverview.taskLabel", { title: task.title }))}
                className="group flex w-full items-start justify-between gap-3 py-3 text-left transition-colors hover:bg-muted/10"
              >
                <div className="min-w-0 flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium">{task.title}</span>
                    <span className={"text-xs font-medium " + (taskStateTone[task.state] ?? "")}>
                      {t(`v3.timeline.taskState.${task.state}`, { defaultValue: task.state })}
                    </span>
                    <span className={"text-xs " + (riskTone[task.risk_level] ?? "")}>
                      {t("v3.projectOverview.risk", { level: t(`v3.cockpit.riskLevel.${task.risk_level}`, { defaultValue: task.risk_level }) })}
                    </span>
                  </div>
                  <div className="font-mono text-[11px] text-muted-foreground">{task.task_id}</div>
                  <div className="mb-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">{t("v3.projectOverview.acceptance")}</div>
                  <div className="flex flex-wrap items-center gap-1">
                    {task.criteria.map((c) => (
                      <CriterionBadge key={c.criterion_id} criterion={c} />
                    ))}
                  </div>
                  <BlockerDetailsPanel blockers={task.blocker_details} compact />
                </div>
                <div className="shrink-0 text-right">
                  <div className="text-xs text-muted-foreground">{t("v3.projectOverview.sessionCount", { count: task.active_sessions })}</div>
                  <ChevronRight className="ml-auto mt-1 h-4 w-4 text-muted-foreground/50 transition group-hover:text-foreground" />
                </div>
              </button>
            ))}
          </div>
        )}
      </section>

      {/* 时间线入口 */}
      <DrillCard
        icon={<Clock className="h-4 w-4" />}
        title={t("v3.projectOverview.globalTimeline")}
        detail={t("v3.projectOverview.viewEventStream")}
        onClick={() => onDrillIn("global-timeline", t("v3.projectOverview.globalTimeline"))}
      >
        <p className="text-sm text-muted-foreground">{t("v3.projectOverview.timelineDescription")}</p>
      </DrillCard>
    </div>
  );
}
