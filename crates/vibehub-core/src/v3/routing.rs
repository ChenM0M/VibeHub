use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskRouteKind {
    Continue,
    Extend,
    RelatedNew,
    New,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRouteCandidate {
    pub task_id: String,
    pub title: String,
    pub intent: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRouteDecision {
    pub kind: TaskRouteKind,
    pub candidate_task_id: Option<String>,
    pub confidence: u8,
    pub rationale: String,
}

pub fn route_task(new_intent: &str, candidates: &[TaskRouteCandidate]) -> TaskRouteDecision {
    let new_tokens = tokens(new_intent);
    let Some((candidate, score)) = candidates
        .iter()
        .filter(|c| {
            !matches!(
                c.state.as_str(),
                "completed" | "cancelled" | "closed_with_exceptions"
            )
        })
        .map(|c| {
            (
                c,
                overlap(&new_tokens, &tokens(&format!("{} {}", c.title, c.intent))),
            )
        })
        .max_by(|(_, left), (_, right)| left.partial_cmp(right).unwrap_or(Ordering::Equal))
    else {
        return TaskRouteDecision {
            kind: TaskRouteKind::New,
            candidate_task_id: None,
            confidence: 100,
            rationale: "no active candidate task".to_owned(),
        };
    };
    if score < 0.25 {
        return TaskRouteDecision {
            kind: TaskRouteKind::New,
            candidate_task_id: None,
            confidence: (score * 100.0) as u8,
            rationale: "no substantial intent overlap".to_owned(),
        };
    }
    let confidence = (score.min(1.0) * 100.0) as u8;
    let kind = if score >= 0.75 {
        TaskRouteKind::Continue
    } else if score >= 0.45 {
        TaskRouteKind::Extend
    } else {
        TaskRouteKind::RelatedNew
    };
    TaskRouteDecision {
        kind,
        candidate_task_id: Some(candidate.task_id.clone()),
        confidence,
        rationale: format!("intent token overlap {:.0}%", score * 100.0),
    }
}

fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2)
        .map(str::to_lowercase)
        .collect()
}
fn overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f32 {
    if left.is_empty() {
        return 0.0;
    }
    left.intersection(right).count() as f32 / left.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_request_creates_new_task() {
        let d = route_task(
            "write a billing export",
            &[TaskRouteCandidate {
                task_id: "task.ui".into(),
                title: "Fix window chrome".into(),
                intent: "change native titlebar".into(),
                state: "active".into(),
            }],
        );
        assert_eq!(d.kind, TaskRouteKind::New);
    }

    #[test]
    fn strong_overlap_continues_and_medium_overlap_extends() {
        let candidates = [TaskRouteCandidate {
            task_id: "task.ui".into(),
            title: "Improve V3 task routing".into(),
            intent: "route related task requests safely".into(),
            state: "active".into(),
        }];
        assert_eq!(
            route_task("Improve V3 task routing", &candidates).kind,
            TaskRouteKind::Continue
        );
        assert_eq!(
            route_task("Improve routing task interface review", &candidates).kind,
            TaskRouteKind::Extend
        );
    }

    #[test]
    fn terminal_candidates_are_never_reused() {
        let d = route_task(
            "ship the same task",
            &[TaskRouteCandidate {
                task_id: "task.done".into(),
                title: "ship the same task".into(),
                intent: "ship it".into(),
                state: "completed".into(),
            }],
        );
        assert_eq!(d.kind, TaskRouteKind::New);
        assert_eq!(d.candidate_task_id, None);
    }
}
