use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub project_type: ProjectType,
    pub tags: Vec<String>,
    pub last_opened: Option<DateTime<Utc>>,
    pub starred: bool,
    pub icon: Option<String>,
    pub metadata: ProjectMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub git_branch: Option<String>,
    pub git_has_changes: bool,
    pub dependencies_installed: bool,
    pub language_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Node,
    Rust,
    Python,
    Java,
    Go,
    Dotnet,
    Ruby,
    Php,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub auto_scan: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: String,
    pub category: TagCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagCategory {
    Workspace,
    Ide,
    Cli,
    Env,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub id: String,
    pub name: String,
    pub tool_type: ToolType,
    pub executable_path: String,
    pub arguments: Vec<String>,
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Vscode,
    Idea,
    AntiGravity,
    ClaudeCode,
    GeminiCli,
    Terminal,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub workspaces: Vec<Workspace>,
    pub tags: Vec<Tag>,
    pub launch_configs: Vec<LaunchConfig>,
    pub projects: Vec<Project>,
    pub theme: Theme,
    pub recent_projects: Vec<String>, // Project IDs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            tags: Self::default_tags(),
            launch_configs: Vec::new(),
            projects: Vec::new(),
            theme: Theme::Light,
            recent_projects: Vec::new(),
        }
    }
}

impl AppConfig {
    fn default_tags() -> Vec<Tag> {
        vec![
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Frontend".to_string(),
                color: "#2EAADC".to_string(),
                category: TagCategory::Custom,
            },
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Backend".to_string(),
                color: "#448361".to_string(),
                category: TagCategory::Custom,
            },
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Fullstack".to_string(),
                color: "#D44C47".to_string(),
                category: TagCategory::Custom,
            },
        ]
    }
}
