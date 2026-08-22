import { create } from 'zustand';
import { tauriApi } from '@/services/tauri';
import type { Project } from '@/types';
import type {
    WorkspaceProjectUiContext,
    WorkspaceState,
    WorkspaceStateDiagnostic,
} from '@/v3/contracts/generated';

const DEFAULT_UI_CONTEXT: WorkspaceProjectUiContext = {
    owner: 'v3-cockpit',
    current_view: 'project-overview',
    selected_task_id: null,
    selected_node_id: null,
};

interface TabsState {
    tabs: string[];
    activeTabId: string | null;
    hydrated: boolean;
    hydrating: boolean;
    persistedRevision: number;
    diagnostics: WorkspaceStateDiagnostic[];
    openTab: (projectId: string) => void;
    activateTab: (projectId: string) => void;
    activateIndex: (index: number) => void;
    activateRelative: (offset: number) => void;
    closeTab: (projectId: string) => void;
    closeActiveTab: () => void;
    reorderTabs: (tabs: string[]) => void;
    deactivateTabs: () => void;
    pruneTabs: (validProjectIds: string[]) => void;
    hydrate: (projects: Project[]) => Promise<void>;
    retryHydrate: () => Promise<void>;
    reconcileProjects: (projects: Project[]) => void;
    setProjectUiContext: (projectId: string, context: Partial<WorkspaceProjectUiContext>) => void;
    setProjectUiContextForPath: (projectPath: string, context: Partial<WorkspaceProjectUiContext>) => void;
    projectIdForPath: (projectPath: string) => string | null;
    getProjectUiContextForPath: (projectPath: string) => WorkspaceProjectUiContext;
    getProjectUiContext: (projectId: string) => WorkspaceProjectUiContext;
}

let currentProjects = new Map<string, Project>();
let currentUiContexts = new Map<string, WorkspaceProjectUiContext>();
let hydrationKey: string | null = null;
let restoreRequestId = 0;
let mutationGeneration = 0;
let saveRequestId = 0;
let saveQueue: Promise<void> = Promise.resolve();
let queuedRevision = 0;
const MAX_PERSIST_RETRY_DEPTH = 3;

interface PersistOperation {
    operation: string;
    retryDepth: number;
}

function projectsKey(projects: Project[]): string {
    return projects.map((project) => `${project.id}\u0000${project.path}`).join('\u0001');
}

function defaultContext(): WorkspaceProjectUiContext {
    return { ...DEFAULT_UI_CONTEXT };
}

function errorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    const structured = error as { message?: unknown };
    return typeof structured?.message === 'string' ? structured.message : String(error);
}

function conflictCurrentRevision(error: unknown, fallback: number): number {
    const match = errorMessage(error).match(/\bcurrent\s+(\d+)\b/i);
    if (!match) return fallback;
    const revision = Number(match[1]);
    return Number.isSafeInteger(revision) ? revision : fallback;
}

function revisionConflictDiagnostic(
    error: unknown,
    operation: PersistOperation,
    tabs: string[],
    activeTabId: string | null,
    currentRevision: number,
): WorkspaceStateDiagnostic {
    const projectId = activeTabId ?? tabs[0] ?? 'workspace';
    return {
        code: 'workspace_state.revision_conflict_exhausted',
        severity: 'error',
        project_id: activeTabId ?? tabs[0] ?? null,
        message: `工作区保存冲突已停止。下一步：点击“重试”使用当前 revision 再保存，或点击“打开项目列表”放弃本次操作。project=${projectId}; operation=${operation.operation}; current_revision=${currentRevision}; retry_depth=${operation.retryDepth}/${MAX_PERSIST_RETRY_DEPTH}。原始错误：${errorMessage(error)}`,
        recovery_action: '使用当前 revision 重试保存，或打开项目列表放弃本次操作。',
    };
}

function errorDiagnostic(error: unknown): WorkspaceStateDiagnostic {
    const structured = error as { message?: unknown; code?: unknown; recovery_action?: unknown };
    const message = error instanceof Error
        ? error.message
        : typeof structured?.message === 'string'
            ? structured.message
            : String(error);
    return {
        code: typeof structured?.code === 'string' ? structured.code : 'workspace_state.persistence_failed',
        severity: 'error',
        project_id: null,
        message,
        recovery_action: typeof structured?.recovery_action === 'string' ? structured.recovery_action : '重试保存或重新打开项目。',
    };
}

function hasActionableWorkspaceWarning(diagnostics: WorkspaceStateDiagnostic[]): boolean {
    return diagnostics.some((diagnostic) =>
        diagnostic.severity === 'warning' && diagnostic.code.startsWith('workspace_state.'),
    );
}

function buildState(
    tabs: string[],
    activeTabId: string | null,
    persistedRevision: number,
    reason: string,
): WorkspaceState {
    return {
        schema_version: 3,
        kind: 'workspace_state',
        revision: persistedRevision,
        open_projects: tabs.flatMap((projectId) => {
            const project = currentProjects.get(projectId);
            if (!project) return [];
            return [{
                project_id: project.id,
                canonical_path: project.path,
                display_name: project.name,
                ui_context: currentUiContexts.get(project.id) ?? defaultContext(),
            }];
        }),
        active_project_id: activeTabId && currentProjects.has(activeTabId) ? activeTabId : null,
        updated_at: new Date().toISOString(),
        updated_by: 'v3-tabs',
        provenance: {
            source: reason === 'recovery' ? 'recovery' : 'user',
            writer: 'tabsStore',
            reason,
        },
    };
}

function enqueuePersist(
    reason: string,
    operation: PersistOperation = { operation: reason, retryDepth: 0 },
): Promise<void> {
    const requestId = ++saveRequestId;
    const snapshot = useTabsStore.getState();
    if (!snapshot.hydrated || currentProjects.size === 0) return Promise.resolve();

    const tabs = [...snapshot.tabs];
    const activeTabId = snapshot.activeTabId;
    const task = saveQueue.then(async () => {
        const state = buildState(tabs, activeTabId, queuedRevision, reason);
        try {
            const result = await tauriApi.saveWorkspaceState({
                expected_revision: state.revision,
                state,
            });
            if (requestId >= saveRequestId || useTabsStore.getState().persistedRevision < result.state.revision) {
                const currentDiagnostics = useTabsStore.getState().diagnostics;
                // Keep queuedRevision and persistedRevision in lockstep so a later save never
                // carries a stale expected_revision that would spuriously conflict. Bumping only
                // inside this gate (was unconditional before) removes the divergence where the
                // frontend thought it was at revision N while disk had already moved to N+k.
                const nextRevision = result.state.revision;
                queuedRevision = nextRevision;
                useTabsStore.setState({
                    persistedRevision: nextRevision,
                    diagnostics: result.diagnostics?.length
                        ? result.diagnostics
                        : reason === 'recovery' || hasActionableWorkspaceWarning(currentDiagnostics)
                            ? currentDiagnostics
                            : [],
                });
            }
        } catch (error) {
            // `error` is the Tauri-deserialized WorkspaceStateCommandError object, not a
            // string. Keep the retry depth on this logical operation so the queued replay
            // cannot reset a global counter before the next attempt starts.
            if (errorMessage(error).includes('WORKSPACE_STATE_REVISION_CONFLICT')) {
                const nextRetryDepth = operation.retryDepth + 1;
                if (nextRetryDepth <= MAX_PERSIST_RETRY_DEPTH) {
                    try {
                        const latest = await tauriApi.loadWorkspaceState();
                        const refreshedRevision = latest.state?.revision
                            ?? conflictCurrentRevision(error, queuedRevision);
                        queuedRevision = refreshedRevision;
                        if (requestId >= saveRequestId) {
                            useTabsStore.setState({ persistedRevision: refreshedRevision, diagnostics: latest.diagnostics ?? [] });
                            enqueuePersist('retry-after-revision-conflict', {
                                operation: operation.operation,
                                retryDepth: nextRetryDepth,
                            });
                        }
                        return;
                    } catch {
                        // Loading also failed: fall through and surface the original error.
                    }
                }

                const currentRevision = conflictCurrentRevision(error, queuedRevision);
                queuedRevision = currentRevision;
                if (requestId >= saveRequestId) {
                    useTabsStore.setState({
                        persistedRevision: currentRevision,
                        diagnostics: [revisionConflictDiagnostic(
                            error,
                            { ...operation, retryDepth: Math.min(operation.retryDepth, MAX_PERSIST_RETRY_DEPTH) },
                            tabs,
                            activeTabId,
                            currentRevision,
                        )],
                    });
                }
                return;
            }
            if (requestId >= saveRequestId) {
                useTabsStore.setState({ diagnostics: [errorDiagnostic(error)] });
            }
        }
    });
    saveQueue = task.catch(() => undefined);
    return saveQueue;
}

function mutate(next: Partial<TabsState>): void {
    mutationGeneration += 1;
    useTabsStore.setState(next);
}

export const useTabsStore = create<TabsState>((set, get) => ({
    tabs: [],
    activeTabId: null,
    hydrated: false,
    hydrating: false,
    persistedRevision: 0,
    diagnostics: [],

    openTab: (projectId) => {
        const { tabs } = get();
        mutate({
            tabs: tabs.includes(projectId) ? tabs : [...tabs, projectId],
            activeTabId: projectId,
        });
        enqueuePersist('open');
    },

    activateTab: (projectId) => {
        if (!get().tabs.includes(projectId)) return;
        mutate({ activeTabId: projectId });
        enqueuePersist('activate');
    },

    activateIndex: (index) => {
        const { tabs } = get();
        if (index < 0 || index >= tabs.length) return;
        mutate({ activeTabId: tabs[index] });
        enqueuePersist('activate');
    },

    activateRelative: (offset) => {
        const { tabs, activeTabId } = get();
        if (tabs.length === 0) return;
        const current = activeTabId ? tabs.indexOf(activeTabId) : -1;
        const base = current === -1 ? 0 : current;
        const next = (base + offset + tabs.length) % tabs.length;
        mutate({ activeTabId: tabs[next] });
        enqueuePersist('activate');
    },

    closeTab: (projectId) => {
        const { tabs, activeTabId } = get();
        const index = tabs.indexOf(projectId);
        if (index === -1) return;
        const remaining = tabs.filter((id) => id !== projectId);
        if (activeTabId !== projectId) {
            mutate({ tabs: remaining });
            enqueuePersist('close');
            return;
        }
        const neighbour = remaining[index] ?? remaining[index - 1] ?? null;
        mutate({ tabs: remaining, activeTabId: neighbour });
        enqueuePersist('close');
    },

    closeActiveTab: () => {
        const { activeTabId, closeTab } = get();
        if (!activeTabId) return;
        closeTab(activeTabId);
    },

    reorderTabs: (tabs) => {
        const known = new Set(get().tabs);
        if (tabs.length !== known.size || tabs.some((id) => !known.has(id))) return;
        mutate({ tabs: [...tabs] });
        enqueuePersist('reorder');
    },

    deactivateTabs: () => {
        if (get().activeTabId === null) return;
        mutate({ activeTabId: null });
        enqueuePersist('deactivate');
    },

    pruneTabs: (validProjectIds) => {
        const { tabs, activeTabId } = get();
        const valid = new Set(validProjectIds);
        const remaining = tabs.filter((id) => valid.has(id));
        const nextActive = activeTabId && valid.has(activeTabId) ? activeTabId : null;
        if (remaining.length === tabs.length && nextActive === activeTabId) return;
        mutate({ tabs: remaining, activeTabId: nextActive });
        enqueuePersist('prune');
    },

    hydrate: async (projects) => {
        const key = projectsKey(projects);
        currentProjects = new Map(projects.map((project) => [project.id, project]));
        if (hydrationKey === key && get().hydrating) return;
        if (hydrationKey === key && get().hydrated) return;
        hydrationKey = key;
        const requestId = ++restoreRequestId;
        const restoreMutationGeneration = mutationGeneration;
        set({ hydrating: true });
        try {
            const result = await tauriApi.loadWorkspaceState();
            if (requestId !== restoreRequestId) return;
            const state = result.state;
            const nextRevision = state?.revision ?? 0;
            queuedRevision = nextRevision;
            const diagnostics = result.diagnostics ?? [];
            if (restoreMutationGeneration !== mutationGeneration) {
                set({ hydrated: true, hydrating: false, persistedRevision: nextRevision, diagnostics });
                enqueuePersist('user-change-during-recovery');
                return;
            }
            const restoredTabs = state?.open_projects
                .filter((project) => currentProjects.has(project.project_id))
                .map((project) => {
                    currentUiContexts.set(project.project_id, project.ui_context);
                    return project.project_id;
                }) ?? [];
            const activeTabId = state?.active_project_id && restoredTabs.includes(state.active_project_id)
                ? state.active_project_id
                : null;
            set({
                tabs: restoredTabs,
                activeTabId,
                hydrated: true,
                hydrating: false,
                persistedRevision: nextRevision,
                diagnostics,
            });
            if (result.status === 'partial' || result.status === 'recovered') enqueuePersist('recovery');
        } catch (error) {
            if (requestId !== restoreRequestId) return;
            set({ hydrated: true, hydrating: false, diagnostics: [errorDiagnostic(error)] });
        }
    },

    retryHydrate: async () => {
        if (get().diagnostics.some((diagnostic) => diagnostic.code === 'workspace_state.revision_conflict_exhausted')) {
            set({ hydrating: true });
            try {
                await enqueuePersist('manual-retry');
            } finally {
                set({ hydrating: false });
            }
            return;
        }
        hydrationKey = null;
        await get().hydrate([...currentProjects.values()]);
    },

    reconcileProjects: (projects) => {
        const previousProjects = currentProjects;
        const changedIdentity = new Set(
            get().tabs.filter((id) => {
                const previous = previousProjects.get(id);
                const next = projects.find((project) => project.id === id);
                return Boolean(previous && next && previous.path !== next.path);
            }),
        );
        currentProjects = new Map(projects.map((project) => [project.id, project]));
        const validIds = projects.map((project) => project.id);
        get().pruneTabs(validIds.filter((id) => !changedIdentity.has(id)));
        if (changedIdentity.size > 0) {
            useTabsStore.setState({
                diagnostics: Array.from(changedIdentity.values(), (projectId) => ({
                    code: 'workspace_state.project_identity_changed',
                    severity: 'warning',
                    project_id: projectId,
                    message: '项目路径或身份已变化。',
                    recovery_action: '从项目列表重新打开项目。',
                })),
            });
        }
    },

    setProjectUiContext: (projectId, context) => {
        if (!currentProjects.has(projectId)) return;
        currentUiContexts.set(projectId, {
            ...(currentUiContexts.get(projectId) ?? defaultContext()),
            ...context,
            owner: 'v3-cockpit',
        });
        mutationGeneration += 1;
        enqueuePersist('context');
    },

    setProjectUiContextForPath: (projectPath, context) => {
        const project = [...currentProjects.values()].find((candidate) => candidate.path === projectPath);
        if (project) get().setProjectUiContext(project.id, context);
    },

    projectIdForPath: (projectPath) => [...currentProjects.values()].find((project) => project.path === projectPath)?.id ?? null,

    getProjectUiContext: (projectId) => currentUiContexts.get(projectId) ?? defaultContext(),

    getProjectUiContextForPath: (projectPath) => {
        const projectId = [...currentProjects.values()].find((project) => project.path === projectPath)?.id;
        return projectId ? (currentUiContexts.get(projectId) ?? defaultContext()) : defaultContext();
    },
}));
