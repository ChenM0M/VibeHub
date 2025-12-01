use crate::models::{Project, ProjectMetadata, ProjectType};
use anyhow::Result;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct Scanner;

impl Scanner {
    pub fn scan_directory(path: &str, max_depth: usize) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        
        for entry in WalkDir::new(path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                if let Some(project) = Self::detect_project(entry.path()) {
                    projects.push(project);
                }
            }
        }
        
        Ok(projects)
    }

    fn detect_project(path: &Path) -> Option<Project> {
        let project_type = Self::detect_project_type(path)?;
        
        let name = path
            .file_name()?
            .to_string_lossy()
            .to_string();
        
        let metadata = Self::extract_metadata(path, &project_type);
        
        Some(Project {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: Self::extract_description(path, &project_type),
            path: path.to_string_lossy().to_string(),
            project_type,
            tags: Vec::new(),
            last_opened: None,
            starred: false,
            icon: None,
            cover_image: None,
            theme_color: None,
            metadata,
        })
    }

    fn detect_project_type(path: &Path) -> Option<ProjectType> {
        // Node.js
        if path.join("package.json").exists() {
            return Some(ProjectType::Node);
        }
        
        // Rust
        if path.join("Cargo.toml").exists() {
            return Some(ProjectType::Rust);
        }
        
        // Python
        if path.join("requirements.txt").exists() 
            || path.join("pyproject.toml").exists()
            || path.join("setup.py").exists() {
            return Some(ProjectType::Python);
        }
        
        // Java
        if path.join("pom.xml").exists() 
            || path.join("build.gradle").exists()
            || path.join("build.gradle.kts").exists() {
            return Some(ProjectType::Java);
        }
        
        // Go
        if path.join("go.mod").exists() {
            return Some(ProjectType::Go);
        }
        
        // .NET
        if path.read_dir().ok()?.any(|e| {
            e.ok()
                .and_then(|e| e.path().extension().map(|ext| ext == "csproj" || ext == "fsproj" || ext == "vbproj"))
                .unwrap_or(false)
        }) {
            return Some(ProjectType::Dotnet);
        }
        
        // Ruby
        if path.join("Gemfile").exists() {
            return Some(ProjectType::Ruby);
        }
        
        // PHP
        if path.join("composer.json").exists() {
            return Some(ProjectType::Php);
        }
        
        None
    }

    fn extract_description(path: &Path, project_type: &ProjectType) -> Option<String> {
        match project_type {
            ProjectType::Node => {
                let package_json = path.join("package.json");
                if let Ok(content) = fs::read_to_string(package_json) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        return json.get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                    }
                }
            }
            ProjectType::Rust => {
                let cargo_toml = path.join("Cargo.toml");
                if let Ok(content) = fs::read_to_string(cargo_toml) {
                    // Simple TOML parsing for description
                    for line in content.lines() {
                        if line.trim().starts_with("description") {
                            if let Some(desc) = line.split('=').nth(1) {
                                return Some(desc.trim().trim_matches('"').to_string());
                            }
                        }
                    }
                }
            }
            ProjectType::Python => {
                let _setup_py = path.join("setup.py");
                let pyproject = path.join("pyproject.toml");
                
                // Try pyproject.toml first
                if let Ok(content) = fs::read_to_string(pyproject) {
                    for line in content.lines() {
                        if line.trim().starts_with("description") {
                            if let Some(desc) = line.split('=').nth(1) {
                                return Some(desc.trim().trim_matches('"').to_string());
                            }
                        }
                    }
                }
                
                // Try README
                if let Some(readme_desc) = Self::extract_from_readme(path) {
                    return Some(readme_desc);
                }
            }
            _ => {}
        }
        
        None
    }

    fn extract_from_readme(path: &Path) -> Option<String> {
        let readme_files = ["README.md", "readme.md", "README", "README.txt"];
        
        for readme_name in &readme_files {
            let readme_path = path.join(readme_name);
            if let Ok(content) = fs::read_to_string(readme_path) {
                // Get first non-empty line after title
                let lines: Vec<&str> = content.lines()
                    .filter(|l| !l.is_empty())
                    .collect();
                
                if lines.len() > 1 {
                    return Some(lines[1].trim().to_string());
                }
            }
        }
        
        None
    }

    fn extract_metadata(path: &Path, project_type: &ProjectType) -> ProjectMetadata {
        let git_dir = path.join(".git");
        let git_branch = if git_dir.exists() {
            Self::get_git_branch(path)
        } else {
            None
        };

        let dependencies_installed = Self::check_dependencies_installed(path, project_type);

        ProjectMetadata {
            git_branch,
            git_has_changes: false, // Would require running git status
            dependencies_installed,
            language_version: None,
        }
    }

    fn get_git_branch(path: &Path) -> Option<String> {
        let head_file = path.join(".git").join("HEAD");
        if let Ok(content) = fs::read_to_string(head_file) {
            if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                return Some(branch.trim().to_string());
            }
        }
        None
    }

    fn check_dependencies_installed(path: &Path, project_type: &ProjectType) -> bool {
        match project_type {
            ProjectType::Node => path.join("node_modules").exists(),
            ProjectType::Python => {
                path.join("venv").exists() 
                || path.join(".venv").exists()
                || path.join("env").exists()
            }
            ProjectType::Rust => path.join("target").exists(),
            _ => false,
        }
    }
}
