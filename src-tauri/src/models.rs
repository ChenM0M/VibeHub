use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub workspaces: Vec<Workspace>,
    pub tags: Vec<Tag>,
    pub projects: Vec<Project>,
    pub theme: String,
    pub recent_projects: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            tags: Tag::default_tags(),
            projects: Vec::new(),
            theme: "auto".to_string(),
            recent_projects: Vec::new(),
        }
    }
}

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
    pub cover_image: Option<String>,
    pub theme_color: Option<String>,
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
    Other,
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
    pub config: Option<TagConfig>,
}

impl Tag {
    pub fn default_tags() -> Vec<Tag> {
        vec![
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Frontend".to_string(),
                color: "#2EAADC".to_string(),
                category: TagCategory::Custom,
                config: None,
            },
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Backend".to_string(),
                color: "#448361".to_string(),
                category: TagCategory::Custom,
                config: None,
            },
            Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Fullstack".to_string(),
                color: "#D44C47".to_string(),
                category: TagCategory::Custom,
                config: None,
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagConfig {
    pub executable: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TagCategory {
    Workspace,
    Ide,
    Cli,
    Environment,
    Startup,
    Custom,
}
