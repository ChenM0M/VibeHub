import { invoke } from '@tauri-apps/api/core';
import {
    AppConfig,
    Project,
    SettingsImportResult,
    StorageInfo,
    Tag,
    Workspace,
    LocalAgentUsageOverview,
    V3BootstrapResult,
    V3ProjectLayoutStatus,
    V3RepairCandidate,
    V3RepairResult,
} from '../types';
import type {
    V3AgentSpecInspection,
    V3AgentSpecSyncRequest,
    V3AgentSpecSyncResult,
    V3ProjectSettings,
    V3ProjectSettingsInspection,
    V3ProjectSettingsUpdateRequest,
    V3TaskCreateRequest,
    V3TaskCreateResult,
} from '../v3/contracts/generated';
import type { V3AppendResult, V3PlanAddNodeCommand, V3PlanSetDependenciesCommand, V3PlanSetStateCommand } from '../types';

export const tauriApi = {
    loadConfig: async (): Promise<AppConfig> => {
        return await invoke('load_config');
    },

    saveConfig: async (config: AppConfig): Promise<void> => {
        return await invoke('save_config', { config });
    },

    exportSettingsBundle: async (path: string): Promise<void> => {
        return await invoke('export_settings_bundle', { path });
    },

    importSettingsBundle: async (path: string): Promise<SettingsImportResult> => {
        return await invoke('import_settings_bundle', { path });
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

    launchCustom: async (projectId: string, config: any, category?: string): Promise<void> => {
        return await invoke('launch_custom', { projectId, config, category: category || null });
    },

    openInExplorer: async (path: string): Promise<void> => {
        return await invoke('open_in_explorer', { path });
    },

    getStorageInfo: async (): Promise<StorageInfo> => {
        return await invoke('get_storage_info');
    },

    setCustomDataDir: async (path: string): Promise<StorageInfo> => {
        return await invoke('set_custom_data_dir', { path });
    },

    clearCustomDataDir: async (): Promise<StorageInfo> => {
        return await invoke('clear_custom_data_dir');
    },

    dismissStorageMigrationNotice: async (): Promise<StorageInfo> => {
        return await invoke('dismiss_storage_migration_notice');
    },

    dismissStorageCustomDirNotice: async (): Promise<StorageInfo> => {
        return await invoke('dismiss_storage_custom_dir_notice');
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
    },

    v3InspectProjectLayout: async (projectPath: string): Promise<V3ProjectLayoutStatus> => {
        return await invoke('v3_inspect_project_layout', { projectPath });
    },

    v3InitializeProject: async (projectPath: string): Promise<V3BootstrapResult> => {
        return await invoke('v3_initialize_project', { projectPath });
    },

    v3MigrateProject: async (projectPath: string): Promise<V3BootstrapResult> => {
        return await invoke('v3_migrate_project', { projectPath });
    },

    v3RecoverProjectMigration: async (projectPath: string): Promise<V3BootstrapResult> => {
        return await invoke('v3_recover_project_migration', { projectPath });
    },

    v3InspectProjectRepairCandidates: async (projectPath: string): Promise<V3RepairCandidate[]> => {
        return await invoke('v3_inspect_project_repair_candidates', { projectPath });
    },

    v3RepairProject: async (projectPath: string, taskId: string): Promise<V3RepairResult> => {
        return await invoke('v3_repair_project', { projectPath, taskId });
    },

    v3CreateTask: async (
        projectPath: string,
        request: V3TaskCreateRequest,
    ): Promise<V3TaskCreateResult> => {
        return await invoke('v3_create_task', { projectPath, request });
    },

    v3GetProjectSettings: async (projectPath: string): Promise<V3ProjectSettingsInspection> => {
        return await invoke('v3_get_project_settings', { projectPath });
    },

    v3UpdateProjectSettings: async (
        projectPath: string,
        request: V3ProjectSettingsUpdateRequest,
    ): Promise<V3ProjectSettings> => {
        return await invoke('v3_update_project_settings', { projectPath, request });
    },

    v3AgentSpecsStatus: async (projectPath: string): Promise<V3AgentSpecInspection> => {
        return await invoke('v3_agent_specs_status', { projectPath });
    },

    v3AgentSpecsSync: async (
        projectPath: string,
        request: V3AgentSpecSyncRequest,
    ): Promise<V3AgentSpecSyncResult> => {
        return await invoke('v3_agent_specs_sync', { projectPath, request });
    },

    v3PlanAddNode: async (projectPath: string, command: V3PlanAddNodeCommand): Promise<V3AppendResult> => {
        return await invoke('v3_plan_add_node', { projectPath, command });
    },

    v3PlanSetDependencies: async (projectPath: string, command: V3PlanSetDependenciesCommand): Promise<V3AppendResult> => {
        return await invoke('v3_plan_set_dependencies', { projectPath, command });
    },

    v3PlanSetState: async (projectPath: string, command: V3PlanSetStateCommand): Promise<V3AppendResult> => {
        return await invoke('v3_plan_set_state', { projectPath, command });
    },

    vibehubReadLocalAgentUsage: async (projectPath: string, taskId: string | null = null): Promise<LocalAgentUsageOverview> => {
        return await invoke('vibehub_read_local_agent_usage', { projectPath, taskId });
    },

    vibehubOpenVibehubFile: async (
        projectPath: string,
        relativePath: string,
    ): Promise<void> => {
        return await invoke('vibehub_open_vibehub_file', { projectPath, relativePath });
    },

    vibehubRevealProjectFile: async (
        projectPath: string,
        taskId: string,
        relativePath: string,
    ): Promise<void> => {
        return await invoke('vibehub_reveal_project_file', { projectPath, taskId, relativePath });
    },

    vibehubOpenProjectFile: async (
        projectPath: string,
        taskId: string,
        relativePath: string,
    ): Promise<void> => {
        return await invoke('vibehub_open_project_file', { projectPath, taskId, relativePath });
    },
};
