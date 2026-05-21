import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { AlertCircle, Bot, CheckCircle2, Code2, Eye, FileText, GitBranch, Lightbulb, RefreshCw, SearchCheck, ShieldCheck, Wrench, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AgentAdapterStatus, AgentTool, PhaseValidationResult, Project, ResearchStatus, VibehubCockpitStatus, VibehubContextViewData, VibehubDiffViewData, VibehubFileReadResult, VibehubFlowDetail, VibehubGitBranchesView, VibehubHandoffViewData, VibehubProjectDigest, VibehubReviewViewData, WorkspaceDriftReport } from '@/types';
import { tauriApi } from '@/services/tauri';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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

type RecommendedReadOnlyAction = {
    command: string;
    title: string;
    description: string;
};

type PreviewCandidate = { path: string; label: string; exists: boolean };
type DashboardDetail = 'phase' | 'git' | 'context' | 'output' | 'handoff' | 'review' | 'research' | 'evidence' | 'preview' | 'adapters';

const AGENT_TOOL_OPTIONS: Array<{ id: AgentTool; label: string }> = [
    { id: 'codex', label: 'Codex' },
    { id: 'claude_code', label: 'Claude Code' },
    { id: 'opencode', label: 'OpenCode' },
];
const LOCALE_OPTIONS = ['en', 'zh-CN', 'zh-TW'] as const;
const FLOW_STAGES = ['align', 'research', 'plan', 'implement', 'review'] as const;
type FlowStage = (typeof FLOW_STAGES)[number];
const MODE_STAGE_PHASES: Record<string, Partial<Record<FlowStage, string>>> = {
    yolo_drive: { align: 'align_lite', implement: 'implement', review: 'review_lite' },
    guided_drive: { align: 'align', plan: 'plan', implement: 'implement', review: 'review' },
    evidence_drive: { align: 'align', research: 'research', plan: 'plan', implement: 'implement', review: 'review' },
};

export function VibehubCockpitDialog({ isOpen, onClose, project }: VibehubCockpitDialogProps) {
    const { t } = useTranslation();

    return (
        <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
            <DialogContent className="max-w-5xl">
                <DialogHeader>
                    <DialogTitle>{t('vibehub.cockpit.title')}</DialogTitle>
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
    const { t, i18n } = useTranslation();
    const [status, setStatus] = useState<VibehubCockpitStatus | null>(null);
    const [adapterStatus, setAdapterStatus] = useState<AgentAdapterStatus | null>(null);
    const [driftReport, setDriftReport] = useState<WorkspaceDriftReport | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [actionState, setActionState] = useState<ActionState | null>(null);
    const [isInitializing, setIsInitializing] = useState(false);
    const [isSyncingAdapters, setIsSyncingAdapters] = useState(false);
    const [isSavingLocale, setIsSavingLocale] = useState(false);
    const [agentTools, setAgentTools] = useState<AgentTool[]>(['codex', 'claude_code', 'opencode']);
    const [selectedLocale, setSelectedLocale] = useState<string>(() => normalizeLocale(i18n.language));
    const [firstTaskTitle, setFirstTaskTitle] = useState('');
    const [firstTaskMode, setFirstTaskMode] = useState<string>('evidence_drive');
    const [isStartingFirstTask, setIsStartingFirstTask] = useState(false);
    const [detailPanel, setDetailPanel] = useState<DashboardDetail | null>(null);
    const [contextView, setContextView] = useState<VibehubContextViewData | null>(null);
    const [reviewView, setReviewView] = useState<VibehubReviewViewData | null>(null);
    const [handoffView, setHandoffView] = useState<VibehubHandoffViewData | null>(null);
    const [researchStatus, setResearchStatus] = useState<ResearchStatus | null>(null);
    const [diffView, setDiffView] = useState<VibehubDiffViewData | null>(null);
    const [projectDigest, setProjectDigest] = useState<VibehubProjectDigest | null>(null);
    const [gitBranches, setGitBranches] = useState<VibehubGitBranchesView | null>(null);
    const [flowDetails, setFlowDetails] = useState<VibehubFlowDetail[]>([]);
    const [phaseValidation, setPhaseValidation] = useState<PhaseValidationResult | null>(null);
    const [selectedPhase, setSelectedPhase] = useState<string | null>(null);
    const [previewPath, setPreviewPath] = useState('');
    const [previewFile, setPreviewFile] = useState<VibehubFileReadResult | null>(null);
    const [previewError, setPreviewError] = useState<string | null>(null);
    const [isPreviewLoading, setIsPreviewLoading] = useState(false);
    const previewCandidates = getPreviewCandidates(status, contextView, reviewView, handoffView);
    const previewCandidateKey = previewCandidates.map((candidate) => candidate.path).join('|');

    const loadDashboard = useCallback(async (clearActionState = true) => {
        if (!project) return;
        setIsLoading(true);
        try {
            const overview = await tauriApi.vibehubReadOverview(project.path);
            setStatus(overview.status);
            setContextView(overview.context);
            setReviewView(overview.review);
            setHandoffView(overview.handoff);
            setDiffView(overview.diff);
            setResearchStatus(overview.research);
            setProjectDigest(overview.project_digest);
            setGitBranches(overview.git_branches);
            setFlowDetails(overview.flow_details);
            if (overview.status.locale) {
                setSelectedLocale(overview.status.locale);
            }

            if (overview.initialized) {
                const [driftResult, adapterResult, validationResult] = await Promise.allSettled([
                    tauriApi.vibehubCheckWorkspaceDrift(project.path, i18n.language),
                    tauriApi.vibehubGetAgentAdapterStatus(project.path),
                    overview.status.current_task_id && overview.status.current_run_id && overview.status.current_phase
                        ? tauriApi.vibehubValidatePhase(project.path)
                        : Promise.resolve(null),
                ]);

                setDriftReport(driftResult.status === 'fulfilled' ? driftResult.value : null);
                if (adapterResult.status === 'fulfilled') {
                    setAdapterStatus(adapterResult.value);
                    setAgentTools((current) => adapterResult.value.enabled_tools.length ? adapterResult.value.enabled_tools : current);
                } else {
                    setAdapterStatus(null);
                }
                setPhaseValidation(validationResult.status === 'fulfilled' ? validationResult.value : null);
            } else {
                setAdapterStatus(null);
                setDriftReport(null);
                setPhaseValidation(null);
            }
            if (clearActionState) {
                setActionState(null);
            }
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsLoading(false);
        }
    }, [i18n.language, project?.path]);

    useEffect(() => {
        if (enabled && project) {
            loadDashboard();
        } else {
            setStatus(null);
            setAdapterStatus(null);
            setDriftReport(null);
            setActionState(null);
            setIsInitializing(false);
            setContextView(null);
            setReviewView(null);
            setHandoffView(null);
            setDiffView(null);
            setResearchStatus(null);
            setProjectDigest(null);
            setGitBranches(null);
            setFlowDetails([]);
            setPhaseValidation(null);
            setSelectedPhase(null);
            setDetailPanel(null);
            setPreviewPath('');
            setPreviewFile(null);
            setPreviewError(null);
        }
    }, [enabled, loadDashboard, project]);

    useEffect(() => {
        if (!enabled || !project) return;
        const timer = window.setInterval(() => {
            loadDashboard(false);
        }, 12000);
        return () => window.clearInterval(timer);
    }, [enabled, loadDashboard, project]);

    useEffect(() => {
        if (detailPanel && project) {
            loadDashboard(false);
        }
    }, [detailPanel, loadDashboard, project]);

    useEffect(() => {
        if (detailPanel !== 'preview') return;
        if (!previewCandidates.length) {
            setPreviewPath('');
            setPreviewFile(null);
            setPreviewError(null);
            return;
        }
        if (!previewPath || !previewCandidates.some((candidate) => candidate.path === previewPath)) {
            setPreviewPath(previewCandidates[0].path);
        }
    }, [detailPanel, previewCandidateKey, previewCandidates, previewPath]);

    useEffect(() => {
        if (detailPanel !== 'preview' || !project || !previewPath) return;
        let cancelled = false;
        setIsPreviewLoading(true);
        setPreviewError(null);
        tauriApi.vibehubReadVibehubFile(project.path, previewPath)
            .then((result) => {
                if (!cancelled) setPreviewFile(result);
            })
            .catch((error) => {
                if (!cancelled) {
                    setPreviewFile(null);
                    setPreviewError(String(error));
                }
            })
            .finally(() => {
                if (!cancelled) setIsPreviewLoading(false);
            });
        return () => {
            cancelled = true;
        };
    }, [detailPanel, project?.path, previewPath]);

    const hasActiveContextTarget = Boolean(
        status?.initialized && status.current_task_id && status.current_run_id && status.current_phase
    );
    const dashboardDisabled = !status || !status.initialized;
    const currentFlow = useMemo(() => getModeFlow(status, status?.current_mode || 'guided_drive'), [status]);
    const readOnlyActions = useMemo(
        () => getReadOnlyRecommendedActions(status, driftReport, contextView, handoffView, phaseValidation, t),
        [contextView, driftReport, handoffView, phaseValidation, status, t]
    );
    const fileHealth = useMemo(
        () => getFileHealthCards(status, contextView, reviewView, handoffView, researchStatus, t),
        [contextView, handoffView, researchStatus, reviewView, status, t]
    );

    const toggleAgentTool = (tool: AgentTool, checked: boolean) => {
        setAgentTools((current) => {
            const next = checked ? [...current, tool] : current.filter((item) => item !== tool);
            return next.length ? Array.from(new Set(next)) : current;
        });
    };

    const runStartFirstTask = async () => {
        if (!project) return;
        const title = firstTaskTitle.trim();
        if (!title) {
            setActionState({ message: t('vibehub.dashboard.firstTaskTitleRequired'), error: true });
            return;
        }
        setIsStartingFirstTask(true);
        try {
            const result = await tauriApi.vibehubStartTask(project.path, title, firstTaskMode);
            setActionState({
                message: t('vibehub.dashboard.firstTaskCreated', { task: result.task_id, run: result.run_id }),
                error: false,
            });
            setFirstTaskTitle('');
            await loadDashboard(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsStartingFirstTask(false);
        }
    };

    const runInitialize = async () => {
        if (!project) return;
        setIsInitializing(true);
        try {
            const result = await tauriApi.vibehubInit(project.path, agentTools, true);
            if (selectedLocale) {
                await tauriApi.vibehubSetProjectLocale(project.path, selectedLocale);
            }
            const createdCount = result.created_files.length;
            const keptCount = result.skipped_existing_files.length;
            const errorSuffix = result.errors.length ? ` ${t('vibehub.messages.issueCount', { count: result.errors.length })}` : '';
            setActionState({
                message: `${t('vibehub.messages.initialized', {
                    root: result.vibehub_root,
                    created: createdCount,
                    kept: keptCount,
                })}${errorSuffix}`,
                error: result.errors.length > 0,
            });
            await loadDashboard(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsInitializing(false);
        }
    };

    const runSyncAgentAdapters = async () => {
        if (!project) return;
        setIsSyncingAdapters(true);
        try {
            const result = await tauriApi.vibehubSyncAgentAdapters(project.path, agentTools, false);
            setActionState({
                message: t('vibehub.messages.agentAdaptersSynced', {
                    created: result.created_files.length,
                    updated: result.updated_files.length,
                    skipped: result.skipped_files.length,
                    conflicts: result.conflict_files.length,
                }),
                error: result.conflict_files.length > 0,
            });
            await loadDashboard(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsSyncingAdapters(false);
        }
    };

    const runSaveLocale = async () => {
        if (!project || !selectedLocale) return;
        setIsSavingLocale(true);
        try {
            const locale = await tauriApi.vibehubSetProjectLocale(project.path, selectedLocale);
            setActionState({ message: t('vibehub.messages.localeSaved', { locale }), error: false });
            await loadDashboard(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setIsSavingLocale(false);
        }
    };

    const taskLabel = status?.current_task_id
        ? `${status.current_task_id}${status.current_task_title ? ` - ${status.current_task_title}` : ''}`
        : t('vibehub.status.noCurrentTask');

    return (
        <div className="relative space-y-5">
            <div className="flex flex-wrap items-start justify-between gap-3 border-b pb-4">
                <div className="min-w-0">
                    <div className="text-sm font-medium">{showOverview ? t('vibehub.dashboard.title') : t('vibehub.cockpit.title')}</div>
                    <div className="mt-1 truncate text-xl font-semibold tracking-tight">{project?.name || t('common.unknown')}</div>
                    <div className="mt-1 break-all text-xs text-muted-foreground">{project?.path || ''}</div>
                    <div className="mt-2 max-w-3xl text-sm text-muted-foreground">
                        {projectDigest?.summary_line || project?.description || t('vibehub.dashboard.noSummary')}
                    </div>
                    {projectDigest?.status_line && (
                        <div className="mt-1 max-w-3xl text-xs text-muted-foreground">{projectDigest.status_line}</div>
                    )}
                </div>
                <Button size="sm" onClick={() => loadDashboard()} disabled={isLoading || !project} className="shrink-0">
                    <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                    {t('vibehub.dashboard.refreshDetect')}
                </Button>
            </div>

            {!status && (
                <div className="rounded-md border bg-muted/20 p-8 text-center text-sm text-muted-foreground">
                    {isLoading ? t('common.loading') : t('vibehub.unavailable.loading')}
                </div>
            )}

            {status && dashboardDisabled && (
                <div className="rounded-md border bg-muted/40 p-8 text-center opacity-90">
                    <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full border bg-background text-muted-foreground">
                        <Wrench className="h-5 w-5" />
                    </div>
                    <div className="mt-4 text-lg font-semibold">{t('vibehub.dashboard.uninitializedTitle')}</div>
                    <div className="mx-auto mt-2 max-w-lg text-sm text-muted-foreground">
                        {t('vibehub.dashboard.uninitializedBody')}
                    </div>
                    <div className="mx-auto mt-6 max-w-2xl rounded-md border bg-background p-4 text-left">
                        <div className="grid gap-4 md:grid-cols-[0.8fr_1.2fr]">
                            <div className="space-y-2">
                                <Label htmlFor="vibehub-init-locale">{t('vibehub.status.locale')}</Label>
                                <select
                                    id="vibehub-init-locale"
                                    value={selectedLocale}
                                    onChange={(event) => setSelectedLocale(event.target.value)}
                                    disabled={isInitializing}
                                    className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                                >
                                    {LOCALE_OPTIONS.map((locale) => (
                                        <option key={locale} value={locale}>
                                            {t(`vibehub.locales.${locale}`)}
                                        </option>
                                    ))}
                                </select>
                                <div className="text-xs text-muted-foreground">{t('vibehub.dashboard.initLanguageHint')}</div>
                            </div>
                            <div className="space-y-2">
                                <div className="text-sm font-medium">{t('vibehub.dashboard.agentTools')}</div>
                                <div className="grid gap-2 sm:grid-cols-3">
                                    {AGENT_TOOL_OPTIONS.map((tool) => (
                                        <label key={tool.id} className="flex items-start gap-2 rounded-md border bg-muted/20 p-3 text-sm">
                                            <Checkbox
                                                checked={agentTools.includes(tool.id)}
                                                onCheckedChange={(checked) => toggleAgentTool(tool.id, checked === true)}
                                                disabled={isInitializing}
                                            />
                                            <span>
                                                <span className="block font-medium">{tool.label}</span>
                                                <span className="block text-xs text-muted-foreground">{t(`vibehub.ai.tools.${tool.id}`)}</span>
                                            </span>
                                        </label>
                                    ))}
                                </div>
                            </div>
                        </div>
                        <div className="mt-4 flex justify-end">
                            <Button onClick={runInitialize} disabled={isInitializing || !project}>
                                <Wrench className="mr-2 h-4 w-4" />
                                {isInitializing ? t('common.loading') : t('vibehub.actions.initialize')}
                            </Button>
                        </div>
                    </div>
                </div>
            )}

            {status?.initialized && (
                <>
                    {!status.current_task_id && (
                        <div className="rounded-md border bg-muted/20 p-4">
                            <div className="flex items-center gap-2 text-sm font-semibold">
                                <Lightbulb className="h-4 w-4" />
                                {t('vibehub.dashboard.firstTaskTitle')}
                            </div>
                            <div className="mt-1 text-xs text-muted-foreground">
                                {t('vibehub.dashboard.firstTaskHint')}
                            </div>
                            <div className="mt-3 grid gap-3 md:grid-cols-[1.4fr_0.8fr_auto]">
                                <div className="space-y-1">
                                    <Label htmlFor="vibehub-first-task-title" className="text-xs">{t('vibehub.dashboard.firstTaskTitleLabel')}</Label>
                                    <Input
                                        id="vibehub-first-task-title"
                                        value={firstTaskTitle}
                                        onChange={(event) => setFirstTaskTitle(event.target.value)}
                                        placeholder={t('vibehub.dashboard.firstTaskTitlePlaceholder')}
                                        disabled={isStartingFirstTask}
                                    />
                                </div>
                                <div className="space-y-1">
                                    <Label htmlFor="vibehub-first-task-mode" className="text-xs">{t('vibehub.status.mode')}</Label>
                                    <select
                                        id="vibehub-first-task-mode"
                                        value={firstTaskMode}
                                        onChange={(event) => setFirstTaskMode(event.target.value)}
                                        disabled={isStartingFirstTask}
                                        className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                                    >
                                        {Object.keys(MODE_STAGE_PHASES).map((mode) => (
                                            <option key={mode} value={mode}>{t(`vibehub.modes.${mode}`)}</option>
                                        ))}
                                    </select>
                                </div>
                                <div className="flex items-end">
                                    <Button onClick={runStartFirstTask} disabled={isStartingFirstTask || !firstTaskTitle.trim() || !project}>
                                        {isStartingFirstTask ? t('common.loading') : t('vibehub.actions.startTask')}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    )}

                    {status.warnings.length > 0 && (
                        <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3">
                            <div className="flex items-center gap-2 text-sm font-medium text-amber-700 dark:text-amber-400">
                                <AlertCircle className="h-4 w-4" />
                                {t('vibehub.tabs.warnings')}
                                <Badge variant="destructive">{status.warnings.length}</Badge>
                            </div>
                            <ul className="mt-2 space-y-1 text-xs text-amber-800 dark:text-amber-300">
                                {status.warnings.map((warning) => (
                                    <li key={warning} className="break-words">• {warning}</li>
                                ))}
                            </ul>
                        </div>
                    )}

                    <div className="grid gap-3 md:grid-cols-4">
                        <Metric label={t('vibehub.status.mode')} value={formatMode(status.current_mode, t)} />
                        <Metric label={t('vibehub.status.currentTaskRun')} value={taskLabel} />
                        <Metric label={t('vibehub.status.phase')} value={status.current_phase || t('common.none')} />
                        <Metric label={t('vibehub.status.observability')} value={formatStatusValue(status.observability_level || 'best_effort', t)} />
                    </div>

                    <div className="grid gap-4 lg:grid-cols-[1.15fr_0.85fr]">
                        <DashboardCard
                            title={t('vibehub.dashboard.phaseFlow')}
                            description={t('vibehub.dashboard.phaseFlowHint')}
                            icon={<ShieldCheck className="h-4 w-4" />}
                            tone={phaseValidation?.missing_outputs.length ? 'warn' : 'neutral'}
                            onClick={() => setDetailPanel('phase')}
                        >
                            <div className="grid gap-2 sm:grid-cols-5">
                                {currentFlow.map((item) => (
                                    <button
                                        key={item.stage}
                                        type="button"
                                        onClick={(event) => {
                                            event.stopPropagation();
                                            setSelectedPhase(item.phase);
                                            setDetailPanel('phase');
                                        }}
                                        className={`rounded-md border px-3 py-2 text-left transition hover:border-primary/50 ${item.phase === status.current_phase ? 'min-h-28 ring-2 ring-primary/30' : 'min-h-24'} ${flowStatusClass(item.status)}`}
                                    >
                                        <div className="text-sm font-medium">{t(`vibehub.flow.${item.stage}`)}</div>
                                        <div className="mt-1 text-xs text-muted-foreground">
                                            {item.phase ? t(`vibehub.flowPhases.${item.phase}`) : t('vibehub.flow.skipped')}
                                        </div>
                                        <Badge variant={phaseBadgeVariant(item.status)} className="mt-3 text-[10px]">
                                            {formatPhaseStatus(item.status, t)}
                                        </Badge>
                                    </button>
                                ))}
                            </div>
                        </DashboardCard>

                        <DashboardCard
                            title={t('vibehub.dashboard.gitSummary')}
                            description={driftReport?.head_changed ? t('vibehub.dashboard.headChanged') : t('vibehub.dashboard.gitSummaryHint')}
                            icon={<GitBranch className="h-4 w-4" />}
                            tone={status.git_dirty || driftReport?.head_changed ? 'warn' : 'ok'}
                            onClick={() => setDetailPanel('git')}
                        >
                            <div className="grid gap-3 sm:grid-cols-3">
                                <StatusPanel
                                    icon={<GitBranch className="h-4 w-4" />}
                                    label={t('vibehub.dashboard.branch')}
                                    value={gitBranches?.current_branch || project?.metadata.git_branch || t('common.unknown')}
                                    tone="neutral"
                                />
                                <StatusPanel
                                    icon={<GitBranch className="h-4 w-4" />}
                                    label={t('vibehub.status.gitDirty')}
                                    value={gitStatusLabel(status, t)}
                                    tone={status.git_dirty ? 'warn' : 'ok'}
                                />
                                <StatusPanel
                                    icon={<RefreshCw className="h-4 w-4" />}
                                    label={t('vibehub.drift.headChanged')}
                                    value={driftReport?.head_changed ? t('common.yes') : t('common.no')}
                                    tone={driftReport?.head_changed ? 'warn' : 'ok'}
                                />
                            </div>
                            {gitBranches?.branches.slice(0, 3).map((branch) => (
                                <div key={branch.name} className="flex items-center justify-between gap-3 rounded border bg-muted/20 px-2 py-1 text-xs">
                                    <span className="truncate font-mono">{branch.is_current ? '* ' : ''}{branch.name}</span>
                                    <span className="shrink-0 text-muted-foreground">
                                        {formatAheadBehind(branch.ahead, branch.behind, t)}
                                    </span>
                                </div>
                            ))}
                            {diffView?.changed_files.slice(0, 4).map((file) => (
                                <div key={file} className="truncate rounded border bg-muted/20 px-2 py-1 font-mono text-xs">
                                    {file}
                                </div>
                            ))}
                        </DashboardCard>
                    </div>

                    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
                        {fileHealth.map((item) => (
                            <DashboardCard
                                key={item.detail}
                                title={item.title}
                                description={item.description}
                                icon={item.icon}
                                tone={item.tone}
                                compact
                                onClick={() => setDetailPanel(item.detail)}
                            >
                                <Badge variant={item.tone === 'warn' ? 'destructive' : item.tone === 'ok' ? 'secondary' : 'outline'}>
                                    {item.value}
                                </Badge>
                                {item.path && <div className="mt-2 truncate text-xs text-muted-foreground">{item.path}</div>}
                            </DashboardCard>
                        ))}
                    </div>

                    <div className="grid gap-4 lg:grid-cols-[1fr_0.9fr]">
                        <div className="rounded-md border bg-muted/10 p-4">
                            <div className="flex items-center justify-between gap-3">
                                <div>
                                    <div className="text-sm font-medium">{t('vibehub.dashboard.readOnlyActions')}</div>
                                    <div className="mt-1 text-xs text-muted-foreground">{t('vibehub.dashboard.readOnlyHint')}</div>
                                </div>
                                <Badge variant="outline">{t('vibehub.dashboard.readOnlyBadge')}</Badge>
                            </div>
                            <div className="mt-3 grid gap-2">
                                {readOnlyActions.length ? readOnlyActions.map((action) => (
                                    <div key={`${action.command}-${action.title}`} className="rounded-md border bg-background p-3">
                                        <div className="flex flex-wrap items-center gap-2">
                                            <Badge>{action.command}</Badge>
                                            <span className="text-sm font-medium">{action.title}</span>
                                        </div>
                                        <div className="mt-1 text-xs text-muted-foreground">{action.description}</div>
                                    </div>
                                )) : (
                                    <div className="rounded-md border bg-background p-3 text-sm text-muted-foreground">
                                        {t('vibehub.dashboard.noActionNeeded')}
                                    </div>
                                )}
                            </div>
                        </div>

                        <DashboardCard
                            title={t('vibehub.dashboard.evidenceMap')}
                            description={t('vibehub.dashboard.evidenceMapHint')}
                            icon={<ShieldCheck className="h-4 w-4" />}
                            tone={status.warnings.length ? 'warn' : 'neutral'}
                            onClick={() => setDetailPanel('evidence')}
                        >
                            <div className="grid grid-cols-2 gap-2 text-xs">
                                <ViewField label={t('vibehub.tabs.context')} value={contextView?.pack_exists ? t('common.yes') : t('common.no')} />
                                <ViewField label={t('vibehub.tabs.review')} value={reviewView?.review_exists ? t('common.yes') : t('common.no')} />
                                <ViewField label={t('vibehub.tabs.handoff')} value={handoffView?.handoff_exists ? t('common.yes') : t('common.no')} />
                                <ViewField label={t('vibehub.tabs.research')} value={researchStatus?.status || t('common.unknown')} />
                            </div>
                        </DashboardCard>
                    </div>

                    {adapterStatus && (
                        <DashboardCard
                            title={t('vibehub.dashboard.adapterStatus')}
                            description={t('vibehub.dashboard.adapterStatusHint')}
                            icon={<Bot className="h-4 w-4" />}
                            tone={adapterStatus.files.some((file) => file.status === 'conflict') ? 'warn' : 'neutral'}
                            onClick={() => setDetailPanel('adapters')}
                        >
                            <div className="flex flex-wrap gap-2">
                                {adapterStatus.enabled_tools.map((tool) => (
                                    <Badge key={tool} variant="secondary">{tool}</Badge>
                                ))}
                                <Badge variant="outline">{t('vibehub.ai.commandCount', { count: adapterStatus.commands.length })}</Badge>
                            </div>
                        </DashboardCard>
                    )}

                    {!hasActiveContextTarget && (
                        <Notice error={false} message={t('vibehub.dashboard.noActiveTaskReadOnly')} />
                    )}
                </>
            )}

            {actionState && <Notice error={actionState.error} message={actionState.message} />}

            {detailPanel && status?.initialized && (
                <DetailDrawer
                    detail={detailPanel}
                    onClose={() => setDetailPanel(null)}
                    status={status}
                    adapterStatus={adapterStatus}
                    contextView={contextView}
                    reviewView={reviewView}
                    handoffView={handoffView}
                    researchStatus={researchStatus}
                    diffView={diffView}
                    driftReport={driftReport}
                    phaseValidation={phaseValidation}
                    selectedPhase={selectedPhase}
                    flowDetails={flowDetails}
                    gitBranches={gitBranches}
                    previewCandidates={previewCandidates}
                    previewPath={previewPath}
                    setPreviewPath={setPreviewPath}
                    previewFile={previewFile}
                    isPreviewLoading={isPreviewLoading}
                    previewError={previewError}
                    readOnlyActions={readOnlyActions}
                    t={t}
                    agentTools={agentTools}
                    selectedLocale={selectedLocale}
                    isSyncingAdapters={isSyncingAdapters}
                    isSavingLocale={isSavingLocale}
                    onToggleAgentTool={toggleAgentTool}
                    onSelectedLocaleChange={setSelectedLocale}
                    onSyncAgentAdapters={runSyncAgentAdapters}
                    onSaveLocale={runSaveLocale}
                />
            )}
        </div>
    );
}

function DetailDrawer({
    detail,
    onClose,
    status,
    adapterStatus,
    contextView,
    reviewView,
    handoffView,
    researchStatus,
    diffView,
    driftReport,
    phaseValidation,
    selectedPhase,
    flowDetails,
    gitBranches,
    previewCandidates,
    previewPath,
    setPreviewPath,
    previewFile,
    isPreviewLoading,
    previewError,
    readOnlyActions,
    t,
    agentTools,
    selectedLocale,
    isSyncingAdapters,
    isSavingLocale,
    onToggleAgentTool,
    onSelectedLocaleChange,
    onSyncAgentAdapters,
    onSaveLocale,
}: {
    detail: DashboardDetail;
    onClose: () => void;
    status: VibehubCockpitStatus;
    adapterStatus: AgentAdapterStatus | null;
    contextView: VibehubContextViewData | null;
    reviewView: VibehubReviewViewData | null;
    handoffView: VibehubHandoffViewData | null;
    researchStatus: ResearchStatus | null;
    diffView: VibehubDiffViewData | null;
    driftReport: WorkspaceDriftReport | null;
    phaseValidation: PhaseValidationResult | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    gitBranches: VibehubGitBranchesView | null;
    previewCandidates: PreviewCandidate[];
    previewPath: string;
    setPreviewPath: (path: string) => void;
    previewFile: VibehubFileReadResult | null;
    isPreviewLoading: boolean;
    previewError: string | null;
    readOnlyActions: RecommendedReadOnlyAction[];
    t: (key: string, options?: Record<string, unknown>) => string;
    agentTools: AgentTool[];
    selectedLocale: string;
    isSyncingAdapters: boolean;
    isSavingLocale: boolean;
    onToggleAgentTool: (tool: AgentTool, checked: boolean) => void;
    onSelectedLocaleChange: (locale: string) => void;
    onSyncAgentAdapters: () => void;
    onSaveLocale: () => void;
}) {
    const title = getDetailTitle(detail, t);

    return (
        <div
            className="fixed inset-0 z-50 flex justify-end bg-background/40 backdrop-blur-[1px]"
            onMouseDown={onClose}
        >
            <div
                className="flex h-full w-full max-w-xl flex-col border-l bg-background shadow-xl"
                onMouseDown={(event) => event.stopPropagation()}
            >
                <div className="flex flex-none items-start justify-between gap-3 border-b px-5 py-4">
                    <div>
                        <div className="text-xs text-muted-foreground">{t('vibehub.dashboard.details')}</div>
                        <div className="text-lg font-semibold">{title}</div>
                    </div>
                    <Button variant="ghost" size="sm" onClick={onClose} aria-label={t('common.close')}>
                        <X className="h-4 w-4" />
                    </Button>
                </div>

                <div className="flex-1 space-y-4 overflow-y-auto p-5">
                    {detail === 'phase' && (
                        <StatusTabContent
                            status={status}
                            phaseValidation={phaseValidation}
                            selectedPhase={selectedPhase}
                            flowDetails={flowDetails}
                            readOnlyActions={readOnlyActions}
                            t={t}
                        />
                    )}
                    {detail === 'git' && (
                        <>
                            <DiffTabContent diffView={diffView} t={t} />
                            <GitBranchesContent gitBranches={gitBranches} t={t} />
                            {driftReport && (
                                <div className="rounded-md border bg-muted/10 p-3 text-sm">
                                    <div className="font-medium">{t('vibehub.drift.title')}</div>
                                    <div className="mt-2 grid grid-cols-2 gap-2 text-xs">
                                        <ViewField label="HEAD" value={driftReport.head || t('common.unknown')} />
                                        <ViewField label={t('vibehub.drift.lastSeen')} value={driftReport.last_seen_head || t('common.unknown')} />
                                        <ViewField label={t('vibehub.drift.headChanged')} value={driftReport.head_changed ? t('common.yes') : t('common.no')} />
                                        <ViewField label={t('vibehub.drift.contextStale')} value={driftReport.context_stale ? t('common.yes') : t('common.no')} />
                                    </div>
                                    {driftReport.warnings.length > 0 && (
                                        <div className="mt-3 space-y-1">
                                            {driftReport.warnings.map((warning) => (
                                                <Notice key={warning} error message={warning} />
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )}
                        </>
                    )}
                    {detail === 'context' && <ContextTabContent contextView={contextView} t={t} />}
                    {detail === 'output' && (
                        <div className="space-y-3">
                            <div className="text-sm font-medium">{t('vibehub.status.agentOutput')}</div>
                            <div className="grid grid-cols-2 gap-2 text-xs">
                                <ViewField label={t('vibehub.tabs.agentOutputStatus')} value={formatStatusValue(status.agent_output_status.status, t)} />
                                <ViewField label={t('vibehub.tabs.previewPath')} value={status.agent_output_status.path || t('vibehub.tabs.notAvailable')} />
                                <ViewField label={t('vibehub.tabs.packExists')} value={status.agent_output_status.exists ? t('common.yes') : t('common.no')} />
                                <ViewField label={t('vibehub.tabs.stale')} value={status.agent_output_status.stale ? t('common.yes') : t('common.no')} />
                            </div>
                            <RecommendedCommand command="vibehub-checkpoint" text={t('vibehub.dashboard.agentOutputHint')} />
                        </div>
                    )}
                    {detail === 'handoff' && <HandoffTabContent handoffView={handoffView} t={t} />}
                    {detail === 'review' && <ReviewTabContent reviewView={reviewView} t={t} />}
                    {detail === 'research' && <ResearchTabContent researchStatus={researchStatus} t={t} />}
                    {detail === 'evidence' && (
                        <EvidenceTabContent
                            status={status}
                            contextView={contextView}
                            reviewView={reviewView}
                            handoffView={handoffView}
                            researchStatus={researchStatus}
                            diffView={diffView}
                            t={t}
                        />
                    )}
                    {detail === 'preview' && (
                        <PreviewTabContent
                            candidates={previewCandidates}
                            selectedPath={previewPath}
                            onSelectedPathChange={setPreviewPath}
                            file={previewFile}
                            loading={isPreviewLoading}
                            error={previewError}
                            t={t}
                        />
                    )}
                    {detail === 'adapters' && (
                        <AdapterDetailContent
                            adapterStatus={adapterStatus}
                            agentTools={agentTools}
                            selectedLocale={selectedLocale}
                            isSyncingAdapters={isSyncingAdapters}
                            isSavingLocale={isSavingLocale}
                            onToggleAgentTool={onToggleAgentTool}
                            onSelectedLocaleChange={onSelectedLocaleChange}
                            onSyncAgentAdapters={onSyncAgentAdapters}
                            onSaveLocale={onSaveLocale}
                            t={t}
                        />
                    )}

                    {readOnlyActions.length > 0 && detail !== 'phase' && (
                        <div className="rounded-md border bg-muted/10 p-3">
                            <div className="text-sm font-medium">{t('vibehub.dashboard.recommendedCommands')}</div>
                            <div className="mt-2 space-y-2">
                                {readOnlyActions.map((action) => (
                                    <RecommendedCommand key={`${action.command}-${action.title}`} command={action.command} text={action.description} />
                                ))}
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

function DashboardCard({
    title,
    description,
    icon,
    tone,
    compact = false,
    children,
    onClick,
}: {
    title: string;
    description: string;
    icon: ReactNode;
    tone: 'ok' | 'warn' | 'neutral';
    compact?: boolean;
    children: ReactNode;
    onClick: () => void;
}) {
    const toneClass = tone === 'warn'
        ? 'border-destructive/40 bg-destructive/5'
        : tone === 'ok'
            ? 'border-emerald-500/30 bg-emerald-50/60 dark:bg-emerald-950/10'
            : 'bg-card';

    return (
        <button
            type="button"
            onClick={onClick}
            className={`w-full rounded-md border p-4 text-left transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${toneClass} ${compact ? 'min-h-36' : ''}`}
        >
            <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                    <div className="flex items-center gap-2 text-sm font-medium">
                        {icon}
                        <span className="truncate">{title}</span>
                    </div>
                    <div className="mt-1 text-xs text-muted-foreground">{description}</div>
                </div>
                {tone === 'ok' ? (
                    <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-600" />
                ) : tone === 'warn' ? (
                    <AlertCircle className="h-4 w-4 shrink-0 text-destructive" />
                ) : (
                    <Eye className="h-4 w-4 shrink-0 text-muted-foreground" />
                )}
            </div>
            <div className="mt-3 space-y-2">{children}</div>
        </button>
    );
}

function AdapterDetailContent({
    adapterStatus,
    agentTools,
    selectedLocale,
    isSyncingAdapters,
    isSavingLocale,
    onToggleAgentTool,
    onSelectedLocaleChange,
    onSyncAgentAdapters,
    onSaveLocale,
    t,
}: {
    adapterStatus: AgentAdapterStatus | null;
    agentTools: AgentTool[];
    selectedLocale: string;
    isSyncingAdapters: boolean;
    isSavingLocale: boolean;
    onToggleAgentTool: (tool: AgentTool, checked: boolean) => void;
    onSelectedLocaleChange: (locale: string) => void;
    onSyncAgentAdapters: () => void;
    onSaveLocale: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!adapterStatus) {
        return <div className="text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3">
            <div className="text-sm font-medium">{t('vibehub.dashboard.adapterStatus')}</div>
            <div className="rounded-md border bg-muted/10 p-3">
                <div className="text-xs font-medium">{t('vibehub.dashboard.adapterRepair')}</div>
                <div className="mt-1 text-xs text-muted-foreground">{t('vibehub.dashboard.adapterRepairHint')}</div>
                <div className="mt-3 grid gap-2 sm:grid-cols-3">
                    {AGENT_TOOL_OPTIONS.map((tool) => (
                        <label key={tool.id} className="flex items-start gap-2 rounded-md border bg-background p-2 text-xs">
                            <Checkbox
                                checked={agentTools.includes(tool.id)}
                                onCheckedChange={(checked) => onToggleAgentTool(tool.id, checked === true)}
                                disabled={isSyncingAdapters}
                            />
                            <span>
                                <span className="block font-medium">{tool.label}</span>
                                <span className="block text-muted-foreground">{t(`vibehub.ai.tools.${tool.id}`)}</span>
                            </span>
                        </label>
                    ))}
                </div>
                <div className="mt-3 grid gap-2 sm:grid-cols-[1fr_auto]">
                    <select
                        value={selectedLocale}
                        onChange={(event) => onSelectedLocaleChange(event.target.value)}
                        disabled={isSavingLocale}
                        className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                    >
                        {LOCALE_OPTIONS.map((locale) => (
                            <option key={locale} value={locale}>
                                {t(`vibehub.locales.${locale}`)}
                            </option>
                        ))}
                    </select>
                    <Button variant="outline" size="sm" onClick={onSaveLocale} disabled={isSavingLocale}>
                        {isSavingLocale ? t('common.loading') : t('vibehub.actions.saveLanguage')}
                    </Button>
                </div>
                <Button className="mt-3 w-full" size="sm" onClick={onSyncAgentAdapters} disabled={isSyncingAdapters}>
                    <RefreshCw className={`mr-2 h-4 w-4 ${isSyncingAdapters ? 'animate-spin' : ''}`} />
                    {isSyncingAdapters ? t('common.loading') : t('vibehub.actions.repairAgentInstructions')}
                </Button>
            </div>
            <div className="flex flex-wrap gap-2">
                {adapterStatus.enabled_tools.map((tool) => (
                    <Badge key={tool} variant="secondary">{tool}</Badge>
                ))}
                <Badge variant="outline">{t('vibehub.ai.commandCount', { count: adapterStatus.commands.length })}</Badge>
            </div>
            <div className="grid gap-2">
                {adapterStatus.files.map((file) => (
                    <div key={`${file.tool}:${file.path}`} className="rounded-md border bg-muted/10 p-3 text-xs">
                        <div className="flex items-center justify-between gap-2">
                            <span className="truncate font-medium">{file.path}</span>
                            <Badge variant={file.status === 'in_sync' ? 'secondary' : file.status === 'missing' ? 'outline' : 'destructive'}>
                                {formatStatusValue(file.status, t)}
                            </Badge>
                        </div>
                        <div className="mt-1 text-muted-foreground">{formatAdapterDescription(file.description, t)}</div>
                    </div>
                ))}
            </div>
            {adapterStatus.warnings.map((warning) => (
                <Notice key={warning} error message={warning} />
            ))}
            <RecommendedCommand command="vibehub-sync" text={t('vibehub.dashboard.adapterCommandHint')} />
        </div>
    );
}

function RecommendedCommand({ command, text }: { command: string; text: string }) {
    return (
        <div className="rounded-md border bg-background p-2 text-xs">
            <Badge className="mb-1">{command}</Badge>
            <div className="text-muted-foreground">{text}</div>
        </div>
    );
}

// Tab content components.

function FlowDetailPanel({
    detail,
    t,
}: {
    detail: VibehubFlowDetail;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <div className="rounded-md border bg-muted/10 p-3">
            <div className="flex items-center justify-between gap-2">
                <div className="text-xs font-medium">{t(`vibehub.flowPhases.${detail.phase}`)}</div>
                <Badge variant={phaseBadgeVariant(detail.status)}>{formatPhaseStatus(detail.status, t)}</Badge>
            </div>
            <div className="mt-3 grid gap-3 sm:grid-cols-2">
                <ArtifactList title={t('vibehub.dashboard.readInputs')} artifacts={detail.read_inputs} t={t} />
                <ArtifactList title={t('vibehub.dashboard.writtenOutputs')} artifacts={detail.written_outputs} t={t} />
            </div>
        </div>
    );
}

function ArtifactList({
    title,
    artifacts,
    t,
}: {
    title: string;
    artifacts: Array<{ label: string; path: string; exists: boolean }>;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <div>
            <div className="text-xs font-medium text-muted-foreground">{title}</div>
            <div className="mt-2 space-y-2">
                {artifacts.map((artifact) => (
                    <div key={`${artifact.label}:${artifact.path}`} className="rounded border bg-background px-2 py-1.5 text-xs">
                        <div className="flex items-center justify-between gap-2">
                            <span className="font-medium">{artifact.label}</span>
                            <Badge variant={artifact.exists ? 'secondary' : 'outline'}>
                                {artifact.exists ? t('vibehub.stateValues.exists') : t('vibehub.stateValues.missing')}
                            </Badge>
                        </div>
                        <div className="mt-1 break-all font-mono text-[11px] text-muted-foreground">{artifact.path}</div>
                    </div>
                ))}
            </div>
        </div>
    );
}

function GitBranchesContent({
    gitBranches,
    t,
}: {
    gitBranches: VibehubGitBranchesView | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!gitBranches?.git_available) return null;
    return (
        <div className="rounded-md border bg-muted/10 p-3 text-sm">
            <div className="font-medium">{t('vibehub.dashboard.gitBranches')}</div>
            <div className="mt-2 space-y-2">
                {gitBranches.branches.map((branch) => (
                    <div key={branch.name} className="rounded border bg-background p-2 text-xs">
                        <div className="flex items-center justify-between gap-2">
                            <span className="truncate font-mono">{branch.is_current ? '* ' : ''}{branch.name}</span>
                            <Badge variant={branch.is_current ? 'default' : 'outline'}>{formatAheadBehind(branch.ahead, branch.behind, t)}</Badge>
                        </div>
                        <div className="mt-1 truncate text-muted-foreground">{branch.last_commit_subject}</div>
                        <div className="mt-1 font-mono text-[11px] text-muted-foreground">{branch.head_sha} · {branch.last_commit_time}</div>
                    </div>
                ))}
            </div>
        </div>
    );
}

function StatusTabContent({
    status,
    phaseValidation,
    selectedPhase,
    flowDetails,
    readOnlyActions,
    t,
}: {
    status: VibehubCockpitStatus | null;
    phaseValidation: PhaseValidationResult | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    readOnlyActions: RecommendedReadOnlyAction[];
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!status) return null;
    const selectedDetail = flowDetails.find((detail) => detail.phase === selectedPhase)
        || flowDetails.find((detail) => detail.phase === status.current_phase)
        || null;

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.statusDetail')}</div>

            {selectedDetail && (
                <FlowDetailPanel detail={selectedDetail} t={t} />
            )}

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.status.mode')} value={formatMode(status.current_mode, t)} />
                <ViewField label={t('vibehub.status.phase')} value={status.current_phase || t('common.none')} />
                <ViewField label={t('vibehub.status.phaseStatus')} value={formatStatusValue(status.phase_status, t)} />
                <ViewField label={t('vibehub.status.observability')} value={formatStatusValue(status.observability_level || 'best_effort', t)} />
                <ViewField label={t('vibehub.drift.dirty')} value={status.git_dirty ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.contextPackStatus')} value={formatStatusValue(status.context_pack_status.status, t)} />
                <ViewField label={t('vibehub.tabs.agentOutputStatus')} value={formatStatusValue(status.agent_output_status.status, t)} />
                <ViewField label={t('vibehub.tabs.handoffStatus')} value={formatStatusValue(status.handoff_status.status, t)} />
            </div>

            {status.warnings.length > 0 && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.tabs.warnings')}</div>
                    {status.warnings.map((w) => (
                        <div key={w} className="rounded border border-destructive/30 bg-destructive/5 px-2 py-1 text-xs text-destructive">{w}</div>
                    ))}
                </div>
            )}

            <div className="rounded-md border bg-muted/10 p-3">
                <div className="text-xs font-medium">{t('vibehub.phase.title')}</div>
                {phaseValidation ? (
                    <div className="mt-2 space-y-2 text-xs">
                        <div className="grid grid-cols-2 gap-2">
                            <ViewField label={t('vibehub.phase.phase')} value={phaseValidation.phase} />
                            <ViewField label={t('vibehub.phase.validationStatus')} value={formatStatusValue(phaseValidation.status, t)} />
                            <ViewField label={t('vibehub.phase.requiredOutputs')} value={String(phaseValidation.required_outputs.length)} />
                            <ViewField label={t('vibehub.phase.missingOutputs')} value={String(phaseValidation.missing_outputs.length)} />
                        </div>
                        {phaseValidation.source_output_path && (
                            <ViewField label={t('vibehub.phase.sourceOutput')} value={phaseValidation.source_output_path} />
                        )}
                        {phaseValidation.missing_outputs.length > 0 && (
                            <div>
                                <div className="text-xs font-medium text-destructive">{t('vibehub.phase.missingOutputs')}</div>
                                <div className="mt-1 flex flex-wrap gap-1">
                                    {phaseValidation.missing_outputs.map((item) => (
                                        <Badge key={item} variant="destructive">{item}</Badge>
                                    ))}
                                </div>
                            </div>
                        )}
                    </div>
                ) : (
                    <div className="mt-1 text-xs text-muted-foreground">{t('vibehub.phase.notValidated')}</div>
                )}
            </div>

            {readOnlyActions.length > 0 && (
                <div className="rounded-md border bg-muted/10 p-3">
                    <div className="text-xs font-medium">{t('vibehub.dashboard.recommendedCommands')}</div>
                    <div className="mt-2 space-y-2">
                        {readOnlyActions.map((action) => (
                            <RecommendedCommand key={`${action.command}-${action.title}`} command={action.command} text={action.description} />
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}

function ContextTabContent({
    contextView,
    t,
}: {
    contextView: VibehubContextViewData | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!contextView) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.contextPackDetail')}</div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.tabs.contextPackPath')} value={contextView.pack_path || t('vibehub.tabs.notAvailable')} />
                <ViewField label={t('vibehub.tabs.manifestPath')} value={contextView.manifest_path || t('vibehub.tabs.notAvailable')} />
                <ViewField label={t('vibehub.tabs.phase')} value={contextView.phase || t('common.none')} />
                <ViewField label={t('vibehub.tabs.packExists')} value={contextView.pack_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.manifestExists')} value={contextView.manifest_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.stale')} value={contextView.stale ? t('common.yes') : t('common.no')} />
            </div>

            <div className="grid grid-cols-3 gap-2">
                <CountBadge label={t('vibehub.tabs.included')} count={contextView.included_count} tone="ok" />
                <CountBadge label={t('vibehub.tabs.missing')} count={contextView.missing_count} tone={contextView.missing_count > 0 ? 'warn' : 'ok'} />
                <CountBadge label={t('vibehub.tabs.excluded')} count={contextView.excluded_count} tone="neutral" />
            </div>

            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.contextDataSource')}: .vibehub/tasks/&#123;T&#125;/runs/&#123;R&#125;/context-packs/
            </div>
        </div>
    );
}

function EvidenceTabContent({
    status,
    contextView,
    reviewView,
    handoffView,
    researchStatus,
    diffView,
    t,
}: {
    status: VibehubCockpitStatus | null;
    contextView: VibehubContextViewData | null;
    reviewView: VibehubReviewViewData | null;
    handoffView: VibehubHandoffViewData | null;
    researchStatus: ResearchStatus | null;
    diffView: VibehubDiffViewData | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!status) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    const rows = [
        {
            grade: 'hard_observed',
            icon: <ShieldCheck className="h-4 w-4" />,
            title: t('vibehub.evidence.taskRunState'),
            value: status.current_task_id && status.current_run_id
                ? `${status.current_task_id} / ${status.current_run_id}`
                : t('vibehub.status.noCurrentTask'),
        },
        {
            grade: 'hard_observed',
            icon: <GitBranch className="h-4 w-4" />,
            title: t('vibehub.evidence.workspaceDiff'),
            value: diffView ? t('vibehub.dashboard.changedFilesCount', { count: diffView.changed_files_count }) : t('common.unknown'),
        },
        {
            grade: 'hard_observed',
            icon: <FileText className="h-4 w-4" />,
            title: t('vibehub.status.contextPack'),
            value: contextView?.pack_exists
                ? t('vibehub.dashboard.contextCounts', { included: contextView.included_count, missing: contextView.missing_count })
                : formatStatusValue(status.context_pack_status.status, t),
        },
        {
            grade: 'hard_observed',
            icon: <FileText className="h-4 w-4" />,
            title: t('vibehub.status.reviewEvidence'),
            value: reviewView?.review_exists ? t('vibehub.dashboard.changedFilesCount', { count: reviewView.changed_files_count }) : t('vibehub.dashboard.notGenerated'),
        },
        {
            grade: 'agent_reported',
            icon: <Bot className="h-4 w-4" />,
            title: t('vibehub.status.agentOutput'),
            value: status.agent_output_status.exists
                ? status.agent_output_status.path || formatStatusValue(status.agent_output_status.status, t)
                : formatStatusValue(status.agent_output_status.status, t),
        },
        {
            grade: 'agent_reported',
            icon: <FileText className="h-4 w-4" />,
            title: t('vibehub.status.handoff'),
            value: handoffView?.complete ? t('vibehub.stateValues.complete') : t('vibehub.dashboard.missingSectionsCount', { count: handoffView?.missing_sections.length || 0 }),
        },
        {
            grade: 'inferred',
            icon: <Lightbulb className="h-4 w-4" />,
            title: t('vibehub.evidence.researchGate'),
            value: researchStatus?.required ? formatStatusValue(researchStatus.status, t) : t('vibehub.dashboard.notRequired'),
        },
        {
            grade: 'inferred',
            icon: <AlertCircle className="h-4 w-4" />,
            title: t('vibehub.tabs.warnings'),
            value: status.warnings.length ? t('vibehub.evidence.warningCount', { count: status.warnings.length }) : t('common.none'),
        },
    ];

    return (
        <div className="space-y-3 pt-1">
            <div className="flex items-center gap-2 text-sm font-medium">
                <ShieldCheck className="h-4 w-4" />
                {labelOrFallback(t, 'vibehub.tabs.evidenceDetail', 'Evidence map')}
            </div>
            <div className="grid gap-2 md:grid-cols-2">
                {rows.map((row) => (
                    <div key={`${row.grade}-${row.title}`} className="rounded-md border bg-muted/10 p-3">
                        <div className="flex items-center justify-between gap-2">
                            <div className="flex items-center gap-2 text-xs font-medium">
                                {row.icon}
                                <span>{row.title}</span>
                            </div>
                            <Badge variant={row.grade === 'hard_observed' ? 'default' : 'secondary'} className="text-[10px]">
                                {row.grade}
                            </Badge>
                        </div>
                        <div className="mt-2 break-all text-xs text-muted-foreground">{row.value}</div>
                    </div>
                ))}
            </div>
            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.evidenceDataSource')}: {t('vibehub.evidence.dataSource')}
            </div>
        </div>
    );
}

function PreviewTabContent({
    candidates,
    selectedPath,
    onSelectedPathChange,
    file,
    loading,
    error,
    t,
}: {
    candidates: PreviewCandidate[];
    selectedPath: string;
    onSelectedPathChange: (path: string) => void;
    file: VibehubFileReadResult | null;
    loading: boolean;
    error: string | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!candidates.length) {
        return <div className="pt-1 text-xs text-muted-foreground">{labelOrFallback(t, 'vibehub.tabs.noPreviewTargets', 'No VibeHub preview targets are available.')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="flex items-center gap-2 text-sm font-medium">
                <Eye className="h-4 w-4" />
                {labelOrFallback(t, 'vibehub.tabs.previewDetail', 'Markdown / Mermaid preview')}
            </div>
            <div className="flex flex-col gap-2 md:flex-row md:items-center">
                <select
                    value={selectedPath}
                    onChange={(event) => onSelectedPathChange(event.target.value)}
                    className="h-9 min-w-0 flex-1 rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                >
                    {candidates.map((candidate) => (
                        <option key={candidate.path} value={candidate.path}>
                            {formatPreviewLabel(candidate.label, t)} - {candidate.path}
                        </option>
                    ))}
                </select>
                <Badge variant={file?.exists ? 'default' : 'secondary'}>
                    {file?.exists ? t('vibehub.stateValues.exists') : t('vibehub.stateValues.missing')}
                </Badge>
            </div>

            {loading && <div className="text-xs text-muted-foreground">{t('common.loading')}</div>}
            {error && <Notice error message={error} />}
            {file && (
                <div className="space-y-2">
                    <div className="grid grid-cols-2 gap-2 text-xs">
                        <ViewField label={labelOrFallback(t, 'vibehub.tabs.previewPath', 'Path')} value={file.path} />
                        <ViewField label={labelOrFallback(t, 'vibehub.tabs.previewSize', 'Size')} value={`${file.size} bytes`} />
                    </div>
                    {file.exists ? (
                        <MarkdownPreview content={file.content} />
                    ) : (
                        <Notice error={false} message={labelOrFallback(t, 'vibehub.tabs.previewMissing', 'Selected file does not exist yet.')} />
                    )}
                </div>
            )}
        </div>
    );
}

function ReviewTabContent({
    reviewView,
    t,
}: {
    reviewView: VibehubReviewViewData | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!reviewView) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.reviewDetail')}</div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.tabs.reviewExists')} value={reviewView.review_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.diffPatchExists')} value={reviewView.diff_patch_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.reviewPath')} value={reviewView.review_path || t('vibehub.tabs.notAvailable')} />
                <ViewField label={t('vibehub.tabs.diffPatchPath')} value={reviewView.diff_patch_path || t('vibehub.tabs.notAvailable')} />
                <ViewField label={t('vibehub.tabs.changedFilesCount')} value={String(reviewView.changed_files_count)} />
                <ViewField label={t('vibehub.tabs.changedFilesPath')} value={reviewView.changed_files_path || t('vibehub.tabs.notAvailable')} />
            </div>

            {reviewView.review_exists && reviewView.review_summary && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.tabs.reviewSummary')}</div>
                    <pre className="max-h-40 overflow-auto rounded-md border bg-muted/20 p-2 text-xs whitespace-pre-wrap break-all">
                        {reviewView.review_summary || t('vibehub.tabs.noContent')}
                    </pre>
                </div>
            )}

            {reviewView.review_exists && reviewView.evidence_grades.length > 0 && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.tabs.evidenceGrades')}</div>
                    <div className="flex flex-wrap gap-1">
                        {reviewView.evidence_grades.map((grade) => (
                            <Badge key={grade} variant="secondary" className="text-xs">{grade}</Badge>
                        ))}
                    </div>
                </div>
            )}

            {reviewView.review_exists && reviewView.evidence_map_summary && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.tabs.evidenceMap')}</div>
                    <pre className="max-h-52 overflow-auto rounded-md border bg-muted/20 p-2 text-xs whitespace-pre-wrap break-all">
                        {reviewView.evidence_map_summary}
                    </pre>
                </div>
            )}

            {!reviewView.review_exists && (
                <Notice error={false} message={t('vibehub.tabs.reviewNotGenerated')} />
            )}

            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.reviewDataSource')}: phases/review.md, evidence/diff.patch, evidence/changed-files.txt
            </div>
        </div>
    );
}

function HandoffTabContent({
    handoffView,
    t,
}: {
    handoffView: VibehubHandoffViewData | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!handoffView) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.handoffDetail')}</div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.tabs.handoffExists')} value={handoffView.handoff_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.handoffComplete')} value={handoffView.complete ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.handoffPath')} value={handoffView.handoff_path || t('vibehub.tabs.notAvailable')} />
                <ViewField label={t('vibehub.tabs.handoffSectionsCount')} value={String(handoffView.sections_count)} />
            </div>

            {!handoffView.complete && handoffView.missing_sections.length > 0 && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-destructive">{t('vibehub.tabs.missingSections')}</div>
                    <div className="flex flex-wrap gap-1">
                        {handoffView.missing_sections.map((s) => (
                            <Badge key={s} variant="destructive" className="text-xs">{s}</Badge>
                        ))}
                    </div>
                </div>
            )}

            {handoffView.complete && (
                <div className="rounded-md border border-emerald-500/30 bg-emerald-50 dark:bg-emerald-950/20 px-3 py-2">
                    <div className="text-xs font-medium text-emerald-700 dark:text-emerald-400">{t('vibehub.tabs.handoffIsComplete')}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{t('vibehub.tabs.handoffSectionsCount')}: {handoffView.sections_count}</div>
                </div>
            )}

            {!handoffView.handoff_exists && (
                <Notice error={false} message={t('vibehub.tabs.handoffNotGenerated')} />
            )}

            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.handoffDataSource')}: .vibehub/agent-view/handoff.md
            </div>
        </div>
    );
}

function ResearchTabContent({
    researchStatus,
    t,
}: {
    researchStatus: ResearchStatus | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const RESEARCH_PACK_PATH = '.vibehub/research/current/research-pack.md';

    if (!researchStatus) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.researchDetail')}</div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.tabs.researchRequired')} value={researchStatus.required ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.researchStatus')} value={formatStatusValue(researchStatus.status, t)} />
                <ViewField label={t('vibehub.tabs.researchPackExists')} value={researchStatus.research_pack_exists ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.researchPackPath')} value={RESEARCH_PACK_PATH} />
            </div>

            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.researchDataSource')}: {RESEARCH_PACK_PATH}
                <br />
                {t('vibehub.tabs.researchSuppFiles')}: source-log.yaml, findings.yaml
            </div>
        </div>
    );
}

function DiffTabContent({
    diffView,
    t,
}: {
    diffView: VibehubDiffViewData | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!diffView) {
        return <div className="pt-1 text-xs text-muted-foreground">{t('common.loading')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.diffDetail')}</div>

            <div className="flex items-center gap-3">
                <div className="rounded-md border bg-muted/20 px-3 py-2">
                    <div className="text-xs text-muted-foreground">{t('vibehub.tabs.dirty')}</div>
                    <div className="text-lg font-semibold">{diffView.dirty ? t('common.yes') : t('common.no')}</div>
                </div>
                <div className="rounded-md border bg-muted/20 px-3 py-2">
                    <div className="text-xs text-muted-foreground">{t('vibehub.tabs.changedFilesCount')}</div>
                    <div className="text-lg font-semibold">{diffView.changed_files_count}</div>
                </div>
            </div>

            {diffView.diff_stat.length > 0 && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.tabs.diffStat')}</div>
                    <pre className="max-h-40 overflow-auto rounded-md border bg-muted/20 p-2 text-xs whitespace-pre-wrap break-all">
                        {diffView.diff_stat.join('\n')}
                    </pre>
                </div>
            )}

            {diffView.changed_files.length > 0 && (
                <div className="space-y-1">
                    <div className="text-xs font-medium text-muted-foreground">
                        {t('vibehub.tabs.changedFiles')} ({diffView.changed_files_count})
                    </div>
                    <div className="max-h-40 overflow-auto rounded-md border bg-muted/20 p-2">
                        {diffView.changed_files.map((file) => (
                            <div key={file} className="font-mono text-xs truncate py-0.5">{file}</div>
                        ))}
                    </div>
                </div>
            )}

            {!diffView.dirty && (
                <div className="text-xs text-muted-foreground">{t('vibehub.tabs.diffClean')}</div>
            )}

            <div className="text-xs text-muted-foreground">
                {t('vibehub.tabs.diffDataSource')}: git diff --stat + git diff --name-only
            </div>
        </div>
    );
}

function MarkdownPreview({ content }: { content: string }) {
    const nodes: ReactNode[] = [];
    const lines = content.split(/\r?\n/);
    let codeLang: string | null = null;
    let codeLines: string[] = [];

    const flushCode = (key: number) => {
        if (codeLang == null) return;
        const body = codeLines.join('\n');
        if (codeLang.toLowerCase() === 'mermaid') {
            nodes.push(
                <div key={`code-${key}`} className="rounded-md border border-primary/20 bg-primary/5 p-3">
                    <div className="mb-2 flex items-center gap-2 text-xs font-medium text-primary">
                        <Code2 className="h-3.5 w-3.5" />
                        Mermaid
                    </div>
                    <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-all font-mono text-xs">{body}</pre>
                </div>
            );
        } else {
            nodes.push(
                <pre key={`code-${key}`} className="max-h-64 overflow-auto rounded-md border bg-muted/20 p-3 whitespace-pre-wrap break-all font-mono text-xs">
                    {body}
                </pre>
            );
        }
        codeLang = null;
        codeLines = [];
    };

    lines.forEach((line, index) => {
        const fence = line.match(/^```([\w-]*)\s*$/);
        if (fence) {
            if (codeLang == null) {
                codeLang = fence[1] || '';
                codeLines = [];
            } else {
                flushCode(index);
            }
            return;
        }

        if (codeLang != null) {
            codeLines.push(line);
            return;
        }

        const heading = line.match(/^(#{1,4})\s+(.+)$/);
        if (heading) {
            const size = heading[1].length === 1 ? 'text-base' : heading[1].length === 2 ? 'text-sm' : 'text-xs';
            nodes.push(<div key={index} className={`${size} pt-1 font-semibold`}>{heading[2]}</div>);
            return;
        }

        const bullet = line.match(/^\s*[-*]\s+(.+)$/);
        if (bullet) {
            nodes.push(
                <div key={index} className="flex gap-2 text-xs">
                    <span className="mt-1 h-1 w-1 flex-shrink-0 rounded-full bg-muted-foreground" />
                    <span className="break-words">{bullet[1]}</span>
                </div>
            );
            return;
        }

        if (!line.trim()) {
            nodes.push(<div key={index} className="h-1" />);
            return;
        }

        nodes.push(<div key={index} className="break-words text-xs">{line}</div>);
    });

    flushCode(lines.length);

    return (
        <div className="max-h-96 overflow-auto rounded-md border bg-background p-3">
            <div className="space-y-1.5">{nodes}</div>
        </div>
    );
}

// Shared sub-components.

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

function Notice({ message, error }: { message: string; error: boolean }) {
    return (
        <div className={`flex items-start gap-2 rounded-md border p-3 text-sm ${error ? 'border-destructive/40 text-destructive' : 'border-border text-muted-foreground'}`}>
            <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
            <span className="break-all">{message}</span>
        </div>
    );
}

function ViewField({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-md border bg-muted/20 px-2 py-1.5">
            <div className="text-[10px] text-muted-foreground">{label}</div>
            <div className="truncate text-xs font-medium">{value}</div>
        </div>
    );
}

function CountBadge({ label, count, tone }: { label: string; count: number; tone: 'ok' | 'warn' | 'neutral' }) {
    const variant = tone === 'warn' ? 'destructive' : tone === 'ok' ? 'default' : 'secondary';
    return (
        <div className="rounded-md border p-2 text-center">
            <div className="text-xs text-muted-foreground">{label}</div>
            <div className="mt-1">
                <Badge variant={variant}>{count}</Badge>
            </div>
        </div>
    );
}

function normalizeLocale(locale: string) {
    if (locale.startsWith('zh-TW')) return 'zh-TW';
    if (locale.startsWith('zh')) return 'zh-CN';
    return 'en';
}

function getDetailTitle(detail: DashboardDetail, t: (key: string, options?: Record<string, unknown>) => string) {
    const titles: Record<DashboardDetail, string> = {
        phase: t('vibehub.dashboard.phaseFlow'),
        git: t('vibehub.dashboard.gitSummary'),
        context: t('vibehub.status.contextPack'),
        output: t('vibehub.status.agentOutput'),
        handoff: t('vibehub.status.handoff'),
        review: t('vibehub.status.reviewEvidence'),
        research: t('vibehub.tabs.research'),
        evidence: t('vibehub.dashboard.evidenceMap'),
        preview: labelOrFallback(t, 'vibehub.tabs.preview', 'Preview'),
        adapters: t('vibehub.dashboard.adapterStatus'),
    };
    return titles[detail];
}

function phaseBadgeVariant(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    if (status === 'active') return 'default';
    if (status === 'completed') return 'secondary';
    if (status === 'needs_action' || status === 'blocked' || status === 'failed') return 'destructive';
    return 'outline';
}

function getFileHealthCards(
    status: VibehubCockpitStatus | null,
    contextView: VibehubContextViewData | null,
    reviewView: VibehubReviewViewData | null,
    handoffView: VibehubHandoffViewData | null,
    researchStatus: ResearchStatus | null,
    t: (key: string, options?: Record<string, unknown>) => string
): Array<{
    detail: DashboardDetail;
    title: string;
    description: string;
    value: string;
    path?: string | null;
    tone: 'ok' | 'warn' | 'neutral';
    icon: ReactNode;
}> {
    if (!status) return [];

    const contextTone = status.context_pack_status.exists && status.context_pack_status.stale !== true && contextView?.stale !== true
        ? 'ok'
        : 'warn';
    const outputTone = status.agent_output_status.exists && status.agent_output_status.stale !== true ? 'ok' : 'warn';
    const handoffTone = handoffView?.complete || (status.handoff_status.exists && status.handoff_status.stale !== true) ? 'ok' : 'warn';
    const reviewTone = reviewView?.review_exists ? 'ok' : 'neutral';
    const researchTone = researchStatus?.required && researchStatus.status !== 'complete' ? 'warn' : 'neutral';

    return [
        {
            detail: 'context',
            title: t('vibehub.status.contextPack'),
            description: contextView?.stale
                ? t('vibehub.dashboard.stale')
                : t('vibehub.dashboard.contextCounts', { included: contextView?.included_count ?? 0, missing: contextView?.missing_count ?? 0 }),
            value: formatStatusValue(status.context_pack_status.status, t),
            path: contextView?.pack_path || status.context_pack_status.path,
            tone: contextTone,
            icon: <FileText className="h-4 w-4" />,
        },
        {
            detail: 'output',
            title: t('vibehub.status.agentOutput'),
            description: status.agent_output_status.exists ? t('vibehub.dashboard.reportedStateAvailable') : t('vibehub.dashboard.missingOutput'),
            value: formatStatusValue(status.agent_output_status.status, t),
            path: status.agent_output_status.path,
            tone: outputTone,
            icon: <Bot className="h-4 w-4" />,
        },
        {
            detail: 'handoff',
            title: t('vibehub.status.handoff'),
            description: handoffView?.complete ? t('common.success') : t('vibehub.dashboard.missingSectionsCount', { count: handoffView?.missing_sections.length ?? 0 }),
            value: formatStatusValue(status.handoff_status.status, t),
            path: handoffView?.handoff_path || status.handoff_status.path,
            tone: handoffTone,
            icon: <FileText className="h-4 w-4" />,
        },
        {
            detail: 'review',
            title: t('vibehub.status.reviewEvidence'),
            description: reviewView?.review_exists ? t('vibehub.dashboard.changedFilesCount', { count: reviewView.changed_files_count }) : t('vibehub.dashboard.notGenerated'),
            value: reviewView?.review_exists ? t('vibehub.dashboard.available') : t('vibehub.dashboard.missing'),
            path: reviewView?.review_path,
            tone: reviewTone,
            icon: <SearchCheck className="h-4 w-4" />,
        },
        {
            detail: 'research',
            title: t('vibehub.tabs.research'),
            description: researchStatus?.required ? t('vibehub.dashboard.requiredGate') : t('vibehub.dashboard.notRequired'),
            value: formatStatusValue(researchStatus?.status, t),
            path: '.vibehub/research/current/research-pack.md',
            tone: researchTone,
            icon: <Lightbulb className="h-4 w-4" />,
        },
    ];
}

function getModeFlow(status: VibehubCockpitStatus | null, fallbackMode: string) {
    const mode = status?.current_mode || fallbackMode;
    const phases = MODE_STAGE_PHASES[mode] || MODE_STAGE_PHASES.guided_drive;
    const statuses = new Map((status?.flow || []).map((item) => [item.phase, item.status]));
    return FLOW_STAGES.map((stage) => {
        const phase = phases[stage] || null;
        const phaseStatus = phase
            ? statuses.get(phase) || (status?.current_phase === phase ? status.phase_status || 'active' : 'pending')
            : 'skipped';
        return {
            stage,
            phase,
            status: phaseStatus,
        };
    });
}

function flowStatusClass(status: string) {
    if (status === 'active') return 'border-primary bg-primary/10 text-primary';
    if (status === 'completed') return 'border-emerald-500/40 bg-emerald-50 text-emerald-700 dark:bg-emerald-950/20 dark:text-emerald-400';
    if (status === 'needs_action' || status === 'blocked' || status === 'failed') return 'border-destructive/40 bg-destructive/5 text-destructive';
    if (status === 'skipped') return 'bg-muted/30 text-muted-foreground';
    return 'bg-background';
}

function formatPhaseStatus(status: string, t: (key: string, options?: Record<string, unknown>) => string) {
    return formatStatusValue(status, t);
}

function formatMode(mode: string | null | undefined, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!mode) return t('common.unknown');
    return labelOrFallback(t, `vibehub.modes.${mode}`, mode);
}

function formatStatusValue(value: string | null | undefined, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!value) return t('common.unknown');
    return labelOrFallback(t, `vibehub.stateValues.${value}`, value);
}

function formatAheadBehind(ahead: number | null, behind: number | null, t: (key: string, options?: Record<string, unknown>) => string) {
    if (ahead === null && behind === null) return t('vibehub.stateValues.local');
    return `+${ahead ?? 0}/-${behind ?? 0}`;
}

function formatPreviewLabel(label: string, t: (key: string, options?: Record<string, unknown>) => string) {
    const key = label.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');
    return labelOrFallback(t, `vibehub.previewLabels.${key}`, label);
}

function formatAdapterDescription(description: string, t: (key: string, options?: Record<string, unknown>) => string) {
    const key = description.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_|_$/g, '');
    return labelOrFallback(t, `vibehub.adapterDescriptions.${key}`, description);
}

function getPreviewCandidates(
    status: VibehubCockpitStatus | null,
    contextView: VibehubContextViewData | null,
    reviewView: VibehubReviewViewData | null,
    handoffView: VibehubHandoffViewData | null
): PreviewCandidate[] {
    const candidates: PreviewCandidate[] = [];
    addPreviewCandidate(candidates, '.vibehub/agent-view/current.md', 'Current view', true);
    addPreviewCandidate(candidates, '.vibehub/agent-view/current-context.md', 'Current context', true);
    addPreviewCandidate(candidates, handoffView?.handoff_path || status?.handoff_status.path, 'Handoff', Boolean(handoffView?.handoff_exists || status?.handoff_status.exists));
    addPreviewCandidate(candidates, contextView?.pack_path || status?.context_pack_status.path, 'Context pack', Boolean(contextView?.pack_exists || status?.context_pack_status.exists));
    addPreviewCandidate(candidates, contextView?.manifest_path, 'Context manifest', Boolean(contextView?.manifest_exists));
    addPreviewCandidate(candidates, reviewView?.review_path, 'Review evidence', Boolean(reviewView?.review_exists));
    addPreviewCandidate(candidates, reviewView?.diff_patch_path, 'Diff patch', Boolean(reviewView?.diff_patch_exists));
    addPreviewCandidate(candidates, reviewView?.changed_files_path, 'Changed files', Boolean(reviewView?.changed_files_path));
    // Prefer canonical agent-view sync report. Do NOT inject the workspace-root
    // `.vibehub/sync.md` — that path is an r9 orphan and not produced by the
    // current sync pipeline. If the backend later surfaces `state.sync.last_report`
    // on the cockpit status, this should be threaded through here as the first
    // sync candidate, with `.vibehub/agent-view/sync.md` as the fallback.
    addPreviewCandidate(candidates, '.vibehub/agent-view/sync.md', 'Sync report', true);
    return candidates;
}

function addPreviewCandidate(candidates: PreviewCandidate[], path: string | null | undefined, label: string, exists: boolean) {
    if (!path || candidates.some((candidate) => candidate.path === path)) return;
    candidates.push({ path, label, exists });
}

function labelOrFallback(t: (key: string, options?: Record<string, unknown>) => string, key: string, fallback: string) {
    const value = t(key);
    return value === key ? fallback : value;
}

function getReadOnlyRecommendedActions(
    status: VibehubCockpitStatus | null,
    driftReport: WorkspaceDriftReport | null,
    contextView: VibehubContextViewData | null,
    handoffView: VibehubHandoffViewData | null,
    phaseValidation: PhaseValidationResult | null,
    t: (key: string, options?: Record<string, unknown>) => string
): RecommendedReadOnlyAction[] {
    if (!status?.initialized) return [];
    const actions: RecommendedReadOnlyAction[] = [];

    if (!status.current_task_id || !status.current_run_id || !status.current_phase) {
        actions.push({
            command: 'vibehub-sync',
            title: t('vibehub.dashboard.actionNeedTask'),
            description: t('vibehub.dashboard.actionNeedTaskDesc'),
        });
    }
    if (status.git_dirty || driftReport?.head_changed || driftReport?.dirty) {
        actions.push({
            command: 'vibehub-sync',
            title: t('vibehub.dashboard.actionDrift'),
            description: t('vibehub.dashboard.actionDriftDesc', { count: status.git_changed_files_count || 0 }),
        });
    }
    if (driftReport?.context_stale || contextView?.stale || !status.context_pack_status.exists) {
        actions.push({
            command: 'vibehub-context',
            title: t('vibehub.dashboard.actionContext'),
            description: t('vibehub.dashboard.actionContextDesc'),
        });
    }
    if (phaseValidation?.missing_outputs.length) {
        actions.push({
            command: 'vibehub-checkpoint',
            title: t('vibehub.dashboard.actionPhaseOutput'),
            description: t('vibehub.dashboard.actionPhaseOutputDesc', { count: phaseValidation.missing_outputs.length }),
        });
    }
    if (!handoffView?.handoff_exists || handoffView.missing_sections.length > 0) {
        actions.push({
            command: 'vibehub-handoff',
            title: t('vibehub.dashboard.actionHandoff'),
            description: t('vibehub.dashboard.actionHandoffDesc'),
        });
    }
    return actions;
}

function gitStatusLabel(status: VibehubCockpitStatus, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!status.git_available) return t('vibehub.status.gitUnavailable');
    if (status.git_dirty == null) return t('common.unknown');
    return status.git_dirty
        ? t('vibehub.status.dirtyCount', { count: status.git_changed_files_count || 0 })
        : t('vibehub.status.clean');
}
