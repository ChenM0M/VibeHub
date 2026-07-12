import { WarningList } from "@/v3/components/common/WarningList";
import { ErrorList } from "@/v3/components/common/ErrorList";
import { EvidenceLink } from "@/v3/components/common/EvidenceLink";
import { CriterionBadge } from "@/v3/components/common/CriterionBadge";
import { GitBranch, Database, Layers, ShieldCheck, ListTodo, Clock, FolderTree, ChevronRight } from "lucide-react";
import type { V3View } from "@/v3/stores/v3Store";
import type { ProjectOverviewView } from "@/v3/contracts/generated/project-overview-view";

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

const taskStateLabel: Record<string, string> = {
  planned: "已规划",
  active: "进行中",
  blocked: "阻塞",
  review: "审查中",
  completed: "已完成",
  cancelled: "已取消",
};

const riskLabel: Record<string, string> = {
  none: "无",
  low: "低",
  medium: "中",
  high: "高",
  critical: "严重",
};

const repoStateLabel: Record<string, string> = {
  available: "可用",
  not_repository: "非仓库",
  unavailable: "不可用",
};

const modelStateLabel: Record<string, string> = {
  uninitialized: "未初始化",
  rebuilding: "重建中",
  ready: "就绪",
  error: "错误",
};

const protocolStateLabel: Record<string, string> = {
  complete: "完整",
  partial: "部分",
  gapped: "有缺口",
  unknown: "未知",
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
  return (
    <div className="space-y-8 pb-8">
      <WarningList warnings={data.warnings} />
      <ErrorList errors={data.errors} />

      {/* 项目信息卡片网格 — 每个可点击进入详细视图 */}
      <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-4">
        <DrillCard icon={<GitBranch className="h-4 w-4" />} title="仓库" >
          <div className="space-y-2">
            <Field label="状态">{repoStateLabel[data.repository.state] ?? data.repository.state}</Field>
            {data.repository.branch && <Field label="分支"><span className="font-mono">{data.repository.branch}</span></Field>}
            {data.repository.head && <Field label="HEAD"><span className="font-mono">{data.repository.head}</span></Field>}
            <Field label="工作树">{data.repository.worktree_count}</Field>
            <Field label="改动">{data.repository.dirty === null ? "—" : data.repository.dirty ? "是" : "否"}</Field>
          </div>
        </DrillCard>

        <DrillCard icon={<Database className="h-4 w-4" />} title="模型">
          <div className="space-y-2">
            <Field label="状态">{modelStateLabel[data.model.state] ?? data.model.state}</Field>
            <Field label="已索引文件">{data.model.indexed_files}</Field>
            {data.model.last_evidence_at && <Field label="最后证据">{new Date(data.model.last_evidence_at).toLocaleString()}</Field>}
          </div>
        </DrillCard>

        <DrillCard
          icon={<Layers className="h-4 w-4" />}
          title="架构"
          detail={`${data.architecture.modules} 模块`}
          onClick={() => onDrillIn("architecture-map", "架构地图")}
        >
          <div className="space-y-2">
            <Field label="声明文档">{data.architecture.declared_docs}</Field>
            <Field label="关系数">{data.architecture.relationships}</Field>
            <Field label="置信度">{(data.architecture.confidence * 100).toFixed(0)}%</Field>
            <EvidenceLink evidenceRefs={data.architecture.evidence_refs} />
          </div>
        </DrillCard>

        <DrillCard icon={<ShieldCheck className="h-4 w-4" />} title="协议覆盖">
          <div className="space-y-2">
            <Field label="状态">{protocolStateLabel[data.protocol_coverage.state] ?? data.protocol_coverage.state}</Field>
            <Field label="已开启会话">{data.protocol_coverage.opened_sessions}</Field>
            <Field label="已关闭会话">{data.protocol_coverage.closed_sessions}</Field>
            <Field label="缺口">{data.protocol_coverage.gaps}</Field>
          </div>
        </DrillCard>
      </div>

      {/* 结构浏览入口 */}
      <DrillCard
        icon={<FolderTree className="h-4 w-4" />}
        title="结构浏览"
        detail="查看项目文件树与模块关系"
        onClick={() => onDrillIn("structure-explorer", "结构浏览")}
      >
        <p className="text-sm text-muted-foreground">浏览项目的文件和模块结构，查看 Git 状态和证据引用</p>
      </DrillCard>

      {/* 活跃任务 — 点击进入任务时间线 */}
      <section>
        <div className="mb-3 flex items-center gap-2">
          <ListTodo className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-semibold">活跃任务</span>
          <span className="text-xs text-muted-foreground">({data.active_tasks.length})</span>
        </div>
        {data.active_tasks.length === 0 ? (
          <div className="border border-dashed border-border px-6 py-12 text-center">
            <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center border border-border text-muted-foreground">
              <ListTodo className="h-4 w-4" />
            </div>
            <div className="text-sm font-medium">暂无活跃任务</div>
          </div>
        ) : (
          <div className="divide-y divide-border/70 border-y border-border/70">
            {data.active_tasks.map((task) => (
              <button
                key={task.task_id}
                type="button"
                onClick={() => onDrillIn("task-timeline", `任务：${task.title}`)}
                className="group flex w-full items-start justify-between gap-3 py-3 text-left transition-colors hover:bg-muted/10"
              >
                <div className="min-w-0 flex-1 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium">{task.title}</span>
                    <span className={"text-xs font-medium " + (taskStateTone[task.state] ?? "")}>
                      {taskStateLabel[task.state] ?? task.state}
                    </span>
                    <span className={"text-xs " + (riskTone[task.risk_level] ?? "")}>
                      风险：{riskLabel[task.risk_level] ?? task.risk_level}
                    </span>
                  </div>
                  <div className="font-mono text-[11px] text-muted-foreground">{task.task_id}</div>
                  <div className="flex flex-wrap items-center gap-1">
                    {task.criteria.map((c) => (
                      <CriterionBadge key={c.criterion_id} criterion={c} />
                    ))}
                  </div>
                </div>
                <div className="shrink-0 text-right">
                  <div className="text-xs text-muted-foreground">{task.active_sessions} 个会话</div>
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
        title="全局时间线"
        detail="查看事件流"
        onClick={() => onDrillIn("global-timeline", "全局时间线")}
      >
        <p className="text-sm text-muted-foreground">按时间顺序查看项目事件流，筛选不同类型的事件</p>
      </DrillCard>
    </div>
  );
}
