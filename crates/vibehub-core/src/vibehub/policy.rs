use crate::vibehub::util::{
    canonical_initialized_project_root, normalize_path, relative_to_project,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const POLICY_REL_PATH: &str = ".vibehub/policy.yaml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VibehubPolicy {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_max_open_risks")]
    pub max_open_risks: usize,
    #[serde(default = "default_max_concurrent_claims")]
    pub max_concurrent_claims: usize,
    #[serde(default)]
    pub pack: PackPolicy,
    #[serde(default)]
    pub schema: SchemaPolicy,
    #[serde(default)]
    pub loop_detection: LoopDetectionPolicy,
    #[serde(default)]
    pub sub_agent: SubAgentPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackPolicy {
    #[serde(default = "default_pack_warn_at")]
    pub warn_at: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaPolicy {
    #[serde(default = "default_schema_strict")]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopDetectionPolicy {
    #[serde(default = "default_events_write_per_minute_warn_at")]
    pub events_write_per_minute_warn_at: u32,
    #[serde(default = "default_schema_failure_rate_warn_per_mille")]
    pub schema_failure_rate_warn_per_mille: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubAgentPolicy {
    #[serde(default = "default_sub_agent_max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default = "default_sub_agent_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl Default for VibehubPolicy {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            max_open_risks: default_max_open_risks(),
            max_concurrent_claims: default_max_concurrent_claims(),
            pack: PackPolicy::default(),
            schema: SchemaPolicy::default(),
            loop_detection: LoopDetectionPolicy::default(),
            sub_agent: SubAgentPolicy::default(),
        }
    }
}

impl Default for PackPolicy {
    fn default() -> Self {
        Self {
            warn_at: default_pack_warn_at(),
        }
    }
}

impl Default for SchemaPolicy {
    fn default() -> Self {
        Self {
            strict: default_schema_strict(),
        }
    }
}

impl Default for LoopDetectionPolicy {
    fn default() -> Self {
        Self {
            events_write_per_minute_warn_at: default_events_write_per_minute_warn_at(),
            schema_failure_rate_warn_per_mille: default_schema_failure_rate_warn_per_mille(),
        }
    }
}

impl Default for SubAgentPolicy {
    fn default() -> Self {
        Self {
            max_concurrent: default_sub_agent_max_concurrent(),
            timeout_seconds: default_sub_agent_timeout_seconds(),
        }
    }
}

pub fn read_policy(project_root: impl AsRef<Path>) -> Result<VibehubPolicy> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let path = project_root.join(POLICY_REL_PATH);
    if !path.is_file() {
        return Ok(VibehubPolicy::default());
    }
    let content =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    serde_yaml::from_str::<VibehubPolicy>(&content)
        .with_context(|| format!("Invalid YAML in {}", path.display()))
}

pub fn ensure_policy_file(project_root: impl AsRef<Path>) -> Result<Option<String>> {
    let project_root = canonical_initialized_project_root(project_root.as_ref())?;
    let path = project_root.join(POLICY_REL_PATH);
    if path.exists() {
        return Ok(None);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&path, default_policy_yaml())
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(Some(normalize_path(&relative_to_project(
        &project_root,
        &path,
    )?)))
}

pub fn default_policy_yaml() -> &'static str {
    r#"schema_version: 1
max_open_risks: 5
max_concurrent_claims: 5

pack:
  warn_at: 12000

schema:
  strict: true

loop_detection:
  events_write_per_minute_warn_at: 30
  schema_failure_rate_warn_per_mille: 250

sub_agent:
  max_concurrent: 3
  timeout_seconds: 60
"#
}

fn default_schema_version() -> u32 {
    1
}

fn default_max_open_risks() -> usize {
    5
}

fn default_max_concurrent_claims() -> usize {
    5
}

fn default_pack_warn_at() -> usize {
    12_000
}

fn default_schema_strict() -> bool {
    true
}

fn default_events_write_per_minute_warn_at() -> u32 {
    30
}

fn default_schema_failure_rate_warn_per_mille() -> u32 {
    250
}

fn default_sub_agent_max_concurrent() -> usize {
    3
}

fn default_sub_agent_timeout_seconds() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_project() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("vibehub-policy-test-{}", Uuid::new_v4()));
        fs::create_dir_all(path.join(".vibehub")).expect("create .vibehub");
        path
    }

    #[test]
    fn missing_policy_uses_defaults() {
        let project = temp_project();

        let policy = read_policy(&project).expect("read policy");

        assert_eq!(policy.max_open_risks, 5);
        assert_eq!(policy.max_concurrent_claims, 5);
        assert_eq!(policy.pack.warn_at, 12_000);
        assert!(policy.schema.strict);
        assert_eq!(policy.sub_agent.max_concurrent, 3);

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn partial_policy_fills_defaults() {
        let project = temp_project();
        fs::write(
            project.join(POLICY_REL_PATH),
            "max_concurrent_claims: 2\npack:\n  warn_at: 42\n",
        )
        .expect("write policy");

        let policy = read_policy(&project).expect("read policy");

        assert_eq!(policy.max_concurrent_claims, 2);
        assert_eq!(policy.pack.warn_at, 42);
        assert_eq!(policy.max_open_risks, 5);
        assert!(policy.schema.strict);

        fs::remove_dir_all(project).ok();
    }

    #[test]
    fn ensure_policy_file_writes_default_once() {
        let project = temp_project();

        let created = ensure_policy_file(&project).expect("ensure policy");
        let skipped = ensure_policy_file(&project).expect("ensure policy again");

        assert_eq!(created.as_deref(), Some(POLICY_REL_PATH));
        assert!(skipped.is_none());
        assert!(project.join(POLICY_REL_PATH).is_file());

        fs::remove_dir_all(project).ok();
    }
}
