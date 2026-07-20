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

export interface ProjectStructureView {
  schema_version: "1.0";
  project_id: string;
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  index_state: "uninitialized" | "indexing" | "ready" | "interrupted" | "error";
  workspace: {
    root: NativePath;
    source: "session_worktree" | "session_working_directory" | "detected_git_root" | "project_root_fallback";
    session_id: string | null;
    worktree_id: string | null;
    fallback_reason: string | null;
    ignored_directories: string[];
  };
  nodes: {
    node_id: string;
    parent_id: string | null;
    name: string;
    kind: "root" | "directory" | "file" | "module" | "package" | "symbol";
    path: NativePath;
    git_state: "clean" | "modified" | "added" | "deleted" | "ignored" | "unknown";
    module_id: string | null;
    ide_target?: string | null;
    evidence_refs: EvidenceRefs;
  }[];
  edges: {
    edge_id: string;
    from_node_id: string;
    to_node_id: string;
    kind: "contains" | "imports" | "depends_on" | "declares" | "generates";
    source_kind: "filesystem" | "git" | "manifest" | "parser" | "documentation" | "inference";
    confidence: number;
    evidence_refs: EvidenceRefs;
  }[];
  architecture_nodes: {
    node_id: string;
    name: string;
    kind: "workspace" | "module" | "package";
    path: NativePath;
    file_count: number;
    source_kind: "manifest" | "parser" | "documentation" | "inference";
    confidence: number;
    generator_version: string;
    evidence_refs: EvidenceRefs;
  }[];
  architecture_edges: {
    edge_id: string;
    from_node_id: string;
    to_node_id: string;
    kind: "contains" | "imports" | "depends_on";
    source_kind: "manifest" | "parser" | "documentation" | "inference";
    confidence: number;
    generator_version: string;
    evidence_refs: EvidenceRefs;
  }[];
  unsupported_analyzers: string[];
  page: Page;
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
export interface Page {
  cursor: string | null;
  next_cursor: string | null;
  limit: number;
  returned: number;
  total_estimate: number | null;
  truncated: boolean;
  truncation_reason: "none" | "page_limit" | "size_budget" | "permission" | "unsupported";
  model_version: string;
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
