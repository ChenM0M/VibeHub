/* Generated from contracts/v3. Do not edit directly. */

export type SessionTaskRoutingContract = RouteRequest | TaskRouteDecision;
export type RouteTrigger =
  | "ordinary_continuation"
  | "explicit_task"
  | "explicit_switch"
  | "new_execution"
  | "binding_invalid"
  | "binding_stale"
  | "scope_conflict"
  | "ambiguous_candidate";
export type BindingFreshness = "fresh" | "stale" | "unknown";
export type BindingSource =
  | "explicit_task_id"
  | "explicit_title"
  | "current_session"
  | "created_and_start"
  | "unique_candidate"
  | "user_confirmed"
  | "legacy_session_open"
  | "unknown";
export type BindingStatus = "unbound" | "bound" | "stale" | "invalid";
export type HostCapabilityState = "known" | "unknown";
export type RouteAction = "continue" | "bind" | "ask" | "new";

export interface RouteRequest {
  schema_version: "1.0";
  identity: SessionTaskIdentity;
  intent: string;
  trigger: RouteTrigger;
  explicit_task_id?: string | null;
  explicit_task_title?: string | null;
  current_binding?: SessionTaskBinding | null;
  just_created_task_id?: string | null;
  explicit_start?: boolean;
  candidates: TaskRouteCandidate[];
  project_current_default_task_id?: string | null;
  ui_selected_task_id?: string | null;
  host_capabilities: HostCapabilities;
}
export interface SessionTaskIdentity {
  project_id: string;
  interaction_id: string;
  session_id: string;
  agent_id?: string | null;
  host?: string | null;
}
export interface SessionTaskBinding {
  schema_version: "1.0";
  project_id: string;
  interaction_id: string;
  session_id: string;
  agent_id?: string | null;
  host?: string | null;
  bound_task_id?: string | null;
  binding_revision: number;
  freshness: BindingFreshness;
  source: BindingSource;
  status: BindingStatus;
  target_task_revision?: number | null;
  bound_at?: string | null;
}
export interface TaskRouteCandidate {
  task_id: string;
  title: string;
  intent: string;
  state: string;
  project_id?: string | null;
  is_current_default?: boolean;
  is_ui_selected?: boolean;
  active_session_count?: number;
}
export interface HostCapabilities {
  state: HostCapabilityState;
  supports_session_binding: boolean;
  supports_binding_preconditions: boolean;
  provider?: string | null;
}
export interface TaskRouteDecision {
  schema_version: "1.0";
  project_id: string;
  interaction_id: string;
  session_id: string;
  action: RouteAction;
  kind: RouteAction;
  binding_status: BindingStatus;
  candidate_task_id?: string | null;
  selected_task_id?: string | null;
  binding_source?: BindingSource | null;
  binding_revision?: number | null;
  confidence: number;
  trigger: RouteTrigger;
  /**
   * @maxItems 3
   */
  options?: RouteOption[];
  confirmation_question?: string | null;
  diagnostic_code?: string | null;
  reroute_performed: boolean;
  rationale: string;
}
export interface RouteOption {
  task_id: string;
  title: string;
  state: string;
  confidence: number;
}
