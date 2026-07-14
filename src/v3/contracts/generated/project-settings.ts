/* Generated from contracts/v3. Do not edit directly. */

export type V3ProjectSettingsContract =
  V3ProjectSettingsInspection | V3ProjectSettingsUpdateRequest | V3ProjectSettings;
export type V3OutputLanguage = "zh-CN" | "zh-TW" | "en-US";
export type V3AgentSpecTarget = "claude_code" | "opencode" | "codex";

export interface V3ProjectSettingsInspection {
  status: "missing" | "present";
  settings: V3ProjectSettings | null;
  recommended_action: string | null;
  [k: string]: unknown;
}
export interface V3ProjectSettings {
  schema_version: 1;
  kind: "v3_project_settings";
  revision: number;
  output_language: V3OutputLanguage;
  /**
   * @minItems 1
   */
  agent_spec_targets: V3AgentSpecTarget[];
  repository_remote_url?: string | null;
  updated_at: string;
  updated_by: string;
}
export interface V3ProjectSettingsUpdateRequest {
  expected_revision: number;
  output_language: V3OutputLanguage;
  /**
   * @minItems 1
   */
  agent_spec_targets: V3AgentSpecTarget[];
  repository_remote_url?: string | null;
}
