#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const REQUIRED_SECTIONS = [
  "Completed",
  "Not Yet Done",
  "Key Decisions Made",
  "Files Changed",
  "Files Reportedly Read",
  "Commands Run",
  "Tests Run",
  "Context Still Needed",
  "Warnings",
  "Next Session Should",
];

function readStdinJson() {
  try {
    const input = fs.readFileSync(0, "utf8").trim();
    return input ? JSON.parse(input) : {};
  } catch {
    return {};
  }
}

function findProjectRoot(input) {
  const starts = [
    process.env.VIBEHUB_PROJECT_ROOT,
    process.env.CLAUDE_PROJECT_DIR,
    input.cwd,
    process.cwd(),
  ].filter(Boolean);
  for (const start of starts) {
    let current = path.resolve(start);
    while (true) {
      if (fs.existsSync(path.join(current, ".vibehub"))) return current;
      const parent = path.dirname(current);
      if (parent === current) break;
      current = parent;
    }
  }
  return null;
}

function readYamlString(content, key) {
  const match = content.match(new RegExp(`^\\s*${key}:\\s*['"]?([^'"\\r\\n#]+)`, "m"));
  return match ? match[1].trim() : null;
}

function readCurrent(root) {
  const statePath = path.join(root, ".vibehub", "state.yaml");
  if (!fs.existsSync(statePath)) return null;
  const state = fs.readFileSync(statePath, "utf8");
  let taskId = readYamlString(state, "task_id");
  let runId = readYamlString(state, "run_id");
  const phase = readYamlString(state, "phase");
  const phaseStatus = readYamlString(state, "phase_status") || readYamlString(state, "status");

  if (!taskId) {
    const currentTask = path.join(root, ".vibehub", "tasks", "current");
    if (fs.existsSync(currentTask)) taskId = readYamlString(fs.readFileSync(currentTask, "utf8"), "task_id");
  }
  if (taskId && !runId) {
    const currentRun = path.join(root, ".vibehub", "tasks", taskId, "runs", "current");
    if (fs.existsSync(currentRun)) runId = readYamlString(fs.readFileSync(currentRun, "utf8"), "run_id");
  }
  return taskId && runId ? { taskId, runId, phase, phaseStatus } : null;
}

function outputCandidates(root, current) {
  const runDir = path.join(root, ".vibehub", "tasks", current.taskId, "runs", current.runId);
  const candidates = [path.join(runDir, "outputs", "output.md")];
  const sessionsDir = path.join(runDir, "sessions");
  if (fs.existsSync(sessionsDir)) {
    for (const entry of fs.readdirSync(sessionsDir, { withFileTypes: true })) {
      if (entry.isDirectory()) candidates.push(path.join(sessionsDir, entry.name, "output.md"));
    }
  }
  return candidates;
}

function missingSections(content) {
  return REQUIRED_SECTIONS.filter((section) => {
    const escaped = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    // Match exact English title or bilingual format: "English / Translation"
    const re = new RegExp(`^##\\s+${escaped}(\\s*/\\s*[^\\s].*?)?\\s*$([\\s\\S]*?)(?=^##\\s+|(?![\\s\\S]))`, "mi");
    const match = content.match(re);
    return !match || !match[2].trim();
  });
}

function hasGitChanges(root) {
  try {
    const output = execFileSync("git", ["-C", root, "status", "--porcelain"], { encoding: "utf8" });
    return output.trim().length > 0;
  } catch {
    return true;
  }
}

function block(reason) {
  if (process.env.CLAUDE_PROJECT_DIR) {
    console.error(reason);
    process.exit(2);
  }
  process.stdout.write(JSON.stringify({ decision: "block", reason }));
  process.exit(0);
}

const input = readStdinJson();
const root = findProjectRoot(input);
if (!root) process.exit(0);
if (process.env.VIBEHUB_ALLOW_NO_OUTPUT === "1") process.exit(0);

const current = readCurrent(root);
if (!current) process.exit(0);

const candidates = outputCandidates(root, current).filter((candidate) => fs.existsSync(candidate));
if (candidates.length === 0) {
  const preferred = path.join(".vibehub", "tasks", current.taskId, "runs", current.runId, "outputs", "output.md");
  block(`VibeHub active task ${current.taskId}/${current.runId} requires ${preferred} before ending. Write the required phase output sections first.`);
}

candidates.sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
const latest = candidates[0];
const missing = missingSections(fs.readFileSync(latest, "utf8"));
if (missing.length > 0) {
  block(`VibeHub output is incomplete: ${path.relative(root, latest)} is missing non-empty sections: ${missing.join(", ")}.`);
}

if (!hasGitChanges(root)) process.exit(0);
process.exit(0);
