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

export interface WorktreeOrchestrationView {
  schema_version: "1.0";
  project_id: string;
  task_id: string;
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  entry_gate: {
    state: "open" | "closed";
    m4_gate_digest: string | null;
    windows_native_evidence: boolean;
    owner_approved: boolean;
    self_host_writes_allowed: boolean;
    evidence_refs: EvidenceRefs;
  };
  policy: {
    integration_policy: "merge_no_ff" | "rebase" | "cherry_pick";
    unknown_scope_decision: "allow" | "warn" | "block";
    denylist: string[];
    case_sensitive: boolean;
    retention_seconds: number;
  };
  worktrees: {
    worktree_id: string;
    node_id: string;
    session_ids: string[];
    display_name: string;
    branch: string;
    base_sha: string;
    head_sha: string;
    native_path: NativePath;
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
    read_only: boolean;
    eligibility: {
      decision: "allow" | "warn" | "block";
      digest: string;
      reason_codes: string[];
      declared_scope: string[];
      observed_delta: string[];
      override: boolean;
      evidence_refs: EvidenceRefs;
    };
    lease: null | {
      lease_id: string;
      owner_session_id: string;
      owner_host: string;
      owner_tool: string;
      state: "active" | "stale" | "released" | "reclaim_pending" | "reclaimed";
      generation: number;
      acquired_at: string;
      heartbeat_at: string;
      expires_at: string;
      reclaim_challenge: string | null;
      evidence_refs: EvidenceRefs;
    };
    git: {
      presence: "present" | "missing" | "moved" | "pruned" | "unknown";
      locked: boolean;
      dirty: boolean;
      detached: boolean;
      unborn: boolean;
      base_drift: boolean;
      changed_files: string[];
      observed_at: string;
      evidence_refs: EvidenceRefs;
    };
    integration: {
      operation_id: string | null;
      policy: "merge_no_ff" | "rebase" | "cherry_pick";
      state: "not_requested" | "queued" | "integrating" | "integrated" | "conflicted" | "blocked";
      queue_position: number | null;
      target_branch: string;
      evidence_refs: EvidenceRefs;
    };
    conflict: null | {
      owner_session_id: string;
      /**
       * @minItems 1
       */
      files: [string, ...string[]];
      base_sha: string;
      head_sha: string;
      target_sha: string;
      next_action: string;
      evidence_refs: EvidenceRefs;
    };
    recovery: {
      state: "none" | "inspect_required" | "retry_ready" | "blocked" | "repaired";
      operation_id: string | null;
      attempts: number;
      last_error: string | null;
      owner_process_state: "alive" | "dead" | "unknown" | "not_applicable";
      evidence_refs: EvidenceRefs;
    };
    next_action: string;
    evidence_refs: EvidenceRefs;
  }[];
  integration_queue: {
    operation_id: string;
    worktree_id: string;
    node_id: string;
    topology_rank: number;
    ready_at: string;
    state: "queued" | "integrating" | "blocked";
  }[];
  orphan_candidates: {
    worktree_id: string;
    session_id: string;
    reason_code: string;
    process_state: "alive" | "dead" | "unknown";
    inspect_required: boolean;
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
