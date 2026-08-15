/* Generated from contracts/v3. Do not edit directly. */

export type WorkspaceStateContract =
  WorkspaceState | WorkspaceStateReadResult | WorkspaceStateSaveRequest | WorkspaceStateSaveResult;

export interface WorkspaceState {
  schema_version: 3;
  kind: "workspace_state";
  revision: number;
  open_projects: WorkspaceProjectIdentity[];
  active_project_id: string | null;
  updated_at: string;
  updated_by: string;
  provenance: WorkspaceStateProvenance;
}
export interface WorkspaceProjectIdentity {
  project_id: string;
  canonical_path: string;
  display_name: string;
  ui_context: WorkspaceProjectUiContext;
}
export interface WorkspaceProjectUiContext {
  owner: "v3-cockpit";
  current_view: string;
  selected_task_id: string | null;
  selected_node_id: string | null;
}
export interface WorkspaceStateProvenance {
  source: "default" | "user" | "recovery" | "migration";
  writer: string;
  reason: string;
}
export interface WorkspaceStateReadResult {
  status: "missing" | "present" | "recovered" | "partial" | "invalid";
  state: WorkspaceState | null;
  diagnostics: WorkspaceStateDiagnostic[];
  recommended_action: string | null;
}
export interface WorkspaceStateDiagnostic {
  code: string;
  severity: "info" | "warning" | "error";
  project_id?: string | null;
  message: string;
  recovery_action: string;
}
export interface WorkspaceStateSaveRequest {
  expected_revision: number;
  state: WorkspaceState;
}
export interface WorkspaceStateSaveResult {
  state: WorkspaceState;
  previous_revision: number;
  backup_path: string | null;
  diagnostics: WorkspaceStateDiagnostic[];
}
