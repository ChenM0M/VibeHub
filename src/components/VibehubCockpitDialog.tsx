import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import { listen } from '@tauri-apps/api/event';
import { AlertCircle, Archive, Bot, Boxes, CheckCircle2, ChevronDown, ChevronRight, ClipboardList, Code2, Copy, ExternalLink, Eye, FileText, FolderOpen, GitBranch, History, Layers3, Lightbulb, Network, RefreshCw, Settings, ShieldCheck, Wrench, X, XCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AgentAdapterStatus, AgentTool, AgentUsageSourceSummary, AgentUsageTokenBreakdown, LocalAgentUsageOverview, PhaseValidationResult, Project, ResearchStatus, VibehubArchivedTaskCard, VibehubArchiveViewData, VibehubActiveTask, VibehubCockpitStatus, VibehubContextViewData, VibehubDiffViewData, VibehubEventTimelineItem, VibehubFileReadResult, VibehubFlowDetail, VibehubGitBranchesView, VibehubHandoffViewData, VibehubProjectDigest, VibehubProjectStructureGraphNode, VibehubProjectStructureTreeNode, VibehubProjectStructureViewData, VibehubPromptRenderResult, VibehubPromptTemplateId, VibehubPromptTemplateOption, VibehubReviewViewData, WorkspaceDriftReport } from '@/types';
import { tauriApi } from '@/services/tauri';
import { ProjectDetailBoard } from './ProjectDetailBoard';
import { ProjectStructureExplorer } from './ProjectStructureExplorer';
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

export type RecommendedReadOnlyAction = {
    command: string;
    title: string;
    description: string;
    promptTemplateId: VibehubPromptTemplateId;
};

type PreviewCandidate = { path: string; label: string; exists: boolean };
type VibehubStatusChangedEvent = { project_path: string; source: string };
export type DashboardDetail = 'task' | 'phase' | 'git' | 'activity' | 'context' | 'output' | 'handoff' | 'review' | 'research' | 'evidence' | 'preview' | 'adapters' | 'settings' | 'structure' | 'archive' | 'agentUsage';
export type FocusTarget = { taskId?: string | null; capability?: string | null; eventId?: string | null };
export type DetailOpenTarget = { taskId?: string | null; phase?: string | null };
type StructureNodeLike = VibehubProjectStructureGraphNode | VibehubProjectStructureTreeNode;
type LifecycleNode = ProcessStep & {
    index: number;
    stage: FlowStage;
    detail: VibehubFlowDetail | null;
    events: VibehubEventTimelineItem[];
};

const AGENT_TOOL_OPTIONS: Array<{ id: AgentTool; label: string }> = [
    { id: 'amp_code', label: 'Amp Code' },
    { id: 'codex', label: 'Codex' },
    { id: 'claude_code', label: 'Claude Code' },
    { id: 'opencode', label: 'OpenCode' },
    { id: 'cursor', label: 'Cursor' },
    { id: 'antigravity', label: 'Antigravity' },
];
const LOCALE_OPTIONS = ['en', 'zh-CN', 'zh-TW'] as const;
const DEFAULT_PROMPT_TEMPLATES: VibehubPromptTemplateOption[] = [
    { id: 'new-task', filename: 'new-task.md', dangerous: false },
    { id: 'sync', filename: 'sync.md', dangerous: false },
    { id: 'claim-capability', filename: 'claim-capability.md', dangerous: false },
    { id: 'release-capability', filename: 'release-capability.md', dangerous: false },
    { id: 'cancel-task', filename: 'cancel-task.md', dangerous: true },
    { id: 'force-rebuild', filename: 'force-rebuild.md', dangerous: true },
    { id: 'fix-schema', filename: 'fix-schema.md', dangerous: false },
];
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
    const [agentTools, setAgentTools] = useState<AgentTool[]>(['amp_code', 'codex', 'claude_code', 'opencode', 'cursor', 'antigravity']);
    const [selectedLocale, setSelectedLocale] = useState<string>(() => normalizeLocale(i18n.language));
    const [firstTaskTitle, setFirstTaskTitle] = useState('');
    const [firstTaskMode, setFirstTaskMode] = useState<string>('evidence_drive');
    const [isStartingFirstTask, setIsStartingFirstTask] = useState(false);
    const [detailPanel, setDetailPanel] = useState<DashboardDetail | null>(null);
    const [selectedDetailTaskId, setSelectedDetailTaskId] = useState<string | null>(null);
    const [contextView, setContextView] = useState<VibehubContextViewData | null>(null);
    const [reviewView, setReviewView] = useState<VibehubReviewViewData | null>(null);
    const [handoffView, setHandoffView] = useState<VibehubHandoffViewData | null>(null);
    const [researchStatus, setResearchStatus] = useState<ResearchStatus | null>(null);
    const [diffView, setDiffView] = useState<VibehubDiffViewData | null>(null);
    const [projectDigest, setProjectDigest] = useState<VibehubProjectDigest | null>(null);
    const [gitBranches, setGitBranches] = useState<VibehubGitBranchesView | null>(null);
    const [flowDetails, setFlowDetails] = useState<VibehubFlowDetail[]>([]);
    const [eventTimeline, setEventTimeline] = useState<VibehubEventTimelineItem[]>([]);
    const [archiveView, setArchiveView] = useState<VibehubArchiveViewData | null>(null);
    const [localAgentUsage, setLocalAgentUsage] = useState<LocalAgentUsageOverview | null>(null);
    const [projectStructure, setProjectStructure] = useState<VibehubProjectStructureViewData | null>(null);
    const [phaseValidation, setPhaseValidation] = useState<PhaseValidationResult | null>(null);
    const [selectedPhase, setSelectedPhase] = useState<string | null>(null);
    const [focusedTarget, setFocusedTarget] = useState<FocusTarget | null>(null);
    const [previewPath, setPreviewPath] = useState('');
    const [previewFile, setPreviewFile] = useState<VibehubFileReadResult | null>(null);
    const [previewError, setPreviewError] = useState<string | null>(null);
    const [isPreviewLoading, setIsPreviewLoading] = useState(false);
    const [promptTemplates, setPromptTemplates] = useState<VibehubPromptTemplateOption[]>(DEFAULT_PROMPT_TEMPLATES);
    const [promptModalOpen, setPromptModalOpen] = useState(false);
    const [promptTemplateId, setPromptTemplateId] = useState<VibehubPromptTemplateId>('new-task');
    const [promptResult, setPromptResult] = useState<VibehubPromptRenderResult | null>(null);
    const [promptError, setPromptError] = useState<string | null>(null);
    const [isPromptLoading, setIsPromptLoading] = useState(false);
    const [promptConfirmed, setPromptConfirmed] = useState(false);
    const [promptCopied, setPromptCopied] = useState(false);
    const previewCandidates = getPreviewCandidates(status, contextView, reviewView, handoffView, archiveView);
    const previewCandidateKey = previewCandidates.map((candidate) => candidate.path).join('|');

    const renderPrompt = useCallback(async (templateId: VibehubPromptTemplateId, open = true) => {
        if (!project) return;
        if (open) setPromptModalOpen(true);
        setPromptTemplateId(templateId);
        setPromptResult(null);
        setPromptError(null);
        setPromptConfirmed(false);
        setPromptCopied(false);
        setIsPromptLoading(true);
        try {
            const result = await tauriApi.vibehubRenderPrompt(project.path, templateId);
            setPromptResult(result);
            setPromptConfirmed(!result.confirmation_required);
        } catch (error) {
            setPromptError(String(error));
        } finally {
            setIsPromptLoading(false);
        }
    }, [project?.path]);

    const copyPrompt = useCallback(async () => {
        if (!promptResult?.content) return;
        try {
            await navigator.clipboard.writeText(promptResult.content);
            setPromptCopied(true);
            window.setTimeout(() => setPromptCopied(false), 1400);
        } catch (error) {
            setPromptError(String(error));
        }
    }, [promptResult?.content]);

    const loadDashboard = useCallback(async (clearActionState = true) => {
        if (!project) return;
        setIsLoading(true);
        let adapterWarnings: string[] = [];
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
            setEventTimeline(overview.event_timeline || []);
            setArchiveView(overview.archive || { cards: [], warnings: [] });
            setProjectStructure(overview.project_structure || null);
            if (overview.status.locale) {
                setSelectedLocale(overview.status.locale);
            }

            if (overview.initialized) {
                const [driftResult, adapterResult, validationResult, localUsageResult] = await Promise.allSettled([
                    tauriApi.vibehubCheckWorkspaceDrift(project.path, i18n.language),
                    tauriApi.vibehubGetAgentAdapterStatus(project.path),
                    overview.status.current_task_id && overview.status.current_run_id && overview.status.current_phase
                        ? tauriApi.vibehubValidatePhase(project.path)
                        : Promise.resolve(null),
                    tauriApi.vibehubReadLocalAgentUsage(project.path),
                ]);

                setDriftReport(driftResult.status === 'fulfilled' ? driftResult.value : null);
                if (adapterResult.status === 'fulfilled') {
                    setAdapterStatus(adapterResult.value);
                    setAgentTools((current) => adapterResult.value.enabled_tools.length ? adapterResult.value.enabled_tools : current);
                    adapterWarnings = adapterResult.value.warnings || [];
                } else {
                    setAdapterStatus(null);
                }
                setPhaseValidation(validationResult.status === 'fulfilled' ? validationResult.value : null);
                setLocalAgentUsage(localUsageResult.status === 'fulfilled' ? localUsageResult.value : null);
            } else {
                setAdapterStatus(null);
                setDriftReport(null);
                setPhaseValidation(null);
                setLocalAgentUsage(null);
            }
            if (clearActionState) {
                setActionState(adapterWarnings.length ? { message: adapterWarnings[0], error: false } : null);
            }
        } catch (error) {
            setLocalAgentUsage(null);
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
            setEventTimeline([]);
            setArchiveView(null);
            setLocalAgentUsage(null);
            setPhaseValidation(null);
            setSelectedPhase(null);
            setFocusedTarget(null);
            setDetailPanel(null);
            setSelectedDetailTaskId(null);
            setPreviewPath('');
            setPreviewFile(null);
            setPreviewError(null);
            setPromptModalOpen(false);
            setPromptResult(null);
            setPromptError(null);
            setPromptCopied(false);
        }
    }, [enabled, loadDashboard, project]);

    useEffect(() => {
        if (!enabled) return;
        let cancelled = false;
        tauriApi.vibehubListPromptTemplates()
            .then((templates) => {
                if (!cancelled && templates.length) {
                    setPromptTemplates(templates);
                }
            })
            .catch(() => {
                if (!cancelled) setPromptTemplates(DEFAULT_PROMPT_TEMPLATES);
            });
        return () => {
            cancelled = true;
        };
    }, [enabled]);

    useEffect(() => {
        if (!enabled || !project) return;
        const unlistenPromise = listen<VibehubStatusChangedEvent>('vibehub://status-changed', (event) => {
            if (event.payload.project_path === project.path) {
                loadDashboard(false);
            }
        });
        return () => {
            unlistenPromise.then((unlisten) => unlisten());
        };
    }, [enabled, loadDashboard, project]);

    useEffect(() => {
        if (detailPanel && project) {
            loadDashboard(false);
        }
    }, [detailPanel, loadDashboard, project]);

    useEffect(() => {
        if (detailPanel !== 'preview' && detailPanel !== 'activity') return;
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
        if ((detailPanel !== 'preview' && detailPanel !== 'activity') || !project || !previewPath) return;
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

    const openDetail = (detail: DashboardDetail, target?: DetailOpenTarget) => {
        if (target?.taskId !== undefined) {
            setSelectedDetailTaskId(target.taskId || null);
        } else if (detail !== 'phase') {
            setSelectedDetailTaskId((current) => current || status?.current_task_id || null);
        }
        if (target?.phase !== undefined) {
            setSelectedPhase(target.phase || null);
        }
        setDetailPanel(detail);
    };

    return (
        <div className="relative space-y-5">
            {!showOverview && (
                <div className="flex flex-wrap items-start justify-between gap-3 border-b pb-4">
                    <div className="min-w-0">
                        <div className="text-sm font-medium">{t('vibehub.cockpit.title')}</div>
                        <div className="mt-1 truncate text-xl font-semibold">{project?.name || t('common.unknown')}</div>
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
            )}

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
                    {!status.current_task_id && !showOverview && (
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

                    <ProjectDetailBoard
                        status={status}
                        phaseValidation={phaseValidation}
                        project={project}
                        projectDigest={projectDigest}
                        gitBranches={gitBranches}
                        diffView={diffView}
                        eventTimeline={eventTimeline}
                        archiveView={archiveView}
                        localAgentUsage={localAgentUsage}
                        projectStructure={projectStructure}
                        currentFlow={currentFlow}
                        focusedTarget={focusedTarget}
                        readOnlyActions={readOnlyActions}
                        promptTemplates={promptTemplates}
                        isLoading={isLoading}
                        t={t}
                        onRefresh={() => loadDashboard()}
                        onOpenPrompt={renderPrompt}
                        onOpenDetail={openDetail}
                        onOpenPhase={(phase, target) => {
                            openDetail('phase', { taskId: target?.taskId, phase });
                        }}
                    />

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
                    selectedTaskId={selectedDetailTaskId}
                    selectedPhase={selectedPhase}
                    flowDetails={flowDetails}
                    gitBranches={gitBranches}
                    eventTimeline={eventTimeline}
                    archiveView={archiveView}
                    localAgentUsage={localAgentUsage}
                    projectStructure={projectStructure}
                    previewCandidates={previewCandidates}
                    previewPath={previewPath}
                    setPreviewPath={setPreviewPath}
                    previewFile={previewFile}
                    isPreviewLoading={isPreviewLoading}
                    previewError={previewError}
                    readOnlyActions={readOnlyActions}
                    promptTemplates={promptTemplates}
                    t={t}
                    agentTools={agentTools}
                    selectedLocale={selectedLocale}
                    isSyncingAdapters={isSyncingAdapters}
                    isSavingLocale={isSavingLocale}
                    onToggleAgentTool={toggleAgentTool}
                    onSelectedLocaleChange={setSelectedLocale}
                    onSyncAgentAdapters={runSyncAgentAdapters}
                    onSaveLocale={runSaveLocale}
                    onOpenPrompt={renderPrompt}
                    onOpenDetail={openDetail}
                    onFocusTarget={(target) => {
                        setFocusedTarget(target);
                        setDetailPanel(null);
                    }}
                />
            )}

            <PromptGeneratorModal
                open={promptModalOpen}
                onOpenChange={setPromptModalOpen}
                templates={promptTemplates}
                selectedTemplateId={promptTemplateId}
                result={promptResult}
                loading={isPromptLoading}
                error={promptError}
                confirmed={promptConfirmed}
                copied={promptCopied}
                onTemplateChange={(templateId) => renderPrompt(templateId, true)}
                onConfirm={() => setPromptConfirmed(true)}
                onCopy={copyPrompt}
                t={t}
            />
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
    selectedTaskId,
    selectedPhase,
    flowDetails,
    gitBranches,
    eventTimeline,
    archiveView,
    localAgentUsage,
    projectStructure,
    previewCandidates,
    previewPath,
    setPreviewPath,
    previewFile,
    isPreviewLoading,
    previewError,
    readOnlyActions,
    promptTemplates,
    t,
    agentTools,
    selectedLocale,
    isSyncingAdapters,
    isSavingLocale,
    onToggleAgentTool,
    onSelectedLocaleChange,
    onSyncAgentAdapters,
    onSaveLocale,
    onOpenPrompt,
    onOpenDetail,
    onFocusTarget,
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
    selectedTaskId: string | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    gitBranches: VibehubGitBranchesView | null;
    eventTimeline: VibehubEventTimelineItem[];
    archiveView: VibehubArchiveViewData | null;
    localAgentUsage: LocalAgentUsageOverview | null;
    projectStructure: VibehubProjectStructureViewData | null;
    previewCandidates: PreviewCandidate[];
    previewPath: string;
    setPreviewPath: (path: string) => void;
    previewFile: VibehubFileReadResult | null;
    isPreviewLoading: boolean;
    previewError: string | null;
    readOnlyActions: RecommendedReadOnlyAction[];
    promptTemplates: VibehubPromptTemplateOption[];
    t: (key: string, options?: Record<string, unknown>) => string;
    agentTools: AgentTool[];
    selectedLocale: string;
    isSyncingAdapters: boolean;
    isSavingLocale: boolean;
    onToggleAgentTool: (tool: AgentTool, checked: boolean) => void;
    onSelectedLocaleChange: (locale: string) => void;
    onSyncAgentAdapters: () => void;
    onSaveLocale: () => void;
    onOpenPrompt: (templateId: VibehubPromptTemplateId) => void;
    onOpenDetail: (detail: DashboardDetail) => void;
    onFocusTarget: (target: FocusTarget) => void;
}) {
    const title = getDetailTitle(detail, t);
    const drawerWidthClass = getDetailDrawerWidthClass(detail);

    return (
        <div
            className="fixed inset-0 z-50 flex justify-end bg-background/40 backdrop-blur-[1px]"
            onMouseDown={onClose}
        >
            <div
                className={`flex h-full w-full flex-col border-l bg-background shadow-xl ${drawerWidthClass}`}
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

                <div className="flex-1 overflow-y-auto px-5 py-4">
                    {detail === 'task' && (
                        <TaskDetailContent
                            status={status}
                            selectedTaskId={selectedTaskId}
                            selectedPhase={selectedPhase}
                            flowDetails={flowDetails}
                            phaseValidation={phaseValidation}
                            events={eventTimeline}
                            t={t}
                        />
                    )}
                    {detail === 'phase' && (
                        <StatusTabContent
                            status={status}
                            phaseValidation={phaseValidation}
                            selectedTaskId={selectedTaskId}
                            selectedPhase={selectedPhase}
                            flowDetails={flowDetails}
                            events={eventTimeline}
                            t={t}
                        />
                    )}
                    {detail === 'activity' && (
                        <ActivityDetailContent
                            status={status}
                            phaseValidation={phaseValidation}
                            events={eventTimeline}
                            onFocusTarget={onFocusTarget}
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
                            projectPath={status.project_root}
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
                    {detail === 'settings' && (
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
                    {detail === 'structure' && (
                        <ProjectStructureExplorer
                            projectStructure={projectStructure}
                            projectPath={status.project_root}
                            t={t}
                        />
                    )}
                    {detail === 'archive' && (
                        <ArchiveDetailContent
                            archiveView={archiveView}
                            onOpenArtifact={(path) => {
                                setPreviewPath(path);
                                onOpenDetail('preview');
                            }}
                            t={t}
                        />
                    )}
                    {detail === 'agentUsage' && (
                        <AgentUsageTabContent localAgentUsage={localAgentUsage} t={t} />
                    )}

                    {detail === 'settings' && readOnlyActions.length > 0 && (
                        <div className="rounded-md border bg-muted/10 p-3">
                            <div className="text-sm font-medium">{t('vibehub.dashboard.recommendedCommands')}</div>
                            <div className="mt-2 space-y-2">
                                {readOnlyActions.slice(0, 3).map((action) => (
                                    <RecommendedCommand
                                        key={`${action.command}-${action.title}`}
                                        command={action.command}
                                        text={action.description}
                                        promptTemplateId={action.promptTemplateId}
                                        onOpenPrompt={onOpenPrompt}
                                        t={t}
                                    />
                                ))}
                            </div>
                        </div>
                    )}
                    {detail === 'settings' && (
                        <PromptActionGrid templates={promptTemplates} onOpenPrompt={onOpenPrompt} t={t} />
                    )}
                </div>
            </div>
        </div>
    );
}

export type ProjectTaskCardState = {
    task_id: string;
    title?: string | null;
    run_id?: string | null;
    mode?: string | null;
    phase?: string | null;
    phase_status?: string | null;
    current: boolean;
    active_capabilities: string[];
    dependencies: string[];
    intake_order?: number | null;
    shared_files: string[];
};

export function VibehubProjectDetailShell({
    status,
    phaseValidation,
    project,
    projectDigest,
    gitBranches,
    diffView,
    eventTimeline,
    archiveView,
    projectStructure,
    currentFlow,
    focusedTarget,
    readOnlyActions,
    promptTemplates,
    isLoading,
    onRefresh,
    onOpenPrompt,
    onOpenDetail,
    onOpenPhase,
    t,
}: {
    status: VibehubCockpitStatus;
    phaseValidation: PhaseValidationResult | null;
    project: Project | null;
    projectDigest: VibehubProjectDigest | null;
    gitBranches: VibehubGitBranchesView | null;
    diffView: VibehubDiffViewData | null;
    eventTimeline: VibehubEventTimelineItem[];
    archiveView: VibehubArchiveViewData | null;
    projectStructure: VibehubProjectStructureViewData | null;
    currentFlow: ReturnType<typeof getModeFlow>;
    focusedTarget: FocusTarget | null;
    readOnlyActions: RecommendedReadOnlyAction[];
    promptTemplates: VibehubPromptTemplateOption[];
    isLoading: boolean;
    onRefresh: () => void;
    onOpenPrompt: (templateId: VibehubPromptTemplateId) => void;
    onOpenDetail: (detail: DashboardDetail) => void;
    onOpenPhase: (phase: string | null) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const tasks = getProjectTaskCards(status);
    const [archiveExpanded, setArchiveExpanded] = useState(true);
    const archiveCards = archiveView?.cards || [];
    const commit = getRecentCommitSummary(gitBranches, project, t);
    const activity = getRecentActivitySummary(status, phaseValidation, eventTimeline, t);
    const structureNodes = (projectStructure?.graph_nodes || [])
        .filter((node) => node.kind !== 'root')
        .slice(0, 6);
    const structureFiles = flattenStructureTree(projectStructure?.tree || []).slice(0, 5);

    return (
        <section className="space-y-5">
            <div className="rounded-md border bg-background p-5">
                <div className="flex flex-wrap items-start justify-between gap-4">
                    <div className="min-w-0">
                        <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                            <Layers3 className="h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.title', 'Project detail')}
                        </div>
                        <h2 className="mt-2 truncate text-2xl font-semibold">{project?.name || t('common.unknown')}</h2>
                        <div className="mt-1 break-all text-xs text-muted-foreground">{project?.path || status.project_root}</div>
                        <div className="mt-3 max-w-3xl text-sm text-muted-foreground">
                            {projectDigest?.summary_line || project?.description || t('vibehub.dashboard.noSummary')}
                        </div>
                    </div>
                    <div className="flex shrink-0 items-center gap-2">
                        <Button variant="outline" size="sm" onClick={onRefresh} disabled={isLoading}>
                            <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                            {t('home.refresh')}
                        </Button>
                        <Button variant="outline" size="sm" onClick={() => onOpenDetail('settings')}>
                            <Settings className="mr-2 h-4 w-4" />
                            {t('common.settings')}
                        </Button>
                        <Button variant="outline" size="sm" onClick={() => onOpenPrompt('new-task')}>
                            <Bot className="mr-2 h-4 w-4" />
                            {t('vibehub.promptGenerator.title')}
                        </Button>
                    </div>
                </div>

                <div className="mt-4 grid gap-3 lg:grid-cols-2">
                    <button
                        type="button"
                        onClick={() => onOpenDetail('git')}
                        className="min-h-20 rounded-md border bg-muted/10 p-3 text-left transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                            <GitBranch className="h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.recentCommit', 'Recent commit')}
                            <ExternalLink className="ml-auto h-3.5 w-3.5" />
                        </div>
                        <div className="mt-2 truncate text-sm font-medium">{commit.title}</div>
                        <div className="mt-1 truncate text-xs text-muted-foreground">{commit.detail}</div>
                    </button>
                    <button
                        type="button"
                        onClick={() => onOpenDetail('activity')}
                        className="min-h-20 rounded-md border bg-muted/10 p-3 text-left transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
                            <History className="h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.recentActivity', 'Recent activity')}
                        </div>
                        <div className="mt-2 truncate text-sm font-medium">{activity.title}</div>
                        <div className="mt-1 truncate text-xs text-muted-foreground">{activity.detail}</div>
                    </button>
                </div>
            </div>

            {status.warnings.length > 0 && (
                <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3">
                    <div className="flex items-center gap-2 text-sm font-medium text-amber-700 dark:text-amber-400">
                        <AlertCircle className="h-4 w-4" />
                        {t('vibehub.tabs.warnings')}
                        <Badge variant="destructive">{status.warnings.length}</Badge>
                    </div>
                    <ul className="mt-2 space-y-1 text-xs text-amber-800 dark:text-amber-300">
                        {status.warnings.map((warning) => (
                            <li key={warning} className="break-words">{warning}</li>
                        ))}
                    </ul>
                </div>
            )}

            <div>
                <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                    <div className="flex items-center gap-2 text-sm font-semibold">
                        <ClipboardList className="h-4 w-4" />
                        {labelOrFallback(t, 'vibehub.projectMap.activeTasks', 'Active tasks')}
                    </div>
                    {phaseValidation?.missing_outputs.length ? (
                        <button
                            type="button"
                            onClick={() => onOpenPhase(status.current_phase || null)}
                            className="inline-flex items-center gap-1 rounded-md border border-destructive/40 bg-destructive/5 px-2 py-1 text-xs text-destructive transition hover:border-destructive"
                        >
                            <AlertCircle className="h-3.5 w-3.5" />
                            {labelOrFallback(t, 'vibehub.kanban.schemaIssue', 'Schema issue')}
                        </button>
                    ) : (
                        <Badge variant="outline">{tasks.length}</Badge>
                    )}
                </div>

                {tasks.length === 0 ? (
                    <div className="rounded-md border bg-muted/20 p-8 text-center">
                        <div className="mx-auto flex h-11 w-11 items-center justify-center rounded-md border bg-background text-muted-foreground">
                            <ClipboardList className="h-5 w-5" />
                        </div>
                        <div className="mt-3 text-sm font-medium">{labelOrFallback(t, 'vibehub.projectMap.emptyTasks', 'No active tasks')}</div>
                        <div className="mx-auto mt-1 max-w-md text-xs text-muted-foreground">
                            {labelOrFallback(t, 'vibehub.projectMap.emptyTasksHint', 'Ask an agent to run vibehub-start so VibeHub can create one or more tasks from the current request.')}
                        </div>
                    </div>
                ) : (
                    <div className="space-y-3">
                        {tasks.map((task) => (
                            <ProjectTaskCard
                                key={`${task.current ? 'current' : 'task'}:${task.task_id}`}
                                task={task}
                                status={status}
                                currentFlow={currentFlow}
                                focusedTarget={focusedTarget}
                                onOpenDetail={onOpenDetail}
                                onOpenPhase={onOpenPhase}
                                t={t}
                            />
                        ))}
                    </div>
                )}
            </div>

            <div className="grid gap-4 lg:grid-cols-[minmax(0,1.1fr)_minmax(260px,0.9fr)]">
                <button
                    type="button"
                    onClick={() => onOpenDetail('structure')}
                    className="min-h-52 rounded-md border bg-background p-4 text-left transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    <div className="flex items-center gap-2 text-sm font-semibold">
                        <Boxes className="h-4 w-4" />
                        {labelOrFallback(t, 'vibehub.projectMap.structure', 'Project structure')}
                        {projectStructure?.truncated && (
                            <Badge variant="secondary" className="ml-auto">
                                {labelOrFallback(t, 'vibehub.projectMap.structureTruncated', 'Truncated')}
                            </Badge>
                        )}
                    </div>
                    <div className="mt-1 text-xs text-muted-foreground">
                        {labelOrFallback(t, 'vibehub.projectMap.structureSource', 'Filesystem scan; semantic graph is not configured yet.')}
                    </div>
                    <div className="mt-3 grid gap-3 md:grid-cols-2">
                        <div className="rounded-md border bg-muted/10 p-3">
                            <div className="text-xs font-medium text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.moduleMap', 'Module map')}</div>
                            <div className="mt-3 flex flex-wrap gap-2">
                                {structureNodes.length ? (
                                    structureNodes.map((node) => (
                                        <Badge key={node.id} variant={node.changed ? 'default' : 'secondary'}>
                                            {node.label}
                                        </Badge>
                                    ))
                                ) : (
                                    <span className="text-xs text-muted-foreground">
                                        {labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}
                                    </span>
                                )}
                            </div>
                        </div>
                        <div className="rounded-md border bg-muted/10 p-3">
                            <div className="text-xs font-medium text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.filePreview', 'File preview')}</div>
                            <div className="mt-2 space-y-1">
                                {(structureFiles.length ? structureFiles : (diffView?.changed_files.length ? diffView.changed_files.slice(0, 4) : ['.vibehub/', 'src-tauri/', 'src/'])).map((file) => (
                                    <div key={file} className="truncate font-mono text-xs text-muted-foreground">{file}</div>
                                ))}
                            </div>
                        </div>
                    </div>
                </button>

                <div className="min-h-52 rounded-md border bg-background p-4">
                    <div className="flex items-center justify-between gap-3">
                        <button
                            type="button"
                            onClick={() => setArchiveExpanded((value) => !value)}
                            className="flex min-w-0 items-center gap-2 text-left text-sm font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                            aria-expanded={archiveExpanded}
                        >
                            {archiveExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                            <Archive className="h-4 w-4" />
                            <span>{labelOrFallback(t, 'vibehub.projectMap.archive', 'Archive')}</span>
                        </button>
                        <div className="flex items-center gap-2">
                            <Badge variant="outline">{archiveCards.length}</Badge>
                            <Button variant="ghost" size="sm" onClick={() => onOpenDetail('archive')}>
                                <ExternalLink className="mr-2 h-4 w-4" />
                                {labelOrFallback(t, 'vibehub.projectMap.archiveOpen', 'Open')}
                            </Button>
                        </div>
                    </div>
                    {archiveExpanded && (
                        <div className="mt-3 space-y-2">
                            {archiveCards.length === 0 ? (
                                <div className="rounded-md border bg-muted/10 px-3 py-3 text-xs text-muted-foreground">
                                    {labelOrFallback(t, 'vibehub.projectMap.archiveEmpty', 'No completed or cancelled tasks yet.')}
                                </div>
                            ) : (
                                archiveCards.slice(0, 3).map((card) => (
                                    <ArchiveCardButton
                                        key={card.task_id}
                                        card={card}
                                        compact
                                        onClick={() => onOpenDetail('archive')}
                                        t={t}
                                    />
                                ))
                            )}
                            {archiveCards.length > 3 && (
                                <button
                                    type="button"
                                    onClick={() => onOpenDetail('archive')}
                                    className="w-full rounded-md border border-dashed bg-muted/10 px-3 py-2 text-left text-xs text-muted-foreground transition hover:border-primary/50"
                                >
                                    {labelOrFallback(t, 'vibehub.projectMap.archiveMore', '{{count}} more archived tasks', { count: archiveCards.length - 3 })}
                                </button>
                            )}
                        </div>
                    )}
                    {archiveView?.warnings.length ? (
                        <div className="mt-3 space-y-1">
                            {archiveView.warnings.slice(0, 2).map((warning) => (
                                <Notice key={warning} error message={warning} />
                            ))}
                        </div>
                    ) : null}
                </div>
            </div>

            {readOnlyActions.length > 0 && (
                <div className="rounded-md border bg-muted/10 p-4">
                    <div className="flex items-center justify-between gap-3">
                        <div>
                            <div className="text-sm font-medium">{t('vibehub.dashboard.readOnlyActions')}</div>
                            <div className="mt-1 text-xs text-muted-foreground">{t('vibehub.dashboard.readOnlyHint')}</div>
                        </div>
                        <Badge variant="outline">{t('vibehub.dashboard.readOnlyBadge')}</Badge>
                    </div>
                    <div className="mt-3 grid gap-2 md:grid-cols-2">
                        {readOnlyActions.map((action) => (
                            <div key={`${action.command}-${action.title}`} className="rounded-md border bg-background p-3">
                                <div className="flex flex-wrap items-center gap-2">
                                    <Badge>{action.command}</Badge>
                                    <span className="text-sm font-medium">{action.title}</span>
                                </div>
                                <div className="mt-1 text-xs text-muted-foreground">{action.description}</div>
                                <Button
                                    className="mt-3"
                                    variant="outline"
                                    size="sm"
                                    onClick={() => onOpenPrompt(action.promptTemplateId)}
                                >
                                    <Bot className="mr-2 h-4 w-4" />
                                    {t('vibehub.promptGenerator.generate')}
                                </Button>
                            </div>
                        ))}
                    </div>
                </div>
            )}
            <PromptActionGrid templates={promptTemplates} onOpenPrompt={onOpenPrompt} t={t} />
        </section>
    );
}

function ProjectTaskCard({
    task,
    status,
    currentFlow,
    focusedTarget,
    onOpenDetail,
    onOpenPhase,
    t,
}: {
    task: ProjectTaskCardState;
    status: VibehubCockpitStatus;
    currentFlow: ReturnType<typeof getModeFlow>;
    focusedTarget: FocusTarget | null;
    onOpenDetail: (detail: DashboardDetail) => void;
    onOpenPhase: (phase: string | null) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const steps = getTaskProcessSteps(task, status, currentFlow);
    const shortLabel = getTaskShortLabel(task, t);
    const hasSharedFiles = task.shared_files.length > 0;
    const taskFocused = focusedTarget?.taskId === task.task_id;

    return (
        <div className={`rounded-md border bg-background p-4 ${taskFocused ? 'border-primary shadow-sm ring-2 ring-primary/35' : task.current ? 'border-primary/40 shadow-sm ring-1 ring-primary/15' : hasSharedFiles ? 'border-amber-500/40' : ''}`}>
            <div className="grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(280px,1.1fr)] lg:items-center">
                <button
                    type="button"
                    onClick={() => onOpenDetail('task')}
                    className="min-w-0 rounded-md p-1 text-left transition hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    <div className="flex min-w-0 items-start gap-3">
                        <div className="flex h-10 min-w-12 max-w-20 items-center justify-center rounded-md border bg-muted/30 px-2 text-sm font-semibold">
                            <span className="truncate">{shortLabel}</span>
                        </div>
                        <div className="min-w-0 flex-1">
                            <div className="flex flex-wrap items-center gap-2">
                                <div className="truncate text-sm font-semibold">{task.title || task.task_id}</div>
                                {task.current && <Badge>{labelOrFallback(t, 'vibehub.kanban.current', 'Current')}</Badge>}
                                {task.intake_order && <Badge variant="outline">#{task.intake_order}</Badge>}
                            </div>
                            <div className="mt-1 break-all font-mono text-[11px] text-muted-foreground">{task.task_id}{task.run_id ? ` / ${task.run_id}` : ''}</div>
                            {task.dependencies.length > 0 && (
                                <div className="mt-2 flex flex-wrap gap-1">
                                    {task.dependencies.slice(0, 3).map((dependency) => (
                                        <Badge key={dependency} variant="secondary" className="text-[10px]">{dependency}</Badge>
                                    ))}
                                </div>
                            )}
                        </div>
                    </div>
                </button>

                <div
                    role="button"
                    tabIndex={0}
                    onClick={() => onOpenPhase(task.phase || status.current_phase || null)}
                    onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') onOpenPhase(task.phase || status.current_phase || null);
                    }}
                    className="rounded-md border bg-muted/10 p-3 transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    <div className="flex min-h-10 items-center gap-2 overflow-x-auto pb-1">
                        {steps.map((step, index) => (
                            <ProcessPill
                                key={`${task.task_id}:${step.phase || index}:${step.kind}`}
                                step={step}
                                focused={Boolean(taskFocused && focusedTarget?.capability && focusedTarget.capability === step.phase)}
                                onOpenPhase={onOpenPhase}
                                t={t}
                            />
                        ))}
                    </div>
                    {hasSharedFiles && (
                        <div className="mt-2 text-xs text-amber-800 dark:text-amber-300">
                            {labelOrFallback(t, 'vibehub.kanban.sharedFiles', 'Shared files')}: {task.shared_files.slice(0, 3).join(', ')}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

type ProcessStep = {
    kind: 'phase' | 'ellipsis';
    phase: string | null;
    status: string;
};

export function ProcessPill({
    step,
    focused,
    onOpenPhase,
    t,
}: {
    step: ProcessStep;
    focused?: boolean;
    onOpenPhase: (phase: string | null) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (step.kind === 'ellipsis') {
        return <span className="shrink-0 px-1 text-xs text-muted-foreground">...</span>;
    }
    const active = step.status === 'active' || step.status === 'running';
    return (
        <button
            type="button"
            onClick={(event) => {
                event.stopPropagation();
                onOpenPhase(step.phase);
            }}
            className={`shrink-0 rounded-md border px-3 py-1.5 text-xs transition hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${focused ? 'border-primary bg-primary/15 font-semibold text-primary ring-2 ring-primary/35' : active ? 'border-primary bg-primary/10 font-semibold text-primary ring-1 ring-primary/30' : flowStatusClass(step.status)}`}
        >
            {step.phase ? formatCapabilityLabel(step.phase, t) : t('common.unknown')}
        </button>
    );
}

function TaskDetailContent({
    status,
    selectedTaskId,
    selectedPhase,
    flowDetails,
    phaseValidation,
    events,
    t,
}: {
    status: VibehubCockpitStatus;
    selectedTaskId: string | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    phaseValidation: PhaseValidationResult | null;
    events: VibehubEventTimelineItem[];
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const tasks = getProjectTaskCards(status);
    if (!tasks.length) {
        return (
            <div className="py-2">
                <Notice error={false} message={labelOrFallback(t, 'vibehub.projectMap.emptyTasksHint', 'Ask an agent to run vibehub-start so VibeHub can create one or more tasks from the current request.')} />
            </div>
        );
    }

    return (
        <TaskLifecycleCanvas
            status={status}
            selectedTaskId={selectedTaskId}
            selectedPhase={selectedPhase}
            flowDetails={flowDetails}
            phaseValidation={phaseValidation}
            events={events}
            t={t}
        />
    );
}

function ActivityDetailContent({
    status,
    phaseValidation,
    events,
    onFocusTarget,
    t,
}: {
    status: VibehubCockpitStatus;
    phaseValidation: PhaseValidationResult | null;
    events: VibehubEventTimelineItem[];
    onFocusTarget: (target: FocusTarget) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const [taskFilter, setTaskFilter] = useState('all');
    const [capabilityFilter, setCapabilityFilter] = useState('all');
    const [typeFilter, setTypeFilter] = useState('all');
    const [selectedEventId, setSelectedEventId] = useState<string | null>(events[0]?.event_id || null);
    const activity = getRecentActivitySummary(status, phaseValidation, events, t);
    const taskOptions = uniqueValues(events.map((event) => event.task_id).filter(Boolean));
    const capabilityOptions = uniqueValues(events.map((event) => event.capability || '').filter(Boolean));
    const typeOptions = uniqueValues(events.map((event) => event.event_type).filter(Boolean));
    const filteredEvents = events.filter((event) => (
        (taskFilter === 'all' || event.task_id === taskFilter)
        && (capabilityFilter === 'all' || event.capability === capabilityFilter)
        && (typeFilter === 'all' || event.event_type === typeFilter)
    ));
    const selectedEvent = filteredEvents.find((event) => event.event_id === selectedEventId) || filteredEvents[0] || null;

    return (
        <div className="grid min-h-0 gap-5 lg:grid-cols-[320px_minmax(0,1fr)]">
            <div className="space-y-4 border-r pr-4">
                <div className="border-b pb-3">
                    <SectionEyebrow>{labelOrFallback(t, 'vibehub.projectMap.recentActivity', 'Recent activity')}</SectionEyebrow>
                    <div className="mt-2 text-sm font-semibold leading-snug">{activity.title}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{activity.detail}</div>
                </div>

                <div className="grid gap-2">
                    <TimelineFilterSelect
                        label={labelOrFallback(t, 'vibehub.activity.taskFilter', 'Task')}
                        value={taskFilter}
                        options={taskOptions}
                        allLabel={labelOrFallback(t, 'vibehub.activity.allTasks', 'All tasks')}
                        onChange={setTaskFilter}
                    />
                    <TimelineFilterSelect
                        label={labelOrFallback(t, 'vibehub.activity.capabilityFilter', 'Capability')}
                        value={capabilityFilter}
                        options={capabilityOptions}
                        allLabel={labelOrFallback(t, 'vibehub.activity.allCapabilities', 'All capabilities')}
                        onChange={setCapabilityFilter}
                        format={(value) => formatCapabilityLabel(value, t)}
                    />
                    <TimelineFilterSelect
                        label={labelOrFallback(t, 'vibehub.activity.typeFilter', 'Event type')}
                        value={typeFilter}
                        options={typeOptions}
                        allLabel={labelOrFallback(t, 'vibehub.activity.allTypes', 'All types')}
                        onChange={setTypeFilter}
                    />
                </div>

                <div>
                    <div className="mb-2 flex items-center justify-between gap-2">
                        <SectionEyebrow>{labelOrFallback(t, 'vibehub.activity.timeline', 'Timeline')}</SectionEyebrow>
                        <Badge variant="outline">{filteredEvents.length}</Badge>
                    </div>
                    {filteredEvents.length === 0 ? (
                        <Notice error={false} message={labelOrFallback(t, 'vibehub.activity.noEvents', 'No events match the current filters.')} />
                    ) : (
                        <div className="max-h-[calc(100vh-22rem)] overflow-auto pr-1">
                            {filteredEvents.map((event) => {
                                const selected = selectedEvent?.event_id === event.event_id;
                                return (
                                    <button
                                        key={event.event_id}
                                        type="button"
                                        onClick={() => setSelectedEventId(event.event_id)}
                                        className={`w-full border-l-2 px-3 py-2 text-left transition hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected ? 'border-primary bg-primary/5' : 'border-transparent'}`}
                                    >
                                        <div className="flex items-center gap-2">
                                            <span className="h-2 w-2 shrink-0 rounded-full bg-primary/70" />
                                            <span className="min-w-0 flex-1 truncate text-xs font-medium">{event.summary || event.event_type}</span>
                                            <span className="shrink-0 text-[11px] text-muted-foreground">{formatCompactDate(event.timestamp)}</span>
                                        </div>
                                        <div className="mt-1 flex min-w-0 items-center gap-1.5 pl-4 text-[11px] text-muted-foreground">
                                            <span className="truncate">{event.event_type}</span>
                                            {event.capability && <span>· {formatCapabilityLabel(event.capability, t)}</span>}
                                        </div>
                                    </button>
                                );
                            })}
                        </div>
                    )}
                </div>
            </div>

            <div className="min-w-0">
                {selectedEvent ? (
                    <div className="space-y-5">
                        <div className="border-b pb-4">
                            <div className="flex flex-wrap items-center gap-2">
                                <Badge variant="outline">{selectedEvent.event_type}</Badge>
                                {selectedEvent.capability && <Badge variant="secondary">{formatCapabilityLabel(selectedEvent.capability, t)}</Badge>}
                                <Badge variant="outline">{formatTimelineTime(selectedEvent.timestamp)}</Badge>
                            </div>
                            <h3 className="mt-3 text-base font-semibold leading-snug">{selectedEvent.summary || selectedEvent.event_type}</h3>
                            <div className="mt-2 break-all font-mono text-xs text-muted-foreground">{selectedEvent.event_id}</div>
                        </div>

                        <div className="flex flex-wrap gap-2">
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => onFocusTarget({
                                    taskId: selectedEvent.task_id,
                                    capability: selectedEvent.capability,
                                    eventId: selectedEvent.event_id,
                                })}
                            >
                                <Eye className="mr-2 h-4 w-4" />
                                {labelOrFallback(t, 'vibehub.activity.focusBoard', 'Focus board')}
                            </Button>
                        </div>

                        <DetailSection title={labelOrFallback(t, 'vibehub.activity.eventTarget', 'Event target')}>
                            <KeyValueRows
                                rows={[
                                    [t('vibehub.status.task'), selectedEvent.task_title || selectedEvent.task_id || t('common.unknown')],
                                    [t('vibehub.status.run'), selectedEvent.run_id || t('common.unknown')],
                                    [t('vibehub.status.phase'), selectedEvent.capability ? formatCapabilityLabel(selectedEvent.capability, t) : t('common.none')],
                                    [labelOrFallback(t, 'vibehub.activity.eventLog', 'Event log'), selectedEvent.event_log_path],
                                ]}
                            />
                        </DetailSection>

                        <DetailSection title={labelOrFallback(t, 'vibehub.activity.eventPayload', 'Event payload')}>
                            <div className="max-h-[calc(100vh-28rem)] overflow-auto border bg-muted/10 p-3">
                                <StructuredValueView value={selectedEvent.raw} />
                            </div>
                        </DetailSection>
                    </div>
                ) : (
                    <Notice error={false} message={labelOrFallback(t, 'vibehub.activity.noEvents', 'No events match the current filters.')} />
                )}
            </div>
        </div>
    );
}

function AgentUsageTabContent({
    localAgentUsage,
    t,
}: {
    localAgentUsage: LocalAgentUsageOverview | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!localAgentUsage) {
        return (
            <Notice
                error={false}
                message={labelOrFallback(t, 'vibehub.agentUsage.unavailable', 'Local agent usage has not been loaded yet.')}
            />
        );
    }

    const sources = [
        { title: 'Codex', summary: localAgentUsage.codex },
        { title: 'OpenCode', summary: localAgentUsage.opencode },
    ];

    return (
        <div className="space-y-5">
            <div className="grid gap-2 text-xs sm:grid-cols-2">
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.nonCachedTokens', 'Non-cache tokens')} value={formatAgentUsageTokens(localAgentUsage.non_cached_total_tokens)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.totalTokensWithCache', 'Total incl. cache')} value={formatAgentUsageTokens(localAgentUsage.total_tokens)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.sources', 'Sources')} value={String(localAgentUsage.source_count)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.generatedAt', 'Generated')} value={formatUsageIsoDate(localAgentUsage.generated_at)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.projectPath', 'Project')} value={localAgentUsage.project_path} />
            </div>

            {localAgentUsage.warnings.length > 0 && (
                <div className="space-y-2">
                    {localAgentUsage.warnings.map((warning) => (
                        <Notice key={warning} error={false} message={warning} />
                    ))}
                </div>
            )}

            <div className="grid gap-4 lg:grid-cols-2">
                {sources.map(({ title, summary }) => (
                    <AgentUsageSourceCard key={summary.source} title={title} summary={summary} t={t} />
                ))}
            </div>
        </div>
    );
}

function AgentUsageSourceCard({
    title,
    summary,
    t,
}: {
    title: string;
    summary: AgentUsageSourceSummary;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <section className="space-y-3 rounded-md border bg-muted/10 p-3">
            <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                    <div className="text-sm font-semibold">{title}</div>
                    <div className="mt-1 break-all text-[11px] text-muted-foreground">
                        {summary.data_path || labelOrFallback(t, 'vibehub.agentUsage.noSourcePath', 'No source path')}
                    </div>
                </div>
                <Badge variant={summary.available ? 'default' : 'secondary'}>
                    {summary.available ? labelOrFallback(t, 'vibehub.agentUsage.available', 'Available') : labelOrFallback(t, 'vibehub.agentUsage.noRecords', 'No records')}
                </Badge>
            </div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.records', 'Records')} value={String(summary.records)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.nonCachedTokens', 'Non-cache tokens')} value={formatAgentUsageTokens(summary.non_cached_total_tokens)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.totalTokensWithCache', 'Total incl. cache')} value={formatAgentUsageTokens(summary.total_tokens)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.cost', 'Cost')} value={formatAgentUsageCost(summary.cost)} />
                <ViewField label={labelOrFallback(t, 'vibehub.agentUsage.latest', 'Latest')} value={formatUsageTimestampMs(summary.latest_updated_at_ms)} />
            </div>

            <AgentUsageTokenBreakdownView tokens={summary.tokens} t={t} />

            {summary.recent.length > 0 ? (
                <div className="space-y-2">
                    <SectionEyebrow>{labelOrFallback(t, 'vibehub.agentUsage.recent', 'Recent')}</SectionEyebrow>
                    <div className="divide-y rounded-md border bg-background">
                        {summary.recent.slice(0, 6).map((item) => (
                            <div key={item.id} className="space-y-1 px-3 py-2 text-xs">
                                <div className="flex min-w-0 flex-wrap items-center gap-2">
                                    <span className="min-w-0 flex-1 truncate font-medium">{item.title}</span>
                                    <Badge variant="outline">{formatAgentUsageTokens(item.non_cached_total_tokens)}</Badge>
                                </div>
                                <div className="flex flex-wrap gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
                                    {item.agent && <span className="truncate">{item.agent}</span>}
                                    {item.model && <span className="truncate">{item.model}</span>}
                                    <span>{formatUsageTimestampMs(item.updated_at_ms)}</span>
                                    {item.cost != null && <span>{formatAgentUsageCost(item.cost)}</span>}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            ) : (
                <Notice
                    error={false}
                    message={labelOrFallback(t, 'vibehub.agentUsage.emptyRecent', 'No matching local records for this project.')}
                />
            )}

            {summary.warnings.length > 0 && (
                <div className="space-y-2">
                    {summary.warnings.map((warning) => (
                        <Notice key={`${summary.source}:${warning}`} error={false} message={warning} />
                    ))}
                </div>
            )}
        </section>
    );
}

function AgentUsageTokenBreakdownView({
    tokens,
    t,
}: {
    tokens: AgentUsageTokenBreakdown;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const rows: Array<[string, number]> = [
        [labelOrFallback(t, 'vibehub.agentUsage.input', 'Input'), tokens.input],
        [labelOrFallback(t, 'vibehub.agentUsage.output', 'Output'), tokens.output],
        [labelOrFallback(t, 'vibehub.agentUsage.reasoning', 'Reasoning'), tokens.reasoning],
        [labelOrFallback(t, 'vibehub.agentUsage.cachedInput', 'Cached input'), tokens.cached_input],
        [labelOrFallback(t, 'vibehub.agentUsage.cacheRead', 'Cache read'), tokens.cache_read],
        [labelOrFallback(t, 'vibehub.agentUsage.cacheWrite', 'Cache write'), tokens.cache_write],
    ];

    return (
        <div className="space-y-2">
            <SectionEyebrow>{labelOrFallback(t, 'vibehub.agentUsage.breakdown', 'Token breakdown')}</SectionEyebrow>
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
                {rows.map(([label, value]) => (
                    <div key={label} className="rounded-md border bg-background px-2 py-2">
                        <div className="truncate text-[10px] text-muted-foreground">{label}</div>
                        <div className="mt-1 truncate text-xs font-semibold">{formatAgentUsageTokens(value)}</div>
                    </div>
                ))}
            </div>
        </div>
    );
}

export function StructureDetailContent({
    projectStructure,
    projectPath,
    t,
}: {
    projectStructure: VibehubProjectStructureViewData | null;
    projectPath: string;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
    const [actionError, setActionError] = useState<string | null>(null);
    const allNodes = useMemo(
        () => dedupeStructureNodes([
            ...(projectStructure?.graph_nodes || []),
            ...flattenStructureNodes(projectStructure?.tree || []),
        ]),
        [projectStructure]
    );
    const selectedNode = allNodes.find((node) => node.id === selectedNodeId) || allNodes[0] || null;

    useEffect(() => {
        if (!allNodes.length) {
            setSelectedNodeId(null);
            return;
        }
        if (!selectedNodeId || !allNodes.some((node) => node.id === selectedNodeId)) {
            setSelectedNodeId(allNodes[0].id);
        }
    }, [allNodes, selectedNodeId]);

    const runFileAction = async (mode: 'reveal' | 'open') => {
        if (!selectedNode) return;
        setActionError(null);
        try {
            if (mode === 'reveal') {
                await tauriApi.vibehubRevealProjectFile(projectPath, selectedNode.path);
            } else {
                await tauriApi.vibehubOpenProjectFile(projectPath, selectedNode.path);
            }
        } catch (error) {
            setActionError(String(error));
        }
    };

    if (!projectStructure) {
        return <Notice error={false} message={labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')} />;
    }

    return (
        <div className="space-y-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                    <div className="text-sm font-medium">{labelOrFallback(t, 'vibehub.projectMap.structure', 'Project structure')}</div>
                    <div className="mt-1 text-xs text-muted-foreground">
                        {labelOrFallback(t, 'vibehub.projectMap.structureCounts', '{{dirs}} dirs · {{files}} files', {
                            dirs: projectStructure.scanned_dirs_count,
                            files: projectStructure.scanned_files_count,
                        })}
                    </div>
                </div>
                <Badge variant="outline">{projectStructure.source}</Badge>
            </div>

            {!projectStructure.semantic_graph_available && (
                <Notice
                    error={false}
                    message={labelOrFallback(t, 'vibehub.projectMap.structureSource', 'Filesystem scan; semantic graph is not configured yet.')}
                />
            )}

            {projectStructure.warnings.length ? (
                <div className="space-y-1">
                    {projectStructure.warnings.map((warning) => (
                        <Notice key={warning} error={false} message={warning} />
                    ))}
                </div>
            ) : null}

            {allNodes.length === 0 ? (
                <Notice error={false} message={labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')} />
            ) : (
                <div className="grid gap-4 lg:grid-cols-[minmax(0,1.15fr)_minmax(280px,0.85fr)]">
                    <div className="space-y-3 rounded-md border bg-muted/10 p-3">
                        <div className="flex items-center justify-between gap-2">
                            <div className="text-xs font-medium text-muted-foreground">
                                {labelOrFallback(t, 'vibehub.projectMap.structureGraph', 'Graph')}
                            </div>
                            <Badge variant="secondary">{projectStructure.graph_edges.length}</Badge>
                        </div>
                        <div className="grid gap-2 sm:grid-cols-2">
                            {projectStructure.graph_nodes.map((node) => (
                                <StructureGraphNodeButton
                                    key={node.id}
                                    node={node}
                                    selected={selectedNode?.id === node.id}
                                    onSelect={() => setSelectedNodeId(node.id)}
                                    t={t}
                                />
                            ))}
                        </div>
                    </div>

                    <div className="space-y-3 rounded-md border bg-muted/10 p-3">
                        <div className="text-xs font-medium text-muted-foreground">
                            {labelOrFallback(t, 'vibehub.projectMap.structureDirectory', 'Directory')}
                        </div>
                        <div className="max-h-[28rem] overflow-auto pr-1">
                            <StructureTreeList
                                nodes={projectStructure.tree}
                                selectedId={selectedNode?.id || null}
                                onSelect={setSelectedNodeId}
                                t={t}
                            />
                        </div>
                    </div>
                </div>
            )}

            {selectedNode && (
                <div className="rounded-md border bg-background p-3">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="min-w-0">
                            <div className="text-xs font-medium text-muted-foreground">
                                {labelOrFallback(t, 'vibehub.projectMap.structureSelected', 'Selected')}
                            </div>
                            <div className="mt-1 truncate font-mono text-sm">{selectedNode.path}</div>
                            <div className="mt-1 text-xs text-muted-foreground">
                                {formatStructureMeta(selectedNode, t)}
                            </div>
                        </div>
                        {selectedNode.changed && (
                            <Badge variant="default">
                                {labelOrFallback(t, 'vibehub.projectMap.structureChanged', 'Changed')}
                            </Badge>
                        )}
                    </div>
                    <div className="mt-3 grid gap-2 sm:grid-cols-2">
                        <Button variant="outline" size="sm" onClick={() => runFileAction('reveal')}>
                            <FolderOpen className="mr-2 h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.structureRevealFile', 'Show in file manager')}
                        </Button>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={() => runFileAction('open')}
                            disabled={selectedNode.kind !== 'file'}
                        >
                            <ExternalLink className="mr-2 h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.structureOpenFile', 'Open file')}
                        </Button>
                    </div>
                </div>
            )}

            {actionError && <Notice error message={actionError} />}

            <div className="rounded-md border border-dashed bg-muted/10 p-3 text-xs text-muted-foreground">
                {labelOrFallback(t, 'vibehub.projectMap.structureHistoryReserved', 'Module/file history is reserved for a later slice.')}
            </div>
        </div>
    );
}

function StructureGraphNodeButton({
    node,
    selected,
    onSelect,
    t,
}: {
    node: VibehubProjectStructureGraphNode;
    selected: boolean;
    onSelect: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const Icon = node.kind === 'file' ? FileText : Boxes;
    return (
        <button
            type="button"
            onClick={onSelect}
            className={`min-h-16 rounded-md border bg-background p-3 text-left transition hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected ? 'border-primary bg-primary/5' : ''}`}
        >
            <div className="flex items-center gap-2">
                <Icon className="h-4 w-4 text-muted-foreground" />
                <span className="min-w-0 truncate text-sm font-medium">{node.label}</span>
                {node.changed && <span className="ml-auto h-2 w-2 rounded-full bg-primary" />}
            </div>
            <div className="mt-2 truncate font-mono text-[11px] text-muted-foreground">{node.path}</div>
            <div className="mt-1 text-[11px] text-muted-foreground">{formatStructureMeta(node, t)}</div>
        </button>
    );
}

function StructureTreeList({
    nodes,
    selectedId,
    onSelect,
    t,
}: {
    nodes: VibehubProjectStructureTreeNode[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!nodes.length) {
        return <div className="text-xs text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}</div>;
    }

    return (
        <div className="space-y-1">
            {nodes.map((node) => (
                <StructureTreeRow
                    key={node.id}
                    node={node}
                    selectedId={selectedId}
                    onSelect={onSelect}
                    t={t}
                />
            ))}
        </div>
    );
}

function StructureTreeRow({
    node,
    selectedId,
    onSelect,
    t,
}: {
    node: VibehubProjectStructureTreeNode;
    selectedId: string | null;
    onSelect: (id: string) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const Icon = node.kind === 'file' ? FileText : FolderOpen;
    const selected = selectedId === node.id;
    const paddingLeft = Math.min(node.depth - 1, 5) * 14;

    return (
        <div>
            <button
                type="button"
                onClick={() => onSelect(node.id)}
                className={`flex w-full items-center gap-2 rounded-md border px-2 py-1.5 text-left transition hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected ? 'border-primary bg-primary/5' : 'border-transparent'}`}
                style={{ paddingLeft: `${paddingLeft + 8}px` }}
            >
                {node.children.length ? <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" /> : <span className="h-3.5 w-3.5" />}
                <Icon className="h-4 w-4 text-muted-foreground" />
                <span className="min-w-0 flex-1 truncate font-mono text-xs">{node.label}</span>
                {node.changed && <span className="h-2 w-2 rounded-full bg-primary" />}
                {node.truncated && (
                    <Badge variant="secondary" className="text-[10px]">
                        {labelOrFallback(t, 'vibehub.projectMap.structureTruncated', 'Truncated')}
                    </Badge>
                )}
            </button>
            {node.children.length > 0 && (
                <StructureTreeList
                    nodes={node.children}
                    selectedId={selectedId}
                    onSelect={onSelect}
                    t={t}
                />
            )}
        </div>
    );
}

function ArchiveDetailContent({
    archiveView,
    onOpenArtifact,
    t,
}: {
    archiveView: VibehubArchiveViewData | null;
    onOpenArtifact: (path: string) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const cards = archiveView?.cards || [];
    const [selectedTaskId, setSelectedTaskId] = useState<string | null>(cards[0]?.task_id || null);
    const selected = cards.find((card) => card.task_id === selectedTaskId) || cards[0] || null;

    useEffect(() => {
        if (!cards.length) {
            setSelectedTaskId(null);
            return;
        }
        if (!selectedTaskId || !cards.some((card) => card.task_id === selectedTaskId)) {
            setSelectedTaskId(cards[0].task_id);
        }
    }, [cards, selectedTaskId]);

    return (
        <div className="space-y-4">
            {cards.length === 0 ? (
                <Notice error={false} message={labelOrFallback(t, 'vibehub.projectMap.archiveEmpty', 'No completed or cancelled tasks yet.')} />
            ) : (
                <div className="grid gap-5 lg:grid-cols-[260px_minmax(0,1fr)]">
                    <div className="border-r pr-4">
                        <div className="mb-2 flex items-center justify-between gap-2">
                            <SectionEyebrow>{labelOrFallback(t, 'vibehub.projectMap.archive', 'Archive')}</SectionEyebrow>
                            <Badge variant="outline">{cards.length}</Badge>
                        </div>
                        <div className="mb-3 text-xs text-muted-foreground">
                            {labelOrFallback(t, 'vibehub.projectMap.archiveHint', 'Completed and cancelled task history stays inspectable here.')}
                        </div>
                        {archiveView?.warnings.length ? (
                            <div className="mb-3 space-y-2">
                                {archiveView.warnings.map((warning) => (
                                    <Notice key={warning} error message={warning} />
                                ))}
                            </div>
                        ) : null}
                        <div className="space-y-1">
                        {cards.map((card) => (
                            <ArchiveCardButton
                                key={card.task_id}
                                card={card}
                                selected={selected?.task_id === card.task_id}
                                onClick={() => setSelectedTaskId(card.task_id)}
                                t={t}
                            />
                        ))}
                        </div>
                    </div>
                    {selected && (
                        <div className="min-w-0 space-y-5">
                            <div className="border-b pb-4">
                                <div className="flex flex-wrap items-center gap-2">
                                    <ArchiveStatusIcon status={selected.status} />
                                    <Badge variant={selected.status === 'cancelled' ? 'secondary' : 'outline'}>
                                        {formatArchiveStatus(selected.status, t)}
                                    </Badge>
                                    {selected.phase && <Badge variant="secondary">{formatPhaseName(selected.phase, t)}</Badge>}
                                    {selected.updated_at && <Badge variant="outline">{formatCompactDate(selected.updated_at)}</Badge>}
                                </div>
                                <h3 className="mt-3 text-base font-semibold leading-snug">{selected.title || selected.task_id}</h3>
                                <div className="mt-2 break-words text-sm text-muted-foreground">{selected.summary}</div>
                            </div>

                            <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.taskMetadata', 'Task/run metadata')}>
                                <KeyValueRows
                                    rows={[
                                        [t('vibehub.status.task'), selected.task_id],
                                        [t('vibehub.status.run'), selected.run_id || t('common.unknown')],
                                        [t('vibehub.status.mode'), selected.mode ? formatMode(selected.mode, t) : t('common.unknown')],
                                        [t('vibehub.status.phase'), selected.phase || t('common.unknown')],
                                        [t('vibehub.status.phaseStatus'), formatStatusValue(selected.phase_status, t)],
                                        [labelOrFallback(t, 'vibehub.projectMap.archiveEventTotal', 'Events'), String(selected.event_count)],
                                    ]}
                                />
                            </DetailSection>

                            {selected.status_reason && (
                                <Notice error={selected.status === 'cancelled'} message={selected.status_reason} />
                            )}

                            <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.archiveProcess', 'Process')}>
                                <div className="divide-y border-y">
                                    {selected.process.length ? selected.process.map((step) => (
                                        <div key={`${step.name}-${step.status}`} className="flex items-center justify-between gap-3 py-2 text-xs">
                                            <div className="min-w-0">
                                                <div className="truncate font-medium">{formatPhaseName(step.name, t)}</div>
                                                <div className="text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.archiveEventCount', '{{count}} event(s)', { count: step.event_count })}</div>
                                            </div>
                                            <Badge variant="outline">{formatStatusValue(step.status, t)}</Badge>
                                        </div>
                                    )) : (
                                        <div className="py-2 text-xs text-muted-foreground">
                                            {labelOrFallback(t, 'vibehub.projectMap.archiveNoProcess', 'No process events recorded.')}
                                        </div>
                                    )}
                                </div>
                            </DetailSection>

                            <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.archiveArtifacts', 'Artifacts')}>
                                <ArchiveArtifactButtons
                                    title={labelOrFallback(t, 'vibehub.projectMap.archiveHandoffs', 'Handoffs')}
                                    artifacts={selected.handoff_artifacts}
                                    onOpenArtifact={onOpenArtifact}
                                    t={t}
                                />
                                <ArchiveArtifactButtons
                                    title={labelOrFallback(t, 'vibehub.projectMap.archiveOutputs', 'Outputs')}
                                    artifacts={selected.output_artifacts}
                                    onOpenArtifact={onOpenArtifact}
                                    t={t}
                                />
                                <ArchiveArtifactButtons
                                    title={labelOrFallback(t, 'vibehub.projectMap.archiveEvents', 'Events')}
                                    artifacts={selected.event_artifacts}
                                    onOpenArtifact={onOpenArtifact}
                                    t={t}
                                />
                            </DetailSection>

                            <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.archiveTimeline', 'Timeline')}>
                                <div className="divide-y border-y">
                                    {selected.events.length ? selected.events.slice(0, 24).map((event, index) => (
                                        <button
                                            key={`${event.event_id || index}-${event.event_type}`}
                                            type="button"
                                            onClick={() => onOpenArtifact(event.artifact_path)}
                                            className="w-full px-1 py-2 text-left text-xs transition hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                        >
                                            <div className="flex items-center justify-between gap-2">
                                                <span className="truncate font-medium">{event.event_type}</span>
                                                {event.timestamp && <span className="shrink-0 text-muted-foreground">{formatCompactDate(event.timestamp)}</span>}
                                            </div>
                                            <div className="mt-1 break-words text-muted-foreground">{event.summary}</div>
                                        </button>
                                    )) : (
                                        <div className="py-2 text-xs text-muted-foreground">
                                            {labelOrFallback(t, 'vibehub.projectMap.archiveNoEvents', 'No event log entries recorded.')}
                                        </div>
                                    )}
                                </div>
                            </DetailSection>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}

export function ArchiveCardButton({
    card,
    compact = false,
    selected = false,
    onClick,
    t,
}: {
    card: VibehubArchivedTaskCard;
    compact?: boolean;
    selected?: boolean;
    onClick: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const cancelled = card.status === 'cancelled';
    return (
        <button
            type="button"
            onClick={onClick}
            className={`w-full rounded-md border px-3 py-2 text-left text-xs transition hover:border-primary/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected ? 'border-primary bg-primary/5' : cancelled ? 'bg-muted/40 text-muted-foreground' : 'bg-muted/10'}`}
        >
            <div className="flex items-start gap-2">
                <span className={`flex h-8 w-10 shrink-0 items-center justify-center rounded-md border font-semibold ${cancelled ? 'bg-muted text-muted-foreground' : 'bg-background text-foreground'}`}>
                    {card.label}
                </span>
                <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-2">
                        <ArchiveStatusIcon status={card.status} />
                        <span className="truncate font-medium">{card.title || card.task_id}</span>
                    </span>
                    <span className={`mt-1 block ${compact ? 'line-clamp-1' : 'line-clamp-2'} text-muted-foreground`}>
                        {card.summary}
                    </span>
                    {!compact && (
                        <span className="mt-2 flex flex-wrap gap-1">
                            <Badge variant="outline">{formatArchiveStatus(card.status, t)}</Badge>
                            {card.phase && <Badge variant="secondary">{formatPhaseName(card.phase, t)}</Badge>}
                        </span>
                    )}
                </span>
            </div>
        </button>
    );
}

function ArchiveStatusIcon({ status }: { status: string }) {
    return status === 'cancelled'
        ? <XCircle className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
        : <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-600" />;
}

function ArchiveArtifactButtons({
    title,
    artifacts,
    onOpenArtifact,
    t,
}: {
    title: string;
    artifacts: { label: string; path: string; exists: boolean }[];
    onOpenArtifact: (path: string) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <div className="mb-3 last:mb-0">
            <div className="mb-1 text-xs text-muted-foreground">{title}</div>
            {artifacts.length ? (
                <div className="divide-y border-y">
                    {artifacts.map((artifact) => (
                        <button
                            key={artifact.path}
                            type="button"
                            onClick={() => onOpenArtifact(artifact.path)}
                            className="flex w-full items-center gap-2 px-1 py-2 text-left text-xs transition hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                        >
                            <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                            <span className="min-w-0 flex-1 truncate">{artifact.label}</span>
                            {!artifact.exists && (
                                <Badge variant="outline">{t('vibehub.stateValues.missing')}</Badge>
                            )}
                            <ExternalLink className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        </button>
                    ))}
                </div>
            ) : (
                <div className="border-y py-2 text-xs text-muted-foreground">
                    {labelOrFallback(t, 'vibehub.projectMap.archiveNoArtifacts', 'No artifacts found.')}
                </div>
            )}
        </div>
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

export function RecommendedCommand({
    command,
    text,
    promptTemplateId,
    onOpenPrompt,
    t,
}: {
    command: string;
    text: string;
    promptTemplateId?: VibehubPromptTemplateId;
    onOpenPrompt?: (templateId: VibehubPromptTemplateId) => void;
    t?: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <div className="rounded-md border bg-background p-2 text-xs">
            <div className="flex items-start justify-between gap-2">
                <Badge className="mb-1">{command}</Badge>
                {promptTemplateId && onOpenPrompt && t && (
                    <Button variant="ghost" size="sm" className="h-7 px-2" onClick={() => onOpenPrompt(promptTemplateId)}>
                        <Bot className="mr-1.5 h-3.5 w-3.5" />
                        {t('vibehub.promptGenerator.generate')}
                    </Button>
                )}
            </div>
            <div className="text-muted-foreground">{text}</div>
        </div>
    );
}

export function PromptActionGrid({
    templates,
    onOpenPrompt,
    t,
}: {
    templates: VibehubPromptTemplateOption[];
    onOpenPrompt: (templateId: VibehubPromptTemplateId) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <div className="rounded-md border bg-muted/10 p-3">
            <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-sm font-medium">
                    <Bot className="h-4 w-4" />
                    {t('vibehub.promptGenerator.title')}
                </div>
                <Badge variant="outline">{templates.length}</Badge>
            </div>
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
                {templates.map((template) => (
                    <Button
                        key={template.id}
                        variant={template.dangerous ? 'secondary' : 'outline'}
                        size="sm"
                        className="justify-start"
                        onClick={() => onOpenPrompt(template.id)}
                    >
                        <ClipboardList className="mr-2 h-4 w-4" />
                        {t(`vibehub.promptGenerator.templates.${template.id}`)}
                    </Button>
                ))}
            </div>
        </div>
    );
}

function PromptGeneratorModal({
    open,
    onOpenChange,
    templates,
    selectedTemplateId,
    result,
    loading,
    error,
    confirmed,
    copied,
    onTemplateChange,
    onConfirm,
    onCopy,
    t,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    templates: VibehubPromptTemplateOption[];
    selectedTemplateId: VibehubPromptTemplateId;
    result: VibehubPromptRenderResult | null;
    loading: boolean;
    error: string | null;
    confirmed: boolean;
    copied: boolean;
    onTemplateChange: (templateId: VibehubPromptTemplateId) => void;
    onConfirm: () => void;
    onCopy: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const copyDisabled = loading || !result?.content || (result.confirmation_required && !confirmed);
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="max-w-3xl">
                <DialogHeader>
                    <DialogTitle>{t('vibehub.promptGenerator.title')}</DialogTitle>
                    <DialogDescription>
                        {t('vibehub.promptGenerator.description')}
                    </DialogDescription>
                </DialogHeader>

                <div className="space-y-4">
                    <label className="space-y-1">
                        <span className="text-xs font-medium text-muted-foreground">{t('vibehub.promptGenerator.template')}</span>
                        <select
                            value={selectedTemplateId}
                            onChange={(event) => onTemplateChange(event.target.value as VibehubPromptTemplateId)}
                            className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                        >
                            {templates.map((template) => (
                                <option key={template.id} value={template.id}>
                                    {t(`vibehub.promptGenerator.templates.${template.id}`)}
                                </option>
                            ))}
                        </select>
                    </label>

                    {error && <Notice error message={error} />}
                    {loading && <div className="text-xs text-muted-foreground">{t('common.loading')}</div>}

                    {result && (
                        <>
                            <div className="grid grid-cols-2 gap-2 text-xs">
                                <ViewField label={t('vibehub.promptGenerator.source')} value={formatPromptSource(result.source, t)} />
                                <ViewField label={t('vibehub.status.locale')} value={result.locale} />
                                <ViewField label={t('vibehub.promptGenerator.templatePath')} value={result.template_path} />
                                <ViewField label={t('vibehub.promptGenerator.dangerous')} value={result.dangerous ? t('common.yes') : t('common.no')} />
                            </div>

                            {result.confirmation_required && !confirmed && (
                                <div className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3">
                                    <div className="flex items-start gap-2 text-sm">
                                        <AlertCircle className="mt-0.5 h-4 w-4 text-amber-700 dark:text-amber-400" />
                                        <div>
                                            <div className="font-medium text-amber-800 dark:text-amber-300">{t('vibehub.promptGenerator.confirmTitle')}</div>
                                            <div className="mt-1 text-xs text-amber-900/80 dark:text-amber-200/80">
                                                {result.confirmation_message || t('vibehub.promptGenerator.confirmBody')}
                                            </div>
                                            <Button className="mt-3" size="sm" variant="outline" onClick={onConfirm}>
                                                <ShieldCheck className="mr-2 h-4 w-4" />
                                                {t('vibehub.promptGenerator.confirm')}
                                            </Button>
                                        </div>
                                    </div>
                                </div>
                            )}

                            <div className="space-y-2">
                                <div className="flex items-center justify-between gap-2">
                                    <div className="text-xs font-medium text-muted-foreground">{t('vibehub.promptGenerator.renderedPrompt')}</div>
                                    <Button size="sm" onClick={onCopy} disabled={copyDisabled}>
                                        {copied ? <CheckCircle2 className="mr-2 h-4 w-4" /> : <Copy className="mr-2 h-4 w-4" />}
                                        {copied ? t('vibehub.promptGenerator.copied') : t('vibehub.promptGenerator.copy')}
                                    </Button>
                                </div>
                                <pre className="max-h-[46vh] overflow-auto rounded-md border bg-muted/20 p-3 whitespace-pre-wrap break-words text-xs">
                                    {result.content}
                                </pre>
                            </div>
                        </>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    );
}

// Tab content components.

function TaskLifecycleCanvas({
    status,
    selectedTaskId,
    selectedPhase,
    flowDetails,
    phaseValidation,
    events,
    t,
}: {
    status: VibehubCockpitStatus;
    selectedTaskId: string | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    phaseValidation: PhaseValidationResult | null;
    events: VibehubEventTimelineItem[];
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const tasks = getProjectTaskCards(status);
    const selectedTask = tasks.find((task) => task.task_id === selectedTaskId)
        || tasks.find((task) => task.current)
        || tasks[0]
        || null;
    const nodes = selectedTask ? getTaskLifecycleNodes(selectedTask, status, flowDetails, events) : [];
    const preferredPhase = selectedPhase
        || nodes.find((node) => node.status === 'active' || node.status === 'running')?.phase
        || selectedTask?.phase
        || nodes.find((node) => node.kind === 'phase')?.phase
        || null;
    const [inspectedPhase, setInspectedPhase] = useState<string | null>(preferredPhase);

    useEffect(() => {
        setInspectedPhase(preferredPhase);
    }, [preferredPhase, selectedTask?.task_id]);

    if (!selectedTask) {
        return <Notice error={false} message={labelOrFallback(t, 'vibehub.projectMap.emptyTasksHint', 'Ask an agent to run vibehub-start so VibeHub can create one or more tasks from the current request.')} />;
    }

    const inspectedNode = nodes.find((node) => node.phase === inspectedPhase)
        || nodes.find((node) => node.status === 'active' || node.status === 'running')
        || nodes[0]
        || null;
    const totalEvents = nodes.reduce((count, node) => count + node.events.length, 0);
    const artifactCount = nodes.reduce((count, node) => count + (node.detail?.read_inputs.length || 0) + (node.detail?.written_outputs.length || 0), 0);
    const hasWarnings = status.warnings.length > 0 && selectedTask.current;

    return (
        <div className="grid min-h-0 gap-5 xl:grid-cols-[minmax(0,1fr)_21rem]">
            <div className="min-w-0 space-y-5">
                <div className="border-b pb-4">
                    <div className="flex flex-wrap items-center gap-2">
                        <Badge variant={selectedTask.current ? 'default' : 'outline'}>
                            {selectedTask.current ? labelOrFallback(t, 'vibehub.kanban.current', 'Current') : formatStatusValue(selectedTask.phase_status, t)}
                        </Badge>
                        {selectedTask.phase && <Badge variant="secondary">{formatCapabilityLabel(selectedTask.phase, t)}</Badge>}
                        {selectedTask.mode && <Badge variant="outline">{formatMode(selectedTask.mode, t)}</Badge>}
                    </div>
                    <div className="mt-3 flex items-start gap-3">
                        <div className="flex h-11 w-14 shrink-0 items-center justify-center border bg-muted/20 text-xs font-semibold">
                            {getTaskShortLabel(selectedTask, t)}
                        </div>
                        <div className="min-w-0 flex-1">
                            <h3 className="text-base font-semibold leading-snug">{selectedTask.title || selectedTask.task_id}</h3>
                            <div className="mt-1 break-all font-mono text-xs text-muted-foreground">
                                {selectedTask.task_id}{selectedTask.run_id ? ` / ${selectedTask.run_id}` : ''}
                            </div>
                        </div>
                    </div>
                </div>

                <div className="grid gap-3 sm:grid-cols-3">
                    <LifecycleMetric label={labelOrFallback(t, 'vibehub.lifecycle.phases', 'Phases')} value={String(nodes.filter((node) => node.phase).length)} />
                    <LifecycleMetric label={labelOrFallback(t, 'vibehub.lifecycle.artifacts', 'Artifacts')} value={String(artifactCount)} />
                    <LifecycleMetric label={labelOrFallback(t, 'vibehub.lifecycle.events', 'Events')} value={String(totalEvents)} tone={hasWarnings ? 'warn' : 'normal'} />
                </div>

                <section className="space-y-3">
                    <div className="flex items-center justify-between gap-3">
                        <div className="flex items-center gap-2">
                            <Network className="h-4 w-4 text-muted-foreground" />
                            <SectionEyebrow>{labelOrFallback(t, 'vibehub.lifecycle.canvas', 'Lifecycle canvas')}</SectionEyebrow>
                        </div>
                        <Badge variant="outline">{labelOrFallback(t, 'vibehub.dashboard.readOnlyBadge', 'Read-only')}</Badge>
                    </div>

                    <div className="relative overflow-x-auto border bg-muted/10 p-4">
                        <svg className="pointer-events-none absolute left-0 top-0 h-full min-h-[19rem] w-full" aria-hidden="true">
                            {nodes.slice(0, -1).map((node, index) => {
                                const x1 = 12 + (index + 0.5) * (76 / Math.max(nodes.length, 1));
                                const x2 = 12 + (index + 1.5) * (76 / Math.max(nodes.length, 1));
                                return (
                                    <line
                                        key={`${node.phase || node.stage}-line`}
                                        x1={`${x1}%`}
                                        y1="42%"
                                        x2={`${x2}%`}
                                        y2="42%"
                                        stroke="currentColor"
                                        strokeWidth="1.5"
                                        strokeDasharray={node.status === 'skipped' ? '4 5' : undefined}
                                        className="text-border"
                                    />
                                );
                            })}
                        </svg>
                        <div className="relative grid min-w-[760px] gap-3" style={{ gridTemplateColumns: `repeat(${Math.max(nodes.length, 1)}, minmax(8.5rem, 1fr))` }}>
                            {nodes.map((node) => (
                                <LifecycleNodeButton
                                    key={`${node.stage}:${node.phase || 'skipped'}`}
                                    node={node}
                                    selected={inspectedNode?.phase === node.phase}
                                    onSelect={() => setInspectedPhase(node.phase)}
                                    t={t}
                                />
                            ))}
                        </div>
                    </div>
                </section>

                <LifecycleEventRail nodes={nodes} t={t} />
            </div>

            <LifecycleInspector
                task={selectedTask}
                node={inspectedNode}
                status={status}
                phaseValidation={phaseValidation}
                t={t}
            />
        </div>
    );
}

function LifecycleMetric({ label, value, tone = 'normal' }: { label: string; value: string; tone?: 'normal' | 'warn' }) {
    return (
        <div className="border px-3 py-2">
            <div className="text-[10px] font-medium uppercase text-muted-foreground">{label}</div>
            <div className={`mt-1 text-lg font-semibold ${tone === 'warn' ? 'text-amber-700 dark:text-amber-300' : ''}`}>{value}</div>
        </div>
    );
}

function LifecycleNodeButton({
    node,
    selected,
    onSelect,
    t,
}: {
    node: LifecycleNode;
    selected: boolean;
    onSelect: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const inputCount = node.detail?.read_inputs.length || 0;
    const outputCount = node.detail?.written_outputs.length || 0;
    const statusClass = node.status === 'skipped'
        ? 'border-dashed bg-background/70 text-muted-foreground'
        : selected
            ? 'border-primary bg-primary/10 text-primary ring-2 ring-primary/25'
            : flowStatusClass(node.status);

    return (
        <button
            type="button"
            onClick={onSelect}
            disabled={!node.phase}
            className={`group flex min-h-[16rem] flex-col justify-between border p-3 text-left transition hover:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default disabled:hover:border-border ${statusClass}`}
        >
            <div className="space-y-3">
                <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                        <div className="text-[10px] font-medium uppercase text-muted-foreground">{node.stage}</div>
                        <div className="mt-1 truncate text-sm font-semibold">
                            {node.phase ? formatCapabilityLabel(node.phase, t) : labelOrFallback(t, 'vibehub.lifecycle.skipped', 'Skipped')}
                        </div>
                    </div>
                    <Badge variant={phaseBadgeVariant(node.status)} className="shrink-0 text-[10px]">
                        {formatStatusValue(node.status, t)}
                    </Badge>
                </div>

                <div className="space-y-1.5 text-[11px] text-muted-foreground">
                    <LifecycleNodeStat label={t('vibehub.dashboard.readInputs')} value={String(inputCount)} />
                    <LifecycleNodeStat label={t('vibehub.dashboard.writtenOutputs')} value={String(outputCount)} />
                    <LifecycleNodeStat label={labelOrFallback(t, 'vibehub.lifecycle.events', 'Events')} value={String(node.events.length)} />
                </div>
            </div>

            <div className="mt-4 space-y-1 text-[11px]">
                {node.detail?.context_pack_path ? (
                    <div className="truncate font-mono text-muted-foreground">{node.detail.context_pack_path}</div>
                ) : (
                    <div className="text-muted-foreground">{labelOrFallback(t, 'vibehub.lifecycle.noPackage', 'No package recorded')}</div>
                )}
                <div className="h-1.5 w-full bg-border">
                    <div className={`h-1.5 ${node.status === 'completed' ? 'w-full bg-emerald-500' : node.status === 'active' || node.status === 'running' ? 'w-2/3 bg-primary' : node.status === 'skipped' ? 'w-0' : 'w-1/4 bg-muted-foreground/40'}`} />
                </div>
            </div>
        </button>
    );
}

function LifecycleNodeStat({ label, value }: { label: string; value: string }) {
    return (
        <div className="flex items-center justify-between gap-2">
            <span className="truncate">{label}</span>
            <span className="font-mono font-semibold">{value}</span>
        </div>
    );
}

function LifecycleEventRail({ nodes, t }: { nodes: LifecycleNode[]; t: (key: string, options?: Record<string, unknown>) => string }) {
    const events = nodes.flatMap((node) => node.events.map((event) => ({ event, phase: node.phase }))).slice(0, 6);
    if (!events.length) {
        return (
            <DetailSection title={labelOrFallback(t, 'vibehub.lifecycle.history', 'History')}>
                <Notice error={false} message={labelOrFallback(t, 'vibehub.lifecycle.noEvents', 'No timeline events are recorded for this task yet.')} />
            </DetailSection>
        );
    }

    return (
        <DetailSection title={labelOrFallback(t, 'vibehub.lifecycle.history', 'History')}>
            <div className="divide-y border-y">
                {events.map(({ event, phase }) => (
                    <div key={event.event_id} className="grid gap-2 py-2 text-xs sm:grid-cols-[7rem_minmax(0,1fr)_auto]">
                        <div className="font-mono text-[11px] text-muted-foreground">{formatCompactDate(event.timestamp)}</div>
                        <div className="min-w-0">
                            <div className="truncate font-medium">{event.summary || event.event_type}</div>
                            <div className="mt-0.5 truncate text-[11px] text-muted-foreground">{event.event_type}</div>
                        </div>
                        {phase && <Badge variant="outline" className="justify-self-start text-[10px]">{formatCapabilityLabel(phase, t)}</Badge>}
                    </div>
                ))}
            </div>
        </DetailSection>
    );
}

function LifecycleInspector({
    task,
    node,
    status,
    phaseValidation,
    t,
}: {
    task: ProjectTaskCardState;
    node: LifecycleNode | null;
    status: VibehubCockpitStatus;
    phaseValidation: PhaseValidationResult | null;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const detail = node?.detail || null;
    const matchingValidation = node?.phase && phaseValidation?.phase === node.phase ? phaseValidation : null;

    return (
        <aside className="min-w-0 space-y-5 border-l pl-5">
            <div className="border-b pb-4">
                <SectionEyebrow>{labelOrFallback(t, 'vibehub.lifecycle.inspector', 'Inspector')}</SectionEyebrow>
                <h3 className="mt-2 text-base font-semibold leading-snug">
                    {node?.phase ? formatCapabilityLabel(node.phase, t) : labelOrFallback(t, 'vibehub.lifecycle.noPhaseSelected', 'No phase selected')}
                </h3>
                <div className="mt-2 flex flex-wrap gap-2">
                    {node && <Badge variant={phaseBadgeVariant(node.status)}>{formatStatusValue(node.status, t)}</Badge>}
                    {node?.phase && <Badge variant="outline">{node.phase}</Badge>}
                </div>
            </div>

            <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.taskMetadata', 'Task/run metadata')}>
                <KeyValueRows
                    rows={[
                        [t('vibehub.status.task'), task.task_id],
                        [t('vibehub.status.run'), task.run_id || t('common.none')],
                        [t('vibehub.status.mode'), task.mode ? formatMode(task.mode, t) : t('common.unknown')],
                        [t('vibehub.status.phaseStatus'), formatStatusValue(task.phase_status, t)],
                    ]}
                />
            </DetailSection>

            {detail ? (
                <>
                    <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.selectedPhaseTransfer', 'Selected phase transfer')}>
                        <div className="space-y-4">
                            <ArtifactList title={t('vibehub.dashboard.readInputs')} artifacts={detail.read_inputs} t={t} />
                            <ArtifactList title={t('vibehub.dashboard.writtenOutputs')} artifacts={detail.written_outputs} t={t} />
                        </div>
                    </DetailSection>

                    <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.phaseFiles', 'Phase files')}>
                        <KeyValueRows
                            rows={[
                                [labelOrFallback(t, 'vibehub.tabs.contextPackPath', 'Context pack'), detail.context_pack_path],
                                [labelOrFallback(t, 'vibehub.tabs.manifestPath', 'Manifest'), detail.manifest_path],
                                [labelOrFallback(t, 'vibehub.phase.sourceOutput', 'Source output'), detail.phase_output_path],
                                [labelOrFallback(t, 'vibehub.status.reviewEvidence', 'Review evidence'), detail.review_path || t('vibehub.tabs.notAvailable')],
                            ]}
                        />
                    </DetailSection>
                </>
            ) : (
                <Notice error={false} message={labelOrFallback(t, 'vibehub.lifecycle.noTransfer', 'This phase does not have recorded transfer artifacts yet.')} />
            )}

            {matchingValidation && (
                <DetailSection title={t('vibehub.phase.title')}>
                    <KeyValueRows
                        rows={[
                            [t('vibehub.phase.validationStatus'), formatStatusValue(matchingValidation.status, t)],
                            [t('vibehub.phase.requiredOutputs'), String(matchingValidation.required_outputs.length)],
                            [t('vibehub.phase.missingOutputs'), String(matchingValidation.missing_outputs.length)],
                            [t('vibehub.phase.sourceOutput'), matchingValidation.source_output_path || t('common.none')],
                        ]}
                    />
                    {matchingValidation.missing_outputs.length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-1">
                            {matchingValidation.missing_outputs.map((item) => (
                                <Badge key={item} variant="destructive">{item}</Badge>
                            ))}
                        </div>
                    )}
                </DetailSection>
            )}

            {node?.events.length ? (
                <DetailSection title={labelOrFallback(t, 'vibehub.lifecycle.nodeEvents', 'Node events')}>
                    <div className="max-h-48 overflow-auto border-y">
                        {node.events.slice(0, 8).map((event) => (
                            <div key={event.event_id} className="border-b py-2 text-xs last:border-b-0">
                                <div className="truncate font-medium">{event.summary || event.event_type}</div>
                                <div className="mt-1 flex items-center justify-between gap-2 text-[11px] text-muted-foreground">
                                    <span className="truncate">{event.event_type}</span>
                                    <span className="shrink-0">{formatTimelineTime(event.timestamp)}</span>
                                </div>
                            </div>
                        ))}
                    </div>
                </DetailSection>
            ) : null}

            {(task.active_capabilities.length > 0 || task.shared_files.length > 0 || task.dependencies.length > 0) && (
                <DetailSection title={labelOrFallback(t, 'vibehub.projectMap.taskRelations', 'Relations')}>
                    <div className="space-y-3">
                        {task.active_capabilities.length > 0 && (
                            <InlineTokenRow
                                label={labelOrFallback(t, 'vibehub.projectMap.activeCapabilities', 'Active capabilities')}
                                values={task.active_capabilities.map((item) => formatCapabilityLabel(item, t))}
                            />
                        )}
                        {task.dependencies.length > 0 && (
                            <InlineTokenRow
                                label={labelOrFallback(t, 'vibehub.projectMap.dependencies', 'Dependencies')}
                                values={task.dependencies}
                            />
                        )}
                        {task.shared_files.length > 0 && (
                            <InlineTokenRow
                                label={labelOrFallback(t, 'vibehub.projectMap.sharedFiles', 'Shared files')}
                                values={task.shared_files}
                                mono
                            />
                        )}
                    </div>
                </DetailSection>
            )}

            {status.warnings.length > 0 && task.current && (
                <DetailSection title={t('vibehub.tabs.warnings')}>
                    <div className="space-y-2">
                        {status.warnings.map((warning) => (
                            <Notice key={warning} error message={warning} />
                        ))}
                    </div>
                </DetailSection>
            )}
        </aside>
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
            <div className="mt-2 divide-y border-y">
                {artifacts.length ? artifacts.map((artifact) => (
                    <div key={`${artifact.label}:${artifact.path}`} className="grid gap-2 py-2 text-xs sm:grid-cols-[110px_minmax(0,1fr)_auto]">
                        <span className="font-medium">{artifact.label}</span>
                        <span className="min-w-0 break-all font-mono text-[11px] text-muted-foreground">{artifact.path}</span>
                        <Badge variant={artifact.exists ? 'secondary' : 'outline'}>
                            {artifact.exists ? t('vibehub.stateValues.exists') : t('vibehub.stateValues.missing')}
                        </Badge>
                    </div>
                )) : (
                    <div className="py-2 text-xs text-muted-foreground">{t('common.none')}</div>
                )}
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
    selectedTaskId,
    selectedPhase,
    flowDetails,
    events,
    t,
}: {
    status: VibehubCockpitStatus | null;
    phaseValidation: PhaseValidationResult | null;
    selectedTaskId: string | null;
    selectedPhase: string | null;
    flowDetails: VibehubFlowDetail[];
    events: VibehubEventTimelineItem[];
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!status) return null;

    return (
        <TaskLifecycleCanvas
            status={status}
            selectedTaskId={selectedTaskId}
            selectedPhase={selectedPhase}
            flowDetails={flowDetails}
            phaseValidation={phaseValidation}
            events={events}
            t={t}
        />
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
        <div className="space-y-4">
            <div className="flex items-center gap-2 border-b pb-3 text-sm font-medium">
                <ShieldCheck className="h-4 w-4" />
                {labelOrFallback(t, 'vibehub.tabs.evidenceDetail', 'Evidence map')}
            </div>
            <div className="divide-y border-y">
                {rows.map((row) => (
                    <div key={`${row.grade}-${row.title}`} className="grid gap-2 py-3 text-xs md:grid-cols-[190px_minmax(0,1fr)_auto]">
                        <div className="flex items-center gap-2 font-medium">
                            {row.icon}
                            <span>{row.title}</span>
                        </div>
                        <div className="min-w-0 break-all text-muted-foreground">{row.value}</div>
                        <Badge variant={row.grade === 'hard_observed' ? 'default' : 'secondary'} className="justify-self-start text-[10px]">
                            {row.grade}
                        </Badge>
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
    projectPath,
    t,
}: {
    candidates: PreviewCandidate[];
    selectedPath: string;
    onSelectedPathChange: (path: string) => void;
    file: VibehubFileReadResult | null;
    loading: boolean;
    error: string | null;
    projectPath: string;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!candidates.length) {
        return <div className="pt-1 text-xs text-muted-foreground">{labelOrFallback(t, 'vibehub.tabs.noPreviewTargets', 'No VibeHub preview targets are available.')}</div>;
    }

    return (
        <div className="space-y-3 pt-1">
            <div className="flex items-center gap-2 text-sm font-medium">
                <Eye className="h-4 w-4" />
                {labelOrFallback(t, 'vibehub.tabs.previewDetail', 'Structured package viewer')}
            </div>
            <PackageRendererContent
                candidates={candidates}
                selectedPath={selectedPath}
                onSelectedPathChange={onSelectedPathChange}
                file={file}
                loading={loading}
                error={error}
                projectPath={projectPath}
                t={t}
            />
        </div>
    );
}

function PackageRendererContent({
    candidates,
    selectedPath,
    onSelectedPathChange,
    file,
    loading,
    error,
    projectPath,
    t,
}: {
    candidates: PreviewCandidate[];
    selectedPath: string;
    onSelectedPathChange: (path: string) => void;
    file: VibehubFileReadResult | null;
    loading: boolean;
    error: string | null;
    projectPath: string;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!candidates.length) {
        return <div className="pt-1 text-xs text-muted-foreground">{labelOrFallback(t, 'vibehub.tabs.noPreviewTargets', 'No VibeHub preview targets are available.')}</div>;
    }

    const openFile = async (mode: 'reveal' | 'open') => {
        if (!selectedPath) return;
        if (mode === 'reveal') {
            await tauriApi.vibehubRevealVibehubFile(projectPath, selectedPath);
        } else {
            await tauriApi.vibehubOpenVibehubFile(projectPath, selectedPath);
        }
    };

    return (
        <div className="space-y-3">
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

            <div className="grid gap-2 sm:grid-cols-2">
                <Button variant="outline" size="sm" onClick={() => openFile('reveal')} disabled={!selectedPath}>
                    <FolderOpen className="mr-2 h-4 w-4" />
                    {labelOrFallback(t, 'vibehub.activity.revealFile', 'Show in file manager')}
                </Button>
                <Button variant="outline" size="sm" onClick={() => openFile('open')} disabled={!selectedPath}>
                    <ExternalLink className="mr-2 h-4 w-4" />
                    {labelOrFallback(t, 'vibehub.activity.openDefault', 'Open with default app')}
                </Button>
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
                        <StructuredFilePreview path={file.path} content={file.content} />
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

function TimelineFilterSelect({
    label,
    value,
    options,
    allLabel,
    onChange,
    format,
}: {
    label: string;
    value: string;
    options: string[];
    allLabel: string;
    onChange: (value: string) => void;
    format?: (value: string) => string;
}) {
    return (
        <label className="space-y-1">
            <span className="text-[10px] font-medium text-muted-foreground">{label}</span>
            <select
                value={value}
                onChange={(event) => onChange(event.target.value)}
                className="h-9 w-full rounded-md border border-input bg-background px-2 text-xs shadow-sm"
            >
                <option value="all">{allLabel}</option>
                {options.map((option) => (
                    <option key={option} value={option}>{format ? format(option) : option}</option>
                ))}
            </select>
        </label>
    );
}

function StructuredFilePreview({ path, content }: { path: string; content: string }) {
    const jsonlRows = path.endsWith('.jsonl')
        ? content.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => safeJsonParse(line))
        : null;
    const jsonValue = !jsonlRows && (path.endsWith('.json') || path.endsWith('.yaml') || path.endsWith('.yml'))
        ? safeJsonParse(content)
        : null;

    if (jsonlRows?.length) {
        return (
            <div className="max-h-96 space-y-2 overflow-auto rounded-md border bg-background p-3">
                {jsonlRows.map((row, index) => (
                    <div key={index} className="rounded-md border bg-muted/10 p-2">
                        <div className="mb-2 text-[10px] font-medium text-muted-foreground">#{index + 1}</div>
                        <StructuredValueView value={row ?? { parse_error: 'Invalid JSON line' }} />
                    </div>
                ))}
            </div>
        );
    }

    if (jsonValue) {
        return (
            <div className="max-h-96 overflow-auto rounded-md border bg-background p-3">
                <StructuredValueView value={jsonValue} />
            </div>
        );
    }

    return <MarkdownPreview content={content} />;
}

function StructuredValueView({ value }: { value: unknown }) {
    if (Array.isArray(value)) {
        return (
            <div className="space-y-2">
                {value.map((item, index) => (
                    <div key={index} className="rounded-md border bg-background p-2">
                        <div className="mb-1 text-[10px] font-medium text-muted-foreground">[{index}]</div>
                        <StructuredValueView value={item} />
                    </div>
                ))}
            </div>
        );
    }

    if (value && typeof value === 'object') {
        return (
            <div className="space-y-1.5">
                {Object.entries(value as Record<string, unknown>).map(([key, nested]) => (
                    <div key={key} className="grid gap-1 rounded border bg-background px-2 py-1.5 text-xs sm:grid-cols-[120px_minmax(0,1fr)]">
                        <div className="break-all font-mono text-[11px] text-muted-foreground">{key}</div>
                        <div className="min-w-0">
                            {nested && typeof nested === 'object' ? (
                                <StructuredValueView value={nested} />
                            ) : (
                                <span className="break-all">{String(nested ?? 'null')}</span>
                            )}
                        </div>
                    </div>
                ))}
            </div>
        );
    }

    return <span className="break-all text-xs">{String(value ?? 'null')}</span>;
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

export function Notice({ message, error }: { message: string; error: boolean }) {
    return (
        <div className={`flex items-start gap-2 rounded-md border p-3 text-sm ${error ? 'border-destructive/40 text-destructive' : 'border-border text-muted-foreground'}`}>
            <AlertCircle className="mt-0.5 h-4 w-4 flex-shrink-0" />
            <span className="break-all">{message}</span>
        </div>
    );
}

function DetailSection({ title, children }: { title: string; children: ReactNode }) {
    return (
        <section className="space-y-2">
            <SectionEyebrow>{title}</SectionEyebrow>
            {children}
        </section>
    );
}

function SectionEyebrow({ children }: { children: ReactNode }) {
    return (
        <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {children}
        </div>
    );
}

function KeyValueRows({ rows }: { rows: Array<[string, string]> }) {
    return (
        <div className="divide-y border-y">
            {rows.map(([label, value]) => (
                <div key={`${label}:${value}`} className="grid gap-2 py-2 text-xs sm:grid-cols-[150px_minmax(0,1fr)]">
                    <div className="text-muted-foreground">{label}</div>
                    <div className="min-w-0 break-all font-medium">{value}</div>
                </div>
            ))}
        </div>
    );
}

function InlineTokenRow({ label, values, mono = false }: { label: string; values: string[]; mono?: boolean }) {
    return (
        <div>
            <div className="mb-1 text-xs text-muted-foreground">{label}</div>
            <div className="flex flex-wrap gap-1.5">
                {values.map((value) => (
                    <Badge key={value} variant="secondary" className={mono ? 'font-mono' : undefined}>
                        {value}
                    </Badge>
                ))}
            </div>
        </div>
    );
}

function ViewField({ label, value }: { label: string; value: string }) {
    return (
        <div className="border-y px-1 py-1.5">
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
        task: labelOrFallback(t, 'vibehub.projectMap.taskDetail', 'Task detail'),
        phase: t('vibehub.dashboard.phaseFlow'),
        git: t('vibehub.dashboard.gitSummary'),
        activity: labelOrFallback(t, 'vibehub.projectMap.activityDetail', 'Recent activity'),
        context: t('vibehub.status.contextPack'),
        output: t('vibehub.status.agentOutput'),
        handoff: t('vibehub.status.handoff'),
        review: t('vibehub.status.reviewEvidence'),
        research: t('vibehub.tabs.research'),
        evidence: t('vibehub.dashboard.evidenceMap'),
        preview: labelOrFallback(t, 'vibehub.tabs.preview', 'Preview'),
        adapters: t('vibehub.dashboard.adapterStatus'),
        settings: t('common.settings'),
        structure: labelOrFallback(t, 'vibehub.projectMap.structure', 'Project structure'),
        archive: labelOrFallback(t, 'vibehub.projectMap.archive', 'Archive'),
        agentUsage: labelOrFallback(t, 'vibehub.agentUsage.title', 'AI usage'),
    };
    return titles[detail];
}

function getDetailDrawerWidthClass(detail: DashboardDetail) {
    if (detail === 'structure') return 'max-w-5xl';
    if (detail === 'task' || detail === 'phase') return 'max-w-5xl';
    if (detail === 'activity' || detail === 'archive' || detail === 'git' || detail === 'agentUsage') {
        return 'max-w-4xl';
    }
    return 'max-w-xl';
}

function phaseBadgeVariant(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    if (status === 'active') return 'default';
    if (status === 'completed') return 'secondary';
    if (status === 'needs_action' || status === 'blocked' || status === 'failed') return 'destructive';
    return 'outline';
}

export function getProjectTaskCards(status: VibehubCockpitStatus): ProjectTaskCardState[] {
    if (status.active_tasks.length > 0) {
        return status.active_tasks.map((task) => ({
            ...task,
            shared_files: status.neighbor_tasks.find((neighbor) => neighbor.task_id === task.task_id)?.shared_files || [],
        }));
    }
    if (!status.current_task_id) return [];
    const current: ProjectTaskCardState = {
        task_id: status.current_task_id,
        title: status.current_task_title,
        run_id: status.current_run_id,
        mode: status.current_mode,
        phase: status.current_phase,
        phase_status: status.phase_status,
        current: true,
        active_capabilities: status.active_capabilities,
        dependencies: [],
        intake_order: null,
        shared_files: [],
    };
    const neighbors: ProjectTaskCardState[] = status.neighbor_tasks.map((neighbor) => ({
        task_id: neighbor.task_id,
        title: neighbor.title,
        run_id: null,
        mode: null,
        phase: neighbor.active_capabilities[0] || null,
        phase_status: neighbor.active_capabilities.length ? 'active' : 'pending',
        current: false,
        active_capabilities: neighbor.active_capabilities,
        dependencies: [],
        intake_order: null,
        shared_files: neighbor.shared_files,
    }));
    return [current, ...neighbors];
}

export function getRecentCommitSummary(
    gitBranches: VibehubGitBranchesView | null,
    project: Project | null,
    t: (key: string, options?: Record<string, unknown>) => string
) {
    const currentBranch = gitBranches?.branches.find((branch) => branch.is_current);
    if (currentBranch) {
        return {
            title: currentBranch.last_commit_subject || currentBranch.name,
            detail: `${currentBranch.name} · ${currentBranch.head_sha.slice(0, 8)} · ${formatAheadBehind(currentBranch.ahead, currentBranch.behind, t)}`,
        };
    }
    return {
        title: project?.metadata.git_branch || t('common.unknown'),
        detail: project?.metadata.git_has_changes ? t('vibehub.status.gitDirty') : t('vibehub.status.clean'),
    };
}

export function getRecentActivitySummary(
    status: VibehubCockpitStatus,
    phaseValidation: PhaseValidationResult | null,
    events: VibehubEventTimelineItem[],
    t: (key: string, options?: Record<string, unknown>) => string
) {
    if (phaseValidation?.missing_outputs.length) {
        return {
            title: labelOrFallback(t, 'vibehub.kanban.schemaIssue', 'Schema issue'),
            detail: t('vibehub.dashboard.missingSectionsCount', { count: phaseValidation.missing_outputs.length }),
        };
    }
    const latest = events[0];
    if (latest) {
        return {
            title: latest.summary || latest.event_type,
            detail: `${latest.task_title || latest.task_id || t('common.unknown')} · ${formatTimelineTime(latest.timestamp)}`,
        };
    }
    if (status.current_task_id && status.current_phase) {
        return {
            title: `${status.current_task_title || status.current_task_id} · ${formatCapabilityLabel(status.current_phase, t)}`,
            detail: formatStatusValue(status.phase_status || 'active', t),
        };
    }
    return {
        title: labelOrFallback(t, 'vibehub.projectMap.noActivity', 'No activity yet'),
        detail: labelOrFallback(t, 'vibehub.projectMap.noActivityHint', 'VibeHub events will appear here after tasks start.'),
    };
}

export function getTaskProcessSteps(
    task: ProjectTaskCardState,
    status: VibehubCockpitStatus,
    currentFlow: ReturnType<typeof getModeFlow>
): ProcessStep[] {
    const base = task.current
        ? currentFlow
        : flowForTaskMode(task.mode || status.current_mode || 'guided_drive', task.phase, task.phase_status);
    const activeIndex = base.findIndex((item) => item.status === 'active' || item.status === 'running' || item.phase === task.phase);
    if (base.length <= 5 || activeIndex < 0) {
        return base.map((item) => ({ kind: 'phase', phase: item.phase, status: item.status }));
    }
    const visible = new Set([0, base.length - 1, activeIndex - 1, activeIndex, activeIndex + 1].filter((index) => index >= 0 && index < base.length));
    const steps: ProcessStep[] = [];
    base.forEach((item, index) => {
        if (!visible.has(index)) {
            if (steps[steps.length - 1]?.kind !== 'ellipsis') {
                steps.push({ kind: 'ellipsis', phase: null, status: 'pending' });
            }
            return;
        }
        steps.push({ kind: 'phase', phase: item.phase, status: item.status });
    });
    return steps;
}

function getTaskLifecycleNodes(
    task: ProjectTaskCardState,
    status: VibehubCockpitStatus,
    flowDetails: VibehubFlowDetail[],
    events: VibehubEventTimelineItem[]
): LifecycleNode[] {
    const base = task.current
        ? getModeFlow(status, status.current_mode || task.mode || 'guided_drive')
        : flowForTaskMode(task.mode || status.current_mode || 'guided_drive', task.phase, task.phase_status);
    return base.map((item, index) => {
        const detail = item.phase
            ? flowDetails.find((candidate) => candidate.phase === item.phase) || null
            : null;
        const nodeEvents = item.phase
            ? events.filter((event) => (
                (!task.task_id || event.task_id === task.task_id)
                && (event.capability === item.phase || event.event_type.includes(item.phase || ''))
            ))
            : [];
        return {
            kind: 'phase',
            index,
            stage: item.stage,
            phase: item.phase,
            status: item.status,
            detail,
            events: nodeEvents,
        };
    });
}

function flowForTaskMode(mode: string, activePhase?: string | null, activeStatus?: string | null) {
    const phases = MODE_STAGE_PHASES[mode] || MODE_STAGE_PHASES.guided_drive;
    return FLOW_STAGES.map((stage) => {
        const phase = phases[stage] || null;
        return {
            stage,
            phase,
            status: phase
                ? phase === activePhase
                    ? activeStatus || 'active'
                    : 'pending'
                : 'skipped',
        };
    });
}

export function getTaskShortLabel(task: VibehubActiveTask | ProjectTaskCardState, t: (key: string, options?: Record<string, unknown>) => string) {
    const text = `${task.title || ''} ${task.task_id}`.toLowerCase();
    if (/fix|bug|修|錯|错/.test(text)) return labelOrFallback(t, 'vibehub.taskLabels.fix', 'Fix');
    if (/doc|文档|文件|說明|说明/.test(text)) return labelOrFallback(t, 'vibehub.taskLabels.docs', 'Docs');
    if (/research|investigate|调研|研究/.test(text)) return labelOrFallback(t, 'vibehub.taskLabels.research', 'Research');
    if (/test|验证|驗證/.test(text)) return labelOrFallback(t, 'vibehub.taskLabels.validate', 'Test');
    if (/ui|front|界面|看板/.test(text)) return 'UI';
    const source = (task.title || task.task_id).replace(/\s+/g, '');
    return source.slice(0, Math.min(source.length, 6)) || labelOrFallback(t, 'vibehub.taskLabels.task', 'Task');
}

export function getModeFlow(status: VibehubCockpitStatus | null, fallbackMode: string) {
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

function formatMode(mode: string | null | undefined, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!mode) return t('common.unknown');
    return labelOrFallback(t, `vibehub.modes.${mode}`, mode);
}

function formatCapabilityLabel(capability: string, t: (key: string, options?: Record<string, unknown>) => string) {
    return labelOrFallback(t, `vibehub.flowPhases.${capability}`, capability);
}

function formatPhaseName(phase: string, t: (key: string, options?: Record<string, unknown>) => string) {
    return formatCapabilityLabel(phase, t);
}

function formatStatusValue(value: string | null | undefined, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!value) return t('common.unknown');
    return labelOrFallback(t, `vibehub.stateValues.${value}`, value);
}

function formatArchiveStatus(value: string, t: (key: string, options?: Record<string, unknown>) => string) {
    return labelOrFallback(t, `vibehub.archiveStatuses.${value}`, formatStatusValue(value, t));
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

function formatPromptSource(source: string, t: (key: string, options?: Record<string, unknown>) => string) {
    return labelOrFallback(t, `vibehub.promptGenerator.sources.${source}`, source);
}

function formatTimelineTime(timestamp: string) {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    if (Number.isNaN(date.getTime())) return timestamp;
    return date.toLocaleString();
}

function formatCompactDate(timestamp: string) {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    if (Number.isNaN(date.getTime())) return timestamp;
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

function formatUsageIsoDate(timestamp: string) {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    if (Number.isNaN(date.getTime())) return timestamp;
    return date.toLocaleString();
}

function formatUsageTimestampMs(timestamp: number | null | undefined) {
    if (!timestamp) return '--';
    const millis = timestamp < 10_000_000_000 ? timestamp * 1000 : timestamp;
    const date = new Date(millis);
    if (Number.isNaN(date.getTime())) return String(timestamp);
    return date.toLocaleString();
}

function formatAgentUsageTokens(value: number) {
    if (!Number.isFinite(value) || value <= 0) return '0';
    if (value >= 1_000_000_000) return `${trimUsageNumber(value / 1_000_000_000)}B`;
    if (value >= 1_000_000) return `${trimUsageNumber(value / 1_000_000)}M`;
    if (value >= 1_000) return `${trimUsageNumber(value / 1_000)}K`;
    return Math.round(value).toLocaleString();
}

function trimUsageNumber(value: number) {
    return value >= 10 ? String(Math.round(value)) : value.toFixed(1).replace(/\.0$/, '');
}

function formatAgentUsageCost(value: number | null | undefined) {
    if (value == null || !Number.isFinite(value)) return '--';
    return `$${value.toFixed(value >= 10 ? 2 : 4).replace(/0+$/, '').replace(/\.$/, '')}`;
}

function uniqueValues(values: string[]) {
    return Array.from(new Set(values)).sort((a, b) => a.localeCompare(b));
}

function safeJsonParse(content: string): unknown | null {
    try {
        return JSON.parse(content);
    } catch {
        return null;
    }
}

function getPreviewCandidates(
    status: VibehubCockpitStatus | null,
    contextView: VibehubContextViewData | null,
    reviewView: VibehubReviewViewData | null,
    handoffView: VibehubHandoffViewData | null,
    archiveView: VibehubArchiveViewData | null
): PreviewCandidate[] {
    const candidates: PreviewCandidate[] = [];
    addPreviewCandidate(candidates, '.vibehub/agent-view/current.md', 'Current view', true);
    addPreviewCandidate(candidates, '.vibehub/agent-view/current-context.md', 'Current context', true);
    addPreviewCandidate(candidates, handoffView?.handoff_path || status?.handoff_status.path, 'Handoff', Boolean(handoffView?.handoff_exists || status?.handoff_status.exists));
    addPreviewCandidate(candidates, contextView?.pack_path || status?.context_pack_status.path, 'Context pack', Boolean(contextView?.pack_exists || status?.context_pack_status.exists));
    addPreviewCandidate(candidates, contextView?.manifest_path, 'Context manifest', Boolean(contextView?.manifest_exists));
    addPreviewCandidate(candidates, status?.agent_output_status.path, 'Agent output', Boolean(status?.agent_output_status.exists));
    if (status?.current_task_id && status.current_run_id) {
        addPreviewCandidate(
            candidates,
            `.vibehub/tasks/${status.current_task_id}/runs/${status.current_run_id}/events.jsonl`,
            'Event log',
            true
        );
    }
    addPreviewCandidate(candidates, reviewView?.review_path, 'Review evidence', Boolean(reviewView?.review_exists));
    addPreviewCandidate(candidates, reviewView?.diff_patch_path, 'Diff patch', Boolean(reviewView?.diff_patch_exists));
    addPreviewCandidate(candidates, reviewView?.changed_files_path, 'Changed files', Boolean(reviewView?.changed_files_path));
    archiveView?.cards.forEach((card) => {
        addPreviewCandidate(candidates, card.task_path, `Archive task ${card.task_id}`, true);
        card.event_artifacts.forEach((artifact) => addPreviewCandidate(candidates, artifact.path, artifact.label, artifact.exists));
        card.handoff_artifacts.forEach((artifact) => addPreviewCandidate(candidates, artifact.path, artifact.label, artifact.exists));
        card.output_artifacts.forEach((artifact) => addPreviewCandidate(candidates, artifact.path, artifact.label, artifact.exists));
    });
    // Prefer canonical agent-view sync report. Do NOT inject the workspace-root
    // `.vibehub/sync.md` — that path is an r9 orphan and not produced by the
    // current sync pipeline. If the backend later surfaces `state.sync.last_report`
    // on the cockpit status, this should be threaded through here as the first
    // sync candidate, with `.vibehub/agent-view/sync.md` as the fallback.
    addPreviewCandidate(candidates, '.vibehub/agent-view/sync.md', 'Sync report', true);
    return candidates;
}

export function flattenStructureTree(nodes: VibehubProjectStructureTreeNode[]): string[] {
    return flattenStructureNodes(nodes)
        .filter((node) => node.kind === 'file' || node.kind === 'directory')
        .map((node) => node.path);
}

function flattenStructureNodes(nodes: VibehubProjectStructureTreeNode[]): VibehubProjectStructureTreeNode[] {
    const flattened: VibehubProjectStructureTreeNode[] = [];
    const visit = (items: VibehubProjectStructureTreeNode[]) => {
        items.forEach((item) => {
            flattened.push(item);
            visit(item.children);
        });
    };
    visit(nodes);
    return flattened;
}

function dedupeStructureNodes(nodes: StructureNodeLike[]): StructureNodeLike[] {
    const seen = new Set<string>();
    const deduped: StructureNodeLike[] = [];
    nodes.forEach((node) => {
        if (seen.has(node.id)) return;
        seen.add(node.id);
        deduped.push(node);
    });
    return deduped;
}

function formatStructureMeta(node: StructureNodeLike, t: (key: string, options?: Record<string, unknown>) => string) {
    if (node.kind === 'file') {
        return labelOrFallback(t, 'vibehub.projectMap.structureFile', 'File');
    }
    return labelOrFallback(t, 'vibehub.projectMap.structureNodeMeta', '{{dirs}} dirs · {{files}} files', {
        dirs: node.directory_count,
        files: node.file_count,
    });
}

function addPreviewCandidate(candidates: PreviewCandidate[], path: string | null | undefined, label: string, exists: boolean) {
    if (!path || candidates.some((candidate) => candidate.path === path)) return;
    candidates.push({ path, label, exists });
}

export function labelOrFallback(t: (key: string, options?: Record<string, unknown>) => string, key: string, fallback: string, options?: Record<string, unknown>) {
    const value = t(key, options);
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
            promptTemplateId: 'sync',
        });
    }
    if (status.git_dirty || driftReport?.head_changed || driftReport?.dirty) {
        actions.push({
            command: 'vibehub-sync',
            title: t('vibehub.dashboard.actionDrift'),
            description: t('vibehub.dashboard.actionDriftDesc', { count: status.git_changed_files_count || 0 }),
            promptTemplateId: 'sync',
        });
    }
    if (driftReport?.context_stale || contextView?.stale || !status.context_pack_status.exists) {
        actions.push({
            command: 'vibehub-context',
            title: t('vibehub.dashboard.actionContext'),
            description: t('vibehub.dashboard.actionContextDesc'),
            promptTemplateId: 'claim-capability',
        });
    }
    if (phaseValidation?.missing_outputs.length) {
        actions.push({
            command: 'vibehub-checkpoint',
            title: t('vibehub.dashboard.actionPhaseOutput'),
            description: t('vibehub.dashboard.actionPhaseOutputDesc', { count: phaseValidation.missing_outputs.length }),
            promptTemplateId: 'fix-schema',
        });
    }
    if (!handoffView?.handoff_exists || handoffView.missing_sections.length > 0) {
        actions.push({
            command: 'vibehub-handoff',
            title: t('vibehub.dashboard.actionHandoff'),
            description: t('vibehub.dashboard.actionHandoffDesc'),
            promptTemplateId: 'release-capability',
        });
    }
    return actions;
}
