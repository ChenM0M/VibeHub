/* Generated from contracts/v3. Do not edit directly. */

export type V3ApplicationCommand = SessionOpen | EventLog | SessionClose | WorktreeCommand | Rebuild;
export type SessionOpen = WriteScope & {
  command: "session_open";
  [k: string]: unknown;
};
export type EventLog = WriteScope & {
  command: "event_log";
  kind: "progress" | "risk";
  details: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
};
export type SessionClose = WriteScope & {
  command: "session_close";
  [k: string]: unknown;
};
export type WorktreeCommand = WriteScope & {
  command:
    | "worktree_plan"
    | "worktree_create"
    | "worktree_activate"
    | "worktree_mark_dirty"
    | "worktree_submit"
    | "worktree_integrate"
    | "worktree_repair"
    | "worktree_abandon"
    | "worktree_clean"
    | "lease_acquire"
    | "lease_heartbeat"
    | "lease_release"
    | "lease_reclaim";
  node_id: string;
  worktree_id: string;
  lease_id?: string;
  operation_id: string;
  eligibility_digest: string;
  lease_generation?: number;
  integration_policy?: "merge_no_ff" | "rebase" | "cherry_pick";
  details?: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
};

export interface WriteScope {
  project_id: string;
  task_id: string;
  session_id: string;
  actor: string;
  expected_version: number;
  idempotency_key: string;
  [k: string]: unknown;
}
export interface Rebuild {
  command: "rebuild";
  project_id: string;
}
