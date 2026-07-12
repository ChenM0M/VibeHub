use super::domain::{LeaseId, NodeId, SessionId, WorktreeId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const REASON_DEPENDENCY_NOT_READY: &str = "DEPENDENCY_NOT_READY";
pub const REASON_PARALLEL_UNSAFE: &str = "PARALLEL_UNSAFE";
pub const REASON_SCOPE_UNKNOWN: &str = "SCOPE_UNKNOWN";
pub const REASON_SCOPE_PATH_INVALID: &str = "SCOPE_PATH_INVALID";
pub const REASON_SCOPE_OVERLAP: &str = "SCOPE_OVERLAP";
pub const REASON_CASE_COLLISION: &str = "CASE_COLLISION";
pub const REASON_DENYLIST_MATCH: &str = "DENYLIST_MATCH";
pub const REASON_OBSERVED_FILE_COLLISION: &str = "OBSERVED_FILE_COLLISION";
pub const REASON_OWNER_OVERRIDE: &str = "OWNER_OVERRIDE";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeState {
    Planned,
    Creating,
    Ready,
    Active,
    Dirty,
    Submitted,
    Integrating,
    Integrated,
    Conflicted,
    Abandoned,
    Repairing,
    Cleaned,
}

impl WorktreeState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use WorktreeState::*;
        matches!(
            (self, next),
            (Planned, Creating | Abandoned)
                | (Creating, Ready | Repairing | Abandoned)
                | (Ready, Active | Repairing | Abandoned | Cleaned)
                | (Active, Dirty | Submitted | Repairing | Abandoned)
                | (Dirty, Submitted | Repairing | Abandoned)
                | (Submitted, Integrating | Dirty | Repairing | Abandoned)
                | (Integrating, Integrated | Conflicted | Repairing)
                | (Conflicted, Integrating | Repairing | Abandoned)
                | (Integrated, Cleaned | Repairing)
                | (Abandoned, Repairing | Cleaned)
                | (
                    Repairing,
                    Ready | Active | Dirty | Submitted | Integrated | Abandoned | Cleaned
                )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityDecision {
    Allow,
    Warn,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeClaim {
    pub node_id: NodeId,
    pub declared_scope: Vec<String>,
    pub observed_delta: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerOverride {
    pub actor: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibilityInput {
    pub claim: ScopeClaim,
    pub peers: Vec<ScopeClaim>,
    pub dependency_ready: bool,
    pub parallel_safe: bool,
    pub case_sensitive: bool,
    pub denylist: Vec<String>,
    pub owner_override: Option<OwnerOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibilityResult {
    pub decision: EligibilityDecision,
    pub digest: String,
    pub reason_codes: Vec<String>,
    pub normalized_scope: Vec<String>,
    pub normalized_observed_delta: Vec<String>,
    pub override_applied: bool,
}

pub fn evaluate_eligibility(input: &EligibilityInput) -> EligibilityResult {
    let (scope, scope_invalid) = normalize_paths(&input.claim.declared_scope, input.case_sensitive);
    let (observed, observed_invalid) =
        normalize_paths(&input.claim.observed_delta, input.case_sensitive);
    let mut reasons = BTreeSet::new();

    if !input.dependency_ready {
        reasons.insert(REASON_DEPENDENCY_NOT_READY.to_owned());
    }
    if !input.parallel_safe && !input.peers.is_empty() {
        reasons.insert(REASON_PARALLEL_UNSAFE.to_owned());
    }
    if scope.is_empty() {
        reasons.insert(REASON_SCOPE_UNKNOWN.to_owned());
    }
    if scope_invalid || observed_invalid {
        reasons.insert(REASON_SCOPE_PATH_INVALID.to_owned());
    }

    let (denylist, denylist_invalid) = normalize_paths(&input.denylist, input.case_sensitive);
    if denylist_invalid {
        reasons.insert(REASON_SCOPE_PATH_INVALID.to_owned());
    }
    if scope.iter().chain(observed.iter()).any(|candidate| {
        denylist
            .iter()
            .any(|pattern| path_matches(pattern, candidate))
    }) {
        reasons.insert(REASON_DENYLIST_MATCH.to_owned());
    }

    for peer in &input.peers {
        let (peer_scope, peer_scope_invalid) =
            normalize_paths(&peer.declared_scope, input.case_sensitive);
        let (peer_observed, peer_observed_invalid) =
            normalize_paths(&peer.observed_delta, input.case_sensitive);
        if peer_scope_invalid || peer_observed_invalid {
            reasons.insert(REASON_SCOPE_PATH_INVALID.to_owned());
        }
        if paths_overlap(&scope, &peer_scope) {
            reasons.insert(REASON_SCOPE_OVERLAP.to_owned());
        }
        if paths_overlap(&observed, &peer_observed)
            || paths_overlap(&observed, &peer_scope)
            || paths_overlap(&scope, &peer_observed)
        {
            reasons.insert(REASON_OBSERVED_FILE_COLLISION.to_owned());
        }
        if !input.case_sensitive
            && has_case_collision(&input.claim.declared_scope, &peer.declared_scope)
        {
            reasons.insert(REASON_CASE_COLLISION.to_owned());
        }
    }

    let non_overridable_block = reasons.iter().any(|reason| {
        matches!(
            reason.as_str(),
            REASON_DEPENDENCY_NOT_READY | REASON_PARALLEL_UNSAFE | REASON_SCOPE_PATH_INVALID
        )
    });
    let overridable_block = reasons.iter().any(|reason| {
        matches!(
            reason.as_str(),
            REASON_SCOPE_OVERLAP
                | REASON_CASE_COLLISION
                | REASON_DENYLIST_MATCH
                | REASON_OBSERVED_FILE_COLLISION
        )
    });
    let override_applied = overridable_block
        && input.owner_override.as_ref().is_some_and(|owner_override| {
            !owner_override.actor.trim().is_empty() && !owner_override.evidence_refs.is_empty()
        });
    if override_applied {
        reasons.insert(REASON_OWNER_OVERRIDE.to_owned());
    }
    let decision = if non_overridable_block || (overridable_block && !override_applied) {
        EligibilityDecision::Block
    } else if !reasons.is_empty() {
        EligibilityDecision::Warn
    } else {
        EligibilityDecision::Allow
    };
    let reason_codes = reasons.into_iter().collect::<Vec<_>>();
    let digest = eligibility_digest(
        &input.claim.node_id.0,
        decision,
        &reason_codes,
        &scope,
        &observed,
        override_applied,
    );
    EligibilityResult {
        decision,
        digest,
        reason_codes,
        normalized_scope: scope,
        normalized_observed_delta: observed,
        override_applied,
    }
}

fn normalize_paths(paths: &[String], case_sensitive: bool) -> (Vec<String>, bool) {
    let mut invalid = false;
    let normalized = paths
        .iter()
        .filter_map(|path| match normalize_path(path, case_sensitive) {
            Ok(path) if !path.is_empty() => Some(path),
            Ok(_) => None,
            Err(()) => {
                invalid = true;
                None
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    (normalized, invalid)
}

fn normalize_path(path: &str, case_sensitive: bool) -> Result<String, ()> {
    let path = path.trim().replace('\\', "/");
    let has_drive_prefix = path
        .as_bytes()
        .get(1)
        .is_some_and(|separator| *separator == b':');
    if path.starts_with('/') || has_drive_prefix {
        return Err(());
    }

    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err(());
                }
            }
            value => components.push(value),
        }
    }
    let normalized = components.join("/");
    Ok(if case_sensitive {
        normalized
    } else {
        normalized.to_lowercase()
    })
}

fn path_matches(pattern: &str, candidate: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return candidate == prefix || candidate.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return candidate
            .strip_prefix(&format!("{prefix}/"))
            .is_some_and(|suffix| !suffix.contains('/'));
    }
    candidate == pattern
}

fn paths_overlap(left: &[String], right: &[String]) -> bool {
    left.iter().any(|left_path| {
        right.iter().any(|right_path| {
            left_path == right_path
                || left_path.starts_with(&format!("{right_path}/"))
                || right_path.starts_with(&format!("{left_path}/"))
        })
    })
}

fn has_case_collision(left: &[String], right: &[String]) -> bool {
    left.iter().any(|left_path| {
        right
            .iter()
            .any(|right_path| left_path != right_path && left_path.eq_ignore_ascii_case(right_path))
    })
}

fn eligibility_digest(
    node_id: &str,
    decision: EligibilityDecision,
    reasons: &[String],
    scope: &[String],
    observed: &[String],
    override_applied: bool,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{node_id}|{decision:?}|{}|{}|{}|{override_applied}",
        reasons.join(","),
        scope.join(","),
        observed.join(",")
    ));
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeIdentity {
    pub worktree_id: WorktreeId,
    pub node_id: NodeId,
    pub branch: String,
    pub native_path: String,
}

pub fn branch_name(task_slug: &str, node_slug: &str, short_id: &str) -> String {
    fn slug(value: &str) -> String {
        let mut output = String::new();
        let mut separator = false;
        for character in value.chars() {
            if character.is_ascii_alphanumeric() {
                output.push(character.to_ascii_lowercase());
                separator = false;
            } else if !separator && !output.is_empty() {
                output.push('-');
                separator = true;
            }
        }
        output.trim_matches('-').to_owned()
    }
    format!(
        "vibehub/{}/{}-{}",
        slug(task_slug),
        slug(node_slug),
        slug(short_id)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub lease_id: LeaseId,
    pub worktree_id: WorktreeId,
    pub owner_session_id: SessionId,
    pub generation: u64,
    pub reclaim_challenge: Option<String>,
}

impl Lease {
    pub fn reclaim(
        &self,
        challenge: &str,
        owner_process_confirmed_dead: bool,
    ) -> Result<Self, &'static str> {
        if !owner_process_confirmed_dead {
            return Err("LEASE_OWNER_NOT_CONFIRMED_DEAD");
        }
        if self.reclaim_challenge.as_deref() != Some(challenge) || challenge.is_empty() {
            return Err("LEASE_RECLAIM_CHALLENGE_MISMATCH");
        }
        Ok(Self {
            generation: self.generation + 1,
            reclaim_challenge: None,
            ..self.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(node: &str, scope: &[&str], observed: &[&str]) -> ScopeClaim {
        ScopeClaim {
            node_id: NodeId::from(node),
            declared_scope: scope.iter().map(|value| (*value).to_owned()).collect(),
            observed_delta: observed.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn input(claim: ScopeClaim) -> EligibilityInput {
        EligibilityInput {
            claim,
            peers: vec![],
            dependency_ready: true,
            parallel_safe: true,
            case_sensitive: true,
            denylist: vec![".vibehub/**".to_owned(), "migrations/**".to_owned()],
            owner_override: None,
        }
    }

    #[test]
    fn lifecycle_only_allows_explicit_transitions() {
        assert!(WorktreeState::Planned.can_transition_to(WorktreeState::Creating));
        assert!(WorktreeState::Integrating.can_transition_to(WorktreeState::Conflicted));
        assert!(WorktreeState::Conflicted.can_transition_to(WorktreeState::Repairing));
        assert!(!WorktreeState::Dirty.can_transition_to(WorktreeState::Cleaned));
        assert!(!WorktreeState::Cleaned.can_transition_to(WorktreeState::Active));
    }

    #[test]
    fn unknown_scope_warns_but_does_not_block() {
        let result = evaluate_eligibility(&input(claim("node.empty", &[], &[])));
        assert_eq!(result.decision, EligibilityDecision::Warn);
        assert_eq!(result.reason_codes, vec![REASON_SCOPE_UNKNOWN]);
    }

    #[test]
    fn observed_same_file_collision_blocks() {
        let mut value = input(claim("node.left", &["src/left"], &["src/shared.ts"]));
        value.peers = vec![claim("node.right", &["src/right"], &["src/shared.ts"])];
        let result = evaluate_eligibility(&value);
        assert_eq!(result.decision, EligibilityDecision::Block);
        assert!(result
            .reason_codes
            .contains(&REASON_OBSERVED_FILE_COLLISION.to_owned()));
    }

    #[test]
    fn windows_case_collision_is_stable() {
        let mut value = input(claim("node.left", &["src/Readme.md"], &[]));
        value.case_sensitive = false;
        value.peers = vec![claim("node.right", &["src/README.md"], &[])];
        let first = evaluate_eligibility(&value);
        let second = evaluate_eligibility(&value);
        assert_eq!(first.decision, EligibilityDecision::Block);
        assert!(first
            .reason_codes
            .contains(&REASON_CASE_COLLISION.to_owned()));
        assert_eq!(first.digest, second.digest);
        assert_eq!(first.digest.len(), 64);
    }

    #[test]
    fn denylisted_canonical_state_blocks() {
        let result =
            evaluate_eligibility(&input(claim("node.state", &[".vibehub/state.yaml"], &[])));
        assert_eq!(result.decision, EligibilityDecision::Block);
        assert!(result
            .reason_codes
            .contains(&REASON_DENYLIST_MATCH.to_owned()));
    }

    #[test]
    fn owner_override_requires_evidence() {
        let mut value = input(claim("node.left", &["src/shared"], &[]));
        value.peers = vec![claim("node.right", &["src/shared"], &[])];
        value.owner_override = Some(OwnerOverride {
            actor: "owner".to_owned(),
            evidence_refs: vec![],
        });
        assert_eq!(
            evaluate_eligibility(&value).decision,
            EligibilityDecision::Block
        );
        value
            .owner_override
            .as_mut()
            .unwrap()
            .evidence_refs
            .push("ev.owner.approval".to_owned());
        let result = evaluate_eligibility(&value);
        assert_eq!(result.decision, EligibilityDecision::Warn);
        assert!(result.override_applied);
    }

    #[test]
    fn owner_override_cannot_bypass_dependency_or_parallel_gates() {
        let mut value = input(claim("node.left", &["src/left"], &[]));
        value.peers = vec![claim("node.right", &["src/right"], &[])];
        value.dependency_ready = false;
        value.parallel_safe = false;
        value.owner_override = Some(OwnerOverride {
            actor: "owner".to_owned(),
            evidence_refs: vec!["ev.owner.approval".to_owned()],
        });
        let result = evaluate_eligibility(&value);
        assert_eq!(result.decision, EligibilityDecision::Block);
        assert!(!result.override_applied);
        assert!(result
            .reason_codes
            .contains(&REASON_DEPENDENCY_NOT_READY.to_owned()));
        assert!(result
            .reason_codes
            .contains(&REASON_PARALLEL_UNSAFE.to_owned()));
    }

    #[test]
    fn lexical_normalization_exposes_denylist_and_rejects_root_escape() {
        let normalized = evaluate_eligibility(&input(claim(
            "node.state",
            &["src/../.vibehub/state.yaml"],
            &[],
        )));
        assert_eq!(normalized.decision, EligibilityDecision::Block);
        assert_eq!(
            normalized.normalized_scope,
            vec![".vibehub/state.yaml".to_owned()]
        );
        assert!(normalized
            .reason_codes
            .contains(&REASON_DENYLIST_MATCH.to_owned()));

        for path in ["../outside", "/absolute/path", "C:\\absolute\\path"] {
            let result = evaluate_eligibility(&input(claim("node.invalid", &[path], &[])));
            assert_eq!(result.decision, EligibilityDecision::Block);
            assert!(result
                .reason_codes
                .contains(&REASON_SCOPE_PATH_INVALID.to_owned()));
        }
    }

    #[test]
    fn branch_identity_is_slugged_and_stable() {
        assert_eq!(
            branch_name("M5 Worktree Orchestration", "Scope / Lease", "A1B2C3D"),
            "vibehub/m5-worktree-orchestration/scope-lease-a1b2c3d"
        );
    }

    #[test]
    fn reclaim_requires_process_evidence_and_challenge() {
        let lease = Lease {
            lease_id: LeaseId::from("lease.main"),
            worktree_id: WorktreeId::from("worktree.main"),
            owner_session_id: SessionId::from("session.main"),
            generation: 4,
            reclaim_challenge: Some("challenge-5".to_owned()),
        };
        assert_eq!(
            lease.reclaim("challenge-5", false),
            Err("LEASE_OWNER_NOT_CONFIRMED_DEAD")
        );
        assert_eq!(
            lease.reclaim("wrong", true),
            Err("LEASE_RECLAIM_CHALLENGE_MISMATCH")
        );
        let reclaimed = lease.reclaim("challenge-5", true).unwrap();
        assert_eq!(reclaimed.generation, 5);
        assert_eq!(reclaimed.reclaim_challenge, None);
    }
}
