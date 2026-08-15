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

function projectsKey(projects: Project[]): string {
    return projects.map((project) => `${project.id}\u0000${project.path}`).join('\u0001');
}

function defaultContext(): WorkspaceProjectUiContext {
    return { ...DEFAULT_UI_CONTEXT };
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

function enqueuePersist(reason: string): void {
    const requestId = ++saveRequestId;
    const snapshot = useTabsStore.getState();
    if (!snapshot.hydrated || currentProjects.size === 0) return;

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
                queuedRevision = result.state.revision;
                const currentDiagnostics = useTabsStore.getState().diagnostics;
                useTabsStore.setState({
                    persistedRevision: result.state.revision,
                    diagnostics: result.diagnostics?.length
                        ? result.diagnostics
                        : reason === 'recovery' || hasActionableWorkspaceWarning(currentDiagnostics)
                            ? currentDiagnostics
                            : [],
                });
            }
        } catch (error) {
            if (String(error).includes('WORKSPACE_STATE_REVISION_CONFLICT')) {
                try {
                    const latest = await tauriApi.loadWorkspaceState();
                    queuedRevision = latest.state?.revision ?? queuedRevision;
                    if (requestId >= saveRequestId) {
                        useTabsStore.setState({ persistedRevision: queuedRevision, diagnostics: latest.diagnostics ?? [] });
                        enqueuePersist('retry-after-revision-conflict');
                    }
                    return;
                } catch {
                    // The original persistence error below remains visible to the user.
                }
            }
            if (requestId >= saveRequestId) {
                useTabsStore.setState({ diagnostics: [errorDiagnostic(error)] });
            }
        }
    });
    saveQueue = task.catch(() => undefined);
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
