import { useEffect, useState, type ReactNode } from 'react';
import { AlertCircle, Bot, FileText, GitBranch, Lightbulb, PackagePlus, Play, RefreshCw, SearchCheck, Wrench } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { AgentAdapterStatus, AgentTool, Project, VibehubCockpitStatus, VibehubFileStatus, WorkspaceDriftReport } from '@/types';
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

type CockpitAction = 'init' | 'start-task' | 'build-context' | 'continue' | 'agent-sync' | 'workspace-sync' | 'recover-drift' | 'review' | 'handoff' | 'journal' | 'knowledge';

const AGENT_TOOL_OPTIONS: Array<{ id: AgentTool; label: string; description: string }> = [
    { id: 'codex', label: 'Codex', description: 'AGENTS.md + .agents/skills repo skills' },
    { id: 'claude_code', label: 'Claude Code', description: 'CLAUDE.md + .claude commands + constraints' },
    { id: 'opencode', label: 'OpenCode', description: 'AGENTS.md + .opencode commands + constraints' },
];

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
    const { t } = useTranslation();
    const [status, setStatus] = useState<VibehubCockpitStatus | null>(null);
    const [adapterStatus, setAdapterStatus] = useState<AgentAdapterStatus | null>(null);
    const [driftReport, setDriftReport] = useState<WorkspaceDriftReport | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [actionState, setActionState] = useState<ActionState | null>(null);
    const [runningAction, setRunningAction] = useState<CockpitAction | null>(null);
    const [agentTools, setAgentTools] = useState<AgentTool[]>(['codex', 'claude_code', 'opencode']);
    const [selectedCommandName, setSelectedCommandName] = useState('vibehub-sync');
    const [commandOverrideBody, setCommandOverrideBody] = useState('');
    const [journalTitle, setJournalTitle] = useState('');
    const [journalBody, setJournalBody] = useState('');
    const [knowledgeNote, setKnowledgeNote] = useState('');

    const loadStatus = async (clearActionState = true) => {
        if (!project) return;
        setIsLoading(true);
        try {
            const nextStatus = await tauriApi.vibehubReadCockpitStatus(project.path);
            setStatus(nextStatus);
            if (nextStatus.initialized) {
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
            setActionState(null);
            setRunningAction(null);
        }
    }, [enabled, project?.path]);

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
                const result = await tauriApi.vibehubStartTask(project.path);
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
                const result = await tauriApi.vibehubGenerateAgentView(project.path);
                setActionState({ message: t('vibehub.messages.continueUpdated', { path: result.current_path }), error: false });
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
                const result = await tauriApi.vibehubCheckWorkspaceDrift(project.path);
                setDriftReport(result);
                setActionState({
                    message: result.warnings.length
                        ? t('vibehub.messages.workspaceDrift', { warnings: result.warnings.join(' ') })
                        : t('vibehub.messages.workspaceClean'),
                    error: result.warnings.length > 0,
                });
            } else if (action === 'recover-drift') {
                const result = await tauriApi.vibehubSyncWorkspaceState(project.path);
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
                            {['plan', 'context', 'run', 'review', 'handoff'].map((item) => (
                                <div key={item} className="rounded border bg-background px-2 py-1.5 font-medium">
                                    {t(`vibehub.flow.${item}`)}
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

                    <div className="grid gap-3 md:grid-cols-3">
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
                            label={t('vibehub.status.handoff')}
                            fileStatus={status.handoff_status}
                        />
                    </div>

                    <StatusPanel
                        icon={<SearchCheck className="h-4 w-4" />}
                        label={t('vibehub.status.reviewEvidence')}
                        value={hasActiveContextTarget ? t('vibehub.status.reviewAvailable') : t('vibehub.status.needsActiveTask')}
                        tone={hasActiveContextTarget ? 'neutral' : 'warn'}
                    />

                    <Notice
                        error={false}
                        message={t('vibehub.notices.observabilityLimited')}
                    />

                    {!status.initialized && (
                        <Notice error message={t('vibehub.notices.notInitialized')} />
                    )}

                    {status.warnings.map((warning) => (
                        <Notice key={warning} error message={warning} />
                    ))}
                </>
            )}

            <div className="space-y-3 rounded-md border bg-muted/10 p-4">
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

            {actionState && <Notice error={actionState.error} message={actionState.message} />}

            <div className="space-y-2 border-t pt-4">
                {!status?.initialized && (
                    <div className="rounded-md border bg-muted/10 p-3 text-xs text-muted-foreground">
                        {t('vibehub.notices.initializeCreates')}
                    </div>
                )}
                <div className="flex flex-wrap gap-2">
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
                </div>
                {unavailableReason && (
                    <div className="text-xs text-muted-foreground">{unavailableReason}</div>
                )}
            </div>

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

function gitStatusLabel(status: VibehubCockpitStatus, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!status.git_available) return t('vibehub.status.gitUnavailable');
    if (status.git_dirty == null) return t('common.unknown');
    return status.git_dirty
        ? t('vibehub.status.dirtyCount', { count: status.git_changed_files_count || 0 })
        : t('vibehub.status.clean');
}
