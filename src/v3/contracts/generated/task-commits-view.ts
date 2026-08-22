/* Generated from contracts/v3. Do not edit directly. */

export interface TaskCommitsView {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  session_count: number;
  sessions: SessionGitTrace[];
  git_evidence: GitEvidence;
  source: "v3_session_git_head_events";
}
export interface SessionGitTrace {
  session_id: string;
  open_git_head: string | null;
  close_git_head: string | null;
  opened_at: string | null;
  closed_at: string | null;
  git_trace_status: "complete" | "partial" | "missing";
  has_git_evidence: boolean;
  event_commit_shas: string[];
}
export interface GitEvidence {
  status: "complete" | "partial" | "missing" | "unavailable";
  complete_sessions: number;
  partial_sessions: number;
  missing_sessions: number;
  historical_gaps: HistoricalGap[];
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
