import { useEffect, useState, type ReactNode } from 'react';
import { AlertCircle, FileText, GitBranch, Play, RefreshCw, RotateCcw, SearchCheck } from 'lucide-react';
import { Project, VibehubCockpitStatus, VibehubFileStatus } from '@/types';
import { tauriApi } from '@/services/tauri';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
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

export function VibehubCockpitDialog({ isOpen, onClose, project }: VibehubCockpitDialogProps) {
    const [status, setStatus] = useState<VibehubCockpitStatus | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [actionState, setActionState] = useState<ActionState | null>(null);
    const [runningAction, setRunningAction] = useState<string | null>(null);

    const loadStatus = async () => {
        if (!project) return;
        setIsLoading(true);
        try {
            const nextStatus = await tauriApi.vibehubReadCockpitStatus(project.path);
            setStatus(nextStatus);
            setActionState(null);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        if (isOpen) {
            loadStatus();
        } else {
            setStatus(null);
            setActionState(null);
            setRunningAction(null);
        }
    }, [isOpen, project?.path]);

    const runAction = async (action: 'continue' | 'review' | 'recover') => {
        if (!project) return;
        setRunningAction(action);
        try {
            if (action === 'continue') {
                const result = await tauriApi.vibehubGenerateAgentView(project.path);
                setActionState({ message: `Agent view updated: ${result.current_path}`, error: false });
            } else if (action === 'review') {
                const result = await tauriApi.vibehubGenerateReviewEvidence(project.path);
                setActionState({ message: `Review evidence written: ${result.review_path}`, error: false });
            } else {
                const result = await tauriApi.vibehubBuildHandoff(project.path);
                setActionState({ message: `Handoff ${result.complete ? 'available' : 'incomplete'}: ${result.handoff_path}`, error: false });
            }
            await loadStatus();
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setRunningAction(null);
        }
    };

    const taskLabel = status?.current_task_id
        ? `${status.current_task_id}${status.current_task_title ? ` - ${status.current_task_title}` : ''}`
        : 'No current task';

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="max-w-2xl">
                <DialogHeader>
                    <DialogTitle>VibeHub v2 Cockpit</DialogTitle>
                    <DialogDescription className="break-all">
                        {project ? project.path : ''}
                    </DialogDescription>
                </DialogHeader>

                <div className="space-y-4">
                    <div className="flex items-center justify-between gap-3 border-b pb-3">
                        <div className="min-w-0">
                            <div className="text-sm text-muted-foreground">Current task</div>
                            <div className="truncate font-medium">{isLoading ? 'Loading...' : taskLabel}</div>
                        </div>
                        <Button variant="outline" size="sm" onClick={loadStatus} disabled={isLoading || !project}>
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

                            {!status.initialized && (
                                <Notice error message=".vibehub is not initialized for this project." />
                            )}

                            {status.warnings.map((warning) => (
                                <Notice key={warning} error message={warning} />
                            ))}
                        </>
                    )}

                    {actionState && <Notice error={actionState.error} message={actionState.message} />}
                </div>

                <DialogFooter className="gap-2 sm:space-x-0">
                    <Button
                        variant="outline"
                        onClick={() => runAction('recover')}
                        disabled={!status?.initialized || !!runningAction}
                    >
                        <RotateCcw className="mr-2 h-4 w-4" />
                        Recover
                    </Button>
                    <Button
                        variant="outline"
                        onClick={() => runAction('review')}
                        disabled={!status?.initialized || !!runningAction}
                    >
                        <SearchCheck className="mr-2 h-4 w-4" />
                        Review
                    </Button>
                    <Button onClick={() => runAction('continue')} disabled={!status?.initialized || !!runningAction}>
                        <Play className="mr-2 h-4 w-4" />
                        Continue
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
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
