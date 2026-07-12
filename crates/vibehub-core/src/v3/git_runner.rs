use super::domain::{OperationId, V3Error, V3ErrorCategory, WorktreeId};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInvocation {
    pub cwd: PathBuf,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait GitRunner {
    fn run(&self, invocation: &GitInvocation) -> Result<GitOutput, V3Error>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemGitRunner;

impl GitRunner for SystemGitRunner {
    fn run(&self, invocation: &GitInvocation) -> Result<GitOutput, V3Error> {
        if !invocation.cwd.is_absolute() {
            return Err(git_error(
                "V3_GIT_CWD_NOT_ABSOLUTE",
                V3ErrorCategory::Validation,
                false,
                "Git cwd must be an absolute native path",
            ));
        }
        let output = Command::new("git")
            .args(&invocation.argv)
            .current_dir(&invocation.cwd)
            .output()
            .map_err(|error| {
                git_error(
                    "V3_GIT_EXEC_FAILED",
                    V3ErrorCategory::Internal,
                    true,
                    format!("failed to execute git: {error}"),
                )
            })?;
        Ok(GitOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitWorktreePresence {
    Present,
    Missing,
    Prunable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorktreeRecord {
    pub native_path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub bare: bool,
    pub locked_reason: Option<String>,
    pub prunable_reason: Option<String>,
    pub presence: GitWorktreePresence,
}

pub fn parse_worktree_porcelain(output: &str) -> Vec<GitWorktreeRecord> {
    output
        .split("\n\n")
        .filter_map(|block| {
            let mut native_path = None;
            let mut record = GitWorktreeRecord {
                native_path: String::new(),
                head: None,
                branch: None,
                detached: false,
                bare: false,
                locked_reason: None,
                prunable_reason: None,
                presence: GitWorktreePresence::Present,
            };
            for line in block.lines() {
                let (key, value) = line.split_once(' ').unwrap_or((line, ""));
                match key {
                    "worktree" => native_path = Some(value.to_owned()),
                    "HEAD" => record.head = Some(value.to_owned()),
                    "branch" => record.branch = Some(value.to_owned()),
                    "detached" => record.detached = true,
                    "bare" => record.bare = true,
                    "locked" => record.locked_reason = Some(value.to_owned()),
                    "prunable" => {
                        record.prunable_reason = Some(value.to_owned());
                        record.presence = GitWorktreePresence::Prunable;
                    }
                    _ => {}
                }
            }
            record.native_path = native_path?;
            Some(record)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusObservation {
    pub dirty: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub changed_files: Vec<String>,
}

pub fn parse_status_porcelain_v2_z(output: &str) -> GitStatusObservation {
    let mut observation = GitStatusObservation {
        dirty: false,
        branch: None,
        head: None,
        changed_files: Vec::new(),
    };
    for entry in output.split('\0').filter(|entry| !entry.is_empty()) {
        if let Some(value) = entry.strip_prefix("# branch.head ") {
            observation.branch = (value != "(detached)").then(|| value.to_owned());
        } else if let Some(value) = entry.strip_prefix("# branch.oid ") {
            observation.head = (value != "(initial)").then(|| value.to_owned());
        } else if matches!(
            entry.as_bytes().first(),
            Some(b'1' | b'2' | b'u' | b'?' | b'!')
        ) {
            observation.dirty = true;
            if let Some(path) = porcelain_path(entry) {
                observation.changed_files.push(path.to_owned());
            }
        }
    }
    observation.changed_files.sort();
    observation.changed_files.dedup();
    observation
}

fn porcelain_path(entry: &str) -> Option<&str> {
    match entry.as_bytes().first()? {
        b'?' | b'!' => entry.get(2..),
        b'1' => entry.splitn(9, ' ').nth(8),
        b'2' => entry.splitn(10, ' ').nth(9),
        b'u' => entry.splitn(11, ' ').nth(10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOperationPhase {
    Prepared,
    Executed,
    Inspected,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitOperationRecord {
    pub operation_id: OperationId,
    pub worktree_id: WorktreeId,
    pub phase: GitOperationPhase,
    pub argv: Vec<String>,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub evidence_refs: Vec<String>,
    pub error_code: Option<String>,
}

impl GitOperationRecord {
    pub fn prepare(
        operation_id: OperationId,
        worktree_id: WorktreeId,
        cwd: &Path,
        argv: Vec<String>,
    ) -> Result<Self, V3Error> {
        if !cwd.is_absolute() || argv.is_empty() {
            return Err(git_error(
                "V3_GIT_OPERATION_INVALID",
                V3ErrorCategory::Validation,
                false,
                "Git operation requires an absolute cwd and non-empty argv",
            ));
        }
        Ok(Self {
            operation_id,
            worktree_id,
            phase: GitOperationPhase::Prepared,
            argv,
            cwd: cwd.to_string_lossy().into_owned(),
            exit_code: None,
            evidence_refs: Vec::new(),
            error_code: None,
        })
    }

    pub fn record_execution(
        &mut self,
        output: &GitOutput,
        evidence_ref: String,
    ) -> Result<(), V3Error> {
        if self.phase != GitOperationPhase::Prepared {
            return Err(git_error(
                "V3_GIT_OPERATION_PHASE_INVALID",
                V3ErrorCategory::VersionConflict,
                false,
                "Git execution can only be recorded from the prepared phase",
            ));
        }
        self.phase = GitOperationPhase::Executed;
        self.exit_code = Some(output.exit_code);
        self.evidence_refs.push(evidence_ref);
        if output.exit_code != 0 {
            self.phase = GitOperationPhase::Failed;
            self.error_code = Some("V3_GIT_COMMAND_FAILED".to_owned());
        }
        Ok(())
    }

    pub fn record_inspection(
        &mut self,
        matched_expected_state: bool,
        evidence_ref: String,
    ) -> Result<(), V3Error> {
        if self.phase != GitOperationPhase::Executed {
            return Err(git_error(
                "V3_GIT_OPERATION_PHASE_INVALID",
                V3ErrorCategory::VersionConflict,
                false,
                "Git inspection can only be recorded after successful execution",
            ));
        }
        self.evidence_refs.push(evidence_ref);
        self.phase = if matched_expected_state {
            GitOperationPhase::Succeeded
        } else {
            self.error_code = Some("V3_GIT_INSPECTION_MISMATCH".to_owned());
            GitOperationPhase::Failed
        };
        Ok(())
    }
}

pub struct GitWorktreeAdapter<R> {
    runner: R,
    project_root: PathBuf,
}

impl<R: GitRunner> GitWorktreeAdapter<R> {
    pub fn new(runner: R, project_root: PathBuf) -> Result<Self, V3Error> {
        if !project_root.is_absolute() {
            return Err(git_error(
                "V3_GIT_CWD_NOT_ABSOLUTE",
                V3ErrorCategory::Validation,
                false,
                "project root must be absolute",
            ));
        }
        Ok(Self {
            runner,
            project_root,
        })
    }

    pub fn list(&self) -> Result<Vec<GitWorktreeRecord>, V3Error> {
        let output = self.run(vec![
            "worktree".to_owned(),
            "list".to_owned(),
            "--porcelain".to_owned(),
            "-z".to_owned(),
        ])?;
        require_success(&output)?;
        Ok(parse_worktree_porcelain(&output.stdout.replace('\0', "\n")))
    }

    pub fn status(&self, worktree_path: &Path) -> Result<GitStatusObservation, V3Error> {
        let output = self.runner.run(&GitInvocation {
            cwd: worktree_path.to_path_buf(),
            argv: vec![
                "status".to_owned(),
                "--porcelain=v2".to_owned(),
                "--branch".to_owned(),
                "-z".to_owned(),
            ],
        })?;
        require_success(&output)?;
        Ok(parse_status_porcelain_v2_z(&output.stdout))
    }

    pub fn add(&self, path: &Path, branch: &str, base: &str) -> Result<GitOutput, V3Error> {
        self.run(vec![
            "worktree".to_owned(),
            "add".to_owned(),
            "-b".to_owned(),
            branch.to_owned(),
            path.to_string_lossy().into_owned(),
            base.to_owned(),
        ])
        .and_then(require_success_owned)
    }

    pub fn repair(&self, path: &Path) -> Result<GitOutput, V3Error> {
        self.run(vec![
            "worktree".to_owned(),
            "repair".to_owned(),
            path.to_string_lossy().into_owned(),
        ])
        .and_then(require_success_owned)
    }

    pub fn remove_clean(
        &self,
        path: &Path,
        observation: &GitStatusObservation,
        owner_process_confirmed_dead: bool,
    ) -> Result<GitOutput, V3Error> {
        if observation.dirty {
            return Err(git_error(
                "V3_DIRTY_WORKTREE_CLEANUP_REFUSED",
                V3ErrorCategory::VersionConflict,
                false,
                "dirty worktrees are never removed automatically",
            ));
        }
        if !owner_process_confirmed_dead {
            return Err(git_error(
                "V3_WORKTREE_OWNER_UNKNOWN",
                V3ErrorCategory::StaleResource,
                true,
                "worktree owner process must be inspected before removal",
            ));
        }
        self.run(vec![
            "worktree".to_owned(),
            "remove".to_owned(),
            path.to_string_lossy().into_owned(),
        ])
        .and_then(require_success_owned)
    }

    fn run(&self, argv: Vec<String>) -> Result<GitOutput, V3Error> {
        self.runner.run(&GitInvocation {
            cwd: self.project_root.clone(),
            argv,
        })
    }
}

fn require_success(output: &GitOutput) -> Result<(), V3Error> {
    if output.exit_code == 0 {
        Ok(())
    } else {
        Err(git_error(
            "V3_GIT_COMMAND_FAILED",
            V3ErrorCategory::VersionConflict,
            true,
            output.stderr.clone(),
        )
        .with_detail("exit_code", output.exit_code))
    }
}

fn require_success_owned(output: GitOutput) -> Result<GitOutput, V3Error> {
    require_success(&output)?;
    Ok(output)
}

fn git_error(
    code: &str,
    category: V3ErrorCategory,
    retryable: bool,
    message: impl Into<String>,
) -> V3Error {
    V3Error::new(code, category, retryable, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct FakeRunner {
        invocations: RefCell<Vec<GitInvocation>>,
        output: GitOutput,
    }

    impl GitRunner for FakeRunner {
        fn run(&self, invocation: &GitInvocation) -> Result<GitOutput, V3Error> {
            self.invocations.borrow_mut().push(invocation.clone());
            Ok(self.output.clone())
        }
    }

    #[test]
    fn porcelain_worktree_records_preserve_locked_and_prunable_states() {
        let records = parse_worktree_porcelain(
            "worktree /repo\nHEAD a1b2c3d\nbranch refs/heads/main\n\nworktree /repo/wt\nHEAD e4f5a6b\ndetached\nlocked agent alive\nprunable gitdir missing\n",
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].presence, GitWorktreePresence::Prunable);
        assert_eq!(records[1].locked_reason.as_deref(), Some("agent alive"));
        assert!(records[1].detached);
    }

    #[test]
    fn status_porcelain_v2_z_is_parsed_without_human_output() {
        let status = parse_status_porcelain_v2_z(
            "# branch.oid a1b2c3d\0# branch.head feature/m5\01 .M N... 100644 100644 100644 aaaaaaa bbbbbbb src/lib.rs\0? new.txt\0",
        );
        assert_eq!(status.branch.as_deref(), Some("feature/m5"));
        assert_eq!(status.head.as_deref(), Some("a1b2c3d"));
        assert!(status.dirty);
        assert_eq!(status.changed_files, vec!["new.txt", "src/lib.rs"]);
    }

    #[test]
    fn adapter_uses_argv_and_explicit_cwd() {
        let runner = FakeRunner {
            invocations: RefCell::new(Vec::new()),
            output: GitOutput {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        let adapter = GitWorktreeAdapter::new(runner, PathBuf::from("/repo")).unwrap();
        adapter
            .add(
                Path::new("/worktrees/node"),
                "vibehub/m5/node-a1b2c3d",
                "main",
            )
            .unwrap();
        let invocation = &adapter.runner.invocations.borrow()[0];
        assert_eq!(invocation.cwd, PathBuf::from("/repo"));
        assert_eq!(
            invocation.argv,
            vec![
                "worktree",
                "add",
                "-b",
                "vibehub/m5/node-a1b2c3d",
                "/worktrees/node",
                "main"
            ]
        );
    }

    #[test]
    fn dirty_or_uninspected_worktree_is_never_removed() {
        let runner = FakeRunner {
            invocations: RefCell::new(Vec::new()),
            output: GitOutput {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        let adapter = GitWorktreeAdapter::new(runner, PathBuf::from("/repo")).unwrap();
        let mut status = GitStatusObservation {
            dirty: true,
            branch: None,
            head: None,
            changed_files: vec!["src/lib.rs".to_owned()],
        };
        assert_eq!(
            adapter
                .remove_clean(Path::new("/worktrees/node"), &status, true)
                .unwrap_err()
                .code,
            "V3_DIRTY_WORKTREE_CLEANUP_REFUSED"
        );
        status.dirty = false;
        assert_eq!(
            adapter
                .remove_clean(Path::new("/worktrees/node"), &status, false)
                .unwrap_err()
                .code,
            "V3_WORKTREE_OWNER_UNKNOWN"
        );
        assert!(adapter.runner.invocations.borrow().is_empty());
    }

    #[test]
    fn operation_journal_requires_inspection_before_success() {
        let mut operation = GitOperationRecord::prepare(
            OperationId::from("operation.create"),
            WorktreeId::from("worktree.main"),
            Path::new("/repo"),
            vec!["worktree".to_owned(), "add".to_owned()],
        )
        .unwrap();
        operation
            .record_execution(
                &GitOutput {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                },
                "ev.execute".to_owned(),
            )
            .unwrap();
        assert_eq!(operation.phase, GitOperationPhase::Executed);
        operation
            .record_inspection(true, "ev.inspect".to_owned())
            .unwrap();
        assert_eq!(operation.phase, GitOperationPhase::Succeeded);
        assert_eq!(operation.evidence_refs, vec!["ev.execute", "ev.inspect"]);
    }

    #[test]
    fn operation_journal_rejects_skipped_or_repeated_phases() {
        let mut operation = GitOperationRecord::prepare(
            OperationId::from("operation.create"),
            WorktreeId::from("worktree.main"),
            Path::new("/repo"),
            vec!["worktree".to_owned(), "add".to_owned()],
        )
        .unwrap();
        assert_eq!(
            operation
                .record_inspection(true, "ev.inspect.early".to_owned())
                .unwrap_err()
                .code,
            "V3_GIT_OPERATION_PHASE_INVALID"
        );
        operation
            .record_execution(
                &GitOutput {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                },
                "ev.execute".to_owned(),
            )
            .unwrap();
        operation
            .record_inspection(true, "ev.inspect".to_owned())
            .unwrap();
        assert_eq!(
            operation
                .record_inspection(true, "ev.inspect.again".to_owned())
                .unwrap_err()
                .code,
            "V3_GIT_OPERATION_PHASE_INVALID"
        );
    }
}
