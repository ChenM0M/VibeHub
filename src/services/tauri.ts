import { invoke } from '@tauri-apps/api/core';
import { AppConfig, Project, Workspace, Tag } from '../types';

export const tauriApi = {
    loadConfig: async (): Promise<AppConfig> => {
        return await invoke('load_config');
    },

    saveConfig: async (config: AppConfig): Promise<void> => {
        return await invoke('save_config', { config });
    },

    scanWorkspace: async (path: string, maxDepth: number = 1): Promise<Project[]> => {
        return await invoke('scan_workspace', { path, maxDepth });
    },

    addWorkspace: async (name: string, path: string, autoScan: boolean): Promise<Workspace> => {
        return await invoke('add_workspace', { name, path, autoScan });
    },

    removeWorkspace: async (workspaceId: string): Promise<void> => {
        return await invoke('remove_workspace', { workspaceId });
    },

    updateProject: async (project: Project): Promise<void> => {
        return await invoke('update_project', { project });
    },

    deleteProject: async (projectId: string): Promise<void> => {
        return await invoke('delete_project', { projectId });
    },

    addTag: async (tag: Tag): Promise<void> => {
        return await invoke('add_tag', { tag });
    },

    updateTag: async (tag: Tag): Promise<void> => {
        return await invoke('update_tag', { tag });
    },

    deleteTag: async (tagId: string): Promise<void> => {
        return await invoke('delete_tag', { tagId });
    },

    launchTool: async (projectId: string): Promise<void> => {
        return await invoke('launch_tool', { projectId });
    },

    launchCustom: async (projectId: string, config: any): Promise<void> => {
        return await invoke('launch_custom', { projectId, config });
    },

    openInExplorer: async (path: string): Promise<void> => {
        return await invoke('open_in_explorer', { path });
    },

    openTerminal: async (path: string): Promise<void> => {
        return await invoke('open_terminal', { path });
    },

    recordProjectOpen: async (projectId: string): Promise<void> => {
        return await invoke('record_project_open', { projectId });
    },

    toggleProjectStar: async (projectId: string): Promise<boolean> => {
        return await invoke('toggle_project_star', { projectId });
    },

    initializeDefaultConfigs: async (): Promise<void> => {
        return await invoke('initialize_default_configs');
    },

    setTheme: async (theme: string): Promise<void> => {
        return await invoke('set_theme', { theme });
    },

    refreshAllWorkspaces: async (): Promise<void> => {
        return await invoke('refresh_all_workspaces');
    },

    checkForUpdates: async (): Promise<{
        has_update: boolean;
        current_version: string;
        latest_version: string;
        release_notes: string | null;
        release_url: string | null;
        download_url: string | null;
    }> => {
        return await invoke('check_for_updates');
    }
};
