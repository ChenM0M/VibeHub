import { spawn } from "node:child_process";
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const binary = resolve(process.argv[2] ?? "target/debug/vibehub");
const root = await mkdtemp(join(tmpdir(), "vibehub-v3-mcp-contract-"));
const taskId = "task.contract";
const taskYaml = [
  `task_id: ${taskId}`,
  "title: MCP contract",
  "intent: Verify the minimal recovery loop",
  "phase: implement",
  "phase_status: active",
  "acceptance_criteria:",
  "- MCP recovery loop passes",
  "dependencies: []",
  "",
].join("\n");

await mkdir(join(root, ".vibehub", "tasks", "current"), { recursive: true });
await mkdir(join(root, ".vibehub", "tasks", taskId), { recursive: true });
await writeFile(join(root, ".vibehub", "tasks", "current", "task.yaml"), taskYaml);
await writeFile(join(root, ".vibehub", "tasks", taskId, "task.yaml"), taskYaml);

const startedAt = performance.now();
const child = spawn(binary, ["mcp-stdio", root], { stdio: ["pipe", "pipe", "pipe"] });
let stdoutBuffer = "";
let stderr = "";
let nextId = 1;
const pending = new Map();
const timings = [];

child.stderr.setEncoding("utf8");
child.stderr.on("data", (chunk) => { stderr += chunk; });
child.stdout.setEncoding("utf8");
child.stdout.on("data", (chunk) => {
  stdoutBuffer += chunk;
  for (;;) {
    const newline = stdoutBuffer.indexOf("\n");
    if (newline < 0) break;
    const line = stdoutBuffer.slice(0, newline);
    stdoutBuffer = stdoutBuffer.slice(newline + 1);
    if (!line.trim()) continue;
    let message;
    try { message = JSON.parse(line); }
    catch { throw new Error(`stdout contained non-JSON MCP data: ${line}`); }
    const waiter = pending.get(message.id);
    if (waiter) {
      pending.delete(message.id);
      waiter.resolve(message);
    }
  }
});

function request(method, params = {}) {
  const id = nextId++;
  const began = performance.now();
  const response = new Promise((resolveRequest, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out`));
    }, 3000);
    pending.set(id, {
      resolve(message) {
        clearTimeout(timer);
        timings.push({ method, milliseconds: Math.round(performance.now() - began) });
        if (message.error) reject(new Error(`${method}: ${JSON.stringify(message.error)}`));
        else resolveRequest(message.result);
      },
    });
  });
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
  return response;
}

function notify(method, params = {}) {
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", method, params })}\n`);
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

try {
  const initialize = await request("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "vibehub-contract-test", version: "1.0" },
  });
  assert(initialize.serverInfo.name === "vibehub-v3", "unexpected server identity");
  notify("notifications/initialized");

  const resources = await request("resources/list");
  const tools = await request("tools/list");
  assert(resources.resources.length === 7, "expected seven versioned resources");
  assert(resources.resources.every((resource) => resource.uri.startsWith("vibehub://v3/1.0/")), "resource URI is not versioned");
  assert(tools.tools.map((tool) => tool.name).sort().join(",") === "event_log,session_close,session_open", "unexpected tool catalog");

  const contractRoot = resolve("contracts/v3");
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  addFormats(ajv);
  for (const filename of (await readdir(contractRoot)).filter((name) => name.endsWith(".schema.json"))) {
    ajv.addSchema(JSON.parse(await readFile(join(contractRoot, filename), "utf8")), filename);
  }
  const resourceSchemas = {
    "project-overview": "project-overview-view.schema.json",
    "project-structure": "project-structure-view.schema.json",
    "task-timeline": "task-timeline-view.schema.json",
    "plan-graph": "plan-graph-view.schema.json",
    "node-brief": "node-brief.schema.json",
  };
  const productionViews = {};
  for (const [resourceName, schemaName] of Object.entries(resourceSchemas)) {
    const resource = resources.resources.find((candidate) => candidate.name === resourceName);
    const read = await request("resources/read", { uri: resource.uri });
    const view = JSON.parse(read.contents[0].text);
    assert(ajv.validate(schemaName, view), `${resourceName} failed schema validation: ${JSON.stringify(ajv.errors)}`);
    productionViews[resourceName] = view;
  }
  const overview = productionViews["project-overview"];
  const projectId = overview.project_id;
  assert(overview.task_id === undefined && overview.active_tasks[0].task_id === taskId, "resource identity mismatch");

  const scope = {
    project_id: projectId,
    task_id: taskId,
    session_id: "session.contract",
    actor: "contract-test",
    idempotency_key: "contract.open.1",
    expected_version: 0,
  };
  const opened = await request("tools/call", { name: "session_open", arguments: scope });
  const duplicate = await request("tools/call", { name: "session_open", arguments: scope });
  assert(opened.structuredContent.result.status === "appended", "session_open did not append");
  assert(duplicate.structuredContent.result.status === "duplicate", "idempotency duplicate was not preserved");

  const logged = await request("tools/call", { name: "event_log", arguments: { ...scope, idempotency_key: "contract.log.1", expected_version: 1, kind: "progress", details: { summary: "contract" } } });
  const closed = await request("tools/call", { name: "session_close", arguments: { ...scope, idempotency_key: "contract.close.1", expected_version: 2 } });
  assert(logged.structuredContent.result.status === "appended", "event_log did not append");
  assert(closed.structuredContent.result.status === "appended", "session_close did not append");

  notify("notifications/cancelled", { requestId: "already-completed", reason: "contract probe" });
  child.stdin.end();
  const exitCode = await new Promise((resolveExit, reject) => {
    const timer = setTimeout(() => { child.kill("SIGKILL"); reject(new Error("clean shutdown timed out")); }, 3000);
    child.on("close", (code) => { clearTimeout(timer); resolveExit(code); });
  });
  assert(exitCode === 0, `MCP server exited with ${exitCode}`);
  assert(stderr === "", `stderr was not pure: ${stderr}`);
  assert(stdoutBuffer.trim() === "", "stdout ended with a partial frame");
  assert(timings.every(({ milliseconds }) => milliseconds < 2000), "protocol response exceeded 2s budget");
  console.log(JSON.stringify({ status: "passed", protocol: initialize.protocolVersion, resources: resources.resources.length, tools: tools.tools.length, requests: timings, total_ms: Math.round(performance.now() - startedAt) }, null, 2));
} finally {
  if (!child.killed) child.kill("SIGKILL");
  await rm(root, { recursive: true, force: true });
}
