use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

stable_id!(ProjectId);
stable_id!(TaskId);
stable_id!(NodeId);
stable_id!(SessionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceGrade {
    HardObserved,
    AgentReported,
    Inferred,
    UserConfirmed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3EventEnvelope {
    pub event_id: String,
    pub event_type: String,
    pub event_version: String,
    pub aggregate_id: String,
    pub aggregate_version: u64,
    pub expected_version: u64,
    pub idempotency_key: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub actor: String,
    pub evidence_grade: EvidenceGrade,
    pub occurred_at: String,
    pub recorded_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventDraft {
    pub event_type: String,
    pub aggregate_id: String,
    pub expected_version: u64,
    pub idempotency_key: String,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub node_id: Option<NodeId>,
    pub session_id: Option<SessionId>,
    pub actor: String,
    pub evidence_grade: EvidenceGrade,
    pub occurred_at: Option<String>,
    pub commit_sha: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V3ErrorCategory {
    Validation,
    NotFound,
    ScopeMismatch,
    VersionConflict,
    Duplicate,
    PermissionDenied,
    StaleResource,
    CorruptLog,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V3Error {
    pub code: String,
    pub category: V3ErrorCategory,
    pub retryable: bool,
    pub message: String,
    #[serde(default)]
    pub details: Map<String, Value>,
}

impl std::fmt::Display for V3Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for V3Error {}

impl V3Error {
    pub fn new(
        code: &str,
        category: V3ErrorCategory,
        retryable: bool,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_owned(),
            category,
            retryable,
            message: message.into(),
            details: Map::new(),
        }
    }

    pub fn with_detail(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.details.insert(key.to_owned(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AppendResult {
    Appended { event: V3EventEnvelope },
    Duplicate { event: V3EventEnvelope },
}
