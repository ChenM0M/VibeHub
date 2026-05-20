import { invoke } from '@tauri-apps/api/core';
import {
    AgentAdapterConfig,
    AgentAdapterConfigPatch,
    AgentAdapterStatus,
    AgentAdapterSyncResult,
    AgentTool,
    AppConfig,
    ContextPackBuildResult,
    PhaseAdvanceResult,
    PhaseSetResult,
    PhaseValidationResult,
    ResearchPackArchiveResult,
    ResearchPackBuildResult,
    Project,
    Tag,
    VibehubCockpitOverview,
    VibehubFileReadResult,
    VibehubJournalAppendResult,
    VibehubKnowledgeAppendResult,
    VibehubStartTaskResult,
    VibehubStateMigrationReport,
    VibehubSyncReport,
    Workspace,
    WorkspaceDriftReport,
} from '../types';

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

    launchCustom: async (projectId: string, config: any, category?: string): Promise<void> => {
        return await invoke('launch_custom', { projectId, config, category: category || null });
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
    },

    vibehubInit: async (
        projectPath: string,
        agentTools?: AgentTool[],
        syncAdapters: boolean = false,
    ): Promise<{
        project_root: string;
        vibehub_root: string;
        created_files: string[];
        skipped_existing_files: string[];
        errors: string[];
    }> => {
        // Per spec §18.5 manual_by_default: do NOT pass `sync_adapters: true`
        // unless the user explicitly opts in (first-run prompt or the
        // "Sync Adapters" button in the AI Instructions panel).
        return await invoke('vibehub_init', {
            projectPath,
            options:
                agentTools || syncAdapters
                    ? { agent_tools: agentTools ?? null, sync_adapters: syncAdapters }
                    : null,
        });
    },

    vibehubStartTask: async (
        projectPath: string,
        title?: string,
        mode?: string,
        phase?: string
    ): Promise<VibehubStartTaskResult> => {
        return await invoke('vibehub_start_task', { projectPath, title, mode, phase });
    },

    vibehubGenerateAgentView: async (projectPath: string): Promise<{
        current_path: string;
        current_context_path: string;
        handoff_path: string;
        handoff_created: boolean;
        handoff_complete: boolean;
        missing_handoff_sections: string[];
        task_id: string;
        run_id: string;
        phase: string;
    }> => {
        return await invoke('vibehub_generate_agent_view', { projectPath });
    },

    vibehubSyncAgentAdapter: async (
        projectPath: string,
        dryRun: boolean = false
    ): Promise<AgentAdapterSyncResult> => {
        return await invoke('vibehub_sync_agent_adapter', { projectPath, dryRun });
    },

    vibehubGetAgentAdapterStatus: async (projectPath: string): Promise<AgentAdapterStatus> => {
        return await invoke('vibehub_get_agent_adapter_status', { projectPath });
    },

    vibehubUpdateAgentAdapterConfig: async (
        projectPath: string,
        patch: AgentAdapterConfigPatch
    ): Promise<AgentAdapterConfig> => {
        return await invoke('vibehub_update_agent_adapter_config', { projectPath, patch });
    },

    vibehubSyncAgentAdapters: async (
        projectPath: string,
        tools?: AgentTool[],
        dryRun: boolean = false
    ): Promise<AgentAdapterSyncResult> => {
        return await invoke('vibehub_sync_agent_adapters', {
            projectPath,
            tools: tools || null,
            dryRun,
        });
    },

    vibehubCheckWorkspaceDrift: async (
        projectPath: string,
        locale?: string
    ): Promise<WorkspaceDriftReport> => {
        return await invoke('vibehub_check_workspace_drift', { projectPath, locale: locale || null });
    },

    vibehubSyncWorkspaceState: async (
        projectPath: string,
        locale?: string
    ): Promise<WorkspaceDriftReport> => {
        return await invoke('vibehub_sync_workspace_state', { projectPath, locale: locale || null });
    },

    vibehubSyncWorkspace: async (
        projectPath: string,
        locale?: string
    ): Promise<VibehubSyncReport> => {
        return await invoke('vibehub_sync_workspace', { projectPath, locale: locale || null });
    },

    vibehubBuildContextPack: async (
        projectPath: string,
        taskId: string,
        runId: string,
        phase: string
    ): Promise<ContextPackBuildResult> => {
        return await invoke('vibehub_build_context_pack', { projectPath, taskId, runId, phase });
    },

    vibehubBuildHandoff: async (projectPath: string): Promise<{
        handoff_path: string;
        source_output_path: string | null;
        complete: boolean;
        missing_required_sections: string[];
        files_changed_evidence: string;
        task_id: string;
        run_id: string;
        session_id: string | null;
    }> => {
        return await invoke('vibehub_build_handoff', { projectPath });
    },

    vibehubGenerateReviewEvidence: async (projectPath: string): Promise<{
        task_id: string;
        run_id: string;
        review_path: string;
        changed_files_path: string;
        diff_path: string;
        changed_files_count: number;
        baseline_ref: string | null;
        source_output_path: string | null;
    }> => {
        return await invoke('vibehub_generate_review_evidence', { projectPath });
    },

    // Aggregated cockpit overview. Replaces the previous per-tab read
    // methods (`vibehubReadCockpitStatus`, `vibehubReadContextView`,
    // `vibehubReadReviewView`, `vibehubReadHandoffView`,
    // `vibehubReadDiffView`, `vibehubReadResearchStatus`). One IPC round-trip,
    // one cached `git` invocation per call.
    vibehubReadOverview: async (projectPath: string): Promise<VibehubCockpitOverview> => {
        return await invoke('vibehub_read_overview', { projectPath });
    },

    vibehubAppendJournalEntry: async (
        projectPath: string,
        title?: string,
        body?: string
    ): Promise<VibehubJournalAppendResult> => {
        return await invoke('vibehub_append_journal_entry', { projectPath, title, body });
    },

    vibehubAppendKnowledgeNote: async (
        projectPath: string,
        note?: string
    ): Promise<VibehubKnowledgeAppendResult> => {
        return await invoke('vibehub_append_knowledge_note', { projectPath, note });
    },

    vibehubValidatePhase: async (projectPath: string): Promise<PhaseValidationResult> => {
        return await invoke('vibehub_validate_phase', { projectPath });
    },

    vibehubSetPhaseResult: async (
        projectPath: string,
        targetPhase: string,
        status: string
    ): Promise<PhaseSetResult> => {
        return await invoke('vibehub_set_phase_result', { projectPath, targetPhase, status });
    },

    vibehubCompletePhase: async (projectPath: string): Promise<PhaseAdvanceResult> => {
        return await invoke('vibehub_complete_phase', { projectPath });
    },

    vibehubAdvancePhase: async (projectPath: string): Promise<PhaseAdvanceResult> => {
        return await invoke('vibehub_advance_phase', { projectPath });
    },

    vibehubPausePhase: async (projectPath: string): Promise<PhaseSetResult> => {
        return await invoke('vibehub_pause_phase', { projectPath });
    },

    vibehubBuildResearchPack: async (
        projectPath: string,
        title?: string
    ): Promise<ResearchPackBuildResult> => {
        return await invoke('vibehub_build_research_pack', { projectPath, title });
    },

    vibehubArchiveResearch: async (
        projectPath: string
    ): Promise<ResearchPackArchiveResult | null> => {
        return await invoke('vibehub_archive_research', { projectPath });
    },

    vibehubReadVibehubFile: async (
        projectPath: string,
        relativePath: string,
    ): Promise<VibehubFileReadResult> => {
        return await invoke('vibehub_read_vibehub_file', { projectPath, relativePath });
    },

    // Schema-version migration. `dryRun` reports the diff WITHOUT writing.
    vibehubDryRunStateMigration: async (
        projectPath: string,
    ): Promise<VibehubStateMigrationReport> => {
        return await invoke('vibehub_dry_run_state_migration', { projectPath });
    },

    vibehubMigrateState: async (projectPath: string): Promise<VibehubStateMigrationReport> => {
        return await invoke('vibehub_migrate_state', { projectPath });
    },

    vibehubSetProjectLocale: async (projectPath: string, locale: string): Promise<string> => {
        return await invoke('vibehub_set_project_locale', { projectPath, locale });
    },
};
