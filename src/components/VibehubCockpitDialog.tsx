import { useEffect, useState, type ReactNode } from 'react';
import { AlertCircle, Bot, FileText, GitBranch, PackagePlus, Play, RefreshCw, SearchCheck, Wrench } from 'lucide-react';
import { Project, VibehubCockpitStatus, VibehubFileStatus } from '@/types';
import { tauriApi } from '@/services/tauri';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';

interface VibehubCockpitDialogProps {
    isOpen: boolean;
    onClose: () => void;
    project: Project | null;
}

type ActionState = {
    message: string;
    error: boolean;
};

type CockpitAction = 'init' | 'start-task' | 'build-context' | 'continue' | 'agent-sync' | 'review' | 'handoff';

export function VibehubCockpitDialog({ isOpen, onClose, project }: VibehubCockpitDialogProps) {
    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="max-w-2xl">
                <DialogHeader>
                    <DialogTitle>VibeHub v2 Cockpit</DialogTitle>
                    <DialogDescription className="break-all">
                        {project ? project.path : ''}
                    </DialogDescription>
                </DialogHeader>

                <VibehubCockpitContent project={project} enabled={isOpen} />
            </DialogContent>
        </Dialog>
    );
}

interface VibehubCockpitContentProps {
    project: Project | null;
    enabled?: boolean;
    showOverview?: boolean;
}

export function VibehubCockpitContent({ project, enabled = true, showOverview = false }: VibehubCockpitContentProps) {
    const [status, setStatus] = useState<VibehubCockpitStatus | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [actionState, setActionState] = useState<ActionState | null>(null);
    const [runningAction, setRunningAction] = useState<CockpitAction | null>(null);

    const loadStatus = async (clearActionState = true) => {
        if (!project) return;
        setIsLoading(true);
        try {
            const nextStatus = await tauriApi.vibehubReadCockpitStatus(project.path);
            setStatus(nextStatus);
            if (clearActionState) {
                setActionState(null);
            }
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        if (enabled && project) {
            loadStatus();
        } else {
            setStatus(null);
            setActionState(null);
            setRunningAction(null);
        }
    }, [enabled, project?.path]);

    const hasActiveContextTarget = Boolean(
        status?.initialized && status.current_task_id && status.current_run_id && status.current_phase
    );
    const actionDisabled = isLoading || !!runningAction || !project;

    const runAction = async (action: CockpitAction) => {
        if (!project) return;
        setRunningAction(action);
        try {
            if (action === 'init') {
                const result = await tauriApi.vibehubInit(project.path);
                const createdCount = result.created_files.length;
                const keptCount = result.skipped_existing_files.length;
                const errorSuffix = result.errors.length ? ` ${result.errors.length} issue(s) reported.` : '';
                setActionState({
                    message: `Initialized .vibehub at ${result.vibehub_root}. Created ${createdCount} file(s), kept ${keptCount} existing file(s).${errorSuffix}`,
                    error: result.errors.length > 0,
                });
            } else if (action === 'start-task') {
                const result = await tauriApi.vibehubStartTask(project.path);
                setActionState({
                    message: `Started task ${result.task_id} with run ${result.run_id} in ${result.phase} phase.`,
                    error: false,
                });
            } else if (action === 'build-context') {
                if (!status?.current_task_id || !status.current_run_id || !status.current_phase) {
                    setActionState({
                        message: 'Cannot build context because no active task, run, and phase are available.',
                        error: true,
                    });
                    return;
                }
                const result = await tauriApi.vibehubBuildContextPack(
                    project.path,
                    status.current_task_id,
                    status.current_run_id,
                    status.current_phase
                );
                setActionState({
                    message: `Context pack built: ${result.pack_path}. Included ${result.included_count}, missing ${result.missing_count}, excluded ${result.excluded_count}.`,
                    error: result.missing_count > 0,
                });
            } else if (action === 'continue') {
                const result = await tauriApi.vibehubGenerateAgentView(project.path);
                setActionState({ message: `Continue context updated: ${result.current_path}`, error: false });
            } else if (action === 'agent-sync') {
                const result = await tauriApi.vibehubSyncAgentAdapter(project.path);
                const conflictSuffix = result.conflict_files.length
                    ? ` Conflict: ${result.conflict_files.map((file) => `${file.path} (${file.reason})`).join('; ')}`
                    : '';
                setActionState({
                    message: `${result.summary}${conflictSuffix}`,
                    error: result.conflict_files.length > 0,
                });
            } else if (action === 'review') {
                const result = await tauriApi.vibehubGenerateReviewEvidence(project.path);
                setActionState({
                    message: `Review evidence written: ${result.review_path}. ${result.changed_files_count} changed file(s) captured.`,
                    error: false,
                });
            } else {
                const result = await tauriApi.vibehubBuildHandoff(project.path);
                const missingSuffix = result.missing_required_sections.length
                    ? ` Missing section(s): ${result.missing_required_sections.join(', ')}.`
                    : '';
                setActionState({
                    message: `Handoff ${result.complete ? 'built' : 'built but incomplete'}: ${result.handoff_path}.${missingSuffix}`,
                    error: !result.complete,
                });
            }
            await loadStatus(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setRunningAction(null);
        }
    };

    const taskLabel = status?.current_task_id
        ? `${status.current_task_id}${status.current_task_title ? ` - ${status.current_task_title}` : ''}`
        : 'No current task';
    const unavailableReason = !status
        ? 'Loading VibeHub status.'
        : !status.initialized
            ? 'Initialize this project before running task and artifact actions.'
            : !hasActiveContextTarget
                ? 'Start a task before building context, continue context, review evidence, or handoff/recover report.'
                : null;

    return (
        <div className="space-y-4">
            {showOverview && project && (
                <div className="grid gap-3 md:grid-cols-[1.2fr_0.8fr]">
                    <div className="rounded-md border bg-muted/20 p-4">
                        <div className="text-xs text-muted-foreground">Project overview</div>
                        <div className="mt-1 text-lg font-semibold">{project.name}</div>
                        <div className="mt-1 break-all text-xs text-muted-foreground">{project.path}</div>
                        <div className="mt-3 flex flex-wrap gap-2 text-xs">
                            <Badge variant="outline">{project.project_type}</Badge>
                            {project.metadata.git_branch && (
                                <Badge variant="secondary">
                                    <GitBranch className="mr-1 h-3 w-3" />
                                    {project.metadata.git_branch}
                                </Badge>
                            )}
                        </div>
                    </div>
                    <div className="rounded-md border bg-muted/20 p-4">
                        <div className="text-xs text-muted-foreground">4+1 flow</div>
                        <div className="mt-2 grid grid-cols-2 gap-2 text-xs">
                            {['Plan', 'Context', 'Run', 'Review', 'Handoff'].map((item) => (
                                <div key={item} className="rounded border bg-background px-2 py-1.5 font-medium">
                                    {item}
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            )}

            <div className="flex items-center justify-between gap-3 border-b pb-3">
                <div className="min-w-0">
                    <div className="text-sm text-muted-foreground">Current task/run</div>
                    <div className="truncate font-medium">{isLoading ? 'Loading...' : taskLabel}</div>
                    {status?.current_run_id && (
                        <div className="mt-0.5 truncate text-xs text-muted-foreground">Run: {status.current_run_id}</div>
                    )}
                </div>
                <Button variant="outline" size="sm" onClick={() => loadStatus()} disabled={isLoading || !project}>
                    <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                    Refresh
                </Button>
            </div>

            {status && (
                <>
                    <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
                        <Metric label="Mode" value={status.current_mode || 'unknown'} />
                        <Metric label="Phase" value={status.current_phase || 'none'} />
                        <Metric label="Phase status" value={status.phase_status || 'unknown'} />
                        <Metric label="Observability" value={status.observability_level || 'best_effort'} />
                    </div>

                    <div className="grid gap-3 md:grid-cols-3">
                        <StatusPanel
                            icon={<GitBranch className="h-4 w-4" />}
                            label="Git dirty state"
                            value={gitStatusLabel(status)}
                            tone={status.git_dirty ? 'warn' : 'ok'}
                        />
                        <FileStatusPanel
                            icon={<FileText className="h-4 w-4" />}
                            label="Context pack"
                            fileStatus={status.context_pack_status}
                        />
                        <FileStatusPanel
                            icon={<FileText className="h-4 w-4" />}
                            label="Handoff"
                            fileStatus={status.handoff_status}
                        />
                    </div>

                    <StatusPanel
                        icon={<SearchCheck className="h-4 w-4" />}
                        label="Review evidence"
                        value={hasActiveContextTarget ? 'Available after Review Evidence action' : 'Needs active task/run'}
                        tone={hasActiveContextTarget ? 'neutral' : 'warn'}
                    />

                    <Notice
                        error={false}
                        message="Observability is limited to cockpit status and generated artifacts; runtime observation is out of scope."
                    />

                    {!status.initialized && (
                        <Notice error message=".vibehub is not initialized for this project." />
                    )}

                    {status.warnings.map((warning) => (
                        <Notice key={warning} error message={warning} />
                    ))}
                </>
            )}

            {actionState && <Notice error={actionState.error} message={actionState.message} />}

            <div className="space-y-2 border-t pt-4">
                <div className="flex flex-wrap gap-2">
                    <Button onClick={() => runAction('init')} disabled={actionDisabled || !!status?.initialized}>
                        <Wrench className="mr-2 h-4 w-4" />
                        Initialize
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('start-task')}
                        disabled={actionDisabled || !status?.initialized}
                    >
                        <Play className="mr-2 h-4 w-4" />
                        Start Task
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('build-context')}
                        disabled={actionDisabled || !hasActiveContextTarget}
                    >
                        <PackagePlus className="mr-2 h-4 w-4" />
                        Build Context
                    </Button>
                    <Button onClick={() => runAction('continue')} disabled={actionDisabled || !hasActiveContextTarget}>
                        <Play className="mr-2 h-4 w-4" />
                        Continue
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('agent-sync')}
                        disabled={actionDisabled || !status?.initialized}
                    >
                        <Bot className="mr-2 h-4 w-4" />
                        Update AGENTS.md
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('review')}
                        disabled={actionDisabled || !hasActiveContextTarget}
                    >
                        <SearchCheck className="mr-2 h-4 w-4" />
                        Review Evidence
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('handoff')}
                        disabled={actionDisabled || !hasActiveContextTarget}
                    >
                        <FileText className="mr-2 h-4 w-4" />
                        Build Handoff/Recover Report
                    </Button>
                </div>
                {unavailableReason && (
                    <div className="text-xs text-muted-foreground">{unavailableReason}</div>
                )}
            </div>
        </div>
    );
}

function Metric({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-md border bg-muted/20 p-3">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1 truncate text-sm font-medium">{value}</div>
        </div>
    );
}

function StatusPanel({
    icon,
    label,
    value,
    tone,
}: {
    icon: ReactNode;
    label: string;
    value: string;
    tone: 'ok' | 'warn' | 'neutral';
}) {
    const variant = tone === 'warn' ? 'destructive' : tone === 'ok' ? 'secondary' : 'outline';
    return (
        <div className="rounded-md border p-3">
            <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
                {icon}
                {label}
            </div>
            <Badge variant={variant}>{value}</Badge>
        </div>
    );
}

function FileStatusPanel({
    icon,
    label,
    fileStatus,
}: {
    icon: ReactNode;
    label: string;
    fileStatus: VibehubFileStatus;
}) {
    const tone = fileStatus.exists && fileStatus.stale !== true ? 'ok' : fileStatus.configured ? 'warn' : 'neutral';
    return (
        <StatusPanel
            icon={icon}
            label={label}
            value={`${fileStatus.status}${fileStatus.exists ? '' : ' / missing'}`}
            tone={tone}
        />
    );
}

function Notice({ message, error }: { message: string; error: boolean }) {
    return (
        <div className={`flex items-start gap-2 rounded-md border p-3 text-sm ${error ? 'border-destructive/40 text-destructive' : 'border-border text-muted-foreground'}`}>
            <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
            <span className="break-all">{message}</span>
        </div>
    );
}

function gitStatusLabel(status: VibehubCockpitStatus) {
    if (!status.git_available) return 'Git unavailable';
    if (status.git_dirty == null) return 'unknown';
    return status.git_dirty ? `dirty (${status.git_changed_files_count || 0})` : 'clean';
}
