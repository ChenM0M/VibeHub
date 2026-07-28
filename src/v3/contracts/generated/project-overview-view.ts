/* Generated from contracts/v3. Do not edit directly. */

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
export type EvidenceRefs = EvidenceRef[];
export type BlockerDetails = BlockerDetail[];
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface ProjectOverviewView {
  schema_version: "1.0";
  project_id: string;
  name: string;
  root: NativePath;
  scopes: ProjectScopeInspection;
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  repository: {
    state: "available" | "not_repository" | "unavailable";
    branch: string | null;
    head: string | null;
    dirty: boolean | null;
    worktree_count: number;
  };
  model: {
    state: "uninitialized" | "rebuilding" | "ready" | "error";
    last_evidence_at: string | null;
    generator_version: string;
    indexed_files: number;
  };
  architecture: {
    declared_docs: number;
    modules: number;
    relationships: number;
    confidence: number;
    evidence_refs: EvidenceRefs;
  };
  current_task_id?: string | null;
  active_tasks: {
    task_id: string;
    title: string;
    intent: string;
    workflow_profile: "lightweight" | "standard" | "full";
    state: "planned" | "active" | "blocked" | "review" | "completed" | "cancelled" | "closed_with_exceptions";
    risk_level: "none" | "low" | "medium" | "high" | "critical";
    criteria: CriterionSummary[];
    blocker_details?: BlockerDetails;
    relations?: {
      related_task_id: string;
      relation_type: string;
      confidence: number;
      evidence_refs: EvidenceRefs;
    }[];
    active_sessions: number;
  }[];
  archived_tasks?: ArchivedTaskSummary[];
  protocol_coverage: {
    state: "complete" | "partial" | "gapped" | "unknown";
    opened_sessions: number;
    closed_sessions: number;
    gaps: number;
  };
  evidence_refs: EvidenceRefs;
  warnings: Warnings;
  errors: Errors;
}
export interface ProjectScopeInspection {
  control_root: string;
  execution_root: string;
  git_root: string | null;
  host_config_root: string;
  source: "control_root" | "detected_git_root" | "session_working_directory" | "ambiguous";
  nested_repository: boolean;
  warnings: string[];
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
export interface CriterionSummary {
  criterion_id: string;
  title: string;
  status: "proposed" | "accepted" | "passed" | "failed" | "blocked" | "not_applicable";
  required: boolean;
  evidence_refs: EvidenceRefs;
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
export interface ArchivedTaskSummary {
  task_id: string;
  title: string;
  intent: string;
  state: "completed" | "cancelled" | "closed_with_exceptions";
  risk_level: "none" | "low" | "medium" | "high" | "critical";
  terminal_at: string | null;
  completion: CompletionSummary;
  closure: {
    method: "all_green" | "with_exceptions" | null;
    actor: string | null;
    confirmed_by: string | null;
    channel: string | null;
    confirmed_at: string | null;
    reason: string | null;
    criteria_snapshot: CriterionSummary[];
    unresolved_items: string[];
  };
  criteria: CriterionSummary[];
  blocker_details: BlockerDetails;
  plan: {
    total: number;
    completed: number;
    cancelled: number;
    active: number;
    blocked: number;
    planned: number;
  };
  sessions: {
    total: number;
    opened: number;
    closed: number;
    gapped: number;
  };
  findings: {
    total: number;
    open: number;
    closed: number;
  };
  result: {
    status: string;
    summary: string | null;
    artifact_count: number;
    recorded_at: string | null;
  };
  evidence_count: number;
  evidence_refs: EvidenceRefs;
  next_action: string;
  source: "v3_projection";
}
export interface CompletionSummary {
  proposal_event_id: string | null;
  proposed_at_version: number | null;
  digest: string | null;
  valid: boolean;
  confirmed: boolean;
  confirmed_at_version: number | null;
  confirmed_by: string | null;
  channel: string | null;
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
