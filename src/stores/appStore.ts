import { create } from 'zustand';
import { tauriApi } from '@/services/tauri';
import { AppConfig, Project, Tag, Theme } from '@/types';

type EffectiveTheme = 'light' | 'dark';

const systemThemeQuery = '(prefers-color-scheme: dark)';

let mediaQueryList: MediaQueryList | null = null;
let removeSystemThemeListener: (() => void) | null = null;

export function resolveEffectiveTheme(theme: Theme, prefersDark: boolean): EffectiveTheme {
    if (theme === 'auto') {
        return prefersDark ? 'dark' : 'light';
    }
    return theme;
}

function getSystemPrefersDark(): boolean {
    if (typeof window === 'undefined' || !window.matchMedia) {
        return false;
    }
    return window.matchMedia(systemThemeQuery).matches;
}

function applyTheme(theme: Theme, setEffectiveTheme?: (theme: EffectiveTheme) => void) {
    const effectiveTheme = resolveEffectiveTheme(theme, getSystemPrefersDark());
    document.documentElement.classList.toggle('dark', effectiveTheme === 'dark');
    setEffectiveTheme?.(effectiveTheme);
}

function watchSystemTheme(getTheme: () => Theme | undefined, onThemeChange: () => void) {
    if (typeof window === 'undefined' || !window.matchMedia) {
        return;
    }
    if (removeSystemThemeListener) {
        return;
    }

    mediaQueryList = window.matchMedia(systemThemeQuery);
    const handleSystemThemeChange = () => {
        if (getTheme() === 'auto') {
            onThemeChange();
        }
    };

    if (mediaQueryList.addEventListener) {
        mediaQueryList.addEventListener('change', handleSystemThemeChange);
    } else {
        mediaQueryList.addListener(handleSystemThemeChange);
    }
    removeSystemThemeListener = () => {
        if (mediaQueryList?.removeEventListener) {
            mediaQueryList.removeEventListener('change', handleSystemThemeChange);
        } else {
            mediaQueryList?.removeListener(handleSystemThemeChange);
        }
        mediaQueryList = null;
        removeSystemThemeListener = null;
    };
}

interface AppState {
    config: AppConfig | null;
    effectiveTheme: EffectiveTheme;
    isLoading: boolean;
    error: string | null;
    selectedWorkspaceId: string | null;

    initializeApp: () => Promise<void>;
    refreshConfig: () => Promise<void>;
    refreshAllWorkspaces: () => Promise<void>;
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
    launchCustom: (projectId: string, config: any, category?: string) => Promise<void>;
    openInExplorer: (path: string) => Promise<void>;
    openTerminal: (path: string) => Promise<void>;
    setTheme: (theme: Theme) => Promise<void>;
}

export const useAppStore = create<AppState>((set, get) => ({
    config: null,
    effectiveTheme: 'light',
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
            applyTheme(config.theme, (effectiveTheme) => set({ effectiveTheme }));
            watchSystemTheme(
                () => get().config?.theme,
                () => applyTheme(get().config?.theme ?? 'auto', (effectiveTheme) => set({ effectiveTheme }))
            );
        } catch (error) {
            set({ error: (error as Error).message, isLoading: false });
        }
    },

    refreshConfig: async () => {
        try {
            const config = await tauriApi.loadConfig();
            set({ config });
            applyTheme(config.theme, (effectiveTheme) => set({ effectiveTheme }));
        } catch (error) {
            console.error('Failed to refresh config:', error);
        }
    },

    refreshAllWorkspaces: async () => {
        try {
            await tauriApi.refreshAllWorkspaces();
            const config = await tauriApi.loadConfig();
            set({ config });
            applyTheme(config.theme, (effectiveTheme) => set({ effectiveTheme }));
        } catch (error) {
            console.error('Failed to refresh workspaces:', error);
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
        const currentConfig = get().config;
        if (currentConfig) {
            const newConfig = { ...currentConfig, projects };
            set({ config: newConfig });
            try {
                await tauriApi.saveConfig(newConfig);
            } catch (error) {
                console.error('Failed to persist project order:', error);
            }
        }
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

    launchCustom: async (projectId, config, category) => {
        try {
            await tauriApi.launchCustom(projectId, config, category);
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
        applyTheme(theme, (effectiveTheme) => set({ effectiveTheme }));
    },
}));
