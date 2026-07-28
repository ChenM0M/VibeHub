use super::{V3Error, V3ErrorCategory};
use serde::{Deserialize, Serialize};

pub const EXECUTION_POLICY_VERSION: u32 = 1;
pub const ENFORCEMENT_EPOCH: &str = "v3.1-hard-closure";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProfile {
    Lightweight,
    Standard,
    Full,
}

impl ExecutionProfile {
    pub fn parse(value: &str) -> Result<Self, V3Error> {
        match value {
            "lightweight" => Ok(Self::Lightweight),
            "standard" => Ok(Self::Standard),
            "full" => Ok(Self::Full),
            _ => Err(policy_error(
                "V3_PROFILE_INVALID",
                "profile must be lightweight, standard, or full",
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lightweight => "lightweight",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TriggerContext {
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub handoff_required: bool,
    #[serde(default)]
    pub multi_agent: bool,
    #[serde(default)]
    pub cross_platform: bool,
    #[serde(default)]
    pub release: bool,
    #[serde(default)]
    pub migration: bool,
    #[serde(default)]
    pub security_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileOverride {
    pub requested_profile: String,
    pub reason: String,
    #[serde(default)]
    pub user_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyHistoryEntry {
    pub from: String,
    pub to: String,
    pub reason: String,
    pub actor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveExecutionPolicy {
    pub recommended_profile: String,
    pub effective_profile: String,
    pub policy_version: u32,
    pub enforcement_epoch: String,
    pub trigger_reasons: Vec<String>,
    pub milestone_policy: String,
    pub planning_required: bool,
    pub review_required: bool,
    pub required_records: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_record: Option<ProfileOverride>,
    #[serde(default)]
    pub upgrade_history: Vec<PolicyHistoryEntry>,
}

pub fn recommend_profile(
    title: &str,
    intent: &str,
    milestone_count: usize,
    context: &TriggerContext,
) -> (ExecutionProfile, Vec<String>) {
    let text = format!("{title} {intent}").to_lowercase();
    let keyword = [
        "release",
        "publish",
        "migration",
        "migrate",
        "security",
        "credential",
        "cross-platform",
        "windows",
        "macos",
        "multi-agent",
        "发布",
        "迁移",
        "安全",
        "跨平台",
        "多 agent",
    ]
    .into_iter()
    .find(|word| text.contains(word));
    if context.release
        || context.migration
        || context.security_sensitive
        || context.cross_platform
        || context.multi_agent
        || keyword.is_some()
    {
        let mut reasons = vec!["high_risk_or_coordination_trigger".to_owned()];
        if let Some(word) = keyword {
            reasons.push(format!("keyword:{word}"));
        }
        return (ExecutionProfile::Full, reasons);
    }
    if milestone_count >= 2 {
        return (
            ExecutionProfile::Standard,
            vec!["multiple_verifiable_milestones".to_owned()],
        );
    }
    if !context.dependencies.is_empty() || context.handoff_required {
        return (
            ExecutionProfile::Standard,
            vec!["dependency_or_handoff".to_owned()],
        );
    }
    (
        ExecutionProfile::Lightweight,
        vec!["single_atomic_low_risk".to_owned()],
    )
}

pub fn resolve_policy(
    title: &str,
    intent: &str,
    milestone_count: usize,
    requested: &str,
    context: &TriggerContext,
    override_record: Option<ProfileOverride>,
) -> Result<EffectiveExecutionPolicy, V3Error> {
    let (recommended, trigger_reasons) = recommend_profile(title, intent, milestone_count, context);
    let requested = ExecutionProfile::parse(requested)?;
    let mut effective = requested;
    let mut accepted_override = None;
    if requested < recommended {
        let candidate = override_record.ok_or_else(|| {
            policy_error(
                "V3_PROFILE_DOWNGRADE_OVERRIDE_REQUIRED",
                "lowering the recommended profile requires an explicit user override and reason",
            )
        })?;
        if !candidate.user_confirmed
            || candidate.reason.trim().is_empty()
            || ExecutionProfile::parse(&candidate.requested_profile)? != requested
        {
            return Err(policy_error("V3_PROFILE_DOWNGRADE_OVERRIDE_INVALID", "profile downgrade override must be user-confirmed, reasoned, and match requested_profile"));
        }
        accepted_override = Some(candidate);
    }
    if requested > recommended {
        effective = requested;
    }
    Ok(policy_for(
        recommended,
        effective,
        trigger_reasons,
        accepted_override,
    ))
}

pub fn upgrade_policy(
    policy: &EffectiveExecutionPolicy,
    target: &str,
    reason: &str,
    actor: &str,
) -> Result<EffectiveExecutionPolicy, V3Error> {
    let current = ExecutionProfile::parse(&policy.effective_profile)?;
    let target = ExecutionProfile::parse(target)?;
    if target < current {
        return Err(policy_error(
            "V3_RUNTIME_PROFILE_DOWNGRADE_FORBIDDEN",
            "runtime policy may only be upgraded; start an explicit replan for downgrade",
        ));
    }
    if reason.trim().is_empty() {
        return Err(policy_error(
            "V3_RUNTIME_PROFILE_UPGRADE_REASON_REQUIRED",
            "runtime policy upgrade requires an auditable reason",
        ));
    }
    let mut next = policy.clone();
    if target > current {
        next.effective_profile = target.as_str().to_owned();
        let template = policy_for(
            ExecutionProfile::parse(&policy.recommended_profile)?,
            target,
            policy.trigger_reasons.clone(),
            policy.override_record.clone(),
        );
        next.milestone_policy = template.milestone_policy;
        next.planning_required = template.planning_required;
        next.review_required = template.review_required;
        next.required_records = template.required_records;
        next.upgrade_history.push(PolicyHistoryEntry {
            from: current.as_str().to_owned(),
            to: target.as_str().to_owned(),
            reason: reason.trim().to_owned(),
            actor: actor.to_owned(),
        });
    }
    Ok(next)
}

fn policy_for(
    recommended: ExecutionProfile,
    effective: ExecutionProfile,
    trigger_reasons: Vec<String>,
    override_record: Option<ProfileOverride>,
) -> EffectiveExecutionPolicy {
    let (milestone_policy, required_records) = match effective {
        ExecutionProfile::Lightweight => (
            "minimal",
            vec!["session", "result", "review", "risk_if_any"],
        ),
        ExecutionProfile::Standard => (
            "standard",
            vec!["plan", "session", "progress", "result", "review"],
        ),
        ExecutionProfile::Full => (
            "full",
            vec!["plan", "session", "progress", "result", "review"],
        ),
    };
    EffectiveExecutionPolicy {
        recommended_profile: recommended.as_str().to_owned(),
        effective_profile: effective.as_str().to_owned(),
        policy_version: EXECUTION_POLICY_VERSION,
        enforcement_epoch: ENFORCEMENT_EPOCH.to_owned(),
        trigger_reasons,
        milestone_policy: milestone_policy.to_owned(),
        planning_required: effective != ExecutionProfile::Lightweight,
        review_required: true,
        required_records: required_records.into_iter().map(str::to_owned).collect(),
        override_record,
        upgrade_history: Vec::new(),
    }
}

fn policy_error(code: &str, message: &str) -> V3Error {
    V3Error::new(code, V3ErrorCategory::Validation, false, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decision_table_and_runtime_monotonicity() {
        assert_eq!(
            recommend_profile("rename", "one file", 1, &TriggerContext::default()).0,
            ExecutionProfile::Lightweight
        );
        assert_eq!(
            recommend_profile("feature", "two checks", 2, &TriggerContext::default()).0,
            ExecutionProfile::Standard
        );
        assert_eq!(
            recommend_profile("release", "ship", 1, &TriggerContext::default()).0,
            ExecutionProfile::Full
        );
        assert_eq!(
            resolve_policy(
                "release",
                "ship",
                1,
                "lightweight",
                &TriggerContext::default(),
                None
            )
            .unwrap_err()
            .code,
            "V3_PROFILE_DOWNGRADE_OVERRIDE_REQUIRED"
        );
        let p = resolve_policy(
            "feature",
            "two checks",
            2,
            "standard",
            &TriggerContext::default(),
            None,
        )
        .unwrap();
        let p = upgrade_policy(&p, "full", "security signal", "codex").unwrap();
        assert_eq!(p.effective_profile, "full");
        assert_eq!(
            upgrade_policy(&p, "standard", "quiet", "codex")
                .unwrap_err()
                .code,
            "V3_RUNTIME_PROFILE_DOWNGRADE_FORBIDDEN"
        );
    }
}
