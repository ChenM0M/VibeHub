/* Generated from contracts/v3. Do not edit directly. */

export type V3AgentSpecContract = V3AgentSpecInspection | V3AgentSpecSyncRequest | V3AgentSpecSyncResult;
export type AgentSpecTarget = "claude_code" | "opencode" | "codex";
export type V3AgentSpecArtifactStatus =
  "missing" | "in_sync" | "outdated" | "modified_outside" | "legacy_migratable" | "unsupported";
export type V3AgentSpecSyncStatus = "synchronized" | "incomplete";

export interface V3AgentSpecInspection {
  spec_version: string;
  renderer_version: string;
  settings_revision: number;
  scope: ProjectScopeInspection;
  effective_declarations: EffectiveAgentDeclaration[];
  mcp_hosts: McpHostConfigInspection[];
  artifacts: V3AgentSpecArtifactInspection[];
  [k: string]: unknown;
}
export interface ProjectScopeInspection {
  control_root: string;
  execution_root: string;
  git_root: string | null;
  host_config_root: string;
  source: "control_root" | "detected_git_root" | "session_working_directory" | "ambiguous";
  nested_repository: boolean;
  warnings: string[];
}
export interface EffectiveAgentDeclaration {
  path: string;
  consumers: AgentSpecTarget[];
  precedence: number;
  exists: boolean;
  contains_v3_region: boolean;
}
export interface McpHostConfigInspection {
  consumer: AgentSpecTarget;
  path: string;
  status: "missing" | "in_sync" | "mismatched" | "invalid";
  reason: string;
  server_name: string | null;
  configured_project_root: string | null;
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
  status: V3AgentSpecSyncStatus;
  inspection: V3AgentSpecInspection;
  written_paths: string[];
  skipped_paths: string[];
  blocking_paths: string[];
  [k: string]: unknown;
}
