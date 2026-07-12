/* Generated from contracts/v3. Do not edit directly. */

export interface V3EventEnvelope {
  event_id: string;
  event_type: string;
  event_version: "1.0";
  aggregate_id: string;
  aggregate_version: number;
  expected_version: number;
  idempotency_key: string;
  project_id: string;
  task_id: string;
  node_id?: string;
  session_id?: string;
  actor: string;
  evidence_grade: "hard_observed" | "agent_reported" | "inferred" | "user_confirmed";
  occurred_at: string;
  recorded_at: string;
  commit_sha?: string;
  payload: {
    [k: string]: unknown;
  };
}
