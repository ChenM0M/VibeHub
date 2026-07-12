import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const startedAt = new Date().toISOString();
const checks = [
  ["rust_lifecycle", "cargo", ["test", "-p", "vibehub-core", "v3::lifecycle", "--", "--nocapture"]],
  ["contracts", "npm", ["run", "v3:contracts:check"]],
  ["mcp", "npm", ["run", "v3:mcp:check"]],
  ["typescript_build", "npm", ["run", "build"]],
];

const results = checks.map(([id, command, args]) => {
  const result = spawnSync(command, args, { cwd: process.cwd(), encoding: "utf8" });
  return {
    id,
    status: result.status === 0 ? "passed" : "failed",
    exit_code: result.status,
    command: [command, ...args].join(" "),
    output_tail: `${result.stdout ?? ""}\n${result.stderr ?? ""}`.trim().split("\n").slice(-12),
  };
});

const automatedPassed = results.every((result) => result.status === "passed");
const report = {
  schema_version: "1.0",
  milestone: "M4",
  started_at: startedAt,
  completed_at: new Date().toISOString(),
  frozen_thresholds: {
    deterministic_rebuild_iterations: 100,
    host_recovery_iterations_each: 10,
    hosts: ["codex", "opencode", "claude-code"],
    kill_resume_hard_gates: ["stable_identity", "idempotent_retry", "history_preserved", "confirmation_invalidated_by_new_truth"],
    allowed_open_p0_p1: 0,
  },
  automated_gate: automatedPassed ? "passed" : "failed",
  native_gates: {
    macos_local: process.platform === "darwin" && automatedPassed ? "passed" : "not_run",
    windows_native: process.platform === "win32" && automatedPassed ? "passed" : "not_run",
  },
  human_gates: { project_owner_m5_shadow_approval: "pending" },
  m5_entry_gate: "closed",
  results,
};

mkdirSync(resolve("target"), { recursive: true });
const reportPath = resolve("target/m4-stability-report.json");
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ report_path: reportPath, automated_gate: report.automated_gate, m5_entry_gate: report.m5_entry_gate }));
process.exit(automatedPassed ? 0 : 1);
