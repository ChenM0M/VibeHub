import { useState } from "react";
import { AlertTriangle, FolderCog, RefreshCw, Rocket } from "lucide-react";
import { cn } from "@/lib/utils";
import type { V3BootstrapResult, V3ProjectLayoutStatus } from "@/types";
import type { V3LifecycleAction } from "@/v3/stores/v3Store";

interface ProjectLifecycleModalProps {
  projectPath: string;
  status: V3ProjectLayoutStatus | null;
  loading: boolean;
  error: string | null;
  action: V3LifecycleAction | null;
  actionError: string | null;
  result: V3BootstrapResult | null;
  onInspect: () => void;
  onRunAction: (action: V3LifecycleAction) => void;
  onBack: () => void;
}

const actionForState: Partial<Record<V3ProjectLayoutStatus["state"], V3LifecycleAction>> = {
  absent: "initialize",
  v2: "migrate",
  migration_interrupted: "recover",
};

const actionLabel: Record<V3LifecycleAction, string> = {
  initialize: "初始化 V3",
  migrate: "迁移到 V3",
  recover: "恢复迁移",
};

export function ProjectLifecycleModal({ projectPath, status, loading, error, action, actionError, result, onInspect, onRunAction, onBack }: ProjectLifecycleModalProps) {
  const [migrationConfirmed, setMigrationConfirmed] = useState(false);
  const nextAction = status ? actionForState[status.state] ?? null : null;
  const busy = loading || action !== null;
  const canRun = nextAction !== null && !busy && (nextAction !== "migrate" || migrationConfirmed);

  return (
    <div className="absolute inset-0 z-[100] flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="relative z-10 flex max-h-[95%] w-[560px] max-w-[95vw] flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center gap-2 border-b border-border/50 bg-muted/20 px-6 py-4">
          <FolderCog className="h-5 w-5 text-blue-500" />
          <h2 className="text-lg font-semibold tracking-tight">V3 项目准备</h2>
        </div>
        <div className="flex-1 space-y-5 overflow-y-auto p-6">
          <div>
            <div className="text-xs text-muted-foreground">项目路径</div>
            <div className="mt-1 break-all rounded border border-border/50 bg-muted/20 p-2 font-mono text-xs">{projectPath}</div>
          </div>

          {loading && !status && <div className="text-sm text-muted-foreground">正在检查项目布局…</div>}
          {error && <div className="rounded border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">{error}</div>}

          {status && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className="rounded border border-border/60 bg-muted/30 px-2 py-0.5 font-mono text-xs">{status.state}</span>
                {status.schema_version != null && <span className="text-xs text-muted-foreground">schema v{status.schema_version}</span>}
              </div>
              <p className="text-sm leading-relaxed">{status.message}</p>
              {status.recommended_action && <p className="text-xs text-muted-foreground">后端建议：{status.recommended_action}</p>}

              {status.state === "absent" && <div className="rounded border border-blue-500/20 bg-blue-500/5 p-3 text-xs text-muted-foreground">将在当前项目内创建 V3 marker 及 events、projections、indexes、runtime 基础目录；不会配置 Git、语言、Agent adapter 或创建任务。</div>}
              {status.state === "v2" && (
                <label className="flex cursor-pointer items-start gap-2 rounded border border-amber-500/30 bg-amber-500/5 p-3 text-xs">
                  <input type="checkbox" checked={migrationConfirmed} onChange={(event) => setMigrationConfirmed(event.target.checked)} className="mt-0.5" />
                  <span>我理解现有 V2 数据将原样保存为项目内的只读 legacy-v2 归档，不会转换为 V3 domain。</span>
                </label>
              )}
              {status.state === "migration_interrupted" && <div className="rounded border border-amber-500/30 bg-amber-500/5 p-3 text-xs text-muted-foreground">后端将依据磁盘状态安全地向前完成或回滚；界面不会预判恢复方向。</div>}
              {status.state === "conflict" && <div className="flex items-start gap-2 rounded border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive"><AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />检测到冲突布局。为防止覆盖数据，此界面不会提供强制初始化或迁移。</div>}
            </div>
          )}

          {actionError && <div className="rounded border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">{actionError}</div>}
          {result && <div className="rounded border border-emerald-500/30 bg-emerald-500/5 p-3 text-xs"><div className="font-medium">{result.status}</div><div className="mt-1 text-muted-foreground">归档 legacy-v2：{result.archived_legacy_v2 ? "是" : "否"}</div>{result.created_paths.length > 0 && <div className="mt-1 break-all text-muted-foreground">创建：{result.created_paths.join("、")}</div>}</div>}
        </div>

        <div className="flex justify-end gap-3 border-t border-border/50 bg-muted/10 px-6 py-4">
          <button type="button" onClick={onBack} disabled={action !== null} className="rounded-md px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50">返回</button>
          <button type="button" onClick={onInspect} disabled={busy} className="flex items-center gap-2 rounded-md border border-border px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50"><RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />重新检查</button>
          {nextAction && <button type="button" onClick={() => onRunAction(nextAction)} disabled={!canRun} className="flex items-center gap-2 rounded-md bg-foreground px-5 py-2 text-sm font-medium text-background shadow-sm hover:bg-foreground/90 disabled:cursor-not-allowed disabled:opacity-50"><Rocket className="h-4 w-4" />{action === nextAction ? "处理中…" : actionLabel[nextAction]}</button>}
        </div>
      </div>
    </div>
  );
}
