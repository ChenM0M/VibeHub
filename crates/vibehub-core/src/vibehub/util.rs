// Shared utilities for the VibeHub backend.
//
// All other vibehub::* modules SHOULD use these helpers instead of redefining
// their own copies of `canonical_project_root`, `normalize_path`,
// `relative_to_project`, and `yaml_string`. Keeping them in one place lets us
// fix path/escaping bugs (Windows `\\?\` prefix, `..` traversal, etc.) once
// and have every command pick up the fix.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Canonicalize a project path and assert it is a directory.
///
/// Use this for commands that may run *before* `vibehub init` has been called
/// (init itself, scanner, etc.).
pub fn canonical_project_root(project_root: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(project_root)
        .with_context(|| format!("Project path does not exist: {}", project_root.display()))?;
    if !canonical.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            canonical.display()
        ));
    }
    Ok(strip_unc_prefix(canonical))
}

/// Canonicalize a project path, assert it is a directory, AND require that a
/// `.vibehub/` directory already exists. Use this for commands that only make
/// sense after `vibehub init`.
pub fn canonical_initialized_project_root(project_root: &Path) -> Result<PathBuf> {
    let project_root = canonical_project_root(project_root)?;
    let vibehub_root = project_root.join(".vibehub");
    if !vibehub_root.is_dir() {
        return Err(anyhow!(
            "VibeHub directory does not exist: {}",
            vibehub_root.display()
        ));
    }
    Ok(project_root)
}

/// Compute a project-relative `PathBuf` for a target inside the project root.
pub fn relative_to_project(project_root: &Path, target: &Path) -> Result<PathBuf> {
    target
        .strip_prefix(project_root)
        .map(PathBuf::from)
        .with_context(|| format!("Path escapes project root: {}", target.display()))
}

/// Render a path with forward slashes (cross-platform friendly).
pub fn normalize_path(path: &Path) -> String {
    let lossy = path.to_string_lossy();
    let stripped = strip_unc_prefix_str(&lossy);
    stripped.replace('\\', "/")
}

/// Quote a string as a YAML double-quoted scalar with backslash and quote
/// escaping. Use this whenever you build YAML by string concatenation instead
/// of `serde_yaml::to_string` (e.g. embedding paths in template literals).
pub fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Like `fs::canonicalize` but strips the Windows `\\?\` extended-length
/// prefix so the result can be compared (`starts_with`) to project paths
/// produced by [`canonical_project_root`].
pub fn canonicalize_inside_project(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("Failed to canonicalize {}", path.display()))?;
    Ok(strip_unc_prefix(canonical))
}

/// Strip the Windows extended-length path prefix (`\\?\`) from a `PathBuf`,
/// because that prefix leaks through `fs::canonicalize` on Windows and breaks
/// downstream consumers that compare or join paths textually.
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy().to_string();
    let stripped = strip_unc_prefix_str(&s);
    if stripped.len() == s.len() {
        path
    } else {
        PathBuf::from(stripped)
    }
}

fn strip_unc_prefix_str(value: &str) -> String {
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-util-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn canonical_project_root_rejects_missing_path() {
        let missing = std::env::temp_dir().join(format!("nope-{}", Uuid::new_v4()));
        assert!(canonical_project_root(&missing).is_err());
    }

    #[test]
    fn canonical_initialized_project_root_requires_vibehub_dir() {
        let project = temp_project();
        assert!(canonical_initialized_project_root(&project).is_err());

        fs::create_dir_all(project.join(".vibehub")).expect("create .vibehub");
        assert!(canonical_initialized_project_root(&project).is_ok());

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn normalize_path_uses_forward_slashes() {
        let p = PathBuf::from(r"a\b\c");
        assert_eq!(normalize_path(&p), "a/b/c");
    }

    #[test]
    fn normalize_path_strips_unc_prefix() {
        assert_eq!(
            strip_unc_prefix_str(r"\\?\C:\Users\me\project"),
            r"C:\Users\me\project"
        );
        assert_eq!(
            strip_unc_prefix_str(r"\\?\UNC\server\share\dir"),
            r"\\server\share\dir"
        );
        assert_eq!(strip_unc_prefix_str(r"C:\plain\path"), r"C:\plain\path");
    }

    #[test]
    fn yaml_string_escapes_quotes_and_backslashes() {
        assert_eq!(yaml_string("plain"), r#""plain""#);
        assert_eq!(yaml_string(r"a\b"), r#""a\\b""#);
        assert_eq!(yaml_string(r#"he said "hi""#), r#""he said \"hi\"""#);
    }

    #[test]
    fn relative_to_project_rejects_outside_target() {
        let project = temp_project();
        let outside = std::env::temp_dir().join(format!("other-{}", Uuid::new_v4()));
        assert!(relative_to_project(&project, &outside).is_err());
        fs::remove_dir_all(project).ok();
    }
}
