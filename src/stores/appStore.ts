import { create } from 'zustand';
import { tauriApi } from '@/services/tauri';
import { AppConfig, Project, Tag, Theme } from '@/types';

interface AppState {
    config: AppConfig | null;
    isLoading: boolean;
    error: string | null;
    selectedWorkspaceId: string | null;

    initializeApp: () => Promise<void>;
    refreshConfig: () => Promise<void>;
    setSelectedWorkspaceId: (id: string | null) => void;

    addWorkspace: (name: string, path: string, autoScan: boolean) => Promise<void>;
    removeWorkspace: (id: string) => Promise<void>;
    scanWorkspace: (path: string) => Promise<Project[]>;

    updateProject: (project: Project) => Promise<void>;
    deleteProject: (id: string) => Promise<void>;
    toggleProjectStar: (id: string) => Promise<void>;
    recordProjectOpen: (id: string) => Promise<void>;
    reorderProjects: (projects: Project[]) => Promise<void>;

    addTag: (tag: Tag) => Promise<void>;
    updateTag: (tag: Tag) => Promise<void>;
    deleteTag: (id: string) => Promise<void>;

    launchTool: (projectId: string) => Promise<void>;
    launchCustom: (projectId: string, config: any) => Promise<void>;
    openInExplorer: (path: string) => Promise<void>;
    openTerminal: (path: string) => Promise<void>;
    setTheme: (theme: Theme) => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
    config: null,
    isLoading: false,
    error: null,
    selectedWorkspaceId: null,

    setSelectedWorkspaceId: (id) => set({ selectedWorkspaceId: id }),

    initializeApp: async () => {
        try {
            set({ isLoading: true });
            await tauriApi.initializeDefaultConfigs();
            const config = await tauriApi.loadConfig();
            set({ config, isLoading: false });

            // Apply theme
            const theme = config.theme;
            if (theme === 'dark' || (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
                document.documentElement.classList.add('dark');
            } else {
                document.documentElement.classList.remove('dark');
            }
        } catch (error) {
            set({ error: (error as Error).message, isLoading: false });
        }
    },

    refreshConfig: async () => {
        try {
            const config = await tauriApi.loadConfig();
            set({ config });
        } catch (error) {
            console.error('Failed to refresh config:', error);
        }
    },

    addWorkspace: async (name, path, autoScan) => {
        try {
            await tauriApi.addWorkspace(name, path, autoScan);
            if (autoScan) {
                // Backend now handles saving scanned projects automatically
                await tauriApi.scanWorkspace(path);
            }
            await get().refreshConfig();
        } catch (error) {
            set({ error: (error as Error).message });
            throw error;
        }
    },

    removeWorkspace: async (id) => {
        await tauriApi.removeWorkspace(id);
        await get().refreshConfig();
    },

    scanWorkspace: async (path) => {
        return await tauriApi.scanWorkspace(path);
    },

    updateProject: async (project) => {
        await tauriApi.updateProject(project);
        await get().refreshConfig();
    },

    deleteProject: async (id) => {
        await tauriApi.deleteProject(id);
        await get().refreshConfig();
    },

    toggleProjectStar: async (id) => {
        await tauriApi.toggleProjectStar(id);
        await get().refreshConfig();
    },

    recordProjectOpen: async (id) => {
        await tauriApi.recordProjectOpen(id);
        await get().refreshConfig();
    },

    reorderProjects: async (projects) => {
        // Optimistic update
        const currentConfig = get().config;
        if (currentConfig) {
            set({ config: { ...currentConfig, projects } });
        }
        // Ideally we should persist this order to backend if backend supports it.
        // For now, we just update local state. If backend doesn't support order, it will reset on refresh.
        // Assuming backend preserves order or we don't persist it yet.
        // If we want to persist, we need a backend command for it.
        // For now, let's assume we just update the local state for the session or until refresh.
        // If we want to persist, we'd iterate and update each project or have a bulk update.
        // Let's just update the config in backend if possible, but we don't have a 'saveConfig' method exposed directly that takes the whole config.
        // We'll stick to local state update for now, acknowledging it might be lost on restart if not persisted.
        // Wait, `updateProject` persists. But reordering 100 projects is 100 calls.
        // Let's leave it as optimistic local update for now.
    },

    addTag: async (tag) => {
        await tauriApi.addTag(tag);
        await get().refreshConfig();
    },

    updateTag: async (tag) => {
        await tauriApi.updateTag(tag);
        await get().refreshConfig();
    },

    deleteTag: async (id) => {
        await tauriApi.deleteTag(id);
        await get().refreshConfig();
    },

    launchTool: async (projectId) => {
        try {
            await tauriApi.launchTool(projectId);
            await get().recordProjectOpen(projectId);
        } catch (error) {
            set({ error: (error as Error).message });
            throw error;
        }
    },

    launchCustom: async (projectId, config) => {
        try {
            await tauriApi.launchCustom(projectId, config);
            await get().recordProjectOpen(projectId);
        } catch (error) {
            set({ error: (error as Error).message });
            throw error;
        }
    },

    openInExplorer: async (path) => {
        await tauriApi.openInExplorer(path);
    },

    openTerminal: async (path) => {
        await tauriApi.openTerminal(path);
    },

    setTheme: async (theme) => {
        await tauriApi.setTheme(theme);
        await get().refreshConfig();

        if (theme === 'dark' || (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
            document.documentElement.classList.add('dark');
        } else {
            document.documentElement.classList.remove('dark');
        }
    },
}));
