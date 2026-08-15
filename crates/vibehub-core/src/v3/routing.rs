//! Agent/host-independent Session–Task binding and lightweight routing.
//!
//! The routing layer is deliberately deterministic. It never treats the
//! project current pointer or the UI selected task as a session binding, and
//! it never turns a fuzzy match into a write target when the evidence is not
//! unique.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const SESSION_TASK_ROUTING_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingStatus {
    Unbound,
    Bound,
    Stale,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingFreshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingSource {
    ExplicitTaskId,
    ExplicitTitle,
    CurrentSession,
    CreatedAndStart,
    UniqueCandidate,
    UserConfirmed,
    LegacySessionOpen,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAction {
    Continue,
    Bind,
    Ask,
    New,
}

/// Backwards-compatible Rust name retained for callers that used the small
/// pre-binding router. The serialized contract is the four-action set above.
pub type TaskRouteKind = RouteAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteTrigger {
    OrdinaryContinuation,
    ExplicitTask,
    ExplicitSwitch,
    NewExecution,
    BindingInvalid,
    BindingStale,
    ScopeConflict,
    AmbiguousCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostCapabilityState {
    Known,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub state: HostCapabilityState,
    #[serde(default)]
    pub supports_session_binding: bool,
    #[serde(default)]
    pub supports_binding_preconditions: bool,
    #[serde(default)]
    pub provider: Option<String>,
}

impl Default for HostCapabilities {
    fn default() -> Self {
        Self {
            state: HostCapabilityState::Unknown,
            supports_session_binding: false,
            supports_binding_preconditions: false,
            provider: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTaskIdentity {
    pub project_id: String,
    pub interaction_id: String,
    pub session_id: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTaskBinding {
    pub schema_version: String,
    pub project_id: String,
    pub interaction_id: String,
    pub session_id: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub bound_task_id: Option<String>,
    pub binding_revision: u64,
    pub freshness: BindingFreshness,
    pub source: BindingSource,
    pub status: BindingStatus,
    #[serde(default)]
    pub target_task_revision: Option<u64>,
    #[serde(default)]
    pub bound_at: Option<String>,
}

impl SessionTaskBinding {
    pub fn unbound(identity: &SessionTaskIdentity) -> Self {
        Self {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.to_owned(),
            project_id: identity.project_id.clone(),
            interaction_id: identity.interaction_id.clone(),
            session_id: identity.session_id.clone(),
            agent_id: identity.agent_id.clone(),
            host: identity.host.clone(),
            bound_task_id: None,
            binding_revision: 0,
            freshness: BindingFreshness::Unknown,
            source: BindingSource::Unknown,
            status: BindingStatus::Unbound,
            target_task_revision: None,
            bound_at: None,
        }
    }

    pub fn is_valid_for(&self, identity: &SessionTaskIdentity, task_id: &str) -> bool {
        self.schema_version == SESSION_TASK_ROUTING_SCHEMA_VERSION
            && self.project_id == identity.project_id
            && self.interaction_id == identity.interaction_id
            && self.session_id == identity.session_id
            && self.bound_task_id.as_deref() == Some(task_id)
            && self.binding_revision > 0
            && self.status == BindingStatus::Bound
            && self.freshness == BindingFreshness::Fresh
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRouteCandidate {
    pub task_id: String,
    pub title: String,
    pub intent: String,
    pub state: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub is_current_default: bool,
    #[serde(default)]
    pub is_ui_selected: bool,
    #[serde(default)]
    pub active_session_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteOption {
    pub task_id: String,
    pub title: String,
    pub state: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteRequest {
    pub schema_version: String,
    pub identity: SessionTaskIdentity,
    pub intent: String,
    pub trigger: RouteTrigger,
    #[serde(default)]
    pub explicit_task_id: Option<String>,
    #[serde(default)]
    pub explicit_task_title: Option<String>,
    #[serde(default)]
    pub current_binding: Option<SessionTaskBinding>,
    #[serde(default)]
    pub just_created_task_id: Option<String>,
    #[serde(default)]
    pub explicit_start: bool,
    #[serde(default)]
    pub candidates: Vec<TaskRouteCandidate>,
    /// Display/default metadata only; neither value affects binding.
    #[serde(default)]
    pub project_current_default_task_id: Option<String>,
    #[serde(default)]
    pub ui_selected_task_id: Option<String>,
    #[serde(default)]
    pub host_capabilities: HostCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRouteDecision {
    pub schema_version: String,
    pub project_id: String,
    pub interaction_id: String,
    pub session_id: String,
    pub action: RouteAction,
    /// Retained for clients generated against the original router.
    pub kind: RouteAction,
    pub binding_status: BindingStatus,
    #[serde(default)]
    pub candidate_task_id: Option<String>,
    #[serde(default)]
    pub selected_task_id: Option<String>,
    #[serde(default)]
    pub binding_source: Option<BindingSource>,
    #[serde(default)]
    pub binding_revision: Option<u64>,
    pub confidence: u8,
    pub trigger: RouteTrigger,
    #[serde(default)]
    pub options: Vec<RouteOption>,
    #[serde(default)]
    pub confirmation_question: Option<String>,
    #[serde(default)]
    pub diagnostic_code: Option<String>,
    pub reroute_performed: bool,
    pub rationale: String,
}

impl TaskRouteDecision {
    fn new(request: &RouteRequest, action: RouteAction, status: BindingStatus) -> Self {
        Self {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.to_owned(),
            project_id: request.identity.project_id.clone(),
            interaction_id: request.identity.interaction_id.clone(),
            session_id: request.identity.session_id.clone(),
            action,
            kind: action,
            binding_status: status,
            candidate_task_id: None,
            selected_task_id: None,
            binding_source: None,
            binding_revision: None,
            confidence: 0,
            trigger: request.trigger,
            options: Vec::new(),
            confirmation_question: None,
            diagnostic_code: None,
            reroute_performed: !matches!(request.trigger, RouteTrigger::OrdinaryContinuation),
            rationale: String::new(),
        }
    }

    fn with_task(
        mut self,
        task_id: String,
        source: BindingSource,
        confidence: u8,
        rationale: impl Into<String>,
    ) -> Self {
        self.candidate_task_id = Some(task_id.clone());
        self.selected_task_id = Some(task_id);
        self.binding_source = Some(source);
        self.confidence = confidence;
        self.rationale = rationale.into();
        self
    }

    fn with_binding_revision(mut self, revision: u64) -> Self {
        self.binding_revision = Some(revision);
        self
    }
}

pub fn route_session_task(request: &RouteRequest) -> TaskRouteDecision {
    let active = request
        .candidates
        .iter()
        .filter(|candidate| {
            !is_terminal_state(&candidate.state)
                && candidate
                    .project_id
                    .as_deref()
                    .is_none_or(|project_id| project_id == request.identity.project_id)
        })
        .collect::<Vec<_>>();

    if request.schema_version != SESSION_TASK_ROUTING_SCHEMA_VERSION {
        return ask_with_status(
            request,
            &active,
            BindingStatus::Invalid,
            "V3_TASK_ROUTE_SCHEMA_UNSUPPORTED",
            "当前路由契约版本不受支持，请重新读取候选并重试。",
            "route request schema version is not supported",
        );
    }

    // A host that cannot prove binding support must never silently enter a
    // task-scoped write path. Read-only callers can still inspect candidates,
    // but their routing result is an explicit fail-closed ask.
    if !host_can_prove_binding(&request.host_capabilities) {
        let (diagnostic_code, rationale) =
            if request.host_capabilities.state == HostCapabilityState::Unknown {
                (
                    "V3_HOST_BINDING_CAPABILITY_UNKNOWN",
                    "host binding capability is unknown; fail closed",
                )
            } else {
                (
                    "V3_HOST_BINDING_CAPABILITY_UNSUPPORTED",
                    "host binding or precondition capability is not available; fail closed",
                )
            };
        return ask_with_status(
            request,
            &active,
            BindingStatus::Invalid,
            diagnostic_code,
            "当前宿主无法证明 Session–Task 绑定能力，请先确认只读检查、重新选择 Task 或新建 Task。",
            rationale,
        );
    }

    if request.explicit_task_id.is_none()
        && request.explicit_task_title.is_none()
        && matches!(request.trigger, RouteTrigger::OrdinaryContinuation)
    {
        if let Some(binding) = request.current_binding.as_ref() {
            if binding_is_usable(binding, request, &active) {
                let task_id = binding.bound_task_id.clone().unwrap_or_default();
                return TaskRouteDecision::new(
                    request,
                    RouteAction::Continue,
                    BindingStatus::Bound,
                )
                .with_task(
                    task_id,
                    BindingSource::CurrentSession,
                    100,
                    "reuse the current fresh session binding without rescanning candidates",
                )
                .with_binding_revision(binding.binding_revision);
            }
            if let Some(status) = unusable_binding_status(binding, request, &active) {
                return ask_with_status(
                    request,
                    &active,
                    status,
                    "V3_TASK_BINDING_REQUIRED",
                    "当前 Session 的 Task 绑定已失效，请确认继续哪个 Task。",
                    "the previous session binding is no longer usable",
                );
            }
        }
    }

    if let Some(task_id) = request.explicit_task_id.as_deref() {
        if active.iter().any(|candidate| candidate.task_id == task_id) {
            return TaskRouteDecision::new(request, RouteAction::Bind, BindingStatus::Bound)
                .with_task(
                    task_id.to_owned(),
                    BindingSource::ExplicitTaskId,
                    100,
                    "explicit task_id has highest routing priority",
                );
        }
        return ask(
            request,
            &active,
            "V3_TASK_BINDING_MISMATCH",
            "指定的 Task 不存在或已进入终态，请从候选中重新选择一个 Task。",
            "explicit task_id is not an active project task",
        );
    }

    if let Some(title) = request.explicit_task_title.as_deref() {
        let matching = active
            .iter()
            .filter(|candidate| normalize(&candidate.title) == normalize(title))
            .copied()
            .collect::<Vec<_>>();
        if matching.len() == 1 {
            return TaskRouteDecision::new(request, RouteAction::Bind, BindingStatus::Bound)
                .with_task(
                    matching[0].task_id.clone(),
                    BindingSource::ExplicitTitle,
                    100,
                    "explicit task title uniquely identifies an active task",
                );
        }
        return ask(
            request,
            &active,
            "V3_TASK_BINDING_AMBIGUOUS",
            "这个 Task 标题不唯一，请确认要绑定哪一个候选。",
            "explicit title is missing or matches multiple active tasks",
        );
    }

    if request.explicit_start {
        if let Some(task_id) = request.just_created_task_id.as_deref() {
            if active.iter().any(|candidate| candidate.task_id == task_id) {
                return TaskRouteDecision::new(request, RouteAction::Bind, BindingStatus::Bound)
                    .with_task(
                        task_id.to_owned(),
                        BindingSource::CreatedAndStart,
                        100,
                        "the same interaction explicitly requested create-and-start",
                    );
            }
        }
    }

    if let Some(binding) = request.current_binding.as_ref() {
        if let Some(status) = unusable_binding_status(binding, request, &active) {
            return ask_with_status(
                request,
                &active,
                status,
                "V3_TASK_BINDING_REQUIRED",
                "当前 Session 的 Task 绑定已失效，请确认继续哪个 Task。",
                "the previous session binding is no longer usable",
            );
        }
    }

    if matches!(
        request.trigger,
        RouteTrigger::BindingInvalid | RouteTrigger::BindingStale
    ) {
        let status = if request.trigger == RouteTrigger::BindingInvalid {
            BindingStatus::Invalid
        } else {
            BindingStatus::Stale
        };
        return ask_with_status(
            request,
            &active,
            status,
            "V3_TASK_BINDING_REQUIRED",
            "当前 Session 的 Task 绑定已失效，请确认继续哪个 Task。",
            "the previous session binding is no longer usable",
        );
    }

    if contains_cjk(&request.intent) {
        return ask(
            request,
            &active,
            "V3_TASK_BINDING_AMBIGUOUS",
            "这条中文需求缺少唯一 Task 证据，请确认继续哪个 Task，或选择新建。",
            "Chinese or cross-language intent is not strong enough for implicit binding",
        );
    }

    let ranked = rank(&request.intent, &active);
    if let Some((candidate, score)) = ranked.first().copied() {
        let close = ranked
            .get(1)
            .is_some_and(|(_, other)| score - *other < 0.15);
        if !close && is_high_confidence(&request.intent, candidate, score) {
            return TaskRouteDecision::new(request, RouteAction::Bind, BindingStatus::Bound)
                .with_task(
                    candidate.task_id.clone(),
                    BindingSource::UniqueCandidate,
                    percentage(score),
                    format!(
                        "unique high-confidence candidate score {:.0}%",
                        score * 100.0
                    ),
                );
        }
    }

    let no_matching_candidate = ranked.first().is_none_or(|(_, score)| *score < 0.25);
    if matches!(request.trigger, RouteTrigger::NewExecution) && no_matching_candidate {
        let mut decision =
            TaskRouteDecision::new(request, RouteAction::New, BindingStatus::Unbound);
        decision.rationale =
            "no unique active task candidate; independent execution requests create a new task"
                .to_owned();
        decision.options = options(&active, &ranked);
        return decision;
    }

    ask(
        request,
        &active,
        "V3_TASK_BINDING_AMBIGUOUS",
        "无法唯一确定目标 Task，请确认继续哪个候选，或选择新建。",
        "candidate evidence is absent, close, or below the high-confidence threshold",
    )
}

fn ask(
    request: &RouteRequest,
    active: &[&TaskRouteCandidate],
    code: &str,
    question: &str,
    rationale: &str,
) -> TaskRouteDecision {
    ask_with_status(
        request,
        active,
        BindingStatus::Unbound,
        code,
        question,
        rationale,
    )
}

fn ask_with_status(
    request: &RouteRequest,
    active: &[&TaskRouteCandidate],
    binding_status: BindingStatus,
    code: &str,
    question: &str,
    rationale: &str,
) -> TaskRouteDecision {
    let ranked = rank(&request.intent, active);
    let mut decision = TaskRouteDecision::new(request, RouteAction::Ask, binding_status);
    decision.diagnostic_code = Some(code.to_owned());
    decision.confirmation_question = Some(question.to_owned());
    decision.options = options(active, &ranked);
    decision.rationale = rationale.to_owned();
    decision
}

fn options(
    active: &[&TaskRouteCandidate],
    ranked: &[(&TaskRouteCandidate, f32)],
) -> Vec<RouteOption> {
    let source = if ranked.is_empty() {
        let mut sorted = active.to_vec();
        sorted.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        sorted
            .into_iter()
            .map(|candidate| (candidate, 0.0))
            .collect::<Vec<_>>()
    } else {
        ranked.to_vec()
    };
    source
        .into_iter()
        .take(3)
        .map(|(candidate, score)| RouteOption {
            task_id: candidate.task_id.clone(),
            title: candidate.title.clone(),
            state: candidate.state.clone(),
            confidence: percentage(score),
        })
        .collect()
}

fn rank<'a>(
    intent: &str,
    candidates: &[&'a TaskRouteCandidate],
) -> Vec<(&'a TaskRouteCandidate, f32)> {
    let intent_normalized = normalize(intent);
    let left = tokens(intent);
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            let title = normalize(&candidate.title);
            let description = format!("{} {}", candidate.title, candidate.intent);
            let mut score = overlap(&left, &tokens(&description));
            if !intent_normalized.is_empty() && intent_normalized == title {
                score = 1.0;
            }
            (*candidate, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(
        |(left_candidate, left_score), (right_candidate, right_score)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left_candidate.task_id.cmp(&right_candidate.task_id))
        },
    );
    ranked
}

fn percentage(score: f32) -> u8 {
    (score.clamp(0.0, 1.0) * 100.0).round() as u8
}

fn is_terminal_state(state: &str) -> bool {
    matches!(
        state,
        "completed" | "cancelled" | "closed_with_exceptions" | "superseded"
    )
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn contains_cjk(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character as u32,
            0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff
        )
    })
}

fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(str::to_lowercase)
        .collect()
}

fn overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f32 {
    if left.is_empty() {
        return 0.0;
    }
    left.intersection(right).count() as f32 / left.len() as f32
}

fn host_can_prove_binding(capabilities: &HostCapabilities) -> bool {
    capabilities.state == HostCapabilityState::Known
        && capabilities.supports_session_binding
        && capabilities.supports_binding_preconditions
}

fn binding_is_usable(
    binding: &SessionTaskBinding,
    request: &RouteRequest,
    active: &[&TaskRouteCandidate],
) -> bool {
    unusable_binding_status(binding, request, active).is_none()
        && binding.status == BindingStatus::Bound
        && binding.freshness == BindingFreshness::Fresh
}

fn unusable_binding_status(
    binding: &SessionTaskBinding,
    request: &RouteRequest,
    active: &[&TaskRouteCandidate],
) -> Option<BindingStatus> {
    if binding.schema_version != SESSION_TASK_ROUTING_SCHEMA_VERSION
        || binding.project_id != request.identity.project_id
        || binding.interaction_id != request.identity.interaction_id
        || binding.session_id != request.identity.session_id
    {
        return Some(BindingStatus::Invalid);
    }
    if binding.status == BindingStatus::Invalid {
        return Some(BindingStatus::Invalid);
    }
    if binding.status == BindingStatus::Stale || binding.freshness == BindingFreshness::Stale {
        return Some(BindingStatus::Stale);
    }
    if binding.status != BindingStatus::Bound {
        return None;
    }
    if binding.freshness != BindingFreshness::Fresh
        || binding.binding_revision == 0
        || binding.bound_task_id.is_none()
        || (!request.candidates.is_empty()
            && !active.iter().any(|candidate| {
                Some(candidate.task_id.as_str()) == binding.bound_task_id.as_deref()
            }))
    {
        return Some(BindingStatus::Stale);
    }
    None
}

fn is_high_confidence(intent: &str, candidate: &TaskRouteCandidate, score: f32) -> bool {
    if score < 0.75 {
        return false;
    }
    let normalized_intent = normalize(intent);
    if !normalized_intent.is_empty() && normalized_intent == normalize(&candidate.title) {
        return true;
    }
    let intent_tokens = tokens(intent);
    let description = format!("{} {}", candidate.title, candidate.intent);
    intent_tokens.len() >= 2 && intent_tokens.intersection(&tokens(&description)).count() >= 2
}

/// Compatibility helper for older callers that only provide intent and active
/// candidates. New callers should construct `RouteRequest` explicitly.
pub fn route_task(new_intent: &str, candidates: &[TaskRouteCandidate]) -> TaskRouteDecision {
    route_session_task(&RouteRequest {
        schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.to_owned(),
        identity: SessionTaskIdentity {
            project_id: "project.unknown".to_owned(),
            interaction_id: "interaction.unknown".to_owned(),
            session_id: "session.unknown".to_owned(),
            agent_id: None,
            host: None,
        },
        intent: new_intent.to_owned(),
        trigger: RouteTrigger::NewExecution,
        explicit_task_id: None,
        explicit_task_title: None,
        current_binding: None,
        just_created_task_id: None,
        explicit_start: false,
        candidates: candidates.to_vec(),
        project_current_default_task_id: None,
        ui_selected_task_id: None,
        host_capabilities: HostCapabilities {
            state: HostCapabilityState::Known,
            supports_session_binding: true,
            supports_binding_preconditions: true,
            provider: Some("compatibility".to_owned()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> SessionTaskIdentity {
        SessionTaskIdentity {
            project_id: "project.test".into(),
            interaction_id: "interaction.test".into(),
            session_id: "session.test".into(),
            agent_id: Some("codex".into()),
            host: Some("codex".into()),
        }
    }

    fn candidate(task_id: &str, title: &str, intent: &str) -> TaskRouteCandidate {
        TaskRouteCandidate {
            task_id: task_id.into(),
            title: title.into(),
            intent: intent.into(),
            state: "active".into(),
            project_id: Some("project.test".into()),
            is_current_default: false,
            is_ui_selected: false,
            active_session_count: 0,
        }
    }

    fn request(intent: &str, candidates: Vec<TaskRouteCandidate>) -> RouteRequest {
        RouteRequest {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.into(),
            identity: identity(),
            intent: intent.into(),
            trigger: RouteTrigger::NewExecution,
            explicit_task_id: None,
            explicit_task_title: None,
            current_binding: None,
            just_created_task_id: None,
            explicit_start: false,
            candidates,
            project_current_default_task_id: None,
            ui_selected_task_id: None,
            host_capabilities: HostCapabilities {
                state: HostCapabilityState::Known,
                supports_session_binding: true,
                supports_binding_preconditions: true,
                provider: Some("test".into()),
            },
        }
    }

    fn binding(
        status: BindingStatus,
        freshness: BindingFreshness,
        task_id: &str,
    ) -> SessionTaskBinding {
        SessionTaskBinding {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.into(),
            project_id: "project.test".into(),
            interaction_id: "interaction.test".into(),
            session_id: "session.test".into(),
            agent_id: Some("codex".into()),
            host: Some("codex".into()),
            bound_task_id: Some(task_id.into()),
            binding_revision: 4,
            freshness,
            source: BindingSource::ExplicitTaskId,
            status,
            target_task_revision: Some(8),
            bound_at: None,
        }
    }

    #[test]
    fn explicit_task_id_wins_over_default_and_ui_selection() {
        let mut request = request("unrelated", vec![candidate("task.a", "A", "a")]);
        request.explicit_task_id = Some("task.a".into());
        request.project_current_default_task_id = Some("task.other".into());
        request.ui_selected_task_id = Some("task.other".into());
        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Bind);
        assert_eq!(decision.selected_task_id.as_deref(), Some("task.a"));
    }

    #[test]
    fn valid_binding_continues_without_reroute() {
        let mut request = request("ordinary follow up", vec![]);
        request.trigger = RouteTrigger::OrdinaryContinuation;
        request.current_binding = Some(SessionTaskBinding {
            schema_version: SESSION_TASK_ROUTING_SCHEMA_VERSION.into(),
            project_id: "project.test".into(),
            interaction_id: "interaction.test".into(),
            session_id: "session.test".into(),
            agent_id: Some("codex".into()),
            host: Some("codex".into()),
            bound_task_id: Some("task.bound".into()),
            binding_revision: 4,
            freshness: BindingFreshness::Fresh,
            source: BindingSource::ExplicitTaskId,
            status: BindingStatus::Bound,
            target_task_revision: Some(8),
            bound_at: None,
        });
        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Continue);
        assert!(!decision.reroute_performed);
        assert_eq!(decision.selected_task_id.as_deref(), Some("task.bound"));
        assert_eq!(decision.binding_revision, Some(4));
    }

    #[test]
    fn stale_binding_asks_before_reusing_a_candidate() {
        let mut request = request(
            "follow up on routing",
            vec![candidate(
                "task.bound",
                "Routing follow-up",
                "follow up on routing",
            )],
        );
        request.trigger = RouteTrigger::OrdinaryContinuation;
        request.current_binding = Some(binding(
            BindingStatus::Stale,
            BindingFreshness::Stale,
            "task.bound",
        ));

        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(decision.binding_status, BindingStatus::Stale);
        assert_eq!(
            decision.diagnostic_code.as_deref(),
            Some("V3_TASK_BINDING_REQUIRED")
        );
    }

    #[test]
    fn binding_to_a_terminal_task_cannot_continue() {
        let mut done = candidate("task.done", "Finished routing", "follow up on routing");
        done.state = "completed".into();
        let mut request = request("follow up on routing", vec![done]);
        request.trigger = RouteTrigger::OrdinaryContinuation;
        request.current_binding = Some(binding(
            BindingStatus::Bound,
            BindingFreshness::Fresh,
            "task.done",
        ));

        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(decision.binding_status, BindingStatus::Stale);
    }

    #[test]
    fn create_and_start_binds_the_task_created_by_the_same_interaction() {
        let mut request = request(
            "start the new routing task",
            vec![candidate(
                "task.new",
                "New routing task",
                "start the new routing task",
            )],
        );
        request.explicit_start = true;
        request.just_created_task_id = Some("task.new".into());
        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Bind);
        assert_eq!(
            decision.binding_source,
            Some(BindingSource::CreatedAndStart)
        );
        assert_eq!(decision.selected_task_id.as_deref(), Some("task.new"));
    }

    #[test]
    fn close_candidates_ask_before_writing() {
        let decision = route_session_task(&request(
            "improve task routing interface",
            vec![
                candidate("task.a", "Improve task routing", "interface safety"),
                candidate("task.b", "Improve task routing", "interface safety"),
            ],
        ));
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(decision.options.len(), 2);
        assert!(decision.confirmation_question.is_some());
    }

    #[test]
    fn chinese_inference_is_never_silent() {
        let decision = route_session_task(&request(
            "修复任务路由",
            vec![candidate("task.a", "Fix task routing", "route task safely")],
        ));
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(
            decision.diagnostic_code.as_deref(),
            Some("V3_TASK_BINDING_AMBIGUOUS")
        );
    }

    #[test]
    fn unknown_host_capability_fails_closed() {
        let mut request = request("ship it", vec![candidate("task.a", "Ship it", "release")]);
        request.host_capabilities = HostCapabilities::default();
        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(
            decision.diagnostic_code.as_deref(),
            Some("V3_HOST_BINDING_CAPABILITY_UNKNOWN")
        );
    }

    #[test]
    fn known_host_without_binding_preconditions_also_fails_closed() {
        let mut request = request(
            "ship the release package",
            vec![candidate(
                "task.a",
                "Release package",
                "ship the release package",
            )],
        );
        request.host_capabilities.supports_binding_preconditions = false;
        let decision = route_session_task(&request);
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(
            decision.diagnostic_code.as_deref(),
            Some("V3_HOST_BINDING_CAPABILITY_UNSUPPORTED")
        );
    }

    #[test]
    fn foreign_candidates_are_never_considered_for_binding() {
        let mut foreign = candidate(
            "task.foreign",
            "Routing contract",
            "repair routing contract",
        );
        foreign.project_id = Some("project.other".into());
        let decision = route_session_task(&request("repair routing contract", vec![foreign]));
        assert_eq!(decision.action, RouteAction::New);
        assert_eq!(decision.selected_task_id, None);
    }

    #[test]
    fn a_single_generic_token_is_not_high_confidence() {
        let decision = route_session_task(&request(
            "routing",
            vec![candidate(
                "task.routing",
                "Routing work",
                "routing implementation",
            )],
        ));
        assert_eq!(decision.action, RouteAction::Ask);
        assert_eq!(decision.selected_task_id, None);
    }

    #[test]
    fn ask_options_are_stable_when_candidates_arrive_in_a_different_order() {
        let mut first = request(
            "",
            vec![candidate("task.b", "B", ""), candidate("task.a", "A", "")],
        );
        first.trigger = RouteTrigger::AmbiguousCandidate;
        let mut second = request(
            "",
            vec![candidate("task.a", "A", ""), candidate("task.b", "B", "")],
        );
        second.trigger = RouteTrigger::AmbiguousCandidate;

        let first_options = route_session_task(&first).options;
        let second_options = route_session_task(&second).options;
        assert_eq!(first_options, second_options);
        assert_eq!(
            first_options
                .iter()
                .map(|option| option.task_id.as_str())
                .collect::<Vec<_>>(),
            vec!["task.a", "task.b"]
        );
    }

    #[test]
    fn terminal_candidates_are_never_reused() {
        let mut done = candidate("task.done", "ship the same task", "ship it");
        done.state = "completed".into();
        let decision = route_task("ship the same task", &[done]);
        assert_eq!(decision.action, RouteAction::New);
        assert_eq!(decision.candidate_task_id, None);
    }
}
