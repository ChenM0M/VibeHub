import { useEffect, useState, type ReactNode } from 'react';
import { AlertCircle, Bot, Code2, Eye, FileText, GitBranch, Lightbulb, PackagePlus, Pause, Play, RefreshCw, SearchCheck, ShieldCheck, Wrench } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AgentAdapterStatus, AgentTool, PhaseValidationResult, Project, ResearchStatus, VibehubCockpitStatus, VibehubContextViewData, VibehubDiffViewData, VibehubFileReadResult, VibehubFileStatus, VibehubHandoffViewData, VibehubReviewViewData, VibehubSyncReport, WorkspaceDriftReport } from '@/types';
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

interface VibehubCockpitDialogProps {
    isOpen: boolean;
    onClose: () => void;
    project: Project | null;
}

type ActionState = {
    message: string;
    error: boolean;
};

type CockpitAction = 'init' | 'start-task' | 'build-context' | 'continue' | 'agent-sync' | 'workspace-sync' | 'recover-drift' | 'review' | 'handoff' | 'journal' | 'knowledge' | 'validate-phase' | 'complete-phase' | 'advance-phase' | 'pause-phase';

type RecommendedAction = {
    action: CockpitAction;
    title: string;
    description: string;
    disabled?: boolean;
};

const COCKPIT_TABS = ['status', 'context', 'evidence', 'preview', 'review', 'handoff', 'research', 'diff'] as const;
type CockpitTab = (typeof COCKPIT_TABS)[number];
type PreviewCandidate = { path: string; label: string; exists: boolean };

const AGENT_TOOL_OPTIONS: Array<{ id: AgentTool; label: string; description: string }> = [
    { id: 'codex', label: 'Codex', description: 'AGENTS.md + .agents/skills repo skills' },
    { id: 'claude_code', label: 'Claude Code', description: 'CLAUDE.md + .claude commands + constraints' },
    { id: 'opencode', label: 'OpenCode', description: 'AGENTS.md + .opencode commands + constraints' },
];
const DRIVE_MODE_OPTIONS = ['yolo_drive', 'guided_drive', 'evidence_drive'] as const;
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
            <DialogContent className="max-w-2xl">
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
    const [syncReport, setSyncReport] = useState<VibehubSyncReport | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [actionState, setActionState] = useState<ActionState | null>(null);
    const [runningAction, setRunningAction] = useState<CockpitAction | null>(null);
    const [agentTools, setAgentTools] = useState<AgentTool[]>(['codex', 'claude_code', 'opencode']);
    const [taskMode, setTaskMode] = useState<(typeof DRIVE_MODE_OPTIONS)[number]>('guided_drive');
    const [selectedCommandName, setSelectedCommandName] = useState('vibehub-sync');
    const [commandOverrideBody, setCommandOverrideBody] = useState('');
    const [journalTitle, setJournalTitle] = useState('');
    const [journalBody, setJournalBody] = useState('');
    const [knowledgeNote, setKnowledgeNote] = useState('');
    const [showAdvancedDiagnostics, setShowAdvancedDiagnostics] = useState(false);
    const [selectedLocale, setSelectedLocale] = useState<string>('en');

    // Tab state
    const [activeTab, setActiveTab] = useState<CockpitTab>('status');
    const [contextView, setContextView] = useState<VibehubContextViewData | null>(null);
    const [reviewView, setReviewView] = useState<VibehubReviewViewData | null>(null);
    const [handoffView, setHandoffView] = useState<VibehubHandoffViewData | null>(null);
    const [researchStatus, setResearchStatus] = useState<ResearchStatus | null>(null);
    const [diffView, setDiffView] = useState<VibehubDiffViewData | null>(null);
    const [phaseValidation, setPhaseValidation] = useState<PhaseValidationResult | null>(null);
    const [previewPath, setPreviewPath] = useState('');
    const [previewFile, setPreviewFile] = useState<VibehubFileReadResult | null>(null);
    const [previewError, setPreviewError] = useState<string | null>(null);
    const [isPreviewLoading, setIsPreviewLoading] = useState(false);
    const previewCandidates = getPreviewCandidates(status, contextView, reviewView, handoffView);
    const previewCandidateKey = previewCandidates.map((candidate) => candidate.path).join('|');

    const loadStatus = async (clearActionState = true) => {
        if (!project) return;
        setIsLoading(true);
        try {
            // ONE round-trip pulls status + context + review + handoff + diff +
            // research with a single cached `git` invocation. Replaces the old
            // 6 separate vibehubRead* calls that ran on tab switch.
            const overview = await tauriApi.vibehubReadOverview(project.path);
            setStatus(overview.status);
            setContextView(overview.context);
            setReviewView(overview.review);
            setHandoffView(overview.handoff);
            setDiffView(overview.diff);
            setResearchStatus(overview.research);
            if (overview.status.locale) {
                setSelectedLocale(overview.status.locale);
            }
            if (overview.initialized) {
                const nextAdapterStatus = await tauriApi.vibehubGetAgentAdapterStatus(project.path);
                setAdapterStatus(nextAdapterStatus);
                setAgentTools(nextAdapterStatus.enabled_tools.length ? nextAdapterStatus.enabled_tools : agentTools);
            } else {
                setAdapterStatus(null);
            }
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
            setAdapterStatus(null);
            setDriftReport(null);
            setSyncReport(null);
            setActionState(null);
            setRunningAction(null);
            setContextView(null);
            setReviewView(null);
            setHandoffView(null);
            setDiffView(null);
            setResearchStatus(null);
            setPreviewPath('');
            setPreviewFile(null);
            setPreviewError(null);
        }
    }, [enabled, project?.path]);

    // Phase validation is on-demand only (it parses session output and is the
    // one piece of data the overview deliberately does NOT bundle, because it
    // is only meaningful while the Status tab is open).
    useEffect(() => {
        if (!project || !status?.initialized) return;
        if (activeTab === 'status' && status.current_task_id && status.current_run_id && status.current_phase) {
            tauriApi.vibehubValidatePhase(project.path).then(setPhaseValidation).catch(() => setPhaseValidation(null));
        }
    }, [activeTab, project?.path, status?.initialized, status?.current_task_id, status?.current_run_id, status?.current_phase]);

    useEffect(() => {
        if (activeTab !== 'preview') return;
        if (!previewCandidates.length) {
            setPreviewPath('');
            setPreviewFile(null);
            setPreviewError(null);
            return;
        }
        if (!previewPath || !previewCandidates.some((candidate) => candidate.path === previewPath)) {
            setPreviewPath(previewCandidates[0].path);
        }
    }, [activeTab, previewCandidateKey, previewPath]);

    useEffect(() => {
        if (activeTab !== 'preview' || !project || !previewPath) return;
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
    }, [activeTab, project?.path, previewPath]);

    const hasActiveContextTarget = Boolean(
        status?.initialized && status.current_task_id && status.current_run_id && status.current_phase
    );
    const actionDisabled = isLoading || !!runningAction || !project;
    const hasKnowledgeNote = knowledgeNote.trim().length > 0;
    const selectedCommand = adapterStatus?.commands.find((command) => command.name === selectedCommandName)
        || adapterStatus?.commands[0]
        || null;

    useEffect(() => {
        if (selectedCommand) {
            setSelectedCommandName(selectedCommand.name);
            setCommandOverrideBody(selectedCommand.body);
        }
    }, [selectedCommand?.name, selectedCommand?.body]);

    const toggleAgentTool = (tool: AgentTool, checked: boolean) => {
        setAgentTools((current) => {
            const next = checked ? [...current, tool] : current.filter((item) => item !== tool);
            return next.length ? Array.from(new Set(next)) : current;
        });
    };

    const saveCommandOverride = async () => {
        if (!project || !adapterStatus || !selectedCommand) return;
        setRunningAction('agent-sync');
        try {
            const overrides = Object.fromEntries(
                adapterStatus.commands.map((command) => [command.name, command.body])
            );
            overrides[selectedCommand.name] = commandOverrideBody;
            await tauriApi.vibehubUpdateAgentAdapterConfig(project.path, {
                enabled_tools: agentTools,
                command_overrides: overrides,
            });
            const result = await tauriApi.vibehubSyncAgentAdapters(project.path, agentTools);
            setActionState({ message: result.summary, error: result.conflict_files.length > 0 });
            await loadStatus(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setRunningAction(null);
        }
    };

    const runAction = async (action: CockpitAction) => {
        if (!project) return;
        setRunningAction(action);
        try {
            if (action === 'init') {
                const result = await tauriApi.vibehubInit(project.path, agentTools);
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
            } else if (action === 'start-task') {
                const result = await tauriApi.vibehubStartTask(project.path, undefined, taskMode);
                setActionState({
                    message: t('vibehub.messages.startedTask', {
                        task: result.task_id,
                        run: result.run_id,
                        phase: result.phase,
                    }),
                    error: false,
                });
            } else if (action === 'build-context') {
                if (!status?.current_task_id || !status.current_run_id || !status.current_phase) {
                    setActionState({
                        message: t('vibehub.messages.noContextTarget'),
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
                    message: t('vibehub.messages.contextBuilt', {
                        path: result.pack_path,
                        included: result.included_count,
                        missing: result.missing_count,
                        excluded: result.excluded_count,
                    }),
                    error: result.missing_count > 0,
                });
            } else if (action === 'continue') {
                const result = await tauriApi.vibehubSyncWorkspace(project.path, i18n.language);
                setSyncReport(result);
                setActionState({
                    message: t('vibehub.messages.syncReportWritten', {
                        path: result.report_path || result.agent_view_sync_path,
                        questions: result.questions_for_user.length,
                    }),
                    error: result.status !== 'synced',
                });
            } else if (action === 'agent-sync') {
                const result = await tauriApi.vibehubSyncAgentAdapters(project.path, agentTools);
                const conflictSuffix = result.conflict_files.length
                    ? ` ${t('vibehub.messages.conflict')}: ${result.conflict_files.map((file) => `${file.path} (${file.reason})`).join('; ')}`
                    : '';
                setActionState({
                    message: `${result.summary}${conflictSuffix}`,
                    error: result.conflict_files.length > 0,
                });
            } else if (action === 'workspace-sync') {
                const result = await tauriApi.vibehubCheckWorkspaceDrift(project.path, i18n.language);
                setDriftReport(result);
                setActionState({
                    message: result.warnings.length
                        ? t('vibehub.messages.workspaceDrift', { warnings: result.warnings.join(' ') })
                        : t('vibehub.messages.workspaceClean'),
                    error: result.warnings.length > 0,
                });
            } else if (action === 'recover-drift') {
                const result = await tauriApi.vibehubSyncWorkspaceState(project.path, i18n.language);
                setDriftReport(result);
                setActionState({
                    message: result.recover_report_path
                        ? t('vibehub.messages.recoverWritten', { path: result.recover_report_path })
                        : t('vibehub.messages.recoverNotNeeded'),
                    error: result.warnings.length > 0,
                });
            } else if (action === 'review') {
                const result = await tauriApi.vibehubGenerateReviewEvidence(project.path);
                setActionState({
                    message: t('vibehub.messages.reviewWritten', {
                        path: result.review_path,
                        count: result.changed_files_count,
                    }),
                    error: false,
                });
            } else if (action === 'handoff') {
                const result = await tauriApi.vibehubBuildHandoff(project.path);
                const missingSuffix = result.missing_required_sections.length
                    ? ` ${t('vibehub.messages.missingSections', { sections: result.missing_required_sections.join(', ') })}`
                    : '';
                setActionState({
                    message: `${result.complete
                        ? t('vibehub.messages.handoffBuilt', { path: result.handoff_path })
                        : t('vibehub.messages.handoffIncomplete', { path: result.handoff_path })}${missingSuffix}`,
                    error: !result.complete,
                });
            } else if (action === 'validate-phase') {
                const result = await tauriApi.vibehubValidatePhase(project.path);
                setPhaseValidation(result);
                setActionState({
                    message: t('vibehub.messages.phaseValidated', {
                        phase: result.phase,
                        status: result.status,
                        missing: result.missing_outputs.length,
                    }),
                    error: result.missing_outputs.length > 0,
                });
            } else if (action === 'complete-phase') {
                const result = await tauriApi.vibehubCompletePhase(project.path);
                setPhaseValidation(result.validation);
                setActionState({
                    message: t('vibehub.messages.phaseCompleted', {
                        phase: result.current_phase,
                        status: result.current_status,
                    }),
                    error: result.current_status === 'needs_action' || result.current_status === 'failed',
                });
            } else if (action === 'advance-phase') {
                const result = await tauriApi.vibehubAdvancePhase(project.path);
                setPhaseValidation(result.validation);
                setActionState({
                    message: t('vibehub.messages.phaseAdvanced', {
                        from: result.previous_phase,
                        to: result.current_phase,
                        status: result.current_status,
                    }),
                    error: result.current_status === 'needs_action' || result.current_status === 'failed',
                });
            } else if (action === 'pause-phase') {
                const result = await tauriApi.vibehubPausePhase(project.path);
                setActionState({
                    message: t('vibehub.messages.phasePaused', {
                        phase: result.current_phase,
                        status: result.current_phase_status,
                    }),
                    error: result.current_phase_status !== 'blocked',
                });
            } else if (action === 'journal') {
                const result = await tauriApi.vibehubAppendJournalEntry(
                    project.path,
                    journalTitle,
                    journalBody
                );
                setJournalTitle('');
                setJournalBody('');
                setActionState({
                    message: t('vibehub.messages.journalAppended', { path: result.journal_path }),
                    error: false,
                });
            } else {
                const result = await tauriApi.vibehubAppendKnowledgeNote(project.path, knowledgeNote);
                setKnowledgeNote('');
                setActionState({
                    message: t('vibehub.messages.knowledgeAppended', { path: result.knowledge_path }),
                    error: false,
                });
            }
            await loadStatus(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setRunningAction(null);
        }
    };

    const saveLocale = async () => {
        if (!project || !status?.initialized) return;
        setRunningAction('workspace-sync');
        try {
            await tauriApi.vibehubSetProjectLocale(project.path, selectedLocale);
            setActionState({ message: t('vibehub.messages.localeSaved', { locale: selectedLocale }), error: false });
            await loadStatus(false);
        } catch (error) {
            setActionState({ message: String(error), error: true });
        } finally {
            setRunningAction(null);
        }
    };

    const taskLabel = status?.current_task_id
        ? `${status.current_task_id}${status.current_task_title ? ` - ${status.current_task_title}` : ''}`
        : t('vibehub.status.noCurrentTask');
    const unavailableReason = !status
        ? t('vibehub.unavailable.loading')
        : !status.initialized
            ? t('vibehub.unavailable.notInitialized')
            : !hasActiveContextTarget
                ? t('vibehub.unavailable.noActiveTask')
                : null;
    const recommendedAction = getRecommendedAction(status, hasActiveContextTarget, t);

    return (
        <div className="space-y-4">
            {showOverview && project && (
                <div className="grid gap-3 md:grid-cols-[1.2fr_0.8fr]">
                    <div className="rounded-md border bg-muted/20 p-4">
                        <div className="text-xs text-muted-foreground">{t('vibehub.overview.project')}</div>
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
                        <div className="text-xs text-muted-foreground">{t('vibehub.overview.flow')}</div>
                        <div className="mt-2 grid grid-cols-2 gap-2 text-xs">
                            {getModeFlow(status, taskMode).map((item) => (
                                <div
                                    key={item.stage}
                                    className={`rounded border px-2 py-1.5 font-medium ${flowStatusClass(item.status)}`}
                                >
                                    <div>{t(`vibehub.flow.${item.stage}`)}</div>
                                    <div className="mt-0.5 text-[10px] font-normal text-muted-foreground">
                                        {item.phase ? `${t(`vibehub.flowPhases.${item.phase}`)} · ${formatPhaseStatus(item.status)}` : t('vibehub.flow.skipped')}
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            )}

            <div className="flex items-center justify-between gap-3 border-b pb-3">
                <div className="min-w-0">
                    <div className="text-sm text-muted-foreground">{t('vibehub.status.currentTaskRun')}</div>
                    <div className="truncate font-medium">{isLoading ? t('common.loading') : taskLabel}</div>
                    {status?.current_run_id && (
                        <div className="mt-0.5 truncate text-xs text-muted-foreground">{t('vibehub.status.run')}: {status.current_run_id}</div>
                    )}
                </div>
                <Button variant="outline" size="sm" onClick={() => loadStatus()} disabled={isLoading || !project}>
                    <RefreshCw className={`mr-2 h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
                    {t('home.refresh')}
                </Button>
            </div>

            {status && (
                <>
                    <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
                        <Metric label={t('vibehub.status.mode')} value={status.current_mode || t('common.unknown')} />
                        <Metric label={t('vibehub.status.phase')} value={status.current_phase || t('common.none')} />
                        <Metric label={t('vibehub.status.phaseStatus')} value={status.phase_status || t('common.unknown')} />
                        <Metric label={t('vibehub.status.observability')} value={status.observability_level || 'best_effort'} />
                    </div>

                    <div className="grid gap-3 md:grid-cols-2">
                        <div className="space-y-1.5">
                            <Label htmlFor="vibehub-locale-select" className="text-xs">
                                {t('vibehub.status.locale')}
                            </Label>
                            <div className="flex gap-2">
                                <select
                                    id="vibehub-locale-select"
                                    value={selectedLocale}
                                    onChange={(event) => setSelectedLocale(event.target.value)}
                                    disabled={actionDisabled}
                                    className="h-9 flex-1 rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                                >
                                    {LOCALE_OPTIONS.map((locale) => (
                                        <option key={locale} value={locale}>
                                            {t(`vibehub.locales.${locale}`)}
                                        </option>
                                    ))}
                                </select>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={saveLocale}
                                    disabled={actionDisabled || !status?.initialized || selectedLocale === status?.locale}
                                >
                                    {t('common.save')}
                                </Button>
                            </div>
                        </div>
                    </div>

                    <div className="grid gap-3 md:grid-cols-4">
                        <StatusPanel
                            icon={<GitBranch className="h-4 w-4" />}
                            label={t('vibehub.status.gitDirty')}
                            value={gitStatusLabel(status, t)}
                            tone={status.git_dirty ? 'warn' : 'ok'}
                        />
                        <FileStatusPanel
                            icon={<FileText className="h-4 w-4" />}
                            label={t('vibehub.status.contextPack')}
                            fileStatus={status.context_pack_status}
                        />
                        <FileStatusPanel
                            icon={<FileText className="h-4 w-4" />}
                            label={t('vibehub.status.agentOutput')}
                            fileStatus={status.agent_output_status}
                        />
                        <FileStatusPanel
                            icon={<FileText className="h-4 w-4" />}
                            label={t('vibehub.status.handoff')}
                            fileStatus={status.handoff_status}
                        />
                    </div>

                    {!status.initialized && (
                        <Notice error message={t('vibehub.notices.notInitialized')} />
                    )}

                    {status.warnings.map((warning) => (
                        <Notice key={warning} error message={warning} />
                    ))}
                </>
            )}

            {/* P1 observable tabs */}
            {status?.initialized && (
                <div className="rounded-md border bg-card">
                    <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as CockpitTab)}>
                        <div className="border-b px-4 pt-3">
                            <TabsList className="h-auto flex-wrap justify-start">
                                <TabsTrigger value="status">{t('vibehub.tabs.status')}</TabsTrigger>
                                <TabsTrigger value="context">{t('vibehub.tabs.context')}</TabsTrigger>
                                <TabsTrigger value="evidence">{labelOrFallback(t, 'vibehub.tabs.evidence', 'Evidence')}</TabsTrigger>
                                <TabsTrigger value="preview">{labelOrFallback(t, 'vibehub.tabs.preview', 'Preview')}</TabsTrigger>
                                <TabsTrigger value="review">{t('vibehub.tabs.review')}</TabsTrigger>
                                <TabsTrigger value="handoff">{t('vibehub.tabs.handoff')}</TabsTrigger>
                                <TabsTrigger value="research">{t('vibehub.tabs.research')}</TabsTrigger>
                                <TabsTrigger value="diff">{t('vibehub.tabs.diff')}</TabsTrigger>
                            </TabsList>
                        </div>

                        <TabsContent value="status" className="space-y-4 px-4 pb-4">
                            <StatusTabContent
                                status={status}
                                phaseValidation={phaseValidation}
                                recommendedAction={recommendedAction}
                                unavailableReason={unavailableReason}
                                actionDisabled={actionDisabled}
                                hasActiveContextTarget={hasActiveContextTarget}
                                runAction={runAction}
                                t={t}
                            />
                        </TabsContent>

                        <TabsContent value="context" className="space-y-4 px-4 pb-4">
                            <ContextTabContent contextView={contextView} t={t} />
                        </TabsContent>

                        <TabsContent value="evidence" className="space-y-4 px-4 pb-4">
                            <EvidenceTabContent
                                status={status}
                                contextView={contextView}
                                reviewView={reviewView}
                                handoffView={handoffView}
                                researchStatus={researchStatus}
                                diffView={diffView}
                                t={t}
                            />
                        </TabsContent>

                        <TabsContent value="preview" className="space-y-4 px-4 pb-4">
                            <PreviewTabContent
                                candidates={previewCandidates}
                                selectedPath={previewPath}
                                onSelectedPathChange={setPreviewPath}
                                file={previewFile}
                                loading={isPreviewLoading}
                                error={previewError}
                                t={t}
                            />
                        </TabsContent>

                        <TabsContent value="review" className="space-y-4 px-4 pb-4">
                            <ReviewTabContent reviewView={reviewView} t={t} />
                        </TabsContent>

                        <TabsContent value="handoff" className="space-y-4 px-4 pb-4">
                            <HandoffTabContent handoffView={handoffView} t={t} />
                        </TabsContent>

                        <TabsContent value="research" className="space-y-4 px-4 pb-4">
                            <ResearchTabContent researchStatus={researchStatus} t={t} />
                        </TabsContent>

                        <TabsContent value="diff" className="space-y-4 px-4 pb-4">
                            <DiffTabContent diffView={diffView} t={t} />
                        </TabsContent>
                    </Tabs>
                </div>
            )}

            <Notice
                error={false}
                message={t('vibehub.notices.observabilityLimited')}
            />

            <div className="rounded-md border bg-muted/10 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                        <div className="text-sm font-medium">{t('vibehub.next.title')}</div>
                        <div className="mt-1 text-sm text-muted-foreground">
                            {recommendedAction?.description || t('vibehub.next.loading')}
                        </div>
                    </div>
                    {recommendedAction && (
                        <Button
                            onClick={() => runAction(recommendedAction.action)}
                            disabled={actionDisabled || recommendedAction.disabled}
                        >
                            {recommendedAction.action === 'init' ? <Wrench className="mr-2 h-4 w-4" /> : <Play className="mr-2 h-4 w-4" />}
                            {recommendedAction.title}
                        </Button>
                    )}
                </div>
                {unavailableReason && (
                    <div className="mt-2 text-xs text-muted-foreground">{unavailableReason}</div>
                )}
            </div>

            <details
                className="rounded-md border bg-muted/10 p-4"
                open={showAdvancedDiagnostics}
                onToggle={(event) => setShowAdvancedDiagnostics(event.currentTarget.open)}
            >
                <summary className="cursor-pointer text-sm font-medium">
                    {t('vibehub.advanced.title')}
                    <span className="ml-2 text-xs font-normal text-muted-foreground">
                        {t('vibehub.advanced.subtitle')}
                    </span>
                </summary>

                <div className="mt-4 space-y-5">
                    <div className="space-y-3">
                        <div className="flex flex-wrap items-start justify-between gap-3">
                            <div>
                                <div className="text-sm font-medium">{t('vibehub.ai.title')}</div>
                                <div className="text-xs text-muted-foreground">
                                    {t('vibehub.ai.subtitle')}
                                </div>
                            </div>
                            {adapterStatus && (
                                <Badge variant="outline">{t('vibehub.ai.commandCount', { count: adapterStatus.commands.length })}</Badge>
                            )}
                        </div>

                        <div className="grid gap-2 md:grid-cols-3">
                            {AGENT_TOOL_OPTIONS.map((tool) => (
                                <label key={tool.id} className="flex items-start gap-2 rounded-md border bg-background p-3 text-sm">
                                    <Checkbox
                                        checked={agentTools.includes(tool.id)}
                                        onCheckedChange={(checked) => toggleAgentTool(tool.id, checked === true)}
                                        disabled={actionDisabled}
                                    />
                                    <span>
                                        <span className="block font-medium">{tool.label}</span>
                                        <span className="block text-xs text-muted-foreground">{t(`vibehub.ai.tools.${tool.id}`)}</span>
                                    </span>
                                </label>
                            ))}
                        </div>

                        {adapterStatus && (
                            <>
                        <div className="grid gap-2 md:grid-cols-2">
                            {adapterStatus.files.slice(0, 8).map((file) => (
                                <div key={`${file.tool}:${file.path}`} className="rounded-md border bg-background p-2 text-xs">
                                    <div className="flex items-center justify-between gap-2">
                                        <span className="truncate font-medium">{file.path}</span>
                                        <Badge variant={file.status === 'in_sync' ? 'secondary' : file.status === 'missing' ? 'outline' : 'destructive'}>
                                            {file.status}
                                        </Badge>
                                    </div>
                                    <div className="mt-1 text-muted-foreground">{file.description}</div>
                                </div>
                            ))}
                        </div>

                        {selectedCommand && (
                            <div className="grid gap-3 md:grid-cols-[0.7fr_1.3fr]">
                                <div className="space-y-1.5">
                                    <Label htmlFor="vibehub-command-select">{t('vibehub.ai.command')}</Label>
                                    <select
                                        id="vibehub-command-select"
                                        value={selectedCommand.name}
                                        onChange={(event) => setSelectedCommandName(event.target.value)}
                                        disabled={actionDisabled}
                                        className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm"
                                    >
                                        {adapterStatus.commands.map((command) => (
                                            <option key={command.name} value={command.name}>
                                                {command.name}
                                            </option>
                                        ))}
                                    </select>
                                    <div className="rounded-md border bg-background p-2 text-xs text-muted-foreground">
                                        <div>{selectedCommand.description_zh}</div>
                                        <div>{selectedCommand.description_en}</div>
                                    </div>
                                </div>
                                <div className="space-y-1.5">
                                    <Label htmlFor="vibehub-command-body">{t('vibehub.ai.commandBodyOverride')}</Label>
                                    <textarea
                                        id="vibehub-command-body"
                                        value={commandOverrideBody}
                                        onChange={(event) => setCommandOverrideBody(event.target.value)}
                                        disabled={actionDisabled}
                                        className="min-h-40 w-full rounded-md border border-input bg-background px-3 py-2 font-mono text-xs shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                                    />
                                </div>
                            </div>
                        )}

                        <div className="flex flex-wrap gap-2">
                            <Button
                                variant="outline"
                                onClick={() => runAction('agent-sync')}
                                disabled={actionDisabled || !status?.initialized}
                            >
                                <Bot className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.updateAiInstructions')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={saveCommandOverride}
                                disabled={actionDisabled || !status?.initialized || !selectedCommand}
                            >
                                <FileText className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.saveCommandOverride')}
                            </Button>
                        </div>
                            </>
                        )}
                    </div>

                    <div className="space-y-2 border-t pt-4">
                        {!status?.initialized && (
                            <div className="rounded-md border bg-background p-3 text-xs text-muted-foreground">
                                {t('vibehub.notices.initializeCreates')}
                            </div>
                        )}
                        <div className="flex flex-wrap gap-2">
                            <div className="flex min-w-48 flex-col gap-1">
                                <Label htmlFor="vibehub-start-mode" className="text-xs">
                                    {t('vibehub.status.mode')}
                                </Label>
                                <select
                                    id="vibehub-start-mode"
                                    value={taskMode}
                                    onChange={(event) => setTaskMode(event.target.value as (typeof DRIVE_MODE_OPTIONS)[number])}
                                    disabled={actionDisabled || !status?.initialized}
                                    className="h-9 rounded-md border border-input bg-background px-3 text-sm shadow-sm"
                                >
                                    {DRIVE_MODE_OPTIONS.map((mode) => (
                                        <option key={mode} value={mode}>
                                            {t(`vibehub.modes.${mode}`)}
                                        </option>
                                    ))}
                                </select>
                            </div>
                            <Button onClick={() => runAction('init')} disabled={actionDisabled || !!status?.initialized}>
                                <Wrench className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.initialize')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('start-task')}
                                disabled={actionDisabled || !status?.initialized}
                            >
                                <Play className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.startTask')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('build-context')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <PackagePlus className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.buildContext')}
                            </Button>
                            <Button onClick={() => runAction('continue')} disabled={actionDisabled || !hasActiveContextTarget}>
                                <Play className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.continue')}
                            </Button>
                            <Button variant="outline" onClick={() => runAction('workspace-sync')} disabled={actionDisabled || !status?.initialized}>
                                <RefreshCw className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.syncWorkspace')}
                            </Button>
                            <Button variant="outline" onClick={() => runAction('recover-drift')} disabled={actionDisabled || !status?.initialized}>
                                <AlertCircle className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.recoverDrift')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('review')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <SearchCheck className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.reviewEvidence')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('handoff')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <FileText className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.buildHandoff')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('validate-phase')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <SearchCheck className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.validatePhase')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('complete-phase')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <FileText className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.completePhase')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('advance-phase')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <Play className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.advancePhase')}
                            </Button>
                            <Button
                                variant="outline"
                                onClick={() => runAction('pause-phase')}
                                disabled={actionDisabled || !hasActiveContextTarget}
                            >
                                <Pause className="mr-2 h-4 w-4" />
                                {t('vibehub.actions.pausePhase')}
                            </Button>
                        </div>
                    </div>
                </div>
            </details>

            {driftReport && (
                <div className="rounded-md border p-3 text-sm">
                    <div className="font-medium">{t('vibehub.drift.title')}</div>
                    <div className="mt-1 text-xs text-muted-foreground">
                        {t('vibehub.drift.dirty')}: {driftReport.dirty ? t('common.yes') : t('common.no')} | {t('vibehub.drift.headChanged')}: {driftReport.head_changed ? t('common.yes') : t('common.no')} | {t('vibehub.drift.contextStale')}: {driftReport.context_stale ? t('common.yes') : t('common.no')}
                    </div>
                    {driftReport.recommended_actions.length > 0 && (
                        <div className="mt-2 space-y-1 text-xs">
                            {driftReport.recommended_actions.map((action) => (
                                <div key={action}>- {action}</div>
                            ))}
                        </div>
                    )}
                </div>
            )}

            {syncReport && (
                <div className="rounded-md border p-3 text-sm">
                    <div className="font-medium">{t('vibehub.sync.title')}</div>
                    <div className="mt-1 text-xs text-muted-foreground">
                        {t('vibehub.sync.report')}: {syncReport.report_path || syncReport.agent_view_sync_path}
                    </div>
                    {syncReport.questions_for_user.length > 0 && (
                        <div className="mt-2 space-y-1 text-xs">
                            <div className="font-medium text-muted-foreground">{t('vibehub.sync.questions')}</div>
                            {syncReport.questions_for_user.map((question) => (
                                <div key={question}>- {question}</div>
                            ))}
                        </div>
                    )}
                    {syncReport.recommended_actions.length > 0 && (
                        <div className="mt-2 space-y-1 text-xs">
                            <div className="font-medium text-muted-foreground">{t('vibehub.sync.actions')}</div>
                            {syncReport.recommended_actions.map((action) => (
                                <div key={action}>- {action}</div>
                            ))}
                        </div>
                    )}
                </div>
            )}

            {actionState && <Notice error={actionState.error} message={actionState.message} />}

            <div className="space-y-3 border-t pt-4">
                <div className="grid gap-3 md:grid-cols-[0.7fr_1.3fr]">
                    <div className="space-y-1.5">
                        <Label htmlFor="vibehub-journal-title">{t('vibehub.journal.title')}</Label>
                        <Input
                            id="vibehub-journal-title"
                            value={journalTitle}
                            onChange={(event) => setJournalTitle(event.target.value)}
                            placeholder={t('vibehub.journal.titlePlaceholder')}
                            disabled={actionDisabled || !status?.initialized}
                        />
                    </div>
                    <div className="space-y-1.5">
                        <Label htmlFor="vibehub-journal-body">{t('vibehub.journal.body')}</Label>
                        <textarea
                            id="vibehub-journal-body"
                            value={journalBody}
                            onChange={(event) => setJournalBody(event.target.value)}
                            placeholder={t('vibehub.journal.bodyPlaceholder')}
                            disabled={actionDisabled || !status?.initialized}
                            className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                        />
                    </div>
                </div>
                <div className="flex flex-wrap gap-2">
                    <Button
                        variant="outline"
                        onClick={() => runAction('journal')}
                        disabled={actionDisabled || !status?.initialized}
                    >
                        <FileText className="mr-2 h-4 w-4" />
                        {t('vibehub.actions.addJournalNote')}
                    </Button>
                </div>
            </div>

            <div className="space-y-3 border-t pt-4">
                <div className="space-y-1.5">
                    <Label htmlFor="vibehub-knowledge-note">{t('vibehub.knowledge.note')}</Label>
                    <textarea
                        id="vibehub-knowledge-note"
                        value={knowledgeNote}
                        onChange={(event) => setKnowledgeNote(event.target.value)}
                        placeholder={t('vibehub.knowledge.placeholder')}
                        disabled={actionDisabled || !status?.initialized}
                        className="min-h-20 w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                    />
                </div>
                <div className="flex flex-wrap gap-2">
                    <Button
                        variant="outline"
                        onClick={() => runAction('knowledge')}
                        disabled={actionDisabled || !status?.initialized || !hasKnowledgeNote}
                    >
                        <Lightbulb className="mr-2 h-4 w-4" />
                        {t('vibehub.actions.addKnowledgeNote')}
                    </Button>
                </div>
            </div>
        </div>
    );
}

// Tab content components.

function StatusTabContent({
    status,
    phaseValidation,
    recommendedAction,
    unavailableReason,
    actionDisabled,
    hasActiveContextTarget,
    runAction,
    t,
}: {
    status: VibehubCockpitStatus | null;
    phaseValidation: PhaseValidationResult | null;
    recommendedAction: RecommendedAction | null;
    unavailableReason: string | null;
    actionDisabled: boolean;
    hasActiveContextTarget: boolean;
    runAction: (action: CockpitAction) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (!status) return null;

    return (
        <div className="space-y-3 pt-1">
            <div className="text-sm font-medium">{t('vibehub.tabs.statusDetail')}</div>

            <div className="grid grid-cols-2 gap-2 text-xs">
                <ViewField label={t('vibehub.status.mode')} value={status.current_mode || t('common.none')} />
                <ViewField label={t('vibehub.status.phase')} value={status.current_phase || t('common.none')} />
                <ViewField label={t('vibehub.status.phaseStatus')} value={status.phase_status || t('common.unknown')} />
                <ViewField label={t('vibehub.status.observability')} value={status.observability_level || 'best_effort'} />
                <ViewField label={t('vibehub.drift.dirty')} value={status.git_dirty ? t('common.yes') : t('common.no')} />
                <ViewField label={t('vibehub.tabs.contextPackStatus')} value={status.context_pack_status.status} />
                <ViewField label={t('vibehub.tabs.agentOutputStatus')} value={status.agent_output_status.status} />
                <ViewField label={t('vibehub.tabs.handoffStatus')} value={status.handoff_status.status} />
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
                            <ViewField label={t('vibehub.phase.validationStatus')} value={phaseValidation.status} />
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
                <div className="mt-3 flex flex-wrap gap-2">
                    <Button size="sm" variant="outline" onClick={() => runAction('validate-phase')} disabled={actionDisabled || !hasActiveContextTarget}>
                        <SearchCheck className="mr-1 h-3 w-3" />
                        {t('vibehub.actions.validatePhase')}
                    </Button>
                    <Button size="sm" variant="outline" onClick={() => runAction('complete-phase')} disabled={actionDisabled || !hasActiveContextTarget}>
                        <FileText className="mr-1 h-3 w-3" />
                        {t('vibehub.actions.completePhase')}
                    </Button>
                    <Button size="sm" onClick={() => runAction('advance-phase')} disabled={actionDisabled || !hasActiveContextTarget}>
                        <Play className="mr-1 h-3 w-3" />
                        {t('vibehub.actions.advancePhase')}
                    </Button>
                    <Button size="sm" variant="outline" onClick={() => runAction('pause-phase')} disabled={actionDisabled || !hasActiveContextTarget}>
                        <Pause className="mr-1 h-3 w-3" />
                        {t('vibehub.actions.pausePhase')}
                    </Button>
                </div>
            </div>

            {recommendedAction && (
                <div className="rounded-md border bg-muted/10 p-3">
                    <div className="text-xs font-medium">{t('vibehub.next.title')}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{recommendedAction.description}</div>
                    <Button
                        size="sm"
                        className="mt-2"
                        onClick={() => runAction(recommendedAction.action)}
                        disabled={actionDisabled || recommendedAction.disabled}
                    >
                        <Play className="mr-1 h-3 w-3" />
                        {recommendedAction.title}
                    </Button>
                </div>
            )}

            {unavailableReason && (
                <div className="text-xs text-muted-foreground">{unavailableReason}</div>
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
            title: 'Task/run state',
            value: status.current_task_id && status.current_run_id
                ? `${status.current_task_id} / ${status.current_run_id}`
                : t('vibehub.status.noCurrentTask'),
        },
        {
            grade: 'hard_observed',
            icon: <GitBranch className="h-4 w-4" />,
            title: 'Workspace diff',
            value: diffView ? `${diffView.changed_files_count} changed file(s)` : t('common.unknown'),
        },
        {
            grade: 'hard_observed',
            icon: <FileText className="h-4 w-4" />,
            title: 'Context pack',
            value: contextView?.pack_exists ? `${contextView.included_count} included / ${contextView.missing_count} missing` : status.context_pack_status.status,
        },
        {
            grade: 'hard_observed',
            icon: <FileText className="h-4 w-4" />,
            title: 'Review evidence',
            value: reviewView?.review_exists ? `${reviewView.changed_files_count} changed file(s) captured` : 'not generated',
        },
        {
            grade: 'agent_reported',
            icon: <Bot className="h-4 w-4" />,
            title: 'Agent output',
            value: status.agent_output_status.exists ? status.agent_output_status.path || status.agent_output_status.status : status.agent_output_status.status,
        },
        {
            grade: 'agent_reported',
            icon: <FileText className="h-4 w-4" />,
            title: 'Handoff',
            value: handoffView?.complete ? 'complete' : `${handoffView?.missing_sections.length || 0} missing section(s)`,
        },
        {
            grade: 'inferred',
            icon: <Lightbulb className="h-4 w-4" />,
            title: 'Research gate',
            value: researchStatus?.required ? researchStatus.status : 'not required',
        },
        {
            grade: 'inferred',
            icon: <AlertCircle className="h-4 w-4" />,
            title: 'Warnings',
            value: status.warnings.length ? `${status.warnings.length} warning(s)` : 'none',
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
                {labelOrFallback(t, 'vibehub.tabs.evidenceDataSource', 'Source')}: status, context, review, handoff, research, and diff overview data
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
                            {candidate.label} - {candidate.path}
                        </option>
                    ))}
                </select>
                <Badge variant={file?.exists ? 'default' : 'secondary'}>{file?.exists ? t('common.yes') : t('common.no')}</Badge>
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
                <ViewField label={t('vibehub.tabs.researchStatus')} value={researchStatus.status} />
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

function formatPhaseStatus(status: string) {
    return status
        .split('_')
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(' ');
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
    addPreviewCandidate(candidates, '.vibehub/sync.md', 'Sync report', true);
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

function getRecommendedAction(
    status: VibehubCockpitStatus | null,
    hasActiveContextTarget: boolean,
    t: (key: string, options?: Record<string, unknown>) => string
): RecommendedAction | null {
    if (!status) return null;
    if (!status.initialized) {
        return {
            action: 'init',
            title: t('vibehub.actions.initialize'),
            description: t('vibehub.next.initialize'),
        };
    }
    if (!hasActiveContextTarget) {
        return {
            action: 'start-task',
            title: t('vibehub.actions.startTask'),
            description: t('vibehub.next.startTask'),
        };
    }
    if (status.git_dirty) {
        return {
            action: 'workspace-sync',
            title: t('vibehub.actions.syncWorkspace'),
            description: t('vibehub.next.syncWorkspace', {
                count: status.git_changed_files_count || 0,
            }),
        };
    }
    if (!status.context_pack_status.exists || status.context_pack_status.stale === true) {
        return {
            action: 'build-context',
            title: t('vibehub.actions.buildContext'),
            description: t('vibehub.next.buildContext'),
        };
    }
    if (!status.handoff_status.exists || status.handoff_status.status !== 'available') {
        return {
            action: 'handoff',
            title: t('vibehub.actions.buildHandoff'),
            description: t('vibehub.next.handoff'),
        };
    }
    return {
        action: 'continue',
        title: t('vibehub.actions.continue'),
        description: t('vibehub.next.continue'),
    };
}

function gitStatusLabel(status: VibehubCockpitStatus, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!status.git_available) return t('vibehub.status.gitUnavailable');
    if (status.git_dirty == null) return t('common.unknown');
    return status.git_dirty
        ? t('vibehub.status.dirtyCount', { count: status.git_changed_files_count || 0 })
        : t('vibehub.status.clean');
}
