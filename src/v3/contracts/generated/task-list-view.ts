/* Generated from contracts/v3. Do not edit directly. */

export interface TaskListView {
  schema_version: "1.0";
  project_id: string;
  tasks: TaskSummary[];
  total_count: number;
  returned_count: number;
  active_count: number;
  archived_count: number;
  project_task_count: number;
  omitted_archived_count: number;
  include_archived: boolean;
  sort: {
    order: "non_terminal_first_then_terminal_at_desc";
    terminal_at_missing: "last";
    state_tie_breaker: "state_rank_ascending";
    tie_breaker: "task_id_ascending";
  };
}
export interface TaskSummary {
  task_id: string;
  title: string;
  intent: string;
  workflow_profile: "lightweight" | "standard" | "full";
  state: "planned" | "active" | "blocked" | "review" | "completed" | "cancelled" | "closed_with_exceptions";
  is_terminal: boolean;
  terminal_at: string | null;
  criterion_pass_rate: PassRate;
  required_node_completion_rate: CompletionRate;
  active_session_count: number;
  source: "v3_event_log_projection";
}
export interface PassRate {
  passed: number;
  not_applicable: number;
  effective_passed: number;
  total: number;
  ratio: number | null;
}
export interface CompletionRate {
  completed: number;
  waived: number;
  credited: number;
  total: number;
  ratio: number | null;
}
