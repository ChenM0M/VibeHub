/* Generated from contracts/v3. Do not edit directly. */

export type EvidenceRefs = EvidenceRef[];
export type BlockerDetails = BlockerDetail[];
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface TaskTimelineView {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  title: string;
  state: "planned" | "active" | "blocked" | "review" | "completed" | "cancelled";
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  criteria: CriterionSummary[];
  blocker_details?: BlockerDetails;
  completion: {
    proposal_event_id: string | null;
    proposed_at_version: number | null;
    digest: string | null;
    valid: boolean;
    confirmed: boolean;
    confirmed_at_version: number | null;
    confirmed_by: string | null;
    channel: string | null;
  };
  lanes: {
    lane_id: string;
    kind: "task" | "session" | "node" | "worktree";
    label: string;
    state: "open" | "active" | "idle" | "closed" | "gapped" | "abandoned" | "repaired";
    worktree_id?: string | null;
  }[];
  events: {
    timeline_event_id: string;
    kind:
      | "decision"
      | "evidence"
      | "finding"
      | "attempt"
      | "validation"
      | "confirmation"
      | "gap"
      | "session"
      | "plan"
      | "lease"
      | "worktree"
      | "integration"
      | "conflict"
      | "recovery";
    occurred_at: string;
    recorded_at: string;
    order_state: "ordered" | "late" | "reordered";
    lane_id: string;
    actor: string;
    tool?: string | null;
    node_id?: string | null;
    session_id?: string | null;
    commit_sha?: string | null;
    worktree_id?: string | null;
    lease_id?: string | null;
    summary_key: string;
    details?: {
      [k: string]: unknown;
    };
    evidence_refs: EvidenceRefs;
  }[];
  window: Page;
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
  model_version: "1.0";
  blocker_id: string;
  reason_code: string;
  source_type:
    | "criterion"
    | "finding"
    | "session"
    | "plan_node"
    | "worktree"
    | "lease"
    | "integration"
    | "task"
    | "event"
    | "unknown";
  source_id: string;
  summary: string;
  why_blocked: string;
  expected_state: string;
  observed_state: string;
  missing_facts: string[];
  impact: string;
  kind: "external_precondition" | "permission" | "evidence_gap" | "dependency" | "conflict" | "workflow" | "unknown";
  owner: string;
  /**
   * @minItems 1
   */
  preconditions: string[];
  /**
   * @minItems 1
   */
  repair_actions: RepairAction[];
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  provenance: BlockerProvenance;
  precondition: string;
  resume_action: string;
  criterion_id?: string;
  node_id?: string;
  evidence_refs: EvidenceRefs;
}
export interface RepairAction {
  action_id: string;
  kind: "command" | "manual" | "navigate" | "retry" | "collect_evidence";
  label: string;
  instructions: string;
  owner: string;
  preconditions: string[];
  verification: string;
  command?: string;
  target?: string;
  copy_text: string;
}
export interface BlockerProvenance {
  status: "native" | "legacy" | "degraded";
  source_event_ids: string[];
  reconstructed_fields: string[];
  unknown_fields: string[];
}
export interface Page {
  cursor: string | null;
  next_cursor: string | null;
  limit: number;
  returned: number;
  total_estimate: number | null;
  truncated: boolean;
  truncation_reason: "none" | "page_limit" | "size_budget" | "permission" | "unsupported";
  model_version: string;
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
