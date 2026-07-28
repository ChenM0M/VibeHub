/* Generated from contracts/v3. Do not edit directly. */

export interface ProjectMemoryProjection {
  model_version: string;
  project_id: string;
  event_ids: string[];
  entries: {
    [k: string]: MemoryEntry;
  };
  [k: string]: unknown;
}
export interface MemoryEntry {
  entry_id: string;
  kind:
    | "project_profile"
    | "team_profile"
    | "user_preference"
    | "accepted_decision"
    | "curated_knowledge"
    | "implementation_route";
  scope: string[];
  principal_scope: string;
  revision: number;
  status: "active" | "candidate" | "disputed" | "stale" | "superseded" | "archived";
  owner: string;
  content: string;
  evidence_refs: string[];
  verified_against: string[];
  confidence: number;
  freshness: "fresh" | "stale" | "unknown";
  invalidation: string | null;
  supersedes: string[];
  injection_policy: "always" | "task_relevant" | "explicit_only" | "never";
  sensitivity: "public" | "internal" | "confidential" | "secret";
  updated_at: string;
}
