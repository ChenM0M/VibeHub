import { spawn, spawnSync } from "node:child_process";
import { mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const binary = resolve(process.argv[2] ?? (process.platform === "win32"
  ? "target/debug/vibehub.exe"
  : "target/debug/vibehub"));
const base = await mkdtemp(join(tmpdir(), "vibehub-v3-mcp-isolation-"));
const projectA = join(base, "left", "same-name");
const projectB = join(base, "right", "same-name");
const clients = [];
const env = { ...process.env, VIBEHUB_MCP_BINARY: binary };

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function runCli(project, args, input) {
  const result = spawnSync(binary, ["v3", project, ...args], {
    env,
    input,
    encoding: "utf8",
  });
  assert(result.status === 0, args.join(" ") + " failed: " + result.stderr);
  return JSON.parse(result.stdout).result;
}

class Client {
  constructor(root, label) {
    this.label = label;
    this.child = spawn(binary, ["mcp-stdio", root], {
      env,
      stdio: ["pipe", "pipe", "pipe"],
    });
    this.buffer = "";
    this.nextId = 1;
    this.pending = new Map();
    this.stderr = "";
    this.child.stdout.setEncoding("utf8");
    this.child.stderr.setEncoding("utf8");
    this.child.stdout.on("data", (chunk) => this.consume(chunk));
    this.child.stderr.on("data", (chunk) => { this.stderr += chunk; });
    this.child.on("error", (error) => {
      for (const pending of this.pending.values()) pending.reject(error);
      this.pending.clear();
    });
  }

  consume(chunk) {
    this.buffer += chunk;
    for (;;) {
      const newline = this.buffer.indexOf("\n");
      if (newline < 0) return;
      const line = this.buffer.slice(0, newline);
      this.buffer = this.buffer.slice(newline + 1);
      if (!line.trim()) continue;
      const message = JSON.parse(line);
      const waiter = this.pending.get(message.id);
      if (waiter) {
        this.pending.delete(message.id);
        waiter.resolve(message);
      }
    }
  }

  raw(method, params = {}) {
    const id = this.nextId++;
    const promise = new Promise((resolvePromise, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(this.label + " " + method + " timed out"));
      }, 5000);
      this.pending.set(id, {
        resolve: (message) => {
          clearTimeout(timer);
          resolvePromise(message);
        },
        reject,
      });
    });
    this.child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
    return promise;
  }

  notify(method, params = {}) {
    this.child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method, params }) + "\n");
  }

  async request(method, params = {}) {
    const response = await this.raw(method, params);
    assert(!response.error, this.label + " " + method + ": " + JSON.stringify(response.error));
    return response.result;
  }

  async close() {
    this.notify("notifications/cancelled", { requestId: "isolation-smoke" });
    this.child.stdin.end();
    const code = await new Promise((resolveCode) => this.child.once("close", resolveCode));
    assert(code === 0, this.label + " exited " + code + ": " + this.stderr);
  }
}

async function createProject(root, title) {
  await mkdir(root, { recursive: true });
  runCli(root, ["init"]);
  runCli(root, ["task-create", "--stdin"], JSON.stringify({
    title,
    intent: "Verify multi-project MCP isolation",
    acceptance_criteria: ["Project namespace remains isolated"],
    workflow_profile: "standard",
    trigger_context: {},
  }));
}

async function readProject(client, title) {
  const initialized = await client.request("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "vibehub-multi-project-smoke", version: "1.0" },
  });
  assert(initialized.serverInfo.name === "vibehub-v3", client.label + " identity mismatch");
  client.notify("notifications/initialized");
  const resources = await client.request("resources/list");
  const overviewUri = resources.resources.find((item) => item.name === "project-overview").uri;
  const overview = JSON.parse((await client.request("resources/read", { uri: overviewUri })).contents[0].text);
  assert(overview.active_tasks[0].title === title, client.label + " task crossed namespace");
  const diagnosticsUri = resources.resources.find((item) => item.name === "diagnostics").uri;
  const diagnostics = JSON.parse((await client.request("resources/read", { uri: diagnosticsUri })).contents[0].text);
  return { resources, overview, diagnostics };
}

try {
  await createProject(projectA, "Project A task");
  await mkdir(join(projectB, "nested", ".git"), { recursive: true });
  await createProject(projectB, "Project B task");

  const clientA = new Client(projectA, "A");
  const clientB = new Client(projectB, "B");
  clients.push(clientA, clientB);
  const [a, b] = await Promise.all([
    readProject(clientA, "Project A task"),
    readProject(clientB, "Project B task"),
  ]);
  assert(a.overview.project_id !== b.overview.project_id, "same-name project IDs collided");
  assert(b.diagnostics.scopes.execution_root.endsWith("/nested"), "nested execution root missing");
  assert(b.diagnostics.scopes.git_root.endsWith("/nested"), "nested git root missing");

  const candidatesA = await clientA.request("tools/call", {
    name: "task_candidates",
    arguments: { project_id: a.overview.project_id },
  });
  assert(candidatesA.structuredContent.result[0].task_id, "A candidates empty");
  const foreign = await clientA.request("tools/call", {
    name: "task_candidates",
    arguments: { project_id: b.overview.project_id },
  });
  assert(foreign.isError === true && foreign.structuredContent.code === "V3_PROJECT_MISMATCH", "foreign write not rejected");
  const foreignResource = await clientA.raw("resources/read", {
    uri: "vibehub://v3/1.0/projects/" + b.overview.project_id + "/overview",
  });
  assert(foreignResource.error?.data?.code === "V3_PROJECT_MISMATCH", "foreign resource not structured");

  const session = {
    project_id: a.overview.project_id,
    task_id: a.overview.active_tasks[0].task_id,
    session_id: "session.isolation.a",
    actor: "multi-project-smoke",
    node_id: "node." + a.overview.active_tasks[0].task_id + ".initial",
  };
  const binding = await clientA.request("tools/call", {
    name: "session_task_bind",
    arguments: {
      project_id: session.project_id,
      task_id: session.task_id,
      session_id: session.session_id,
      interaction_id: session.session_id,
      actor: session.actor,
      source: "explicit_task_id",
    },
  });
  assert(binding.structuredContent?.result?.status === "appended", "A session bind failed: " + JSON.stringify(binding));
  const bindingRevision = binding.structuredContent?.result?.event?.payload?.binding?.binding_revision;
  assert(Number.isInteger(bindingRevision), "A binding revision missing: " + JSON.stringify(binding));
  const added = await clientA.request("tools/call", {
    name: "plan_node_add",
    arguments: {
      project_id: session.project_id,
      task_id: session.task_id,
      actor: session.actor,
      session_id: session.session_id,
      binding_revision: bindingRevision,
      node_id: session.node_id,
      title: "Initial isolation node",
      goal: "Verify multi-project MCP isolation",
      scope: [],
      dependencies: [],
      criterion_ids: [],
    },
  });
  assert(added.structuredContent?.result?.status === "appended", "A plan node add failed: " + JSON.stringify(added));
  const activated = await clientA.request("tools/call", {
    name: "plan_node_state_set",
    arguments: {
      project_id: session.project_id,
      task_id: session.task_id,
      actor: session.actor,
      session_id: session.session_id,
      binding_revision: bindingRevision,
      node_id: session.node_id,
      state: "active",
    },
  });
  assert(activated.structuredContent?.result?.status === "appended", "A plan activation failed: " + JSON.stringify(activated));
  for (const [name, arguments_] of [
    ["session_open", session],
    ["event_log", { ...session, binding_revision: bindingRevision, kind: "progress", details: { summary: "A progress" } }],
    ["agent_result_record", {
      ...session,
      binding_revision: bindingRevision,
      node_id: "node." + session.task_id + ".initial",
      result_id: "result.isolation.a",
      details: {
        kind: "execution",
        request_source: "user_request",
        instruction: "Verify project isolation",
        status: "succeeded",
        summary: "A result",
      },
    }],
    ["session_close", { ...session, binding_revision: bindingRevision }],
  ]) {
    const result = await clientA.request("tools/call", { name, arguments: arguments_ });
    assert(result.structuredContent?.result?.status === "appended", name + " failed: " + JSON.stringify(result));
  }
  const aTimelineUri = a.resources.resources.find((item) => item.name === "task-timeline").uri;
  const aTimeline = JSON.parse((await clientA.request("resources/read", { uri: aTimelineUri })).contents[0].text);
  assert(aTimeline.events.some((event) => event.session_id === session.session_id), "A event missing");
  const bTimelineUri = b.resources.resources.find((item) => item.name === "task-timeline").uri;
  const bTimeline = JSON.parse((await clientB.request("resources/read", { uri: bTimelineUri })).contents[0].text);
  assert(!bTimeline.events.some((event) => event.session_id === session.session_id), "A event leaked to B");

  let aliasStatus = "skipped";
  if (process.platform !== "win32") {
    const alias = join(base, "alias");
    await symlink(projectA, alias, "dir");
    const aliasClient = new Client(alias, "alias");
    clients.push(aliasClient);
    const aliasView = await readProject(aliasClient, "Project A task");
    assert(aliasView.overview.project_id === a.overview.project_id, "symlink changed project identity");
    await aliasClient.close();
    clients.pop();
    aliasStatus = "passed";
  }

  const untrusted = join(base, "untrusted");
  await mkdir(untrusted, { recursive: true });
  const untrustedResult = spawnSync(binary, ["mcp-stdio", untrusted], { env, encoding: "utf8" });
  assert(untrustedResult.status !== 0, "untrusted project started MCP");

  await clientA.close();
  await clientB.close();
  clients.length = 0;
  console.log(JSON.stringify({
    status: "passed",
    project_ids: [a.overview.project_id, b.overview.project_id],
    nested_git_scope: b.diagnostics.scopes,
    symlink_alias: aliasStatus,
    untrusted_project: "rejected",
  }, null, 2));
} finally {
  for (const client of clients) {
    try { await client.close(); } catch {}
  }
  await rm(base, { recursive: true, force: true });
}
