import {
    Activity,
    AlertCircle,
    Archive,
    Bot,
    Boxes,
    CheckCircle2,
    ChevronRight,
    Circle,
    ClipboardList,
    ExternalLink,
    GitBranch,
    Layers3,
    RefreshCw,
    Settings,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    ArchiveCardButton,
    labelOrFallback,
    getRecentActivitySummary,
    getRecentCommitSummary,
    getProjectTaskCards,
    getModeFlow,
    getTaskProcessSteps,
    getTaskShortLabel,
    flattenStructureTree,
    FocusTarget,
    DashboardDetail,
    DetailOpenTarget,
    RecommendedReadOnlyAction,
} from './VibehubCockpitDialog';
import {
    PhaseValidationResult,
    Project,
    VibehubArchiveViewData,
    VibehubCockpitStatus,
    VibehubDiffViewData,
    VibehubEventTimelineItem,
    VibehubGitBranchesView,
    VibehubProjectDigest,
    VibehubProjectStructureViewData,
    VibehubPromptTemplateId,
    VibehubPromptTemplateOption,
} from '@/types';

export interface ProjectDetailBoardProps {
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
    onOpenDetail: (detail: DashboardDetail, target?: DetailOpenTarget) => void;
    onOpenPhase: (phase: string | null, target?: DetailOpenTarget) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}

export function ProjectDetailBoard({
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
}: ProjectDetailBoardProps) {
    const tasks = getProjectTaskCards(status);
    const archiveCards = archiveView?.cards || [];
    const commit = getRecentCommitSummary(gitBranches, project, t);
    const activity = getRecentActivitySummary(status, phaseValidation, eventTimeline, t);
    const changedFiles = diffView?.changed_files || [];
    const structureFiles = flattenStructureTree(projectStructure?.tree || []).slice(0, 8);
    const structureNodes = (projectStructure?.graph_nodes || [])
        .filter((node) => node.kind !== 'root')
        .slice(0, 8);
    const nextAction = pickPrimaryAction(readOnlyActions, promptTemplates);

    return (
        <section className="bg-background pb-10 text-foreground">
            <div className="space-y-8">
                <header className="space-y-5 border-b border-border/60 pb-6">
                    <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
                        <div className="min-w-0 flex-1">
                            <div className="mb-2 flex items-center gap-2 text-xs font-medium uppercase text-muted-foreground">
                                <Layers3 className="h-3.5 w-3.5" />
                                {labelOrFallback(t, 'vibehub.projectMap.title', 'Project')}
                            </div>
                            <h2 className="truncate text-3xl font-semibold tracking-tight">
                                {project?.name || t('common.unknown')}
                            </h2>
                            <div className="mt-1 truncate font-mono text-xs text-muted-foreground">
                                {project?.path || status.project_root}
                            </div>
                            <p className="mt-4 max-w-3xl text-sm leading-6 text-muted-foreground">
                                {projectDigest?.summary_line || project?.description || t('vibehub.dashboard.noSummary')}
                            </p>
                        </div>

                        <div className="flex flex-wrap items-center gap-2">
                            <Button variant="ghost" size="sm" onClick={onRefresh} disabled={isLoading}>
                                <RefreshCw className={`mr-2 h-3.5 w-3.5 ${isLoading ? 'animate-spin' : ''}`} />
                                {t('home.refresh')}
                            </Button>
                            <Button variant="ghost" size="sm" onClick={() => onOpenDetail('settings')}>
                                <Settings className="mr-2 h-3.5 w-3.5" />
                                {t('common.settings')}
                            </Button>
                            <Button size="sm" onClick={() => onOpenPrompt(nextAction?.promptTemplateId || 'new-task')}>
                                <Bot className="mr-2 h-3.5 w-3.5" />
                                {nextAction?.title || t('vibehub.promptGenerator.title')}
                            </Button>
                        </div>
                    </div>

                    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]">
                        <SummaryLink
                            icon={<GitBranch className="h-4 w-4" />}
                            label={labelOrFallback(t, 'vibehub.projectMap.recentCommit', 'Recent commit')}
                            title={commit.title}
                            detail={commit.detail}
                            onClick={() => onOpenDetail('git')}
                        />
                        <SummaryLink
                            icon={<Activity className="h-4 w-4" />}
                            label={labelOrFallback(t, 'vibehub.projectMap.recentActivity', 'Recent activity')}
                            title={activity.title}
                            detail={activity.detail}
                            onClick={() => onOpenDetail('activity')}
                        />
                        <div className="grid grid-cols-3 gap-2 md:col-span-2 xl:col-span-1">
                            <MetricCell label={labelOrFallback(t, 'vibehub.projectMap.activeTasks', 'Active tasks')} value={String(tasks.length)} />
                            <MetricCell label={t('vibehub.tabs.changedFilesCount')} value={String(diffView?.changed_files_count || 0)} />
                            <MetricCell label={t('vibehub.tabs.warnings')} value={String(status.warnings.length + (archiveView?.warnings.length || 0))} tone={status.warnings.length ? 'warn' : 'normal'} />
                        </div>
                    </div>
                </header>

                <div className="grid gap-8 xl:grid-cols-[minmax(0,1fr)_22rem]">
                    <main className="min-w-0 space-y-8">
                        <section className="space-y-4">
                            <SectionHeader
                                icon={<ClipboardList className="h-4 w-4" />}
                                title={labelOrFallback(t, 'vibehub.projectMap.activeTasks', 'Active tasks')}
                                detail={labelOrFallback(t, 'vibehub.projectMap.activeTasksHint', 'Current work units and their phase/capability transfer line.')}
                                count={tasks.length}
                            />

                            {phaseValidation?.missing_outputs.length ? (
                                <button
                                    type="button"
                                    onClick={() => onOpenPhase(status.current_phase || null)}
                                    className="flex w-full items-center gap-3 border border-destructive/30 bg-destructive/5 px-3 py-2 text-left text-sm text-destructive transition hover:bg-destructive/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                >
                                    <AlertCircle className="h-4 w-4 shrink-0" />
                                    <span className="min-w-0 flex-1">
                                        {labelOrFallback(t, 'vibehub.kanban.schemaIssue', 'Schema issue')}
                                    </span>
                                    <Badge variant="destructive">{phaseValidation.missing_outputs.length}</Badge>
                                </button>
                            ) : null}

                            {tasks.length === 0 ? (
                                <div className="border border-dashed border-border px-6 py-12 text-center">
                                    <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center border border-border text-muted-foreground">
                                        <ClipboardList className="h-4 w-4" />
                                    </div>
                                    <div className="text-sm font-medium">{labelOrFallback(t, 'vibehub.projectMap.emptyTasks', 'No active tasks')}</div>
                                    <div className="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">
                                        {labelOrFallback(t, 'vibehub.projectMap.emptyTasksHint', 'Ask an agent to run vibehub-start so VibeHub can create one or more tasks from the current request.')}
                                    </div>
                                    <Button className="mt-4" size="sm" onClick={() => onOpenPrompt('new-task')}>
                                        <Bot className="mr-2 h-4 w-4" />
                                        {t('vibehub.promptGenerator.title')}
                                    </Button>
                                </div>
                            ) : (
                                <div className="divide-y divide-border/70 border-y border-border/70">
                                    {tasks.map((task) => (
                                        <ProjectTaskRow
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
                        </section>

                        <section className="grid gap-8 lg:grid-cols-[minmax(0,1.1fr)_minmax(20rem,0.9fr)]">
                            <StructurePreview
                                projectStructure={projectStructure}
                                structureNodes={structureNodes}
                                structureFiles={structureFiles}
                                fallbackFiles={changedFiles.slice(0, 6)}
                                onOpen={() => onOpenDetail('structure')}
                                t={t}
                            />
                            <ArchivePreview
                                archiveCards={archiveCards}
                                warnings={archiveView?.warnings || []}
                                onOpen={() => onOpenDetail('archive')}
                                t={t}
                            />
                        </section>
                    </main>

                    <aside className="space-y-6 xl:sticky xl:top-4 xl:self-start">
                        <StatusRail
                            status={status}
                            phaseValidation={phaseValidation}
                            readOnlyActions={readOnlyActions}
                            promptTemplates={promptTemplates}
                            onOpenPrompt={onOpenPrompt}
                            onOpenDetail={onOpenDetail}
                            onOpenPhase={onOpenPhase}
                            t={t}
                        />
                    </aside>
                </div>
            </div>
        </section>
    );
}

function SummaryLink({
    icon,
    label,
    title,
    detail,
    onClick,
}: {
    icon: React.ReactNode;
    label: string;
    title: string;
    detail: string;
    onClick: () => void;
}) {
    return (
        <button
            type="button"
            onClick={onClick}
            className="group flex min-w-0 items-start gap-3 border border-border/70 px-3 py-3 text-left transition hover:border-foreground/30 hover:bg-muted/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
            <span className="mt-0.5 text-muted-foreground">{icon}</span>
            <span className="min-w-0 flex-1">
                <span className="block text-[11px] font-medium uppercase text-muted-foreground">{label}</span>
                <span className="mt-1 block truncate text-sm font-medium">{title}</span>
                <span className="mt-0.5 block truncate text-xs text-muted-foreground">{detail}</span>
            </span>
            <ExternalLink className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground opacity-0 transition group-hover:opacity-100" />
        </button>
    );
}

function MetricCell({ label, value, tone = 'normal' }: { label: string; value: string; tone?: 'normal' | 'warn' }) {
    return (
        <div className="border border-border/70 px-3 py-2">
            <div className="truncate text-[10px] font-medium uppercase text-muted-foreground">{label}</div>
            <div className={`mt-1 text-lg font-semibold ${tone === 'warn' ? 'text-amber-600 dark:text-amber-400' : ''}`}>{value}</div>
        </div>
    );
}

function SectionHeader({
    icon,
    title,
    detail,
    count,
}: {
    icon: React.ReactNode;
    title: string;
    detail?: string;
    count?: number;
}) {
    return (
        <div className="flex flex-wrap items-end justify-between gap-3">
            <div>
                <div className="flex items-center gap-2 text-sm font-semibold">
                    {icon}
                    {title}
                    {typeof count === 'number' && <Badge variant="secondary">{count}</Badge>}
                </div>
                {detail && <div className="mt-1 text-xs text-muted-foreground">{detail}</div>}
            </div>
        </div>
    );
}

function ProjectTaskRow({
    task,
    status,
    currentFlow,
    focusedTarget,
    onOpenDetail,
    onOpenPhase,
    t,
}: {
    task: ReturnType<typeof getProjectTaskCards>[number];
    status: VibehubCockpitStatus;
    currentFlow: ReturnType<typeof getModeFlow>;
    focusedTarget: FocusTarget | null;
    onOpenDetail: (detail: DashboardDetail, target?: DetailOpenTarget) => void;
    onOpenPhase: (phase: string | null, target?: DetailOpenTarget) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const steps = getTaskProcessSteps(task, status, currentFlow);
    const shortLabel = getTaskShortLabel(task, t);
    const taskFocused = focusedTarget?.taskId === task.task_id;
    const activeStep = steps.find((step) => step.kind === 'phase' && (step.status === 'active' || step.status === 'running'))
        || steps.find((step) => step.kind === 'phase' && step.phase === task.phase);

    return (
        <article className={`grid gap-4 px-2 py-4 transition md:grid-cols-[minmax(0,1fr)_minmax(18rem,0.78fr)] ${taskFocused ? 'bg-primary/5 ring-1 ring-primary/30' : ''}`}>
            <button
                type="button"
                onClick={() => onOpenDetail('task', { taskId: task.task_id, phase: activeStep?.phase || task.phase || status.current_phase || null })}
                className="group min-w-0 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
                <div className="flex min-w-0 items-start gap-3">
                    <div className={`flex h-9 w-14 shrink-0 items-center justify-center border text-xs font-semibold ${task.current ? 'border-foreground bg-foreground text-background' : 'border-border text-muted-foreground'}`}>
                        <span className="truncate px-1">{shortLabel}</span>
                    </div>
                    <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 flex-wrap items-center gap-2">
                            <h3 className="min-w-0 truncate text-base font-medium leading-6 group-hover:text-primary">
                                {task.title || task.task_id}
                            </h3>
                            {task.current && <StatusTag tone="active" label={labelOrFallback(t, 'vibehub.kanban.current', 'Current')} />}
                            {task.intake_order && <span className="text-xs text-muted-foreground">#{task.intake_order}</span>}
                        </div>
                        <div className="mt-1 truncate font-mono text-[11px] text-muted-foreground">
                            {task.task_id}{task.run_id ? ` / ${task.run_id}` : ''}
                        </div>
                        <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                            <span>{labelOrFallback(t, 'vibehub.status.phase', 'Phase')}: {activeStep?.phase || task.phase || t('common.unknown')}</span>
                            <span className="h-1 w-1 rounded-full bg-muted-foreground/40" />
                            <span>{formatLocalStatus(task.phase_status || activeStep?.status || 'pending', t)}</span>
                        </div>
                        {task.shared_files.length > 0 && (
                            <div className="mt-2 truncate text-xs text-amber-700 dark:text-amber-400">
                                {labelOrFallback(t, 'vibehub.kanban.sharedFiles', 'Shared')}: {task.shared_files.slice(0, 4).join(', ')}
                            </div>
                        )}
                    </div>
                </div>
            </button>

            <div className="min-w-0">
                <button
                    type="button"
                    onClick={() => onOpenPhase(activeStep?.phase || task.phase || status.current_phase || null, { taskId: task.task_id })}
                    className="mb-2 flex w-full items-center justify-between gap-2 text-left text-xs text-muted-foreground transition hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    <span>{labelOrFallback(t, 'vibehub.projectMap.process', 'Process')}</span>
                    <ChevronRight className="h-3.5 w-3.5" />
                </button>
                <div className="flex min-w-0 items-center gap-1.5 overflow-x-auto pb-1">
                    {steps.map((step, index) => (
                        <ProcessToken
                            key={`${task.task_id}:${step.phase || index}:${step.kind}`}
                            step={step}
                            focused={Boolean(taskFocused && focusedTarget?.capability && focusedTarget.capability === step.phase)}
                            onOpenPhase={(phase) => onOpenPhase(phase, { taskId: task.task_id })}
                            t={t}
                        />
                    ))}
                </div>
            </div>
        </article>
    );
}

function ProcessToken({
    step,
    focused,
    onOpenPhase,
    t,
}: {
    step: { kind: string; phase: string | null; status: string };
    focused: boolean;
    onOpenPhase: (phase: string | null) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    if (step.kind === 'ellipsis') {
        return <span className="px-1.5 text-xs text-muted-foreground">...</span>;
    }
    const active = step.status === 'active' || step.status === 'running';
    const completed = step.status === 'completed';
    const failed = step.status === 'needs_action' || step.status === 'blocked' || step.status === 'failed';

    return (
        <button
            type="button"
            onClick={(event) => {
                event.stopPropagation();
                onOpenPhase(step.phase);
            }}
            className={`flex shrink-0 items-center gap-1.5 border px-2.5 py-1 text-xs transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                focused
                    ? 'border-primary bg-primary/10 text-primary'
                    : active
                        ? 'border-foreground bg-foreground text-background'
                        : failed
                            ? 'border-destructive/50 bg-destructive/5 text-destructive'
                            : completed
                                ? 'border-border bg-muted/30 text-muted-foreground'
                                : 'border-border bg-background text-muted-foreground hover:border-foreground/30 hover:text-foreground'
            }`}
        >
            {completed ? <CheckCircle2 className="h-3 w-3" /> : active ? <Circle className="h-2 w-2 fill-current" /> : null}
            <span>{step.phase ? labelOrFallback(t, `vibehub.flowPhases.${step.phase}`, step.phase) : t('common.unknown')}</span>
        </button>
    );
}

function StatusTag({ label, tone }: { label: string; tone: 'active' | 'warn' | 'neutral' }) {
    return (
        <span className={`border px-1.5 py-0.5 text-[10px] font-medium uppercase ${
            tone === 'active'
                ? 'border-primary/40 bg-primary/10 text-primary'
                : tone === 'warn'
                    ? 'border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-400'
                    : 'border-border text-muted-foreground'
        }`}>
            {label}
        </span>
    );
}

function StatusRail({
    status,
    phaseValidation,
    readOnlyActions,
    promptTemplates,
    onOpenPrompt,
    onOpenDetail,
    onOpenPhase,
    t,
}: {
    status: VibehubCockpitStatus;
    phaseValidation: PhaseValidationResult | null;
    readOnlyActions: RecommendedReadOnlyAction[];
    promptTemplates: VibehubPromptTemplateOption[];
    onOpenPrompt: (templateId: VibehubPromptTemplateId) => void;
    onOpenDetail: (detail: DashboardDetail) => void;
    onOpenPhase: (phase: string | null) => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    const primaryAction = pickPrimaryAction(readOnlyActions, promptTemplates);
    const secondaryActions = readOnlyActions.filter((action) => action !== primaryAction).slice(0, 3);

    return (
        <div className="space-y-4">
            <div className="border border-border/70">
                <div className="border-b border-border/70 px-3 py-2 text-xs font-semibold uppercase text-muted-foreground">
                    {labelOrFallback(t, 'vibehub.projectMap.health', 'Health')}
                </div>
                <div className="divide-y divide-border/60">
                    <RailRow label={t('vibehub.status.mode')} value={status.current_mode || t('common.unknown')} />
                    <RailRow label={t('vibehub.status.phase')} value={status.current_phase || t('common.none')} onClick={() => onOpenPhase(status.current_phase || null)} />
                    <RailRow label={t('vibehub.status.phaseStatus')} value={formatLocalStatus(status.phase_status || 'active', t)} />
                    <RailRow label={t('vibehub.tabs.contextPackStatus')} value={formatLocalStatus(status.context_pack_status.status, t)} onClick={() => onOpenDetail('context')} />
                    <RailRow label={t('vibehub.tabs.agentOutputStatus')} value={formatLocalStatus(status.agent_output_status.status, t)} onClick={() => onOpenDetail('output')} />
                    <RailRow label={t('vibehub.tabs.handoffStatus')} value={formatLocalStatus(status.handoff_status.status, t)} onClick={() => onOpenDetail('handoff')} />
                </div>
            </div>

            {status.warnings.length > 0 && (
                <div className="border border-amber-500/30 bg-amber-500/5">
                    <div className="flex items-center gap-2 border-b border-amber-500/20 px-3 py-2 text-xs font-semibold text-amber-700 dark:text-amber-400">
                        <AlertCircle className="h-3.5 w-3.5" />
                        {t('vibehub.tabs.warnings')}
                    </div>
                    <div className="divide-y divide-amber-500/10">
                        {status.warnings.slice(0, 4).map((warning) => (
                            <div key={warning} className="px-3 py-2 text-xs leading-5 text-amber-800 dark:text-amber-300">
                                {warning}
                            </div>
                        ))}
                        {status.warnings.length > 4 && (
                            <div className="px-3 py-2 text-xs text-muted-foreground">
                                +{status.warnings.length - 4}
                            </div>
                        )}
                    </div>
                </div>
            )}

            {phaseValidation?.missing_outputs.length ? (
                <div className="border border-destructive/30 bg-destructive/5 px-3 py-3">
                    <div className="text-xs font-semibold text-destructive">{labelOrFallback(t, 'vibehub.kanban.schemaIssue', 'Schema issue')}</div>
                    <div className="mt-2 flex flex-wrap gap-1">
                        {phaseValidation.missing_outputs.map((item) => (
                            <Badge key={item} variant="destructive">{item}</Badge>
                        ))}
                    </div>
                </div>
            ) : null}

            <div className="border border-border/70">
                <div className="border-b border-border/70 px-3 py-2 text-xs font-semibold uppercase text-muted-foreground">
                    {labelOrFallback(t, 'vibehub.projectMap.nextAction', 'Next action')}
                </div>
                <div className="space-y-3 px-3 py-3">
                    {primaryAction ? (
                        <>
                            <div>
                                <div className="text-sm font-medium">{primaryAction.title}</div>
                                <div className="mt-1 text-xs leading-5 text-muted-foreground">{primaryAction.description}</div>
                            </div>
                            <Button className="w-full justify-start" size="sm" onClick={() => onOpenPrompt(primaryAction.promptTemplateId)}>
                                <Bot className="mr-2 h-4 w-4" />
                                {t('vibehub.promptGenerator.generate')}
                            </Button>
                        </>
                    ) : (
                        <Button className="w-full justify-start" size="sm" onClick={() => onOpenPrompt('new-task')}>
                            <Bot className="mr-2 h-4 w-4" />
                            {t('vibehub.promptGenerator.title')}
                        </Button>
                    )}
                    {secondaryActions.length > 0 && (
                        <div className="space-y-1 border-t border-border/60 pt-2">
                            {secondaryActions.map((action) => (
                                <button
                                    key={`${action.command}-${action.title}`}
                                    type="button"
                                    onClick={() => onOpenPrompt(action.promptTemplateId)}
                                    className="flex w-full items-center justify-between gap-2 px-0 py-1 text-left text-xs text-muted-foreground transition hover:text-foreground"
                                >
                                    <span className="truncate">{action.title}</span>
                                    <span className="font-mono">{action.command}</span>
                                </button>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

function RailRow({ label, value, onClick }: { label: string; value: string; onClick?: () => void }) {
    const content = (
        <>
            <span className="text-xs text-muted-foreground">{label}</span>
            <span className="min-w-0 truncate text-right text-xs font-medium">{value}</span>
        </>
    );
    if (onClick) {
        return (
            <button type="button" onClick={onClick} className="flex w-full items-center justify-between gap-3 px-3 py-2 text-left hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
                {content}
            </button>
        );
    }
    return <div className="flex items-center justify-between gap-3 px-3 py-2">{content}</div>;
}

function StructurePreview({
    projectStructure,
    structureNodes,
    structureFiles,
    fallbackFiles,
    onOpen,
    t,
}: {
    projectStructure: VibehubProjectStructureViewData | null;
    structureNodes: VibehubProjectStructureViewData['graph_nodes'];
    structureFiles: string[];
    fallbackFiles: string[];
    onOpen: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <section className="space-y-3">
            <SectionHeader
                icon={<Boxes className="h-4 w-4" />}
                title={labelOrFallback(t, 'vibehub.projectMap.structure', 'Project structure')}
                detail={labelOrFallback(t, 'vibehub.projectMap.structureSource', 'Filesystem scan; semantic graph is not configured yet.')}
            />
            <button
                type="button"
                onClick={onOpen}
                className="group grid w-full gap-5 border border-border/70 px-4 py-4 text-left transition hover:border-foreground/30 hover:bg-muted/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring md:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]"
            >
                <div className="min-w-0">
                    <div className="mb-3 flex items-center justify-between gap-2">
                        <div className="text-xs font-semibold uppercase text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.moduleMap', 'Module map')}</div>
                        {projectStructure?.truncated && <Badge variant="secondary">{labelOrFallback(t, 'vibehub.projectMap.structureTruncated', 'Truncated')}</Badge>}
                    </div>
                    <div className="space-y-2">
                        {structureNodes.length ? structureNodes.map((node) => (
                            <div key={node.id} className="flex items-center gap-2 text-sm">
                                <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${node.changed ? 'bg-primary' : 'bg-muted-foreground/40'}`} />
                                <span className="min-w-0 flex-1 truncate">{node.label}</span>
                                <span className="shrink-0 text-xs text-muted-foreground">{node.file_count}</span>
                            </div>
                        )) : (
                            <div className="text-sm text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}</div>
                        )}
                    </div>
                </div>
                <div className="min-w-0 border-t border-border/60 pt-4 md:border-l md:border-t-0 md:pl-4 md:pt-0">
                    <div className="mb-3 flex items-center justify-between gap-2">
                        <div className="text-xs font-semibold uppercase text-muted-foreground">{labelOrFallback(t, 'vibehub.projectMap.filePreview', 'File preview')}</div>
                        <ExternalLink className="h-3.5 w-3.5 text-muted-foreground opacity-0 transition group-hover:opacity-100" />
                    </div>
                    <div className="space-y-1.5">
                        {(structureFiles.length ? structureFiles : fallbackFiles).slice(0, 7).map((file) => (
                            <div key={file} className="truncate font-mono text-xs text-muted-foreground">{file}</div>
                        ))}
                    </div>
                </div>
            </button>
        </section>
    );
}

function ArchivePreview({
    archiveCards,
    warnings,
    onOpen,
    t,
}: {
    archiveCards: VibehubArchiveViewData['cards'];
    warnings: string[];
    onOpen: () => void;
    t: (key: string, options?: Record<string, unknown>) => string;
}) {
    return (
        <section className="space-y-3">
            <SectionHeader
                icon={<Archive className="h-4 w-4" />}
                title={labelOrFallback(t, 'vibehub.projectMap.archive', 'Archive')}
                detail={labelOrFallback(t, 'vibehub.projectMap.archiveHint', 'Completed and cancelled task history stays inspectable here.')}
                count={archiveCards.length}
            />
            <div className="border border-border/70">
                {warnings.slice(0, 2).map((warning) => (
                    <div key={warning} className="border-b border-amber-500/20 bg-amber-500/5 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
                        {warning}
                    </div>
                ))}
                {archiveCards.length === 0 ? (
                    <div className="px-4 py-6 text-sm text-muted-foreground">
                        {labelOrFallback(t, 'vibehub.projectMap.archiveEmpty', 'No completed or cancelled tasks yet.')}
                    </div>
                ) : (
                    <div className="divide-y divide-border/60">
                        {archiveCards.slice(0, 3).map((card) => (
                            <div key={card.task_id} className="px-2 py-2">
                                <ArchiveCardButton card={card} compact onClick={onOpen} t={t} />
                            </div>
                        ))}
                    </div>
                )}
                <button
                    type="button"
                    onClick={onOpen}
                    className="flex w-full items-center justify-between gap-2 border-t border-border/70 px-4 py-2 text-left text-xs font-medium text-muted-foreground transition hover:bg-muted/30 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    <span>{labelOrFallback(t, 'vibehub.projectMap.archiveOpen', 'Open')}</span>
                    <ChevronRight className="h-3.5 w-3.5" />
                </button>
            </div>
        </section>
    );
}

function pickPrimaryAction(readOnlyActions: RecommendedReadOnlyAction[], promptTemplates: VibehubPromptTemplateOption[]) {
    const preferred = ['sync', 'fix-schema', 'claim-capability', 'release-capability', 'new-task'];
    return readOnlyActions.find((action) => preferred.includes(action.promptTemplateId))
        || readOnlyActions[0]
        || (promptTemplates.find((template) => template.id === 'new-task')
            ? {
                command: 'vibehub-start',
                title: 'New task',
                description: 'Generate a prompt for the agent chat window.',
                promptTemplateId: 'new-task' as VibehubPromptTemplateId,
            }
            : null);
}

function formatLocalStatus(value: string | null | undefined, t: (key: string, options?: Record<string, unknown>) => string) {
    if (!value) return t('common.unknown');
    return labelOrFallback(t, `vibehub.stateValues.${value}`, value);
}
