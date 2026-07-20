/* Generated from contracts/v3. Do not edit directly. */

export type EvidenceRefs = EvidenceRef[];
export type BlockerDetails = BlockerDetail[];
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface PlanGraphView {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  plan_version: number;
  generated_at: string;
  model_version: string;
  workflow_profile: "lightweight" | "standard" | "full";
  planning_required: boolean;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  graph_state: "valid" | "cycle" | "unknown_node" | "changed" | "stale";
  nodes: {
    node_id: string;
    title: string;
    goal: string;
    state: "planned" | "ready" | "active" | "blocked" | "review" | "completed" | "cancelled" | "superseded";
    readiness: "ready" | "blocked" | "unknown";
    block_reasons: string[];
    blocker_details?: BlockerDetails;
    scope: string[];
    criterion_ids: string[];
    session_ids?: string[];
    worktree?: WorktreeRef | null;
  }[];
  scheduling_edges: {
    edge_id: string;
    from_node_id: string;
    to_node_id: string;
    kind: "depends_on";
  }[];
  trace_relations: {
    relation_id: string;
    from_id: string;
    to_id: string;
    kind: "repairs" | "addresses" | "validates" | "supersedes";
    evidence_refs: EvidenceRefs;
  }[];
  execution: {
    planned_sessions: number;
    observed_sessions: number;
    planned_worktrees: number;
    observed_worktrees: number;
  };
  evidence_refs: EvidenceRefs;
  warnings: Warnings;
  errors: Errors;
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
export interface EvidenceRef {
  evidence_id: string;
  kind: "event" | "file" | "git" | "command" | "test" | "user" | "external";
  grade: "hard_observed" | "agent_reported" | "inferred" | "user_confirmed";
  label_key: string;
  locator: string;
  captured_at?: string;
  excerpt?: string;
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
