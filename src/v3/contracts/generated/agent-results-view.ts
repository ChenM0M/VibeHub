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
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface AgentResultsView {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  state: "not_executed" | "awaiting_result" | "available" | "failed";
  results: {
    result_id: string;
    kind: "execution" | "evaluation";
    session_id: string;
    node_id: string | null;
    request: {
      source: "user_request" | "evaluation_instruction";
      instruction: string;
    };
    status: "pending" | "running" | "succeeded" | "failed";
    summary: string;
    body: string | null;
    evaluation: null | {
      target: string;
      rubric: string[];
      verdict: "passed" | "needs_revision" | "failed" | "inconclusive";
      findings: {
        title: string;
        detail: string;
        severity: "info" | "low" | "medium" | "high" | "critical";
        evidence_refs: EvidenceRefs;
      }[];
    };
    artifacts: {
      label: string;
      path: NativePath | null;
      uri: string | null;
    }[];
    started_at: string | null;
    completed_at: string | null;
    evidence_refs: EvidenceRefs;
  }[];
  evidence_refs: EvidenceRefs;
  warnings: Warnings;
  errors: Errors;
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
