export type V3ProjectLayoutState = 'absent' | 'v2' | 'v3' | 'migration_interrupted' | 'conflict';

export interface V3ProjectLayoutStatus {
    state: V3ProjectLayoutState;
    schema_version: number | null;
    message: string;
    recommended_action: string | null;
}

export interface V3BootstrapResult {
    status: string;
    archived_legacy_v2: boolean;
    created_paths: string[];
}

export interface V3RepairCandidate {
    task_id: string;
    title: string;
    state: string;
}

export interface V3RepairResult {
    status: string;
    task_id: string;
    created_paths: string[];
}

export type {
    V3AgentSpecArtifactInspection,
    V3AgentSpecArtifactStatus,
    V3AgentSpecInspection,
    V3AgentSpecSyncRequest,
    V3AgentSpecSyncResult,
    V3AgentSpecTarget,
    V3OutputLanguage,
    V3ProjectSettings,
    V3ProjectSettingsInspection,
    V3ProjectSettingsUpdateRequest,
    V3TaskCreateRequest,
    V3TaskCreateResult,
} from '../v3/contracts/generated';

export type V3PlanAddNodeCommand = Omit<
    import('../v3/contracts/generated').PlanNodeAdd,
    'command'
>;
export type V3PlanSetDependenciesCommand = Omit<
    import('../v3/contracts/generated').PlanDependenciesSet,
    'command'
>;
export type V3PlanSetStateCommand = Omit<
    import('../v3/contracts/generated').PlanNodeStateSet,
    'command'
>;
export type V3AppendResult =
    | { status: 'appended'; event: Record<string, unknown> }
    | { status: 'duplicate'; event: Record<string, unknown> };

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
    terminal?: string;
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

export interface SettingsImportAdjustment {
    scope: string;
    item_id?: string | null;
    item_name?: string | null;
    field: string;
    before?: string | null;
    after?: string | null;
    reason: string;
}

export interface SettingsImportResult {
    source_system: string;
    target_system: string;
    tags_added: number;
    tags_updated: number;
    gateway_providers: number;
    adjustments: SettingsImportAdjustment[];
}

export interface LocalAgentUsageOverview {
    project_path: string;
    scope: 'project' | 'task' | string;
    task_id?: string | null;
    requested_session_count: number;
    matched_session_count: number;
    generated_at: string;
    freshness: 'fresh' | 'stale' | 'unknown' | string;
    stale_after_seconds: number;
    completeness: 'complete' | 'partial' | 'empty' | 'unavailable' | string;
    primary_metric: AgentUsagePrimaryMetric;
    non_cached_total_tokens: number;
    total_tokens: number;
    source_count: number;
    claude_code: AgentUsageSourceSummary;
    claude_app: AgentUsageSourceSummary;
    codex: AgentUsageSourceSummary;
    opencode: AgentUsageSourceSummary;
    cursor: AgentUsageSourceSummary;
    warnings: string[];
}

export type AgentUsagePrimaryMetricKind = 'tokens' | 'cost' | 'quota' | 'token_fallback' | 'unavailable' | string;

export interface AgentUsagePrimaryMetric {
    kind: AgentUsagePrimaryMetricKind;
    label: string;
    value?: number | null;
    currency?: string | null;
    tokens?: number | null;
    source: string;
    confidence: string;
    estimated: boolean;
    detail: string;
}

export interface AgentUsageSourceSummary {
    source: string;
    status: 'available' | 'partial' | 'stale' | 'empty' | 'unsupported' | 'permission_denied' | 'error' | string;
    freshness: 'fresh' | 'stale' | 'unknown' | string;
    available: boolean;
    data_path?: string | null;
    records: number;
    non_cached_total_tokens: number;
    total_tokens: number;
    cost?: number | null;
    tokens: AgentUsageTokenBreakdown;
    latest_updated_at_ms?: number | null;
    recent: AgentUsageRecentItem[];
    warnings: string[];
}

export interface AgentUsageTokenBreakdown {
    input: number;
    output: number;
    reasoning: number;
    cached_input: number;
    cache_read: number;
    cache_write: number;
    total: number;
}

export interface AgentUsageRecentItem {
    id: string;
    title: string;
    model?: string | null;
    models: string[];
    agent?: string | null;
    message_count: number;
    tool_uses: number;
    started_at_ms?: number | null;
    duration_seconds?: number | null;
    status: string;
    non_cached_total_tokens: number;
    total_tokens: number;
    cost?: number | null;
    updated_at_ms?: number | null;
}

// Tells the UI why the active data dir was picked. Mirrors `StorageSource`
// in `src-tauri/src/app_paths.rs` (serde renames to lowercase).
export type StorageSource = 'env' | 'custom' | 'portable' | 'default';

export interface StorageInfo {
    active_dir: string;
    source: StorageSource;
    custom_dir?: string | null;
    default_dir: string;
    portable_dir?: string | null;
    portable_available: boolean;
    dual_data_detected: boolean;
    migration_notice_dismissed: boolean;
    custom_dir_notice_dismissed: boolean;
}
