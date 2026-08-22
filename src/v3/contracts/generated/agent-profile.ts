/* Generated from contracts/v3. Do not edit directly. */

export type V3AgentProfileContract =
  | AgentProfileReadResult
  | AgentProfileDiscoverResult
  | AgentProfileCommand
  | AgentProfileSaveResult
  | AgentProfileDiagnosticsResult;
export type AgentProfileReadResult = ContractMetadata & {
  kind: "agent_profile_read_result";
  agent: AgentKind;
  runtime_target: RuntimeTarget;
  source: ConfigSource;
  revision: ConfigRevision;
  managed_fields: ManagedProfileFields;
  default_state: DefaultState;
  protocol: ProtocolCapability;
  profile: AgentProfileDocument;
  schema_capability: SchemaCapability;
  [k: string]: unknown;
};
export type EvidenceRefs = EvidenceRef[];
export type Warnings = Warning[];
export type Errors = StructuredError[];
export type AgentKind = "claude_code" | "opencode" | "codex";
export type RuntimeTargetKind = "host" | "wsl";
export type RuntimePlatform = "macos" | "windows" | "linux";
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
export type ProtocolCapability = {
  [k: string]: unknown;
} & {
  native_protocol: "openai_responses" | "openai_chat_completions" | "anthropic_messages" | "unknown";
  upstream_protocol: "openai_responses" | "openai_chat_completions" | "anthropic_messages" | "unknown";
  route: "direct" | "adapter" | "unavailable";
  compatibility: "supported" | "partial" | "unknown" | "unsupported";
  adapter_id: string | null;
  adapter_version: string | null;
  limitations: string[];
};
export type AgentProfileDiscoverResult = ContractMetadata & {
  kind: "agent_profile_discover_result";
  agent: AgentKind;
  runtime_targets: RuntimeTarget[];
  profiles: AgentProfileSummary[];
  default_profile_id: string | null;
  [k: string]: unknown;
};
export type AgentProfileCommand =
  | DiscoverCommand
  | ReadCommand
  | CreateCommand
  | CloneCommand
  | RenameCommand
  | DeleteCommand
  | SaveCommand
  | ValidateCommand
  | ActivateCommand
  | LaunchCommand
  | RestoreCommand
  | DiagnosticsCommand;
export type DiscoverCommand = AgentProfileCommandBase & {
  command: "discover";
  [k: string]: unknown;
};
export type ReadCommand = AgentProfileCommandBase & {
  command: "read";
  profile_id: string;
  [k: string]: unknown;
};
export type CreateCommand = AgentProfileCommandBase & {
  command: "create";
  profile_name: string;
  template_profile_id?: string;
  [k: string]: unknown;
};
export type CloneCommand = AgentProfileCommandBase & {
  command: "clone";
  source_profile_id: string;
  profile_name: string;
  [k: string]: unknown;
};
export type RenameCommand = AgentProfileCommandBase & {
  command: "rename";
  profile_id: string;
  new_profile_name: string;
  [k: string]: unknown;
};
export type DeleteCommand = AgentProfileCommandBase & {
  command: "delete";
  profile_id: string;
  replacement_profile_id?: string;
  [k: string]: unknown;
};
export type SaveCommand = AgentProfileCommandBase & {
  command: "save";
  profile_id: string;
  expected_revision: number;
  profile: AgentProfileDocument;
  [k: string]: unknown;
};
export type ValidateCommand = AgentProfileCommandBase & {
  command: "validate";
  profile: AgentProfileDocument;
  [k: string]: unknown;
};
export type ActivateCommand = AgentProfileCommandBase & {
  command: "activate";
  profile_id: string;
  expected_revision: number;
  [k: string]: unknown;
};
export type LaunchCommand = AgentProfileCommandBase & {
  command: "launch";
  profile_id: string;
  launch_mode: "temporary" | "default";
  [k: string]: unknown;
};
export type RestoreCommand = AgentProfileCommandBase & {
  command: "restore";
  profile_id: string;
  expected_revision: number;
  backup_path: string;
  [k: string]: unknown;
};
export type DiagnosticsCommand = AgentProfileCommandBase & {
  command: "diagnostics";
  profile_id?: string;
  [k: string]: unknown;
};
export type AgentProfileSaveResult = ContractMetadata & {
  kind: "agent_profile_save_result";
  operation: "create" | "clone" | "rename" | "delete" | "save" | "validate" | "activate" | "launch" | "restore";
  profile: AgentProfileDocument;
  revision: ConfigRevision;
  backup_path: NativePath | null;
  rollback_available: boolean;
  [k: string]: unknown;
};
export type AgentProfileDiagnosticsResult = ContractMetadata & {
  kind: "agent_profile_diagnostics_result";
  agent: AgentKind;
  runtime_target: RuntimeTarget;
  status: "empty" | "ready" | "partial" | "unsupported" | "error";
  profiles: AgentProfileSummary[];
  source_paths: NativePath[];
  auth_files_skipped: true;
  [k: string]: unknown;
};

export interface ContractMetadata {
  schema_version: "1.0";
  generated_at: string;
  model_version: string;
  freshness: "fresh" | "stale" | "rebuilding" | "unavailable";
  completeness: "complete" | "partial" | "unsupported" | "unknown";
  evidence_refs: EvidenceRefs;
  warnings: Warnings;
  errors: Errors;
  [k: string]: unknown;
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
export interface RuntimeTarget {
  target_id: string;
  kind: RuntimeTargetKind;
  platform: RuntimePlatform;
  distribution?: string | null;
  display_name: string;
  home_path: NativePath;
  source: "observed" | "user_selected" | "fixture";
  capability_manifest_revision?: string | null;
}
export interface ConfigSource {
  path: NativePath;
  format: "json" | "jsonc" | "toml" | "unknown";
  scope: "user" | "project" | "profile" | "unknown";
  profile_name: string | null;
  last_modified?: string | null;
}
export interface ConfigRevision {
  revision: number;
  content_sha256: string;
  observed_at: string;
}
export interface ManagedProfileFields {
  providers: ProviderProfile[];
  default_provider_id: string | null;
  default_model_id: string | null;
  small_model_id: string | null;
  claude_advanced?: ClaudeAdvancedSettings;
}
export interface ProviderProfile {
  provider_id: string;
  display_name: string;
  base_url: string;
  credential: CredentialReference;
  protocol: ProtocolCapability;
  models: ModelProfile[];
}
export interface CredentialReference {
  kind: "env" | "keychain" | "credential_manager" | "secret_store" | "none" | "unknown";
  reference: string;
  display: string;
  secret_state: "configured" | "missing" | "unavailable" | "unknown";
  persisted_in_config: false;
}
export interface ModelProfile {
  model_id: string;
  display_name: string;
  enabled: boolean;
  thinking: ThinkingProfile;
}
export interface ThinkingProfile {
  supports_reasoning: boolean;
  supports_effort: boolean;
  selected: string | null;
  options: string[];
  custom_allowed: boolean;
}
/**
 * Claude Code only. Optional overrides for subagent, tier-alias, and small/fast model projection. When a field is null/absent, VibeHub projects the managed default model so subagents and background tasks do not fall back to native-only model IDs.
 */
export interface ClaudeAdvancedSettings {
  /**
   * Maps to env CLAUDE_CODE_SUBAGENT_MODEL. Null/absent falls back to the default model.
   */
  subagent_model?: string | null;
  /**
   * Generic small/fast model; maps to env ANTHROPIC_DEFAULT_HAIKU_MODEL unless haiku_model is set. Null/absent falls back to the default model.
   */
  small_fast_model?: string | null;
  /**
   * Maps to env ANTHROPIC_DEFAULT_SONNET_MODEL. Null/absent falls back to the default model.
   */
  sonnet_model?: string | null;
  /**
   * Maps to env ANTHROPIC_DEFAULT_OPUS_MODEL. Null/absent falls back to the default model.
   */
  opus_model?: string | null;
  /**
   * Maps to env ANTHROPIC_DEFAULT_HAIKU_MODEL. Wins over small_fast_model. Null/absent falls back to the default model.
   */
  haiku_model?: string | null;
  /**
   * Maps to env ANTHROPIC_DEFAULT_FABLE_MODEL. Null/absent falls back to the default model.
   */
  fable_model?: string | null;
  /**
   * When true, sets env DISABLE_PROMPT_CACHING=1 for endpoints that do not support prompt caching.
   */
  disable_prompt_caching?: boolean | null;
}
export interface DefaultState {
  is_default: boolean;
  selected_by: "vibehub" | "native" | "unknown";
  projection: DefaultProjection;
}
export interface DefaultProjection {
  strategy: "native_profile" | "settings_projection" | "base_config_projection" | "none" | "unknown";
  target_path: NativePath | null;
  managed_field_paths: string[];
  warning: string | null;
}
export interface AgentProfileDocument {
  profile_id: string;
  display_name: string;
  agent: AgentKind;
  runtime_target_id: string;
  source: ConfigSource;
  revision: ConfigRevision;
  managed: ManagedProfileFields;
  default_state: DefaultState;
  preservation: PreservationSummary;
  protocol: ProtocolCapability;
  schema_capability: SchemaCapability;
  launch: LaunchSpec;
}
export interface PreservationSummary {
  unknown_fields_preserved: boolean;
  comments_preserved: boolean;
  formatting_preserved: boolean;
  managed_field_paths: string[];
  unmanaged_field_paths: string[];
  backup_path: NativePath | null;
  rollback_available: boolean;
}
export interface SchemaCapability {
  schema_id: string;
  schema_version: string | null;
  compatibility: "supported" | "partial" | "unknown" | "unsupported";
  supported_fields: string[];
  unsupported_fields: string[];
  unknown_fields: string[];
  capability_declaration?: CapabilityDeclaration;
  custom_model_options?: CustomModelOptions;
}
export interface CapabilityDeclaration {
  status: "declared" | "unavailable";
  source: string;
  version: string | null;
  supported_fields: string[];
  /**
   * @minItems 1
   */
  fallback_priority: string[];
  message: string | null;
}
export interface CustomModelOptions {
  status: "available" | "unavailable";
  source: string;
  values: string[];
  allow_custom: boolean;
  /**
   * @minItems 1
   */
  fallback_priority: string[];
  message: string | null;
}
export interface LaunchSpec {
  executable: string;
  profile_argument: string | null;
  settings_argument: string | null;
  extra_arguments: string[];
}
export interface AgentProfileSummary {
  profile_id: string;
  display_name: string;
  agent: AgentKind;
  runtime_target_id: string;
  source_path: NativePath;
  revision: ConfigRevision;
  is_default: boolean;
  compatibility: "supported" | "partial" | "unknown" | "unsupported";
}
export interface AgentProfileCommandBase {
  kind: "agent_profile_command";
  agent: AgentKind;
  runtime_target_id: string;
  [k: string]: unknown;
}
