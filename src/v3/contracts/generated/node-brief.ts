/* Generated from contracts/v3. Do not edit directly. */

export type EvidenceRefs = EvidenceRef[];
export type NativePath = {
  [k: string]: unknown;
} & {
  platform: "macos" | "windows" | "linux" | "unknown";
  native: string;
  display: string;
  identity_key: string;
  path_kind?: "absolute" | "drive" | "unc" | "extended" | "relative";
  accessible?: boolean | null;
  symlink_target?: string;
};
export type BlockerDetails = BlockerDetail[];
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface NodeBrief {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  node_id: string | "";
  workflow_profile: "lightweight" | "standard" | "full";
  execution_policy: {
    recommended_profile: "lightweight" | "standard" | "full";
    effective_profile: "lightweight" | "standard" | "full";
    policy_version: number;
    enforcement_epoch: string;
    trigger_reasons: string[];
    override_record?: {
      [k: string]: unknown;
    } | null;
    upgrade_history: {
      [k: string]: unknown;
    }[];
    milestone_policy: "minimal" | "standard" | "full";
    planning_required: boolean;
    review_required: boolean;
    required_records: ("plan" | "session" | "progress" | "result" | "review" | "risk_if_any")[];
  };
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  goal: string;
  scope: string[];
  non_scope: string[];
  dependencies: string[];
  accepted_decisions: string[];
  project_memory: {
    entry_id: string;
    kind: string;
    source: "project_memory";
    revision: number;
    status: string;
    evidence_refs: string[];
    content: string;
    why_injected: string;
    instructional: false;
    security_boundary: "untrusted_data_only";
  }[];
  protocol_records: {
    record: string;
    status: "complete" | "missing";
    repair_action: string | null;
  }[];
  coverage_mode: "enforced" | "legacy_degraded";
  completion_gate: {
    items: {
      gate: string;
      passed: boolean;
      [k: string]: unknown;
    }[];
    [k: string]: unknown;
  };
  research_summary: string[];
  criteria: CriterionSummary[];
  files: NativePath[];
  validation_commands: string[];
  state: "planned" | "ready" | "active" | "blocked" | "review" | "completed" | "cancelled" | "superseded";
  next_intent: string;
  blocker_details?: BlockerDetails;
  execution?: {
    session_ids: string[];
    worktree: WorktreeRef | null;
    base_sha: string | null;
    native_path: NativePath | null;
    dirty: boolean | null;
    integration_state: "not_requested" | "queued" | "integrating" | "integrated" | "conflicted" | "blocked";
    conflict_owner: string | null;
    next_action: string;
  } | null;
  budget: {
    max_tokens: number;
    estimated_tokens: number;
    truncated_sections: string[];
  };
  source_versions: {
    [k: string]: string;
  };
  protocol_coverage: "complete" | "partial" | "gapped" | "unknown";
  evidence_refs: EvidenceRefs;
  warnings: Warnings;
  errors: Errors;
}
export interface CriterionSummary {
  criterion_id: string;
  title: string;
  status: "proposed" | "accepted" | "passed" | "failed" | "blocked" | "not_applicable";
  required: boolean;
  evidence_refs: EvidenceRefs;
}
export interface EvidenceRef {
  evidence_id: string;
  kind: "event" | "file" | "git" | "command" | "test" | "user" | "external";
  grade: "hard_observed" | "agent_reported" | "inferred" | "user_confirmed";
  label_key: string;
  locator: string;
  captured_at?: string;
  excerpt?: string;
}
export interface BlockerDetail {
  blocker_id: string;
  reason_code: string;
  summary: string;
  kind: "external_precondition" | "permission" | "evidence_gap" | "dependency" | "conflict" | "workflow" | "unknown";
  owner: string;
  precondition: string;
  resume_action: string;
  criterion_id?: string;
  node_id?: string;
  evidence_refs: EvidenceRefs;
}
export interface WorktreeRef {
  worktree_id: string;
  lease_id: string | null;
  branch: string;
  state:
    | "planned"
    | "creating"
    | "ready"
    | "active"
    | "dirty"
    | "submitted"
    | "integrating"
    | "integrated"
    | "conflicted"
    | "abandoned"
    | "repairing"
    | "cleaned";
}
export interface Warning {
  code: string;
  severity: "info" | "warning" | "error";
  message_key: string;
  details?: {
    [k: string]: unknown;
  };
  evidence_refs: EvidenceRefs;
}
export interface StructuredError {
  code: string;
  category: "validation" | "not_found" | "permission" | "conflict" | "unsupported" | "internal";
  recoverable: boolean;
  message_key: string;
  retry_after_ms?: number;
  details?: {
    [k: string]: unknown;
  };
  evidence_refs: EvidenceRefs;
}
