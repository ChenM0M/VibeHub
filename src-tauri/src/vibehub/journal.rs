use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JournalAppendResult {
    pub journal_path: String,
    pub title: String,
    pub timestamp: String,
}

pub fn append_journal_entry(
    project_root: impl AsRef<Path>,
    title: Option<String>,
    body: Option<String>,
) -> Result<JournalAppendResult> {
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

    let journal_path = journal_dir.join("index.md");
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let title = title
        .unwrap_or_default()
        .trim()
        .chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .collect::<String>();
    let title = if title.is_empty() {
        "Session note".to_string()
    } else {
        title
    };
    let body = body.unwrap_or_default();
    let body = body.trim();
    let body = if body.is_empty() {
        "No additional details recorded."
    } else {
        body
    };

    let journal_existed = journal_path.exists();
    let existing = if journal_existed {
        fs::read_to_string(&journal_path)
            .with_context(|| format!("Failed to read {}", journal_path.display()))?
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
    entry.push_str(&format!("## {title}\n\n"));
    entry.push_str(&format!("- Timestamp: {timestamp}\n\n"));
    entry.push_str(body);
    entry.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&journal_path)
        .with_context(|| format!("Failed to open {}", journal_path.display()))?;
    if !journal_existed || existing.is_empty() {
        file.write_all(b"# VibeHub Journal\n")
            .with_context(|| format!("Failed to initialize {}", journal_path.display()))?;
    }
    file.write_all(entry.as_bytes())
        .with_context(|| format!("Failed to append {}", journal_path.display()))?;

    Ok(JournalAppendResult {
        journal_path: format_vibehub_path(&project_root, &journal_path),
        title,
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
        let path = std::env::temp_dir().join(format!("vibehub-journal-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub/journal")).expect("create temp journal");
        path
    }

    #[test]
    fn appends_journal_entry_without_replacing_existing_content() {
        let project = temp_project();
        let journal = project.join(".vibehub/journal/index.md");
        fs::write(&journal, "# VibeHub Journal\n\nExisting note.\n").expect("write journal");

        let result = append_journal_entry(
            &project,
            Some("Manual wrap-up".to_string()),
            Some("Captured a decision for later.".to_string()),
        )
        .expect("append journal entry");
        let content = fs::read_to_string(&journal).expect("read journal");

        assert_eq!(result.journal_path, ".vibehub/journal/index.md");
        assert_eq!(result.title, "Manual wrap-up");
        assert!(content.starts_with("# VibeHub Journal\n\nExisting note.\n"));
        assert!(content.contains("## Manual wrap-up"));
        assert!(content.contains("- Timestamp: "));
        assert!(content.contains("Captured a decision for later."));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn creates_missing_journal_index_and_uses_safe_defaults() {
        let project = temp_project();
        let journal = project.join(".vibehub/journal/index.md");

        let result = append_journal_entry(&project, Some("  \n  ".to_string()), None)
            .expect("append journal entry");
        let content = fs::read_to_string(&journal).expect("read journal");

        assert_eq!(result.title, "Session note");
        assert!(content.starts_with("# VibeHub Journal"));
        assert!(content.contains("## Session note"));
        assert!(content.contains("No additional details recorded."));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn rejects_uninitialized_project() {
        let path = std::env::temp_dir().join(format!("vibehub-journal-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");

        let err =
            append_journal_entry(&path, None, None).expect_err("missing .vibehub should fail");
        assert!(err.to_string().contains(".vibehub directory is missing"));

        fs::remove_dir_all(path).expect("cleanup");
    }
}
