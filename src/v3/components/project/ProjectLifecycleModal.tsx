import { useEffect, useState } from "react";
import { AlertTriangle, FolderCog, RefreshCw, Rocket } from "lucide-react";
import { cn } from "@/lib/utils";
import type { V3BootstrapResult, V3ProjectLayoutStatus, V3RepairCandidate, V3RepairResult } from "@/types";
import type { V3LifecycleAction } from "@/v3/stores/v3Store";
import { useTranslation } from "react-i18next";

interface ProjectLifecycleModalProps {
  projectPath: string;
  status: V3ProjectLayoutStatus | null;
  loading: boolean;
  error: string | null;
  action: V3LifecycleAction | null;
  actionError: string | null;
  result: V3BootstrapResult | V3RepairResult | null;
  repairCandidates: V3RepairCandidate[];
  onInspect: () => void;
  onRunAction: (action: V3LifecycleAction, taskId?: string) => void;
  onBack: () => void;
}

const actionForState: Partial<Record<V3ProjectLayoutStatus["state"], V3LifecycleAction>> = {
  absent: "initialize",
  v2: "migrate",
  migration_interrupted: "recover",
};

export function ProjectLifecycleModal({ projectPath, status, loading, error, action, actionError, result, repairCandidates, onInspect, onRunAction, onBack }: ProjectLifecycleModalProps) {
  const { t } = useTranslation();
  const [migrationConfirmed, setMigrationConfirmed] = useState(false);
  const [repairTaskId, setRepairTaskId] = useState("");
  useEffect(() => {
    if (!repairCandidates.some((candidate) => candidate.task_id === repairTaskId)) {
      setRepairTaskId(repairCandidates[0]?.task_id ?? "");
    }
  }, [repairCandidates, repairTaskId]);
  const nextAction = status
    ? status.state === "conflict" && repairCandidates.length > 0
      ? "repair"
      : actionForState[status.state] ?? null
    : null;
  const busy = loading || action !== null;
  const canRun = nextAction !== null
    && !busy
    && (nextAction !== "migrate" || migrationConfirmed)
    && (nextAction !== "repair" || repairTaskId.length > 0);

  return (
    <div className="absolute inset-0 z-[100] flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="relative z-10 flex max-h-[95%] w-[560px] max-w-[95vw] flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl animate-in fade-in zoom-in-95 duration-200">
        <div className="flex items-center gap-2 border-b border-border/50 bg-muted/20 px-6 py-4">
          <FolderCog className="h-5 w-5 text-blue-500" />
          <h2 className="text-lg font-semibold tracking-tight">{t("v3.lifecycle.title")}</h2>
        </div>
        <div className="flex-1 space-y-5 overflow-y-auto p-6">
          <div>
            <div className="text-xs text-muted-foreground">{t("v3.lifecycle.projectPath")}</div>
            <div className="mt-1 break-all rounded border border-border/50 bg-muted/20 p-2 font-mono text-xs">{projectPath}</div>
          </div>

          {loading && !status && <div className="text-sm text-muted-foreground">{t("v3.lifecycle.inspecting")}</div>}
          {error && <div className="rounded border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">{error}</div>}

          {status && (
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className="rounded border border-border/60 bg-muted/30 px-2 py-0.5 font-mono text-xs">{status.state}</span>
                {status.schema_version != null && <span className="text-xs text-muted-foreground">schema v{status.schema_version}</span>}
              </div>
              <p className="text-sm leading-relaxed">{status.message}</p>
              {status.recommended_action && <p className="text-xs text-muted-foreground">{t("v3.lifecycle.recommendation", { action: status.recommended_action })}</p>}

              {status.state === "absent" && <div className="rounded border border-blue-500/20 bg-blue-500/5 p-3 text-xs text-muted-foreground">{t("v3.lifecycle.absentHint")}</div>}
              {status.state === "v2" && (
                <label className="flex cursor-pointer items-start gap-2 rounded border border-amber-500/30 bg-amber-500/5 p-3 text-xs">
                  <input type="checkbox" checked={migrationConfirmed} onChange={(event) => setMigrationConfirmed(event.target.checked)} className="mt-0.5" />
                  <span>{t("v3.lifecycle.migrationConfirm")}</span>
                </label>
              )}
              {status.state === "migration_interrupted" && <div className="rounded border border-amber-500/30 bg-amber-500/5 p-3 text-xs text-muted-foreground">{t("v3.lifecycle.interruptedHint")}</div>}
              {status.state === "conflict" && repairCandidates.length === 0 && <div className="flex items-start gap-2 rounded border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive"><AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />{t("v3.lifecycle.noRepairCandidate")}</div>}
              {status.state === "conflict" && repairCandidates.length > 0 && <div className="space-y-3 rounded border border-amber-500/30 bg-amber-500/5 p-3 text-xs"><div className="flex items-start gap-2"><AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-600" /><div><div className="font-medium">{t("v3.lifecycle.repairTitle")}</div><p className="mt-1 text-muted-foreground">{t("v3.lifecycle.repairHint")}</p></div></div><label className="block space-y-1"><span className="font-medium">{t("v3.lifecycle.currentTaskAfterRepair")}</span><select value={repairTaskId} onChange={(event) => setRepairTaskId(event.target.value)} className="w-full rounded border border-border bg-background px-2 py-2 text-xs">{repairCandidates.map((candidate) => <option key={candidate.task_id} value={candidate.task_id}>{candidate.title || candidate.task_id} · {candidate.state}</option>)}</select></label></div>}
            </div>
          )}

          {actionError && <div className="rounded border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">{actionError}</div>}
          {result && <div className="rounded border border-emerald-500/30 bg-emerald-500/5 p-3 text-xs"><div className="font-medium">{result.status}</div>{"archived_legacy_v2" in result && <div className="mt-1 text-muted-foreground">{t("v3.lifecycle.archivedLegacy", { value: result.archived_legacy_v2 ? t("v3.projectOverview.yes") : t("v3.projectOverview.no") })}</div>}{"task_id" in result && <div className="mt-1 break-all text-muted-foreground">{t("v3.lifecycle.currentTask", { task: result.task_id })}</div>}{result.created_paths.length > 0 && <div className="mt-1 break-all text-muted-foreground">{t("v3.lifecycle.created", { paths: result.created_paths.join(t("v3.common.listSeparator")) })}</div>}</div>}
        </div>

        <div className="flex justify-end gap-3 border-t border-border/50 bg-muted/10 px-6 py-4">
          <button type="button" onClick={onBack} disabled={action !== null} className="rounded-md px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50">{t("v3.lifecycle.back")}</button>
          <button type="button" onClick={onInspect} disabled={busy} className="flex items-center gap-2 rounded-md border border-border px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50"><RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />{t("v3.lifecycle.reinspect")}</button>
          {nextAction && <button type="button" onClick={() => onRunAction(nextAction, nextAction === "repair" ? repairTaskId : undefined)} disabled={!canRun} className="flex items-center gap-2 rounded-md bg-foreground px-5 py-2 text-sm font-medium text-background shadow-sm hover:bg-foreground/90 disabled:cursor-not-allowed disabled:opacity-50"><Rocket className="h-4 w-4" />{t(action === nextAction ? "v3.lifecycle.processing" : `v3.lifecycle.action.${nextAction}`)}</button>}
        </div>
      </div>
    </div>
  );
}
