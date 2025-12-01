use crate::models::{Project, ProjectMetadata, ProjectType};
use anyhow::Result;
use std::fs;
use std::path::Path;


pub struct Scanner;

impl Scanner {
    pub fn scan_directory(path: &str, _max_depth: usize) -> Result<Vec<Project>> {
        let mut projects = Vec::new();
        let abs_path = fs::canonicalize(path)?;
        
        // User requested to just take all directories under the scanned directory
        // So we iterate immediate children only
        for entry in fs::read_dir(abs_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                
                // Filter out hidden directories and common build artifacts
                if name.starts_with('.') || 
                   name == "node_modules" || 
                   name == "target" || 
                   name == "dist" || 
                   name == "build" ||
                   name == "venv" ||
                   name == "bin" ||
                   name == "obj" {
                    continue;
                }

                if let Some(project) = Self::detect_project(&path) {
                    projects.push(project);
                }
            }
        }
        
        Ok(projects)
    }

    pub fn refresh_project(project: &mut Project) {
        let path = Path::new(&project.path);
        if path.exists() {
            if let Some(pt) = Self::detect_project_type(path) {
                project.project_type = pt.clone();
                project.metadata = Self::extract_metadata(path, &pt);
                if project.description.is_none() {
                    project.description = Self::extract_description(path, &pt);
                }
            }
        }
    }

    fn detect_project(path: &Path) -> Option<Project> {
        // We now accept any directory as a project, defaulting to "Other" if no specific type detected
        let project_type = Self::detect_project_type(path).unwrap_or(ProjectType::Other);
        
        let name = path
            .file_name()?
            .to_string_lossy()
            .to_string();
        
        // Clean path: remove Windows long path prefix \\?\ if present
        let path_str = path.to_string_lossy().to_string();
        let clean_path = if path_str.starts_with(r"\\?\") {
            path_str[4..].to_string()
        } else {
            path_str
        };
        
        let metadata = Self::extract_metadata(path, &project_type);
        
        Some(Project {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: Self::extract_description(path, &project_type),
            path: clean_path,
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
        
        // Default to Other if it's a directory but matches none of the above
        // The caller (detect_project) handles the fallback, but here we return None to indicate "unknown specific type"
        // Wait, detect_project calls this. If I return None, detect_project uses unwrap_or(Other).
        // So I can just return None here.
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
