use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const PROJECT_MARKER: &str = ".vibehub/project.yaml";

/// Return the backwards-compatible identity used by V3 projects created before
/// persistent project identities were introduced.
pub fn legacy_project_id(project_root: &Path) -> String {
    let name = project_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project")
        .to_ascii_lowercase()
        .replace(|character: char| !character.is_ascii_alphanumeric(), "-");
    format!("project.{name}")
}

/// Generate a collision-resistant identity for a newly initialized project.
/// The readable slug is retained for diagnostics; the canonical-root digest
/// makes same-name projects at different absolute paths independent.
pub fn new_project_id(project_root: &Path) -> String {
    let canonical = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{}.{}", legacy_project_id(project_root), &digest[..16])
}

/// Read the persisted identity from the V3 marker. Old markers intentionally
/// fall back to their historical directory-slug identity so existing event
/// stores remain readable and are never silently moved.
pub fn project_id(project_root: &Path) -> String {
    let marker = project_root.join(PROJECT_MARKER);
    if let Some(value) = read_marker_value(&marker) {
        if let Some(value) = value
            .get("project_id")
            .and_then(serde_yaml::Value::as_str)
            .filter(|value| valid_project_id(value))
        {
            return value.to_owned();
        }
    }

    // Repair flows may temporarily run after the marker was lost. A single
    // existing V3 event namespace is an unambiguous durable identity; more
    // than one candidate fails closed to the legacy slug rather than guessing.
    discover_single_project_id(project_root).unwrap_or_else(|| legacy_project_id(project_root))
}

fn discover_single_project_id(project_root: &Path) -> Option<String> {
    let projects = project_root.join(".vibehub/v3/projects");
    let entries = fs::read_dir(projects).ok()?;
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let metadata = entry.file_type().ok()?;
            if !metadata.is_dir() || metadata.is_symlink() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            valid_project_id(&name).then_some(name)
        })
        .collect::<Vec<_>>();
    candidates.sort();
    (candidates.len() == 1).then(|| candidates.remove(0))
}

fn read_marker_value(path: &Path) -> Option<serde_yaml::Value> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&content).ok()
}

fn valid_project_id(value: &str) -> bool {
    (3..=128).contains(&value.len())
        && value.starts_with("project.")
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

#[allow(dead_code)]
pub fn marker_path(project_root: &Path) -> PathBuf {
    project_root.join(PROJECT_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn same_name_roots_receive_distinct_persisted_ids() {
        let parent = std::env::temp_dir().join(format!("vibehub-v3-identity-{}", Uuid::new_v4()));
        let first = parent.join("same-name");
        let second = parent.join("nested/same-name");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();

        let first_id = new_project_id(&first);
        let second_id = new_project_id(&second);
        assert_ne!(first_id, second_id);
        assert!(first_id.starts_with("project.same-name."));
        assert!(second_id.starts_with("project.same-name."));

        fs::create_dir_all(first.join(".vibehub")).unwrap();
        fs::write(
            first.join(PROJECT_MARKER),
            format!("schema_version: 3\nname: same-name\nproject_id: {first_id}\n"),
        )
        .unwrap();
        assert_eq!(project_id(&first), first_id);
        fs::remove_dir_all(parent).unwrap();
    }
}
