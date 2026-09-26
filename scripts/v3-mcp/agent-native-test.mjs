import { spawn } from "node:child_process";
import { mkdtemp, mkdir, readFile, readdir, rm, writeFile, utimes } from "node:fs/promises";
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
      waiter.resolve(message, Buffer.byteLength(line)+1);
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
        timings.push({ method, operation, wire_bytes: wireBytes, milliseconds: Math.round(performance.now() - began) });
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
  await request("initialize",{protocolVersion:"2025-11-25",capabilities:{},clientInfo:{name:"agent-native-contract",version:"1"}});
  notify("notifications/initialized");
  const catalog=await request("tools/list");
  const call=async(name,args)=>{ const r=await request("tools/call",{name,arguments:args}); if(r.isError) throw new Error(`${name}: ${JSON.stringify(r.structuredContent)}`); return r.structuredContent; };
  const failure=async(name,args,code)=>{const r=await request("tools/call",{name,arguments:args});assert(r.isError&&r.structuredContent.code===code,JSON.stringify(r));return r.structuredContent;};
  const ajv=new Ajv2020({strict:false,validateFormats:false});
  const responseSchema=JSON.parse(await readFile("contracts/v3/agent-read.schema.json","utf8"));
  const validateResponse=ajv.compile(responseSchema);
  for(const tool of catalog.tools) ajv.compile(tool.inputSchema);
  const newNames=["workspace_context","task_brief","task_inspect","task_start","task_record","session_finish","operation_status","diagnostic_export","diagnostic_read"];
  const contract=catalog.tools.filter(t=>newNames.includes(t.name)).map(t=>({name:t.name,inputSchema:t.inputSchema})).sort((a,b)=>a.name.localeCompare(b.name));
  const contractPath="contracts/v3/agent-tools.contract.json";
  if(process.env.UPDATE_AGENT_CONTRACT==="1")await writeFile(contractPath,JSON.stringify({contract_version:"agent-tools/1",tools:contract},null,2)+"\n");
  const frozen=JSON.parse(await readFile(contractPath,"utf8"));
  assert(JSON.stringify(contract)===JSON.stringify(frozen.tools),"published schemas match frozen contract; explicit contract update required");

  for(const [name,field,value] of [["task_route","trigger","new_execution"],["criterion_review","outcome","passed"],["event_log","kind","risk"]]) {
    assert(catalog.tools.find(t=>t.name===name).inputSchema.properties[field].enum.includes(value),`${name}.${field} enum`);
  }
  const workspace=await call("workspace_context",{});
  assert(validateResponse(workspace),JSON.stringify(validateResponse.errors));
  assert(workspace.data.candidates.length===1,"one candidate");
  const before=await call("task_brief",{task_id:taskId});
  assert(validateResponse(before),JSON.stringify(validateResponse.errors));
  assert(before.data.goal==="Verify the minimal recovery loop" && before.data.metadata_acceptance.length===1,"sufficient work context");
  assert(before.next_step.kind==="input_required" && before.data.binding===null,"no implicit binding");
  const unchanged=await call("task_brief",{task_id:taskId,if_revision:before.revision});
  assert(validateResponse(unchanged),JSON.stringify(validateResponse.errors));
  assert(unchanged.unchanged&&!unchanged.data&&Buffer.byteLength(JSON.stringify(unchanged))<=1024,"bounded unchanged");
  await failure("task_brief",{task_id:"../task.contract"},"V3_ID_INVALID");
  const unknownTask=await failure("task_brief",{task_id:"task.does-not-exist"},"V3_TASK_NOT_FOUND");
  assert(unknownTask.recovery.tool==="workspace_context"&&Object.keys(unknownTask.recovery.params).length===0,"unknown task gives bounded identity recovery rather than commit inspection");
  await call(unknownTask.recovery.tool,unknownTask.recovery.params);
  for(const name of ["task_record","session_finish"])assert(catalog.tools.find(t=>t.name===name).inputSchema.properties.details.type==="object","nested details schema is inline for host compatibility");
  const stringDetails=await failure("task_record",{context_handle:"not-a-real-handle",request_id:"bad-details",kind:"progress",details:'{"summary":"must remain an object"}'},"MCP_INPUT_INVALID");
  assert(stringDetails.field_errors.some(e=>e.field==="details"&&e.expected_type==="object"),"JSON strings do not bypass typed object validation");

  const missing=await failure("task_brief",{},"MCP_INPUT_INVALID");
  assert(missing.field_errors.some(e=>e.field==="task_id"&&e.required),"missing required field is identified");
  const invalid=await failure("task_brief",{task_id:123},"MCP_INPUT_INVALID");
  assert(invalid.field_errors.some(e=>e.field==="task_id"&&e.expected_type==="string"),"field correction information");
  await call("task_brief",{task_id:taskId});
  await failure("task_brief",{task_id:taskId,password:"must-not-echo"},"INPUT_UNKNOWN_FIELD");
  await failure("task_inspect",{task_id:taskId,section:"timeline",time_from:"invalid"},"TIME_FILTER_INVALID");

  await failure("task_inspect",{task_id:taskId,entity_kind:"event",entity_id:"not-visible"},"ENTITY_NOT_FOUND");
  const projectId=workspace.scope.project_id;
  const scope={project_id:projectId,task_id:taskId,session_id:"session.native",actor:"contract-test"};
  await call("session_open",{...scope,working_directory:root});
  const resumed=await call("workspace_context",{session_id:scope.session_id});
  assert(resumed.data.binding.bound_task_id===taskId,"resume includes authoritative binding");
  assert(validateResponse(resumed),JSON.stringify(validateResponse.errors));
  assert(resumed.next_step.kind==="tool_call","bound standard task without a node needs bounded plan selection");
  const suggested=catalog.tools.find(t=>t.name===resumed.next_step.tool);
  assert(suggested&&ajv.validate(suggested.inputSchema,resumed.next_step.params),"suggestion validates against actual catalog: "+JSON.stringify(ajv.errors));
  await call(resumed.next_step.tool,resumed.next_step.params);
  const compatibility=await call("v3_next_action",{project_id:projectId,task_id:taskId,session_id:scope.session_id});
  assert(compatibility.result.next_action.executable&&compatibility.result.next_action.tool===resumed.next_step.tool,"legacy maps only callable typed suggestions");
  const binding_revision=resumed.data.binding.binding_revision;
  await failure("event_log",{...scope,binding_revision:binding_revision+1,kind:"progress",details:{summary:"rejected"}},"V3_TASK_BINDING_REVISION_CONFLICT");
  const versionConflict=await failure("event_log",{...scope,binding_revision,expected_version:0,kind:"progress",details:{summary:"must not commit"}},"V3_VERSION_CONFLICT");
  assert(ajv.validate(catalog.tools.find(t=>t.name===versionConflict.recovery.tool).inputSchema,versionConflict.recovery.params),"version refresh has complete valid parameters");
  await call(versionConflict.recovery.tool,versionConflict.recovery.params);
  const long="中文 evidence ".repeat(9000);
  const write=await call("event_log",{...scope,binding_revision,kind:"progress",details:{summary:long},idempotency_key:"native.long"});
  assert(write.result.status==="appended","real event append");
  const ids=[]; let cursor; let firstCursor;
  do {
    const page=await call("task_inspect",{task_id:taskId,section:"timeline",limit:1,...(cursor?{cursor}:{})});
    ids.push(...page.data.items.map(e=>e.event_id)); cursor=page.data.next_cursor;firstCursor??=cursor;
    assert(Buffer.byteLength(JSON.stringify(page))<=16384,"page budget");
  } while(cursor);
  assert(new Set(ids).size===ids.length&&ids.length===3,"paging includes binding, opening, progress exactly once");
  const legacy=await call("task_view",{task_id:taskId});
  assert(legacy.result.task_timeline.events.length===ids.length,"page traversal matches full facts");
  const progress=await call("task_inspect",{task_id:taskId,section:"timeline",event_type:"progress.logged",session_id:scope.session_id});
  assert(progress.data.items.length===1,"server filter");
  const eventId=progress.data.items[0].event_id;
  let offset=0,body="",hash;
  do {
    const part=await call("task_inspect",{task_id:taskId,entity_kind:"event",entity_id:eventId,offset,max_bytes:4096});
    assert(Buffer.byteLength(JSON.stringify(part))<=4096,"chunk bounded");
    body+=part.data.text;hash=part.data.sha256;offset=part.data.next_offset;
  } while(offset!==null);
  assert(JSON.parse(body).payload.details?.summary===long || JSON.parse(body).payload.summary===long,"long event reconstructs exactly");
  await call("event_log",{...scope,binding_revision,kind:"progress",details:{summary:"next"},idempotency_key:"native.next"});
  const stale=await failure("task_inspect",{task_id:taskId,section:"timeline",limit:1,cursor:firstCursor},"CURSOR_STALE");
  assert(ajv.validate(catalog.tools.find(t=>t.name===stale.recovery.tool).inputSchema,stale.recovery.params),"recovery schema");
  await call(stale.recovery.tool,stale.recovery.params);
  const changed=await call("task_brief",{task_id:taskId,if_revision:before.revision});
  assert(!changed.unchanged,"event invalidates brief");
  await writeFile(join(root,".vibehub","project-settings.yaml"),"schema_version: 1\nrevision: 1\n");
  const configChanged=await call("task_brief",{task_id:taskId,if_revision:changed.revision});
  assert(!configChanged.unchanged,"configuration invalidates unchanged");
  const created=(await call("task_create",{project_id:projectId,title:"Scoped acceptance fixture",intent:"Verify current-node context and direct entity semantics",acceptance_criteria:["Check A","Check B","Check C"],workflow_profile:"standard",initial_plan:[{title:"Implement fixture",goal:"Exercise a real authored node",scope:["fixture/"],criteria:[1,2,3],depends_on:[]}]})).result;
  const authored=created.task_id;
  const plan=await call("task_inspect",{task_id:authored,section:"plan"});
  const node=plan.data.items[0];
  const started=await call("task_start",{project_id:projectId,task_id:authored,session_id:"session.authored",actor:"contract-test",interaction_id:"interaction.authored",working_directory:root,node_id:node.node_id,expected_binding_revision:0,request_id:"start.authored"});
  assert(started.completed_steps.join(",")==="bind,activate,open","authored start executes all validated steps");
  const authoredBrief=await call("workspace_context",{session_id:"session.authored"});
  assert(authoredBrief.next_step.kind==="work_required"&&authoredBrief.data.criteria.length===3&&authoredBrief.data.scope[0]==="fixture/","one read contains scoped goal/policy/criteria without premature review");
  assert(authoredBrief.data.working_directory.path===root&&authoredBrief.data.working_directory.source==="session_opened","real execution directory is in the initial context");
  const authoredLegacy=(await call("task_view",{task_id:authored,node_id:node.node_id})).result;
  assert(authoredLegacy.node_brief.goal===authoredBrief.data.goal&&authoredLegacy.node_brief.execution_policy.planning_required===authoredBrief.data.execution_policy.planning_required,"legacy/new same-snapshot goal and policy");
  await call("agent_result_record",{project_id:projectId,task_id:authored,session_id:"session.authored",binding_revision:started.binding_revision,actor:"contract-test",result_id:"result.fixture",node_id:node.node_id,details:{kind:"execution",request_source:"user_request",instruction:"Exercise fixture reads",status:"succeeded",summary:"Actual scoped read checks completed; criterion reviews still pending"}});
  const verificationNeeded=await call("workspace_context",{session_id:"session.authored"});
  assert(verificationNeeded.next_step.kind==="evidence_required"&&verificationNeeded.data.criteria.every(c=>c.state==="accepted"),"recording result does not pass criteria and moves suggestion to real verification");
  const criterion=authoredBrief.data.criteria[0].criterion_id;
  await call("criterion_review",{project_id:projectId,task_id:authored,session_id:"session.authored",binding_revision:started.binding_revision,actor:"contract-test",criterion_id:criterion,outcome:"failed",reviewer:"fixture-reviewer",evidence_refs:["test:controlled-fixture-failure"],details:{summary:"Controlled negative fixture, not production acceptance"}});
  const failed=await call("task_inspect",{task_id:authored,section:"criteria",state:"failed",node_id:node.node_id,limit:1});
  assert(failed.data.items.length===1&&failed.data.items[0].criterion_id===criterion,"server status/node filtering");
  const entity=await call("task_inspect",{task_id:authored,entity_kind:"criterion",entity_id:criterion,include:["evidence"]});
  assert(entity.data.state==="failed"&&entity.data.evidence_ids[0]==="test:controlled-fixture-failure","direct entity and deduplicated evidence IDs");
  const evidence=await call("task_inspect",{task_id:authored,entity_kind:"evidence",entity_id:"test:controlled-fixture-failure"});
  assert(evidence.data.source==="registered_reference"&&evidence.data.body_available===false,"reference resolution does not invent a captured body");
  await failure("task_inspect",{task_id:taskId,entity_kind:"evidence",entity_id:"test:controlled-fixture-failure"},"ENTITY_NOT_FOUND");
  const failedEvents=await call("task_inspect",{task_id:authored,section:"timeline",state:"failed"});
  assert(failedEvents.data.items.length===1,"failed event filtering is server-side");
  const seq=failedEvents.data.items[0].sequence;
  const exactEvent=await call("task_inspect",{task_id:authored,section:"timeline",sequence_from:seq,sequence_to:seq});
  assert(exactEvent.data.items.length===1&&exactEvent.data.items[0].event_id===failedEvents.data.items[0].event_id,"sequence range filter");
  const allIds=[];let criterionCursor;
  do {const page=await call("task_inspect",{task_id:authored,section:"criteria",limit:1,...(criterionCursor?{cursor:criterionCursor}:{})});allIds.push(...page.data.items.map(c=>c.criterion_id));criterionCursor=page.data.next_cursor;}while(criterionCursor);
  assert(new Set(allIds).size===3,"domain pagination reaches every criterion");
  const blocked=await call("task_brief",{task_id:authored,session_id:"session.authored"});
  assert(blocked.data.blockers.some(b=>b.entity_id===criterion)&&blocked.next_step.kind!=="tool_call","failed validation remains visible and is not auto-passed");
  await call("event_log",{...scope,binding_revision,kind:"progress",details:{summary:"Secret-field redaction fixture",api_key:"fixture-secret-never-real"},idempotency_key:"native.redaction"});
  const exported=await call("diagnostic_export",{task_id:taskId});
  assert(Buffer.byteLength(JSON.stringify(exported))<=2048,"diagnostic manifest bounded");
  let exportedText="",exportOffset=0;
  do {const chunk=await call("diagnostic_read",{export_id:exported.export_id,offset:exportOffset,max_bytes:16384});exportedText+=chunk.text;exportOffset=chunk.next_offset;}while(exportOffset!==null);
  const snapshot=JSON.parse(exportedText);
  assert(snapshot.events.length===5&&snapshot.scope.task_id===taskId,"export snapshot matches declared task scope");
  const {createHash}=await import("node:crypto");
  assert(createHash("sha256").update(exportedText).digest("hex")===exported.sha256,"export hash round trip");
  await call("diagnostic_read",{export_id:exported.export_id,action:"delete"});
  await failure("diagnostic_read",{export_id:exported.export_id},"EXPORT_NOT_FOUND");
  assert(!exportedText.includes("fixture-secret-never-real")&&exportedText.includes("[REDACTED]"),"diagnostic removes credential fields");
  const projectExport=await call("diagnostic_export",{scope:"project"});
  let projectText="",projectOffset=0;
  do {const chunk=await call("diagnostic_read",{export_id:projectExport.export_id,offset:projectOffset});projectText+=chunk.text;projectOffset=chunk.next_offset;}while(projectOffset!==null);
  const projectSnapshot=JSON.parse(projectText);
  assert(projectSnapshot.task_metadata.length===2&&projectSnapshot.events.filter(e=>e.task_id===taskId).length===snapshot.events.length&&projectSnapshot.events.some(e=>e.task_id===authored),"project export includes declared tasks and domain events");
  assert(!projectText.includes("fixture-secret-never-real")&&!projectSnapshot.projection.project_memory,"project export redaction and Memory exclusion");
  await call("event_log",{...scope,binding_revision,kind:"progress",details:{summary:"A real post-export fact"},idempotency_key:"native.after-export"});
  await failure("diagnostic_export",{scope:"project",if_revision:projectExport.revision},"READ_REVISION_CHANGED");
  const oldTime=new Date(Date.now()-90000*1000);
  await utimes(join(root,".vibehub/diagnostics",projectExport.export_id+".json"),oldTime,oldTime);
  await failure("diagnostic_read",{export_id:projectExport.export_id},"EXPORT_EXPIRED");
  await call("diagnostic_read",{export_id:projectExport.export_id,action:"delete"});

  const samples=[];
  for(let i=0;i<35;i++){const start=performance.now();const r=await call("task_brief",{task_id:taskId});if(i>=5)samples.push({ms:performance.now()-start,bytes:Buffer.byteLength(JSON.stringify(r)),wire_bytes:timings.at(-1).wire_bytes});}
  samples.sort((a,b)=>a.ms-b.ms);
  console.log(JSON.stringify({status:"passed",checks:["frozen real tools schema","schema enums","field error repair","schema-valid executable next-step","no implicit binding","bound recovery","binding rejection","bounded pages","complete traversal","indexed direct evidence","UTF-8 chunks","cursor invalidation","settings invalidation","authored plan start","one-read scoped work","evidence-required after actual result","legacy policy parity","direct criterion/evidence","cross-task evidence rejection","failed event and sequence filters","criterion page traversal","task/project export and secret exclusion","expiry and explicit cleanup","bounded snapshot export/hash/cleanup"],tools:catalog.tools.length,samples:30,p50_ms:samples[15].ms,p95_ms:samples[28].ms,result_bytes:samples[0].bytes,wire_bytes:samples[0].wire_bytes,legacy_result_bytes:Buffer.byteLength(JSON.stringify(legacy)),model_visible_tokens:"unknown",host:"raw stdio; not host acceptance",event_body_sha256:hash},null,2));
  child.stdin.end();await childClose;
} finally {
  if(!childClosed)child.kill("SIGKILL");await childClose;
  await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});
}
