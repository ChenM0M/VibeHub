/* Generated from contracts/v3. Do not edit directly. */

export type V3AgentSpecContract = V3AgentSpecInspection | V3AgentSpecSyncRequest | V3AgentSpecSyncResult;
export type AgentSpecTarget = "claude_code" | "opencode" | "codex";
export type V3AgentSpecArtifactStatus = "missing" | "in_sync" | "outdated" | "modified_outside" | "unsupported";

export interface V3AgentSpecInspection {
  spec_version: string;
  renderer_version: string;
  settings_revision: number;
  artifacts: V3AgentSpecArtifactInspection[];
  [k: string]: unknown;
}
export interface V3AgentSpecArtifactInspection {
  path: string;
  /**
   * @minItems 1
   */
  consumers: AgentSpecTarget[];
  status: V3AgentSpecArtifactStatus;
  reason: string;
  current_hash: string | null;
  desired_hash: string;
  last_written_hash: string | null;
  [k: string]: unknown;
}
export interface V3AgentSpecSyncRequest {
  force_managed_region?: boolean;
}
export interface V3AgentSpecSyncResult {
  inspection: V3AgentSpecInspection;
  written_paths: string[];
  skipped_paths: string[];
  [k: string]: unknown;
}
