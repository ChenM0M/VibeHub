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
