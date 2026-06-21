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

export interface PendingReplayResult {
    pending_dir: string;
    replayed: number;
    skipped: number;
    warnings: string[];
}

export interface DebugDumpOptions {
    include_events?: boolean | null;
    include_packs?: boolean | null;
    redact_secrets?: boolean | null;
}

export interface DebugDumpResult {
    dump_path: string;
    manifest_path: string;
    task_id: string | null;
    run_id: string | null;
    files_copied: number;
    redacted_files: number;
    events_copied: number;
    context_packs_copied: number;
    outputs_copied: number;
    handoffs_copied: number;
    sync_reports_copied: number;
    warnings: string[];
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
    context_pack_path: string;
    context_manifest_path: string;
    context_included_count: number;
    context_missing_count: number;
}

export type VibehubIntakeConfidence = 'high' | 'medium' | 'low';

export interface VibehubTaskDraft {
    title: string;
    intent?: string | null;
    acceptance_criteria?: string[];
    dependencies?: string[];
    suggested_order?: number | null;
    mode?: string | null;
    phase?: string | null;
}

export interface VibehubStartTaskIntakeRequest {
    title?: string | null;
    intent?: string | null;
    mode?: string | null;
    phase?: string | null;
    source_message?: string | null;
    split_confidence?: VibehubIntakeConfidence | null;
    split_reason?: string | null;
    not_split_reason?: string | null;
    intake?: VibehubTaskDraft[];
}

export interface VibehubTaskIntakeItem {
    order: number;
    title: string;
    intent?: string | null;
    acceptance_criteria: string[];
    dependencies: string[];
    task_id?: string | null;
    run_id?: string | null;
    status: string;
}

export interface VibehubTaskIntakeFailure {
    order: number;
    title: string;
    error: string;
}

export interface VibehubStartTaskIntakeResult {
    classification: string;
    confirmation_required: boolean;
    source_message?: string | null;
    split_reason?: string | null;
    not_split_reason?: string | null;
    proposed_tasks: VibehubTaskIntakeItem[];
    created_tasks: VibehubStartTaskResult[];
    failed_tasks: VibehubTaskIntakeFailure[];
    active_tasks: string[];
    current_task_id?: string | null;
    current_run_id?: string | null;
    queue_state_path?: string | null;
    unresolved_risks: string[];
}

export type AgentTool = 'amp_code' | 'codex' | 'claude_code' | 'opencode' | 'cursor' | 'antigravity';

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

export interface VibehubFlowPhaseStatus {
    phase: string;
    status: string;
}

export interface VibehubTaskNeighbor {
    task_id: string;
    title?: string | null;
    active_capabilities: string[];
    shared_files: string[];
}

export interface VibehubActiveTask {
    task_id: string;
    title?: string | null;
    run_id?: string | null;
    mode?: string | null;
    phase?: string | null;
    phase_status?: string | null;
    current: boolean;
    active_capabilities: string[];
    dependencies: string[];
    intake_order?: number | null;
}

export interface CapabilityGateStatus {
    capability: string;
    claimable: boolean;
    active: boolean;
    blocked_by: string[];
    reason: string | null;
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
    agent_output_status: VibehubFileStatus;
    handoff_status: VibehubFileStatus;
    flow: VibehubFlowPhaseStatus[];
    active_tasks: VibehubActiveTask[];
    active_capabilities: string[];
    neighbor_tasks: VibehubTaskNeighbor[];
    claimable_capabilities: string[];
    gate_statuses: CapabilityGateStatus[];
    derivation_trace_path?: string | null;
    observability_level?: string | null;
    locale?: string | null;
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

export interface PhaseValidationResult {
    phase: string;
    status: string;
    required_outputs: string[];
    found_outputs: string[];
    missing_outputs: string[];
    source_output_path?: string | null;
}

export interface PhaseAdvanceResult {
    mode: string;
    previous_phase: string;
    previous_status: string;
    current_phase: string;
    current_status: string;
    next_phase?: string | null;
    validation: PhaseValidationResult;
    handoff_complete: boolean;
    missing_handoff_sections: string[];
    forced: boolean;
}

export interface PhaseSetResult {
    phase: string;
    status: string;
    mode: string;
    current_phase: string;
    current_phase_status: string;
}

export interface ResearchPackBuildResult {
    research_pack_path: string;
    source_log_path: string;
    findings_path: string;
    status: string;
    files_created: string[];
    files_skipped: string[];
}

export interface ResearchPackArchiveResult {
    archived_to: string;
    archive_path: string;
    status: string;
}

export interface ResearchStatus {
    required: boolean;
    status: string;
    research_pack_exists: boolean;
}

export interface VibehubFileReadResult {
    path: string;
    content: string;
    exists: boolean;
    size: number;
}

export interface VibehubContextViewData {
    pack_path: string | null;
    manifest_path: string | null;
    pack_exists: boolean;
    manifest_exists: boolean;
    included_count: number;
    missing_count: number;
    excluded_count: number;
    stale: boolean | null;
    phase: string | null;
}

export interface VibehubReviewViewData {
    review_path: string | null;
    review_exists: boolean;
    review_summary: string;
    evidence_map_summary: string;
    evidence_grades: string[];
    diff_patch_path: string | null;
    diff_patch_exists: boolean;
    changed_files_path: string | null;
    changed_files_count: number;
}

export interface VibehubHandoffViewData {
    handoff_path: string | null;
    handoff_exists: boolean;
    complete: boolean;
    missing_sections: string[];
    sections_count: number;
}

export interface VibehubDiffViewData {
    dirty: boolean;
    changed_files: string[];
    changed_files_count: number;
    diff_stat: string[];
}

export interface VibehubSyncReport {
    project_root: string;
    status: string;
    task_id?: string | null;
    run_id?: string | null;
    phase?: string | null;
    report_path?: string | null;
    agent_view_sync_path: string;
    agent_view_status: string;
    context_status: string;
    adapter_status: string;
    drift_warnings: string[];
    changed_files: string[];
    phase_validation_status?: string | null;
    missing_phase_outputs: string[];
    questions_for_user: string[];
    recommended_actions: string[];
}

// Agent-written project-level digest sourced from `.vibehub/notes/`.
// VibeHub seeds the files at init and NEVER overwrites them; both `*_line`
// fields are `null` until an agent fills them in via the adapter commands.
export interface VibehubProjectDigest {
    summary_line: string | null;
    status_line: string | null;
    summary_path: string;
    status_path: string;
    summary_exists: boolean;
    status_exists: boolean;
}

export interface VibehubGitBranchInfo {
    name: string;
    is_current: boolean;
    upstream: string | null;
    ahead: number | null;
    behind: number | null;
    head_sha: string;
    last_commit_subject: string;
    /// ISO-8601 committer date string emitted directly by `git for-each-ref`.
    last_commit_time: string;
}

export interface VibehubGitBranchesView {
    git_available: boolean;
    current_branch: string | null;
    branches: VibehubGitBranchInfo[];
    detached_head: boolean;
}

export interface VibehubFlowArtifact {
    label: string;
    path: string;
    exists: boolean;
}

export interface VibehubFlowDetail {
    phase: string;
    status: string;
    context_spec_path: string;
    context_spec_exists: boolean;
    context_pack_path: string;
    context_pack_exists: boolean;
    manifest_path: string;
    manifest_exists: boolean;
    phase_output_path: string;
    phase_output_exists: boolean;
    review_path: string | null;
    review_exists: boolean;
    read_inputs: VibehubFlowArtifact[];
    written_outputs: VibehubFlowArtifact[];
}

export interface VibehubEventTimelineItem {
    event_id: string;
    timestamp: string;
    task_id: string;
    task_title?: string | null;
    run_id: string;
    event_type: string;
    capability?: string | null;
    summary: string;
    event_log_path: string;
    raw: unknown;
}

export interface VibehubArchiveArtifact {
    label: string;
    path: string;
    exists: boolean;
}

export interface VibehubArchivedTaskEvent {
    event_id?: string | null;
    timestamp?: string | null;
    event_type: string;
    summary: string;
    artifact_path: string;
}

export interface VibehubArchivedProcessStep {
    name: string;
    status: string;
    event_count: number;
}

export interface VibehubArchivedTaskCard {
    task_id: string;
    title?: string | null;
    label: string;
    summary: string;
    status: string;
    status_reason?: string | null;
    task_path: string;
    run_id?: string | null;
    run_path?: string | null;
    mode?: string | null;
    phase?: string | null;
    phase_status?: string | null;
    updated_at?: string | null;
    event_count: number;
    events: VibehubArchivedTaskEvent[];
    process: VibehubArchivedProcessStep[];
    handoff_artifacts: VibehubArchiveArtifact[];
    output_artifacts: VibehubArchiveArtifact[];
    event_artifacts: VibehubArchiveArtifact[];
}

export interface VibehubArchiveViewData {
    cards: VibehubArchivedTaskCard[];
    warnings: string[];
}

export type VibehubProjectStructureKind = 'root' | 'directory' | 'file';

export interface VibehubProjectStructureGraphNode {
    id: string;
    label: string;
    path: string;
    kind: VibehubProjectStructureKind;
    depth: number;
    changed: boolean;
    file_count: number;
    directory_count: number;
}

export interface VibehubProjectStructureGraphEdge {
    from: string;
    to: string;
    kind: string;
}

export interface VibehubProjectStructureTreeNode {
    id: string;
    label: string;
    path: string;
    kind: VibehubProjectStructureKind;
    depth: number;
    changed: boolean;
    file_count: number;
    directory_count: number;
    children: VibehubProjectStructureTreeNode[];
    truncated: boolean;
}

export interface VibehubProjectStructureViewData {
    root_path: string;
    source: string;
    semantic_graph_available: boolean;
    graph_nodes: VibehubProjectStructureGraphNode[];
    graph_edges: VibehubProjectStructureGraphEdge[];
    tree: VibehubProjectStructureTreeNode[];
    scanned_files_count: number;
    scanned_dirs_count: number;
    truncated: boolean;
    warnings: string[];
}

export type VibehubPromptTemplateId =
    | 'new-task'
    | 'sync'
    | 'claim-capability'
    | 'release-capability'
    | 'cancel-task'
    | 'force-rebuild'
    | 'fix-schema';

export interface VibehubPromptTemplateOption {
    id: VibehubPromptTemplateId;
    filename: string;
    dangerous: boolean;
}

export interface VibehubPromptRenderResult {
    template_id: VibehubPromptTemplateId;
    template_path: string;
    source: string;
    locale: string;
    dangerous: boolean;
    confirmation_required: boolean;
    confirmation_message?: string | null;
    content: string;
    warnings: string[];
}

// Aggregated cockpit overview (replaces the 6 per-tab read commands).
export interface VibehubCockpitOverview {
    status: VibehubCockpitStatus;
    context: VibehubContextViewData | null;
    review: VibehubReviewViewData | null;
    handoff: VibehubHandoffViewData | null;
    diff: VibehubDiffViewData;
    research: ResearchStatus;
    project_digest: VibehubProjectDigest;
    git_branches: VibehubGitBranchesView;
    flow_details: VibehubFlowDetail[];
    event_timeline: VibehubEventTimelineItem[];
    archive: VibehubArchiveViewData;
    project_structure: VibehubProjectStructureViewData;
    initialized: boolean;
}

export interface LocalAgentUsageOverview {
    project_path: string;
    generated_at: string;
    primary_metric: AgentUsagePrimaryMetric;
    non_cached_total_tokens: number;
    total_tokens: number;
    source_count: number;
    codex: AgentUsageSourceSummary;
    opencode: AgentUsageSourceSummary;
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
    agent?: string | null;
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

// Forward migration of `.vibehub/state.yaml` from r9 schema 1 to r10 schema 2.
export interface VibehubStateMigrationReport {
    state_path: string;
    previous_schema_version: number | null;
    current_schema_version: number;
    migrated: boolean;
    backup_path: string | null;
    added_keys: string[];
    notes: string[];
}
