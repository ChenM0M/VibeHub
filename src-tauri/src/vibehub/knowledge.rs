use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeAppendResult {
    pub knowledge_path: String,
    pub timestamp: String,
}

pub fn append_knowledge_note(
    project_root: impl AsRef<Path>,
    note: Option<String>,
) -> Result<KnowledgeAppendResult> {
    let project_root = fs::canonicalize(project_root.as_ref()).with_context(|| {
        format!(
            "Project path does not exist: {}",
            project_root.as_ref().display()
        )
    })?;

    if !project_root.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            project_root.display()
        ));
    }

    let vibehub_root = project_root.join(".vibehub");
    if !vibehub_root.is_dir() {
        return Err(anyhow!(
            ".vibehub directory is missing; run VibeHub init first: {}",
            vibehub_root.display()
        ));
    }

    let journal_dir = vibehub_root.join("journal");
    fs::create_dir_all(&journal_dir)
        .with_context(|| format!("Failed to create {}", journal_dir.display()))?;

    let knowledge_path = journal_dir.join("knowledge.md");
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let note = note.unwrap_or_default();
    let note = note.trim();
    if note.is_empty() {
        return Err(anyhow!("Knowledge note is required"));
    }

    let knowledge_existed = knowledge_path.exists();
    let existing = if knowledge_existed {
        fs::read_to_string(&knowledge_path)
            .with_context(|| format!("Failed to read {}", knowledge_path.display()))?
    } else {
        String::new()
    };

    let mut entry = String::new();
    if !existing.ends_with('\n') {
        entry.push('\n');
    }
    if !existing.trim().is_empty() {
        entry.push('\n');
    }
    entry.push_str("## Knowledge note\n\n");
    entry.push_str(&format!("- Timestamp: {timestamp}\n\n"));
    entry.push_str(note);
    entry.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&knowledge_path)
        .with_context(|| format!("Failed to open {}", knowledge_path.display()))?;
    if !knowledge_existed || existing.is_empty() {
        file.write_all(b"# VibeHub Knowledge Notes\n")
            .with_context(|| format!("Failed to initialize {}", knowledge_path.display()))?;
    }
    file.write_all(entry.as_bytes())
        .with_context(|| format!("Failed to append {}", knowledge_path.display()))?;

    Ok(KnowledgeAppendResult {
        knowledge_path: format_vibehub_path(&project_root, &knowledge_path),
        timestamp,
    })
}

fn format_vibehub_path(project_root: &Path, target: &Path) -> String {
    target
        .strip_prefix(project_root)
        .map(normalize_path)
        .unwrap_or_else(|_| normalize_path(target))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_project() -> PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-knowledge-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/journal")).expect("create temp journal");
        path
    }

    #[test]
    fn appends_knowledge_note_without_replacing_existing_content() {
        let project = temp_project();
        let knowledge = project.join(".vibehub/journal/knowledge.md");
        fs::write(
            &knowledge,
            "# VibeHub Knowledge Notes\n\nExisting knowledge.\n",
        )
        .expect("write knowledge");

        let result = append_knowledge_note(
            &project,
            Some("Prefer manual promotion until automation has evidence gates.".to_string()),
        )
        .expect("append knowledge note");
        let content = fs::read_to_string(&knowledge).expect("read knowledge");

        assert_eq!(result.knowledge_path, ".vibehub/journal/knowledge.md");
        assert!(content.starts_with("# VibeHub Knowledge Notes\n\nExisting knowledge.\n"));
        assert!(content.contains("## Knowledge note"));
        assert!(content.contains("- Timestamp: "));
        assert!(content.contains("Prefer manual promotion until automation has evidence gates."));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn rejects_uninitialized_project() {
        let path = std::env::temp_dir().join(format!("vibehub-knowledge-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");

        let err = append_knowledge_note(&path, None).expect_err("missing .vibehub should fail");
        assert!(err.to_string().contains(".vibehub directory is missing"));

        fs::remove_dir_all(path).expect("cleanup");
    }

    #[test]
    fn rejects_empty_knowledge_note() {
        let project = temp_project();

        let err = append_knowledge_note(&project, Some("  \n\t ".to_string()))
            .expect_err("empty knowledge note should fail");
        assert!(err.to_string().contains("Knowledge note is required"));
        assert!(!project.join(".vibehub/journal/knowledge.md").exists());

        fs::remove_dir_all(project).expect("cleanup");
    }
}
