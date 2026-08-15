/* Generated from contracts/v3. Do not edit directly. */

export type V3AgentSpecContract = V3AgentSpecInspection | V3AgentSpecSyncRequest | V3AgentSpecSyncResult;
export type AgentSpecTarget = "claude_code" | "opencode" | "codex";
export type HostConfigScope = "user" | "project";
export type HostConfigStatus = "missing" | "in_sync" | "mismatched" | "invalid" | "unsupported" | "ambiguous";
export type V3AgentSpecArtifactStatus =
  "missing" | "in_sync" | "outdated" | "modified_outside" | "legacy_migratable" | "unsupported";
export type V3AgentSpecSyncStatus = "synchronized" | "incomplete";
export type HostMcpSyncStatus = "synchronized" | "incomplete";
export type GlobalMcpMigrationStatus = "missing" | "unchanged" | "migrated";

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
  schema_version: string;
  consumer: AgentSpecTarget;
  scope: HostConfigScope;
  path: string;
  status: HostConfigStatus;
  reason: string;
  server_name: string | null;
  binary: string | null;
  configured_project_root: string | null;
  canonical_configured_project_root: string | null;
  revision: string | null;
  owned_fields: string[];
  provenance: string;
  repair_action: string | null;
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
  migrate_global?: boolean;
}
export interface V3AgentSpecSyncResult {
  status: V3AgentSpecSyncStatus;
  inspection: V3AgentSpecInspection;
  written_paths: string[];
  skipped_paths: string[];
  blocking_paths: string[];
  host_mcp: HostMcpSyncResult;
  [k: string]: unknown;
}
export interface HostMcpSyncResult {
  status: HostMcpSyncStatus;
  inspections: McpHostConfigInspection[];
  written_paths: string[];
  skipped_paths: string[];
  blocking_paths: string[];
  global_migration: GlobalMcpMigrationResult;
}
export interface GlobalMcpMigrationResult {
  status: GlobalMcpMigrationStatus;
  path: string | null;
  backup_path: string | null;
  before_revision: string | null;
  after_revision: string | null;
  removed_server_names: string[];
  preserved_bytes: boolean;
  repair_action: string | null;
}
