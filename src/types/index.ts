export type ProjectType =
    | 'node'
    | 'rust'
    | 'python'
    | 'java'
    | 'go'
    | 'dotnet'
    | 'ruby'
    | 'php'
    | 'unknown';

export type TagCategory = 'workspace' | 'ide' | 'cli' | 'environment' | 'startup' | 'custom';

export interface TagConfig {
    executable?: string;
    args?: string[];
    env?: Record<string, string>;
}

export type ToolType =
    | 'vscode'
    | 'idea'
    | 'antigravity'
    | 'claudecode'
    | 'geminicli'
    | 'terminal'
    | 'custom';

export type Theme = 'light' | 'dark' | 'auto';

export interface ProjectMetadata {
    git_branch?: string;
    git_has_changes: boolean;
    dependencies_installed: boolean;
    language_version?: string;
}

export interface Project {
    id: string;
    name: string;
    description?: string;
    path: string;
    project_type: ProjectType;
    tags: string[];
    last_opened?: string; // DateTime<Utc> comes as string
    starred: boolean;
    icon?: string;
    cover_image?: string;
    theme_color?: string;
    tech_stack: string[];
    metadata: ProjectMetadata;
}

export interface Workspace {
    id: string;
    name: string;
    path: string;
    auto_scan: boolean;
    created_at: string;
}

export interface Tag {
    id: string;
    name: string;
    color: string;
    category: TagCategory;
    config?: TagConfig;
}

export interface AppConfig {
    workspaces: Workspace[];
    tags: Tag[];
    projects: Project[];
    theme: Theme;
    recent_projects: string[];
}

export interface ContextPackBuildResult {
    task_id: string;
    run_id: string;
    phase: string;
    pack_path: string;
    manifest_path: string;
    included_count: number;
    missing_count: number;
    excluded_count: number;
    estimated_tokens: number;
}

export interface VibehubStartTaskResult {
    task_id: string;
    run_id: string;
    mode: string;
    phase: string;
    phase_status: string;
    task_path: string;
    run_path: string;
    task_pointer_path: string;
    run_pointer_path: string;
    context_spec_path: string;
}

export type AgentTool = 'codex' | 'claude_code' | 'opencode';

export interface AgentCommandSpec {
    name: string;
    description_zh: string;
    description_en: string;
    argument_hint: string;
    body: string;
}

export interface AgentAdapterConflict {
    path: string;
    reason: string;
}

export interface AgentAdapterFileStatus {
    tool: string;
    path: string;
    exists: boolean;
    status: string;
    generated_hash?: string | null;
    current_hash?: string | null;
    description: string;
}

export interface AgentAdapterStatus {
    project_root: string;
    config_path: string;
    enabled_tools: AgentTool[];
    commands: AgentCommandSpec[];
    files: AgentAdapterFileStatus[];
    warnings: string[];
}

export interface AgentAdapterConfig {
    schema_version: number;
    template_version: string;
    enabled_tools: AgentTool[];
    command_overrides: Record<string, string>;
    generated_hashes: Record<string, string>;
}

export interface AgentAdapterConfigPatch {
    enabled_tools?: AgentTool[];
    command_overrides?: Record<string, string>;
}

export interface AgentAdapterSyncResult {
    project_root: string;
    created_files: string[];
    updated_files: string[];
    skipped_files: string[];
    conflict_files: AgentAdapterConflict[];
    dry_run: boolean;
    summary: string;
    files: AgentAdapterFileStatus[];
}

export interface WorkspaceDriftReport {
    project_root: string;
    git_available: boolean;
    head?: string | null;
    last_seen_head?: string | null;
    head_changed: boolean;
    dirty: boolean;
    changed_files: string[];
    context_stale: boolean;
    adapter_conflicts: string[];
    warnings: string[];
    recommended_actions: string[];
    recover_report_path?: string | null;
}

export interface VibehubFileStatus {
    configured: boolean;
    exists: boolean;
    stale?: boolean | null;
    path?: string | null;
    status: string;
}

export interface VibehubCockpitStatus {
    project_root: string;
    initialized: boolean;
    current_task_id?: string | null;
    current_task_title?: string | null;
    current_run_id?: string | null;
    current_mode?: string | null;
    current_phase?: string | null;
    phase_status?: string | null;
    git_available: boolean;
    git_dirty?: boolean | null;
    git_changed_files_count?: number | null;
    context_pack_status: VibehubFileStatus;
    handoff_status: VibehubFileStatus;
    observability_level?: string | null;
    warnings: string[];
}

export interface VibehubJournalAppendResult {
    journal_path: string;
    title: string;
    timestamp: string;
}

export interface VibehubKnowledgeAppendResult {
    knowledge_path: string;
    timestamp: string;
}
