/* Generated from contracts/v3. Do not edit directly. */

export type V3ApplicationCommand =
  | SessionOpen
  | EventLog
  | SessionClose
  | AgentResultRecord
  | PlanNodeAdd
  | PlanDependenciesSet
  | PlanNodeStateSet
  | CriterionReview
  | TaskCompletionPropose
  | TaskComplete
  | WorktreeCommand
  | Rebuild;
export type SessionOpen = SessionWriteScope & {
  command: "session_open";
  working_directory?: string;
  node_id?: string;
  worktree_id?: string;
  [k: string]: unknown;
};
export type EventLog = SessionWriteScope & {
  command: "event_log";
  kind: "progress" | "risk";
  details: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
};
export type SessionClose = SessionWriteScope & {
  command: "session_close";
  [k: string]: unknown;
};
export type AgentResultRecord = SessionWriteScope & {
  command: "agent_result_record";
  result_id: string;
  node_id?: string;
  kind: "execution" | "evaluation";
  request_source: "user_request" | "evaluation_instruction";
  instruction: string;
  status: "pending" | "running" | "succeeded" | "failed";
  summary: string;
  body?: string | null;
  evaluation?: {
    [k: string]: unknown;
  } | null;
  artifacts?: {
    [k: string]: unknown;
  }[];
  started_at?: string | null;
  completed_at?: string | null;
  evidence_refs?: EvidenceRefs;
  [k: string]: unknown;
};
export type EvidenceRefs = EvidenceRef[];
export type PlanNodeAdd = TaskWriteScope & {
  command: "plan_node_add";
  node_id: string;
  title: string;
  goal: string;
  scope?: string[];
  dependencies?: string[];
  [k: string]: unknown;
};
export type PlanDependenciesSet = TaskWriteScope & {
  command: "plan_dependencies_set";
  node_id: string;
  dependencies: string[];
  [k: string]: unknown;
};
export type PlanNodeStateSet = TaskWriteScope & {
  command: "plan_node_state_set";
  node_id: string;
  state: "planned" | "ready" | "active" | "blocked" | "completed" | "failed" | "cancelled";
  [k: string]: unknown;
};
export type CriterionReview = TaskWriteScope & {
  command: "criterion_review";
  criterion_id: string;
  outcome: "passed" | "failed" | "blocked";
  reviewer: string;
  evidence_refs: EvidenceRefs;
  details?: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
};
export type TaskCompletionPropose = TaskWriteScope & {
  command: "task_completion_propose";
  [k: string]: unknown;
};
export type WorktreeCommand = TaskWriteScope & {
  command: "worktree_event";
  event_type:
    | "worktree.planned"
    | "worktree.create_prepared"
    | "worktree.ready"
    | "worktree.activated"
    | "worktree.dirty"
    | "worktree.submitted"
    | "worktree.integration_prepared"
    | "worktree.integrated"
    | "worktree.conflicted"
    | "worktree.repairing"
    | "worktree.abandoned"
    | "worktree.cleaned"
    | "lease.acquired"
    | "lease.heartbeat"
    | "lease.released"
    | "lease.reclaimed";
  node_id: string;
  worktree_id: string;
  session_id?: string;
  lease_id?: string;
  operation_id?: string;
  eligibility_digest: string;
  lease_generation?: number;
  evidence_grade?: "hard_observed" | "agent_reported" | "inferred" | "user_confirmed";
  payload?: unknown;
  [k: string]: unknown;
};

export interface SessionWriteScope {
  project_id: string;
  task_id: string;
  session_id: string;
  actor: string;
  expected_version: number;
  idempotency_key: string;
  [k: string]: unknown;
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
export interface TaskWriteScope {
  project_id: string;
  task_id: string;
  actor: string;
  expected_version: number;
  idempotency_key: string;
  [k: string]: unknown;
}
export interface TaskComplete {
  command: "task_complete";
  project_id: string;
  task_id: string;
  actor: string;
  confirmed_by: string;
  channel: "desktop_ui" | "cli";
  idempotency_key: string;
}
export interface Rebuild {
  command: "rebuild";
  project_id: string;
}
