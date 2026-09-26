import assert from "node:assert/strict";
import { mkdtemp,mkdir,writeFile,rm } from "node:fs/promises";
import {tmpdir} from "node:os";
import {resolve,join} from "node:path";
import {Client} from "./client.mjs";
const root=await mkdtemp(join(tmpdir(),"vibehub-agent-recovery-"));
const binary=resolve(process.env.VIBEHUB_TEST_BINARY??(process.platform==="win32"?"target/debug/vibehub.exe":"target/debug/vibehub"));let client;
try {
 await mkdir(join(root,".vibehub/tasks/task.test"),{recursive:true});
 await writeFile(join(root,".vibehub/project.yaml"),"schema_version: 3\nproject_id: project.native\nname: recovery\n");
 await writeFile(join(root,".vibehub/tasks/task.test/task.yaml"),"task_id: task.test\ntitle: Recovery\nintent: Verify operation recovery\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria: []\n");
 client=new Client(binary,root,{VIBEHUB_MCP_CATALOG:"agent"});await client.init();
 const catalog=await client.request("tools/list");assert.equal(catalog.tools.length,12);
 assert(!catalog.tools.some(t=>t.name==="task_view"||t.name==="plan_node_add"));
 const start={project_id:"project.native",task_id:"task.test",session_id:"session.native",actor:"test",interaction_id:"interaction.native",working_directory:root,request_id:"start",expected_binding_revision:0};
 const opened=await client.call("task_start",start);assert(!opened.isError,JSON.stringify(opened));
 const handle=opened.structuredContent.context_handle;
 const record={request_id:"record",context_handle:handle,kind:"progress",details:{summary:"real fixture milestone"}};
 const response=await client.call("task_record",record);assert(!response.isError);const original=response.structuredContent;
 // Treat the response as lost, kill the actual server process, then resume through
 // a fresh connection using the retained business request ID and explicit scope.
 await client.kill();client=new Client(binary,root,{VIBEHUB_MCP_CATALOG:"agent"});await client.init();
 const expired=await client.call("task_record",record);assert.equal(expired.structuredContent.code,"CONTEXT_HANDLE_EXPIRED");
 const scope={project_id:"project.native",task_id:"task.test",session_id:"session.native",actor:"test",binding_revision:1};
 const replay=await client.call("task_record",{...scope,request_id:"record",kind:"progress",details:record.details});assert(!replay.isError);assert.deepEqual(replay.structuredContent,original);
 const changed=await client.call("task_record",{...scope,request_id:"record",kind:"progress",details:{summary:"changed"}});assert.equal(changed.structuredContent.code,"IDEMPOTENCY_PAYLOAD_CONFLICT");
 const finished=await client.call("session_finish",{...scope,request_id:"finish",details:{kind:"execution",request_source:"user_request",instruction:"recovery verification",status:"succeeded",summary:"verified"}});assert(!finished.isError,JSON.stringify(finished));
 assert(Buffer.byteLength(JSON.stringify(finished.structuredContent))<=2048);
 await client.close();client=new Client(binary,root,{VIBEHUB_MCP_CATALOG:"legacy"});await client.init();
 const status=await client.call("operation_status",{task_id:"task.test",session_id:"session.native",request_id:"finish"});assert.equal(status.structuredContent.ok,true);
 const legacy=await client.call("task_view",{task_id:"task.test"});assert.equal(legacy.structuredContent.result.task_timeline.events.length,5);
 assert.notEqual(legacy.structuredContent.result.node_brief.task_state,"completed","Session finishing must not complete task");
 console.log(JSON.stringify({status:"passed",agent_tools:catalog.tools.length,catalog_bytes:Buffer.byteLength(JSON.stringify(catalog)),catalog_tokens:"unknown",checks:["fixed agent catalog","actual process kill/restart","expired handle rejected","same request replay returns same receipt","different payload rejected","terminal result and close","legacy rollback reads same five facts","no automatic task completion"],model_visible_tokens:"unknown",host:"raw stdio"},null,2));
} finally {if(client)await client.kill();if (process.env.VIBEHUB_TEST_RETAIN_TEMP !== "1") await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});}
