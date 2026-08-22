/* Generated from contracts/v3. Do not edit directly. */

export interface CommitTasksView {
  schema_version: "1.0";
  project_id: string;
  query_commit_hash: string;
  resolved_commit_hash: string | null;
  match: "none" | "associated";
  task_count: number;
  tasks: TaskAssociation[];
  historical_gaps: HistoricalGap[];
  source: "v3_git_traceability";
}
export interface TaskAssociation {
  task_id: string;
  title: string;
  intent: string;
  state: "planned" | "active" | "blocked" | "review" | "completed" | "cancelled" | "closed_with_exceptions";
  sessions: AssociationSession[];
}
export interface AssociationSession {
  session_id: string;
  /**
   * @minItems 1
   */
  match_kinds: ("open_head" | "close_head" | "session_range" | "event_commit")[];
  open_git_head: string | null;
  close_git_head: string | null;
  git_trace_status: "complete" | "partial" | "missing";
}
export interface HistoricalGap {
  task_id: string;
  session_id: string;
  /**
   * @minItems 1
   */
  missing: ("open_git_head" | "close_git_head")[];
  reason: string;
}
