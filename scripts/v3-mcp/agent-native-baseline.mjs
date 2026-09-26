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

await mkdir(join(root, ".vibehub", "tasks"), { recursive: true });
await mkdir(join(root, ".vibehub", "tasks", taskId), { recursive: true });
await writeFile(join(root, ".vibehub", "project.yaml"), "schema_version: 3\nname: mcp-contract\nproject_id: project.mcp-contract\n");
await writeFile(join(root, ".vibehub", "tasks", taskId, "task.yaml"), taskYaml);
await writeFile(join(root, ".vibehub", "tasks", "current"), [
  "schema_version: 1",
  "kind: current_task_pointer",
  `task_id: ${taskId}`,
  `path: .vibehub/tasks/${taskId}`,
  "updated_at: 2026-08-14T00:00:00Z",
  "updated_by: vibehub",
  "",
].join("\n"));

// Cold initialization includes SQLite migration and is separate from warm RPC latency.
const startupBudgetMs = 30000;
const requestBudgetMs = 2000;
const startedAt = performance.now();
const child = spawn(binary, ["mcp-stdio", root], { stdio: ["pipe", "pipe", "pipe"] });
// Subscribe immediately: cleanup must wait for close, not just successful kill().
let childClosed = false;
let childSpawnError;
const childClose = new Promise((resolveClose) => {
  child.once("error", (error) => { childSpawnError = error; });
  child.once("close", (code) => { childClosed = true; resolveClose(code); });
});
let stdoutBuffer = "";
let stderr = "";
let nextId = 1;
const pending = new Map();
const timings = [];
const responseWireBytes = new WeakMap();
childClose.then((code) => {
  for (const waiter of pending.values()) waiter.reject(childSpawnError ?? new Error(`MCP exited with ${code}; ${stderr}`));
  pending.clear();
});

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
      waiter.resolve(message, Buffer.byteLength(line) + 1);
    }
  }
});

function request(method, params = {}) {
  const operation = params.name ?? params.uri ?? method;
  const id = nextId++;
  const began = performance.now();
  const response = new Promise((resolveRequest, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} (${operation}) timed out; completed requests: ${JSON.stringify(timings)}${stderr ? `; child stderr: ${stderr}` : ""}`));
    }, startupBudgetMs);
    pending.set(id, {
      reject(error) { clearTimeout(timer); reject(error); },
      resolve(message, wireBytes) {
        clearTimeout(timer);
        timings.push({ method, operation, milliseconds: Math.round(performance.now() - began) });
        if (message.error) reject(new Error(`${method}: ${JSON.stringify(message.error)}`));
        else {
          responseWireBytes.set(message.result, wireBytes);
          resolveRequest(message.result);
        }
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
  const initialize = await request("initialize", {protocolVersion:"2025-11-25",capabilities:{},clientInfo:{name:"agent-native-baseline",version:"1"}});
  notify("notifications/initialized");
  const catalog = await request("tools/list");
  const samples = [];
  for (let i=0;i<35;i++) {
    const start=performance.now();
    const response=await request("tools/call",{name:"task_view",arguments:{task_id:taskId}});
    if(i>=5) samples.push({ms:performance.now()-start,result_bytes:Buffer.byteLength(JSON.stringify(response.structuredContent)),wire_bytes:responseWireBytes.get(response)});
  }
  samples.sort((a,b)=>a.ms-b.ms);
  console.log(JSON.stringify({server:initialize.serverInfo,platform:process.platform,arch:process.arch,node:process.version,fixture:"isolated small task; authoritative metadata, no fabricated event log",warmup:5,samples:30,p50_ms:samples[15].ms,p95_ms:samples[28].ms,result_bytes:samples[0].result_bytes,wire_bytes:samples[0].wire_bytes,catalog_bytes:Buffer.byteLength(JSON.stringify(catalog)),tool_count:catalog.tools.length,model_visible_tokens:"unknown",host:"raw stdio; not host acceptance"},null,2));
  child.stdin.end(); await childClose;
} finally {
  if(!childClosed) child.kill("SIGKILL");
  await childClose;
  await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});
}
