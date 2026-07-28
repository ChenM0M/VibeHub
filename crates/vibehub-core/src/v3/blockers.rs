use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const BLOCKER_MODEL_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    ExternalPrecondition,
    Permission,
    EvidenceGap,
    Dependency,
    Conflict,
    Workflow,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairActionKind {
    Command,
    Manual,
    Navigate,
    Retry,
    CollectEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairAction {
    pub action_id: String,
    pub kind: RepairActionKind,
    pub label: String,
    pub instructions: String,
    pub owner: String,
    pub preconditions: Vec<String>,
    pub verification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub copy_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerProvenanceStatus {
    Native,
    Legacy,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockerProvenance {
    pub status: BlockerProvenanceStatus,
    pub source_event_ids: Vec<String>,
    pub reconstructed_fields: Vec<String>,
    pub unknown_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockerDetail {
    pub model_version: String,
    pub blocker_id: String,
    pub kind: BlockerKind,
    pub reason_code: String,
    pub source_type: String,
    pub source_id: String,
    pub summary: String,
    pub why_blocked: String,
    pub expected_state: String,
    pub observed_state: String,
    pub missing_facts: Vec<String>,
    pub impact: String,
    pub owner: String,
    pub preconditions: Vec<String>,
    pub repair_actions: Vec<RepairAction>,
    pub evidence_refs: Vec<Value>,
    pub freshness: String,
    pub provenance: BlockerProvenance,
    // Compatibility aliases for V3 1.0 consumers. New consumers use the arrays above.
    pub precondition: String,
    pub resume_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criterion_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

impl BlockerDetail {
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("model_version", self.model_version.as_str()),
            ("blocker_id", self.blocker_id.as_str()),
            ("reason_code", self.reason_code.as_str()),
            ("source_type", self.source_type.as_str()),
            ("source_id", self.source_id.as_str()),
            ("summary", self.summary.as_str()),
            ("why_blocked", self.why_blocked.as_str()),
            ("expected_state", self.expected_state.as_str()),
            ("observed_state", self.observed_state.as_str()),
            ("impact", self.impact.as_str()),
            ("owner", self.owner.as_str()),
            ("freshness", self.freshness.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("blocker detail field '{field}' must not be empty"));
            }
        }
        if self.preconditions.is_empty() || self.repair_actions.is_empty() {
            return Err("blocker detail requires preconditions and repair_actions".to_owned());
        }
        if self
            .repair_actions
            .iter()
            .any(|action| action.label.trim().is_empty() || action.instructions.trim().is_empty())
        {
            return Err("repair action label/instructions must not be empty".to_owned());
        }
        Ok(())
    }

    pub fn into_value(self) -> Value {
        debug_assert!(self.validate().is_ok());
        serde_json::to_value(self).expect("BlockerDetail is serializable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail() -> BlockerDetail {
        BlockerDetail {
            model_version: BLOCKER_MODEL_VERSION.to_owned(),
            blocker_id: "blocker.test".to_owned(),
            kind: BlockerKind::EvidenceGap,
            reason_code: "criterion.blocked".to_owned(),
            source_type: "criterion".to_owned(),
            source_id: "criterion.task.test.c01".to_owned(),
            summary: "Windows 原生验收尚未完成".to_owned(),
            why_blocked: "发布产物尚未在 Windows 主机执行必需检查".to_owned(),
            expected_state: "A07/E07/F07-F10/G05 passed".to_owned(),
            observed_state: "A07/E07/F07-F10/G05 blocked".to_owned(),
            missing_facts: vec!["Windows host results".to_owned()],
            impact: "发布任务不能完成".to_owned(),
            owner: "Windows 验收执行者".to_owned(),
            preconditions: vec!["取得精确 release artifact 与 hash".to_owned()],
            repair_actions: vec![RepairAction {
                action_id: "repair.blocker.test".to_owned(),
                kind: RepairActionKind::Manual,
                label: "执行 Windows 验收".to_owned(),
                instructions: "在 Windows 主机运行清单并记录 evidence".to_owned(),
                owner: "Windows 验收执行者".to_owned(),
                preconditions: vec!["校验 artifact hash".to_owned()],
                verification: "criterion review passed".to_owned(),
                command: None,
                target: Some("criterion.task.test.c01".to_owned()),
                copy_text: "执行 A07/E07/F07-F10/G05".to_owned(),
            }],
            evidence_refs: Vec::new(),
            freshness: "fresh".to_owned(),
            provenance: BlockerProvenance {
                status: BlockerProvenanceStatus::Native,
                source_event_ids: Vec::new(),
                reconstructed_fields: Vec::new(),
                unknown_fields: Vec::new(),
            },
            precondition: "取得精确 release artifact 与 hash".to_owned(),
            resume_action: "执行 Windows 验收".to_owned(),
            criterion_id: Some("criterion.task.test.c01".to_owned()),
            node_id: None,
        }
    }

    #[test]
    fn typed_blocker_requires_actionable_context() {
        assert_eq!(detail().validate(), Ok(()));
        let mut invalid = detail();
        invalid.repair_actions.clear();
        assert_eq!(
            invalid.validate(),
            Err("blocker detail requires preconditions and repair_actions".to_owned())
        );
    }

    #[test]
    fn typed_blocker_serializes_version_and_provenance() {
        let value = detail().into_value();
        assert_eq!(value["model_version"], "1.0");
        assert_eq!(value["source_type"], "criterion");
        assert_eq!(value["provenance"]["status"], "native");
        assert_eq!(value["repair_actions"][0]["kind"], "manual");
    }
}
