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
    AgentKind,
    AgentProfileDiagnosticsResult,
    AgentProfileDiscoverResult,
    AgentProfileDocument,
    AgentProfileReadResult,
    AgentProfileSaveResult,
    BindingSource,
    RuntimeTarget,
    RouteRequest,
    SessionTaskBinding,
    TaskRouteCandidate,
    TaskRouteDecision,
    V3AgentSpecInspection,
    V3AgentSpecSyncRequest,
    V3AgentSpecSyncResult,
    V3ProjectSettings,
    V3ProjectSettingsInspection,
    V3ProjectSettingsUpdateRequest,
    V3TaskCreateRequest,
    V3TaskCreateResult,
    WorkspaceStateReadResult,
    WorkspaceStateSaveRequest,
    WorkspaceStateSaveResult,
} from '../v3/contracts/generated';
import type { V3AppendResult, V3PlanAddNodeCommand, V3PlanSetDependenciesCommand, V3PlanSetStateCommand } from '../types';

export interface V3AgentProfileTargetRequest {
    agent: AgentKind;
    runtime_target_id: string;
}

export interface V3AgentProfileReadRequest extends V3AgentProfileTargetRequest {
    profile_id: string;
}

export interface V3AgentProfileSaveRequest extends V3AgentProfileReadRequest {
    expected_revision: number;
    profile: AgentProfileDocument;
}

export interface V3AgentProfileValidateRequest extends V3AgentProfileTargetRequest {
    profile: AgentProfileDocument;
}

export interface V3AgentProfileActivateRequest extends V3AgentProfileReadRequest {
    expected_revision: number;
}

export interface V3AgentProfileLaunchRequest extends V3AgentProfileReadRequest {
    launch_mode: 'temporary' | 'default';
}

export interface V3AgentProfileRestoreRequest extends V3AgentProfileReadRequest {
    expected_revision: number;
    backup_path: string;
}

export interface V3AgentProfileDiagnosticsRequest extends V3AgentProfileTargetRequest {
    profile_id?: string;
}

export interface V3AgentProfileCreateRequest extends V3AgentProfileTargetRequest {
    profile_name: string;
    template_profile_id?: string;
}

export interface V3AgentProfileCloneRequest extends V3AgentProfileTargetRequest {
    source_profile_id: string;
    profile_name: string;
}

export interface V3AgentProfileRenameRequest extends V3AgentProfileReadRequest {
    new_profile_name: string;
}

export interface V3AgentProfileDeleteRequest extends V3AgentProfileReadRequest {
    replacement_profile_id?: string;
}

export interface V3AgentProfileListModelsRequest extends V3AgentProfileReadRequest {
    provider_id: string;
    base_url: string;
    protocol?: string;
    api_key?: string;
}

export interface V3AgentProfileListModelsResult {
    endpoint: string;
    models: Array<{
        model_id: string;
        display_name: string;
    }>;
}

export interface V3SessionTaskBindRequest {
    project_id: string;
    task_id: string;
    session_id: string;
    interaction_id: string;
    actor: string;
    source: BindingSource;
    expected_version: number;
    idempotency_key: string;
    expected_binding_revision?: number | null;
    agent_id?: string | null;
    host?: string | null;
    provider_session_id?: string | null;
}

export interface V3SessionTaskUnbindRequest {
    project_id: string;
    task_id: string;
    session_id: string;
    actor: string;
    expected_version: number;
    idempotency_key: string;
    expected_binding_revision?: number | null;
}

export const tauriApi = {
    loadConfig: async (): Promise<AppConfig> => {
        return await invoke('load_config');
    },

    saveConfig: async (config: AppConfig): Promise<void> => {
        return await invoke('save_config', { config });
    },

    loadWorkspaceState: async (): Promise<WorkspaceStateReadResult> => {
        return await invoke('load_workspace_state');
    },

    saveWorkspaceState: async (command: WorkspaceStateSaveRequest): Promise<WorkspaceStateSaveResult> => {
        return await invoke('save_workspace_state', { command });
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

    v3TaskCandidates: async (projectPath: string, expectedProjectId?: string): Promise<TaskRouteCandidate[]> => {
        return await invoke('v3_task_candidates', { projectPath, expectedProjectId });
    },

    v3TaskRoute: async (projectPath: string, request: RouteRequest): Promise<TaskRouteDecision> => {
        return await invoke('v3_task_route', { projectPath, request });
    },

    v3SessionTaskBinding: async (projectPath: string, projectId: string, sessionId: string): Promise<SessionTaskBinding> => {
        return await invoke('v3_session_task_binding', { projectPath, projectId, sessionId });
    },

    v3SessionTaskBind: async (projectPath: string, request: V3SessionTaskBindRequest): Promise<V3AppendResult> => {
        return await invoke('v3_session_task_bind', { projectPath, request });
    },

    v3SessionTaskUnbind: async (projectPath: string, request: V3SessionTaskUnbindRequest): Promise<V3AppendResult> => {
        return await invoke('v3_session_task_unbind', { projectPath, request });
    },

    v3CompleteTask: async (projectPath: string, request: { project_id: string; task_id: string; actor: string; confirmed_by: string; channel: string; idempotency_key: string }): Promise<V3AppendResult> => {
        return await invoke('v3_complete_task', { projectPath, request });
    },

    v3CloseTaskWithExceptions: async (projectPath: string, request: { project_id: string; task_id: string; actor: string; confirmed_by: string; channel: string; reason: string; idempotency_key: string }): Promise<V3AppendResult> => {
        return await invoke('v3_close_task_with_exceptions', { projectPath, request });
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

    v3PlanSetCriteria: async (projectPath: string, command: Record<string, unknown>): Promise<V3AppendResult> => invoke('v3_plan_set_criteria', { projectPath, command }),
    v3LifecycleTypedCommand: async (projectPath: string, command: Record<string, unknown>): Promise<V3AppendResult> => invoke('v3_lifecycle_typed_command', { projectPath, command }),
    v3MemoryCommand: async (projectPath: string, command: Record<string, unknown>): Promise<V3AppendResult> => invoke('v3_memory_command', { projectPath, command }),
    v3MemoryQuery: async (projectPath: string, projectId: string, query: Record<string, unknown>): Promise<Array<Record<string, unknown>>> => invoke('v3_memory_query', { projectPath, projectId, query }),
    v3OrchestrationCommand: async (projectPath: string, command: Record<string, unknown>): Promise<V3AppendResult> => invoke('v3_orchestration_command', { projectPath, command }),
    v3LoadViewBundle: async (projectPath: string, taskId?: string, nodeId?: string, expectedProjectId?: string): Promise<Record<string, unknown>> => invoke('v3_load_view_bundle', { projectPath, taskId, nodeId, expectedProjectId }),

    v3AgentProfileDiscover: async (request: V3AgentProfileTargetRequest): Promise<AgentProfileDiscoverResult> => {
        return await invoke('v3_agent_profile_discover', { request });
    },

    v3AgentProfileRuntimeTargets: async (): Promise<RuntimeTarget[]> => {
        return await invoke('v3_agent_profile_runtime_targets');
    },

    v3AgentProfileRead: async (request: V3AgentProfileReadRequest): Promise<AgentProfileReadResult> => {
        return await invoke('v3_agent_profile_read', { request });
    },

    v3AgentProfileSave: async (request: V3AgentProfileSaveRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_save', { request });
    },

    v3AgentProfileValidate: async (request: V3AgentProfileValidateRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_validate', { request });
    },

    v3AgentProfileActivate: async (request: V3AgentProfileActivateRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_activate', { request });
    },

    v3AgentProfileLaunch: async (request: V3AgentProfileLaunchRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_launch', { request });
    },

    v3AgentProfileRestore: async (request: V3AgentProfileRestoreRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_restore', { request });
    },

    v3AgentProfileDiagnostics: async (request: V3AgentProfileDiagnosticsRequest): Promise<AgentProfileDiagnosticsResult> => {
        return await invoke('v3_agent_profile_diagnostics', { request });
    },

    v3AgentProfileCreate: async (request: V3AgentProfileCreateRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_create', { request });
    },

    v3AgentProfileClone: async (request: V3AgentProfileCloneRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_clone', { request });
    },

    v3AgentProfileRename: async (request: V3AgentProfileRenameRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_rename', { request });
    },

    v3AgentProfileDelete: async (request: V3AgentProfileDeleteRequest): Promise<AgentProfileSaveResult> => {
        return await invoke('v3_agent_profile_delete', { request });
    },

    v3AgentProfileListUpstreamModels: async (request: V3AgentProfileListModelsRequest): Promise<V3AgentProfileListModelsResult> => {
        return await invoke('v3_agent_profile_list_upstream_models', { request });
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
