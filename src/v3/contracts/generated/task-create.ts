/* Generated from contracts/v3. Do not edit directly. */

export type V3TaskCreateContract = V3TaskCreateRequest | V3TaskCreateResult;

export interface V3TaskCreateRequest {
  title: string;
  intent: string;
  workflow_profile?: "lightweight" | "standard" | "full";
  trigger_context?: {
    dependencies?: string[];
    handoff_required?: boolean;
    multi_agent?: boolean;
    cross_platform?: boolean;
    release?: boolean;
    migration?: boolean;
    security_sensitive?: boolean;
  };
  profile_override?: {
    requested_profile: "lightweight" | "standard" | "full";
    reason: string;
    user_confirmed: true;
  } | null;
  /**
   * @minItems 1
   * @maxItems 100
   */
  acceptance_criteria: string[];
  [k: string]: unknown;
}
export interface V3TaskCreateResult {
  status: "created" | "already_exists";
  task_id: string;
  task_path: string;
  current_pointer_path: string;
  initial_node_id: string | null;
  lifecycle_version: number;
  [k: string]: unknown;
}
