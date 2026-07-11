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
export type Warnings = Warning[];
export type Errors = StructuredError[];

export interface ProjectOverviewView {
  schema_version: "1.0";
  project_id: string;
  name: string;
  root: NativePath;
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
  active_tasks: {
    task_id: string;
    title: string;
    state: "planned" | "active" | "blocked" | "review" | "completed" | "cancelled";
    risk_level: "none" | "low" | "medium" | "high" | "critical";
    criteria: CriterionSummary[];
    active_sessions: number;
  }[];
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
