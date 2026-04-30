use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

const AGENT_ENTRY_PATH: &str = "AGENTS.md";
const MANAGED_START: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:START -->";
const MANAGED_END: &str = "<!-- VIBEHUB:AGENT-INTEGRATION:END -->";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterSyncResult {
    pub project_root: String,
    pub created_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub conflict_files: Vec<AgentAdapterConflict>,
    pub dry_run: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AgentAdapterConflict {
    pub path: String,
    pub reason: String,
}

enum SyncDecision {
    Create(String),
    Update(String),
    Skip,
    Conflict(String),
}

pub fn sync_agent_adapter(
    project_path: impl AsRef<Path>,
    dry_run: bool,
) -> Result<AgentAdapterSyncResult> {
    let project_root = fs::canonicalize(project_path.as_ref()).with_context(|| {
        format!(
            "Project path does not exist: {}",
            project_path.as_ref().display()
        )
    })?;

    if !project_root.is_dir() {
        return Err(anyhow!(
            "Project path is not a directory: {}",
            project_root.display()
        ));
    }

    let target = project_root.join(AGENT_ENTRY_PATH);
    let decision = if target.exists() {
        let existing = fs::read_to_string(&target)
            .with_context(|| format!("Failed to read {}", target.display()))?;
        decide_existing_update(&existing)
    } else {
        SyncDecision::Create(build_managed_section())
    };

    let mut created_files = Vec::new();
    let mut updated_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut conflict_files = Vec::new();

    match decision {
        SyncDecision::Create(next_content) => {
            if !dry_run {
                fs::write(&target, next_content)
                    .with_context(|| format!("Failed to write {}", target.display()))?;
            }
            created_files.push(AGENT_ENTRY_PATH.to_string());
        }
        SyncDecision::Update(next_content) => {
            if !dry_run {
                fs::write(&target, next_content)
                    .with_context(|| format!("Failed to write {}", target.display()))?;
            }
            updated_files.push(AGENT_ENTRY_PATH.to_string());
        }
        SyncDecision::Skip => skipped_files.push(AGENT_ENTRY_PATH.to_string()),
        SyncDecision::Conflict(reason) => conflict_files.push(AgentAdapterConflict {
            path: AGENT_ENTRY_PATH.to_string(),
            reason,
        }),
    }

    let summary = if dry_run {
        format!(
            "Agent adapter dry run: would create {}, would update {}, already current {}, conflicts {}. No files were written.",
            created_files.len(),
            updated_files.len(),
            skipped_files.len(),
            conflict_files.len()
        )
    } else {
        format!(
            "Agent adapter sync complete: created {}, updated {}, skipped {}, conflicts {}.",
            created_files.len(),
            updated_files.len(),
            skipped_files.len(),
            conflict_files.len()
        )
    };

    Ok(AgentAdapterSyncResult {
        project_root: normalize_path(&project_root),
        created_files,
        updated_files,
        skipped_files,
        conflict_files,
        dry_run,
        summary,
    })
}

fn decide_existing_update(existing: &str) -> SyncDecision {
    if existing.matches(MANAGED_START).count() > 1 || existing.matches(MANAGED_END).count() > 1 {
        return SyncDecision::Conflict(
            "AGENTS.md contains multiple VibeHub managed marker regions; leaving it unchanged."
                .to_string(),
        );
    }

    let start = existing.find(MANAGED_START);
    let end = existing.find(MANAGED_END);

    match (start, end) {
        (Some(start), Some(end)) if start < end => {
            let after_end = end + MANAGED_END.len();
            let mut next = String::new();
            next.push_str(&existing[..start]);
            next.push_str(&build_managed_section());
            next.push_str(&existing[after_end..]);

            if next == existing {
                SyncDecision::Skip
            } else {
                SyncDecision::Update(next)
            }
        }
        (Some(_), Some(_)) => SyncDecision::Conflict(
            "Managed VibeHub markers are out of order; leaving AGENTS.md unchanged.".to_string(),
        ),
        (Some(_), None) | (None, Some(_)) => SyncDecision::Conflict(
            "AGENTS.md contains only one VibeHub managed marker; leaving it unchanged.".to_string(),
        ),
        (None, None) => {
            let mut next = existing.to_string();
            if !next.ends_with('\n') {
                next.push('\n');
            }
            if !next.ends_with("\n\n") {
                next.push('\n');
            }
            next.push_str(&build_managed_section());
            SyncDecision::Update(next)
        }
    }
}

fn build_managed_section() -> String {
    format!(
        r#"{MANAGED_START}
# VibeHub Agent Entry

Read these VibeHub files before working in this project:

1. `.vibehub/agent-view/current.md`
2. `.vibehub/agent-view/current-context.md`
3. `.vibehub/agent-view/handoff.md`
4. `.vibehub/rules/hard-rules.md`

Treat `.vibehub/agent-view/current.md` as the entry point. Do not edit VibeHub state files directly.
{MANAGED_END}
"#
    )
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
        let path =
            std::env::temp_dir().join(format!("vibehub-agent-adapter-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create temp project");
        path
    }

    #[test]
    fn creates_agents_entry_when_missing() {
        let project = temp_project();

        let result = sync_agent_adapter(&project, false).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert_eq!(result.created_files, vec!["AGENTS.md"]);
        assert!(result.updated_files.is_empty());
        assert!(result.conflict_files.is_empty());
        assert!(content.contains(MANAGED_START));
        assert!(content.contains(".vibehub/agent-view/current.md"));
        assert!(content.contains(".vibehub/rules/hard-rules.md"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn updates_only_managed_region_and_preserves_user_content() {
        let project = temp_project();
        fs::write(
            project.join("AGENTS.md"),
            format!("User intro\n\n{MANAGED_START}\nold\n{MANAGED_END}\n\nUser footer\n"),
        )
        .expect("write agents");

        let result = sync_agent_adapter(&project, false).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert_eq!(result.updated_files, vec!["AGENTS.md"]);
        assert!(content.starts_with("User intro\n\n"));
        assert!(content.ends_with("\n\nUser footer\n"));
        assert!(content.contains(".vibehub/agent-view/current-context.md"));
        assert!(!content.contains("\nold\n"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn appends_managed_region_to_existing_unmanaged_file() {
        let project = temp_project();
        fs::write(project.join("AGENTS.md"), "User rules\n").expect("write agents");

        let result = sync_agent_adapter(&project, false).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert_eq!(result.updated_files, vec!["AGENTS.md"]);
        assert!(content.starts_with("User rules\n\n"));
        assert!(content.contains(MANAGED_START));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn reports_conflict_without_overwriting_malformed_managed_region() {
        let project = temp_project();
        fs::write(
            project.join("AGENTS.md"),
            format!("User intro\n{MANAGED_START}\ncustom edits\n"),
        )
        .expect("write agents");

        let result = sync_agent_adapter(&project, false).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert!(result.created_files.is_empty());
        assert!(result.updated_files.is_empty());
        assert_eq!(result.conflict_files.len(), 1);
        assert!(content.contains("custom edits"));
        assert!(!content.contains(".vibehub/agent-view/current.md"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn reports_conflict_without_overwriting_duplicate_managed_regions() {
        let project = temp_project();
        fs::write(
            project.join("AGENTS.md"),
            format!(
                "User intro\n{MANAGED_START}\nfirst\n{MANAGED_END}\n{MANAGED_START}\nsecond\n{MANAGED_END}\n"
            ),
        )
        .expect("write agents");

        let result = sync_agent_adapter(&project, false).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert!(result.created_files.is_empty());
        assert!(result.updated_files.is_empty());
        assert_eq!(result.conflict_files.len(), 1);
        assert!(content.contains("first"));
        assert!(content.contains("second"));

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn dry_run_reports_create_without_writing() {
        let project = temp_project();

        let result = sync_agent_adapter(&project, true).expect("sync adapter");

        assert_eq!(result.created_files, vec!["AGENTS.md"]);
        assert!(result.dry_run);
        assert!(result.summary.contains("would create 1"));
        assert!(result.summary.contains("No files were written"));
        assert!(!project.join("AGENTS.md").exists());

        fs::remove_dir_all(project).expect("cleanup");
    }

    #[test]
    fn dry_run_reports_update_without_writing() {
        let project = temp_project();
        fs::write(project.join("AGENTS.md"), "User rules\n").expect("write agents");

        let result = sync_agent_adapter(&project, true).expect("sync adapter");
        let content = fs::read_to_string(project.join("AGENTS.md")).expect("read agents");

        assert_eq!(result.updated_files, vec!["AGENTS.md"]);
        assert!(result.summary.contains("would update 1"));
        assert_eq!(content, "User rules\n");

        fs::remove_dir_all(project).expect("cleanup");
    }
}
