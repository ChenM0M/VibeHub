/* Generated from contracts/v3. Do not edit directly. */

export type V3ApplicationCommand = SessionOpen | EventLog | SessionClose | Rebuild;
export type SessionOpen = WriteScope & {
  command: "session_open";
  [k: string]: unknown;
};
export type EventLog = WriteScope & {
  command: "event_log";
  kind: "progress" | "risk";
  details: {
    [k: string]: unknown;
  };
  [k: string]: unknown;
};
export type SessionClose = WriteScope & {
  command: "session_close";
  [k: string]: unknown;
};

export interface WriteScope {
  project_id: string;
  task_id: string;
  session_id: string;
  actor: string;
  expected_version: number;
  idempotency_key: string;
  [k: string]: unknown;
}
export interface Rebuild {
  command: "rebuild";
  project_id: string;
}
