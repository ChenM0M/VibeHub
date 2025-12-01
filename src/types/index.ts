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
