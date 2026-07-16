import { spawn } from "node:child_process";
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const defaultBinary = process.platform === "win32" ? "target/debug/vibehub.exe" : "target/debug/vibehub";
const binary = resolve(process.argv[2] ?? defaultBinary);
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
  assert(tools.tools.map((tool) => tool.name).sort().join(",") === "agent_result_record,event_log,plan_dependencies_set,plan_node_add,plan_node_state_set,session_close,session_open", "unexpected tool catalog");
  const planToolNames = ["plan_node_add", "plan_dependencies_set", "plan_node_state_set"];
  for (const name of planToolNames) {
    const schema = tools.tools.find((tool) => tool.name === name)?.inputSchema;
    assert(schema?.type === "object", `${name} exposes an object input schema`);
    for (const field of ["project_id", "task_id", "actor", "expected_version", "idempotency_key", "node_id"]) {
      assert(schema?.properties?.[field], `${name} schema exposes ${field}`);
    }
  }
  const agentResultSchema = tools.tools.find((tool) => tool.name === "agent_result_record")?.inputSchema;
  assert(agentResultSchema?.type === "object", "agent_result_record exposes an object input schema");
  for (const field of ["project_id", "task_id", "session_id", "actor", "expected_version", "idempotency_key", "result_id", "details"]) {
    assert(agentResultSchema?.properties?.[field], `agent_result_record schema exposes ${field}`);
  }

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

  const planScope = {
    project_id: projectId,
    task_id: taskId,
    actor: "contract-test",
  };
  const addedA = await request("tools/call", { name: "plan_node_add", arguments: {
    ...planScope,
    expected_version: 0,
    idempotency_key: "contract.plan.add-a",
    node_id: "node.contract.a",
    title: "Contract A",
    goal: "Verify plan writes",
    scope: ["contracts/v3"],
    dependencies: [],
  } });
  const duplicateA = await request("tools/call", { name: "plan_node_add", arguments: {
    ...planScope,
    expected_version: 0,
    idempotency_key: "contract.plan.add-a",
    node_id: "node.contract.a",
    title: "Contract A",
    goal: "Verify plan writes",
    scope: ["contracts/v3"],
    dependencies: [],
  } });
  assert(addedA.structuredContent.result.status === "appended", "plan_node_add did not append");
  assert(duplicateA.structuredContent.result.status === "duplicate", "plan_node_add idempotency was not preserved");

  const staleAdd = await request("tools/call", { name: "plan_node_add", arguments: {
    ...planScope,
    expected_version: 0,
    idempotency_key: "contract.plan.stale",
    node_id: "node.contract.stale",
    title: "Stale",
    goal: "Reject stale version",
    dependencies: [],
  } });
  assert(staleAdd.isError === true && staleAdd.structuredContent.code === "V3_VERSION_CONFLICT", "plan write did not expose version conflict");

  const addedB = await request("tools/call", { name: "plan_node_add", arguments: {
    ...planScope,
    expected_version: 1,
    idempotency_key: "contract.plan.add-b",
    node_id: "node.contract.b",
    title: "Contract B",
    goal: "Verify dependencies",
    dependencies: ["node.contract.a"],
  } });
  assert(addedB.structuredContent.result.status === "appended", "second plan node did not append");

  const cycled = await request("tools/call", { name: "plan_dependencies_set", arguments: {
    ...planScope,
    expected_version: 2,
    idempotency_key: "contract.plan.cycle",
    node_id: "node.contract.a",
    dependencies: ["node.contract.b"],
  } });
  assert(cycled.isError === true && cycled.structuredContent.code === "V3_PLAN_CYCLE", "plan cycle was not rejected");

  const activated = await request("tools/call", { name: "plan_node_state_set", arguments: {
    ...planScope,
    expected_version: 2,
    idempotency_key: "contract.plan.activate-a",
    node_id: "node.contract.a",
    state: "active",
  } });
  assert(activated.structuredContent.result.status === "appended", "plan state write did not append");
  const planResource = resources.resources.find((candidate) => candidate.name === "plan-graph");
  const changedPlanRead = await request("resources/read", { uri: planResource.uri });
  const changedPlan = JSON.parse(changedPlanRead.contents[0].text);
  assert(changedPlan.nodes.some((node) => node.node_id === "node.contract.a" && node.state === "active"), "plan resource did not reflect plan writes");

  const scope = {
    project_id: projectId,
    task_id: taskId,
    session_id: "session.contract",
    actor: "contract-test",
    idempotency_key: "contract.open.1",
    expected_version: 0,
    working_directory: root,
    node_id: "node.contract.a",
  };
  const opened = await request("tools/call", { name: "session_open", arguments: scope });
  const duplicate = await request("tools/call", { name: "session_open", arguments: scope });
  assert(opened.structuredContent.result.status === "appended", "session_open did not append");
  assert(duplicate.structuredContent.result.status === "duplicate", "idempotency duplicate was not preserved");

  const logged = await request("tools/call", { name: "event_log", arguments: { ...scope, idempotency_key: "contract.log.1", expected_version: 1, kind: "progress", details: { summary: "contract" } } });
  assert(logged.structuredContent.result.status === "appended", "event_log did not append");
  const resultRecorded = await request("tools/call", { name: "agent_result_record", arguments: {
    ...scope,
    idempotency_key: "contract.agent-result.1",
    expected_version: 2,
    result_id: "result.contract.1",
    details: {
      kind: "evaluation",
      request_source: "evaluation_instruction",
      instruction: "Verify the MCP Agent result path",
      status: "succeeded",
      summary: "Agent result contract passed",
      body: "The result is persisted as a typed V3 event.",
      evaluation: { target: "MCP", rubric: ["typed write"], verdict: "passed", findings: [] },
      artifacts: [],
      evidence_refs: [],
    },
  } });
  assert(resultRecorded.structuredContent.result.status === "appended", "agent_result_record did not append");
  const changedTimelineRead = await request("resources/read", { uri: resources.resources.find((candidate) => candidate.name === "task-timeline").uri });
  const changedTimeline = JSON.parse(changedTimelineRead.contents[0].text);
  assert(changedTimeline.events.some((event) => event.summary_key === "agent.result_recorded" && event.session_id === scope.session_id), "task timeline did not reflect Agent result write");

  const closed = await request("tools/call", { name: "session_close", arguments: { ...scope, idempotency_key: "contract.close.1", expected_version: 3 } });
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
