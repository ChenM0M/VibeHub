// Paired, same-fact MCP interaction measurement. B0 is a real baseline binary.
// Typed writes shared by both workflows are charged equally; fixture setup is not.
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {Client} from './client.mjs';
const baseline=resolve(process.env.VIBEHUB_BASELINE_BINARY??'/tmp/vibehub-b0-target/debug/vibehub');
const binary=resolve(process.env.VIBEHUB_TEST_BINARY??'target/debug/vibehub');
const historyCount=Number(process.env.VIBEHUB_MATRIX_HISTORY??1000);
const root=await mkdtemp(join(tmpdir(),'vibehub-paired-'));
const tokenizer=spawn(process.env.VIBEHUB_TOKENIZER_PYTHON??'/tmp/vibehub-tokenizer-env/bin/python',['-u','-c',`import sys,json,tiktoken
enc=tiktoken.get_encoding('o200k_base')
for line in sys.stdin:
 x=json.loads(line);print(json.dumps({'tokens':len(enc.encode(x['text'],disallowed_special=()))}),flush=True)`],{stdio:['pipe','pipe','inherit']});
const tokenQueue=[];createInterface({input:tokenizer.stdout}).on('line',line=>tokenQueue.shift()(JSON.parse(line).tokens));
const countTokens=text=>new Promise(resolve=>{tokenQueue.push(resolve);tokenizer.stdin.write(JSON.stringify({text})+'\n');});
let newer,older,group,lane;const reports=[];const pendingTokens=[];
const observe=entry=>{
 if(!group||!lane||entry.method!=='tools/call')return;
 const text=JSON.stringify(entry.result.structuredContent??entry.result);
 const target=group[lane];target.calls++;target.result_bytes+=Buffer.byteLength(text);target.wire_bytes+=entry.wire_bytes;
 if(entry.result.isError)target.error_calls++;
 pendingTokens.push(countTokens(text).then(tokens=>target.reference_tokens+=tokens));
 target.tools.push(entry.params.name);
};
const stats=()=>({calls:0,result_bytes:0,wire_bytes:0,reference_tokens:0,error_calls:0,tools:[]});
const measured=async(which,fn)=>{lane=which;try{return await fn();}finally{lane=undefined;}};
const call=async(c,name,args={},error=false)=>{const r=await c.call(name,args);assert.equal(!!r.isError,error,`${name}: ${JSON.stringify(r.structuredContent)}`);return r.structuredContent;};
const scenario=async(id,label,fn)=>{group={id,label,baseline:stats(),new:stats(),assertions:[]};await fn();reports.push(group);group=undefined;};
let task='task.paired';const project='project.paired';const sid='session.paired';
const scope={project_id:project,task_id:task,session_id:sid,binding_revision:1,actor:'fixture'};
const oldView=(task_id=task,node_id)=>measured('baseline',()=>call(older,'task_view',{task_id,...(node_id?{node_id}:{})}));
const fresh=(name,args,error=false)=>measured('new',()=>call(newer,name,args,error));
const bothWrite=async(name,args,error=false)=>{
 const r=await measured('baseline',()=>call(older,name,args,error));
 // The new workflow can retain this unchanged typed write. Charge its complete
 // baseline receipt, including failures, once to each lane (no invented savings).
 const before=group.baseline;
 const request=group.baseline.tools.at(-1); // exact MCP receipt already measured
 const text=JSON.stringify(r);group.new.calls++;group.new.result_bytes+=Buffer.byteLength(text);
 group.new.error_calls+=error?1:0;group.new.tools.push(request);
 // Protocol IDs can differ by one byte; use the actual observed baseline frame.
 group.new.wire_bytes+=older.lastWireBytes;
 const target=group.new;pendingTokens.push(countTokens(text).then(n=>target.reference_tokens+=n));
 return r;
};
const parity=(brief,legacy)=>{assert.equal(brief.data.goal,legacy.result.node_brief.goal);assert.equal(brief.data.state,legacy.result.node_brief.task_state);assert.equal(brief.data.execution_policy.effective_profile,legacy.result.node_brief.execution_policy.effective_profile);assert(brief.data.constraints);group.assertions.push('same goal, state, effective policy and explicit constraints');};
try{
 await mkdir(join(root,'.vibehub/tasks',task),{recursive:true});
 await writeFile(join(root,'.vibehub/project.yaml'),`schema_version: 3\nproject_id: ${project}\nname: paired\n`);
 await writeFile(join(root,'.vibehub/tasks',task,'task.yaml'),`task_id: ${task}\ntitle: Paired interaction\nintent: 中文意图：验证全部必要上下文与真实交互成本\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria:\n- Verify actual fixture reads\n`);
 newer=new Client(binary,root);older=new Client(baseline,root);await newer.init();await older.init();
 newer.observe=e=>{newer.lastWireBytes=e.wire_bytes;observe(e);};older.observe=e=>{older.lastWireBytes=e.wire_bytes;observe(e);};
 task=(await call(older,'task_create',{project_id:project,title:'Paired execution',intent:'中文意图：验证全部必要上下文与真实交互成本',workflow_profile:'lightweight',acceptance_criteria:['Verify actual fixture reads'],initial_plan:[]})).result.task_id;scope.task_id=task;
 const catalogs={baseline:await older.request('tools/list'),new:await newer.request('tools/list')};
 await call(older,'session_task_bind',{...scope,interaction_id:'interaction.paired',source:'user_confirmed',expected_binding_revision:0});
 await call(older,'session_open',{...scope,working_directory:root});
 await scenario('S01','Bound continuation and unchanged read',async()=>{
  const old=await oldView();const brief=await fresh('workspace_context',{session_id:sid});parity(brief,old);
  await oldView();const unchanged=await fresh('task_brief',{task_id:task,session_id:sid,if_revision:brief.revision});assert(unchanged.unchanged);assert(Buffer.byteLength(JSON.stringify(unchanged))<=1024);group.assertions.push('unchanged response, no supplementary context queries');
 });
 await scenario('S02','Read-only selection, Chinese intent and scope rejection',async()=>{
  await measured('baseline',()=>call(older,'task_candidates',{project_id:project}));const ws=await fresh('workspace_context',{});assert.equal(ws.data.candidates.length,2);assert.equal(ws.next_step.kind,'input_required');
  parity(await fresh('task_brief',{task_id:task}),await oldView());
  await bothWrite('event_log',{...scope,project_id:'project.foreign',kind:'progress',details:{summary:'must reject'}},true);
  group.assertions.push('read-only queries do not bind; mismatched project rejected');
 });
 const authored=[];
 for(const profile of ['standard','full']){
  const created=(await call(older,'task_create',{project_id:project,title:profile,intent:'Scoped work',workflow_profile:profile,acceptance_criteria:['Current check','Dependent check'],initial_plan:[{title:'Current',goal:'Verify current work',scope:['fixture/current'],criteria:[1],depends_on:[]},{title:'Dependent',goal:'Verify dependencies',criteria:[2],depends_on:[1]}]})).result;
  const nodes=(await call(newer,'task_inspect',{task_id:created.task_id,section:'plan'})).data.items;
  const n=nodes.find(n=>n.goal==='Verify current work'),dep=nodes.find(n=>n.goal==='Verify dependencies');
  const s={project_id:project,task_id:created.task_id,session_id:`session.${profile}`,actor:'fixture',binding_revision:1};
  await call(newer,'task_start',{...s,binding_revision:undefined,node_id:n.node_id,request_id:`start.${profile}`,interaction_id:`interaction.${profile}`,working_directory:root,expected_binding_revision:0});
  authored.push({profile,task:created.task_id,node:n,dep,scope:s});
 }
 await scenario('S03','Three effective policies and scoped node context',async()=>{
  parity(await fresh('workspace_context',{session_id:sid}),await oldView());
  for(const a of authored)parity(await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,session_id:a.scope.session_id}),await oldView(a.task,a.node.node_id));
 });
 await scenario('S04','Actual work, evidence review, close and awaiting user confirmation',async()=>{
  await bothWrite('event_log',{...scope,kind:'progress',details:{summary:'Verified S01-S03 with assertions'},idempotency_key:'paired.milestone'});
  await bothWrite('agent_result_record',{...scope,result_id:'result.paired',details:{kind:'execution',request_source:'user_request',instruction:'Verify actual fixture reads',status:'succeeded',summary:'S01-S03 assertions passed; criterion review pending'}});
  await oldView();let brief=await fresh('workspace_context',{session_id:sid});assert.equal(brief.next_step.kind,'evidence_required');
  const criterion=brief.data.criteria[0].criterion_id;
  await bothWrite('criterion_review',{...scope,criterion_id:criterion,outcome:'passed',reviewer:'fixture',evidence_refs:['test:paired-S01-S03'],details:{summary:'Actual preceding assertions passed'}});
  await bothWrite('session_close',{...scope});
  await bothWrite('task_completion_propose',{...scope});
  const legacy=await oldView();brief=await fresh('task_brief',{task_id:task,session_id:sid});assert.notEqual(legacy.result.node_brief.task_state,'completed');assert.notEqual(brief.data.state,'completed');assert.equal(brief.next_step.kind,'input_required');
  group.assertions.push('actual assertion evidence reviewed; proposal remains unconfirmed; no task_complete');
 });
 const a=authored[0];
 await scenario('S05','Unmet dependencies, missing evidence, failed review and open finding',async()=>{
  await bothWrite('plan_node_state_set',{...a.scope,node_id:a.dep.node_id,state:'active'},true);
  await bothWrite('criterion_review',{...a.scope,criterion_id:a.node.criterion_ids[0],outcome:'passed',reviewer:'fixture',evidence_refs:[]},true);
  await bothWrite('criterion_review',{...a.scope,criterion_id:a.node.criterion_ids[0],outcome:'failed',reviewer:'fixture',evidence_refs:['test:negative-fixture'],details:{summary:'Controlled negative assertion fixture'}});
  await bothWrite('finding_manage',{...a.scope,action:'open',finding_id:'finding.paired',node_id:a.node.node_id,severity:'medium',evidence_refs:['test:negative-fixture'],details:{summary:'Controlled open finding'}});
  const legacy=await oldView(a.task,a.node.node_id);const brief=await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,session_id:a.scope.session_id});parity(brief,legacy);assert(brief.data.blockers.some(b=>b.entity_id==='finding.paired'));assert(brief.data.criteria.some(c=>c.state==='failed'));group.assertions.push('dependency and evidence gates reject; actual failed criterion/finding remain visible');
 });
 await scenario('S06','Enum/type errors, binding/version conflict and cursor repair',async()=>{
  await bothWrite('plan_node_state_set',{...a.scope,node_id:a.dep.node_id,state:'invalid-state'},true);
  await bothWrite('plan_node_state_set',{...a.scope,node_id:a.dep.node_id,state:'blocked'});
  for(const args of [{...a.scope,binding_revision:9},{...a.scope,expected_version:0}]){
   await measured('baseline',()=>call(older,'event_log',{...args,kind:'progress',details:{summary:'rejected'}},true));await oldView(a.task,a.node.node_id);
   const err=await fresh('event_log',{...args,kind:'progress',details:{summary:'rejected'}},true);assert(err.recovery?.params);await fresh(err.recovery.tool,err.recovery.params);
  }
  await measured('baseline',()=>call(older,'task_view',{},true));await oldView(a.task,a.node.node_id);
  const err=await fresh('task_brief',{},true);assert(err.field_errors.some(e=>e.field==='task_id'));await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id});
  await oldView(a.task);const page=await fresh('task_inspect',{task_id:a.task,section:'timeline',limit:1});
  await bothWrite('event_log',{...a.scope,kind:'progress',details:{summary:'Invalidate cursor'},idempotency_key:'paired.cursor'});
  await oldView(a.task);const stale=await fresh('task_inspect',{task_id:a.task,section:'timeline',limit:1,cursor:page.data.next_cursor},true);assert.equal(stale.code,'CURSOR_STALE');await fresh(stale.recovery.tool,stale.recovery.params);group.assertions.push('one local correction or bounded refresh per injected failure');
 });
 await scenario('S07','Concurrent sessions and explicit task/node switching',async()=>{
  const second={...a.scope,session_id:'session.parallel'};
  await bothWrite('session_task_bind',{...second,interaction_id:'interaction.parallel',source:'user_confirmed',expected_binding_revision:0});
  await bothWrite('session_open',{...second,node_id:a.node.node_id,working_directory:root});
  // Concurrent actual writes, then compare both session-bound read results.
  const parallel=await Promise.all([call(older,'event_log',{...a.scope,kind:'progress',details:{summary:'Parallel A'}}),call(newer,'event_log',{...second,kind:'progress',details:{summary:'Parallel B'}})]);
  for(const [i,r] of parallel.entries())for(const which of ['baseline','new']){const target=group[which],text=JSON.stringify(r);target.calls++;target.result_bytes+=Buffer.byteLength(text);target.wire_bytes+=i===0?older.lastWireBytes:newer.lastWireBytes;target.tools.push('event_log');pendingTokens.push(countTokens(text).then(n=>target.reference_tokens+=n));}
  for(const session_id of [a.scope.session_id,second.session_id])parity(await fresh('workspace_context',{session_id}),await oldView(a.task,a.node.node_id));
  const other=authored[1];parity(await fresh('task_brief',{task_id:other.task,node_id:other.node.node_id}),await oldView(other.task,other.node.node_id));group.assertions.push('both concurrent facts survive; explicit scope switching; UI late-response checks in cache harness');
 });
 await scenario('S08','Lost response and process restart with stable operation ID',async()=>{
  const request={...a.scope,kind:'progress',details:{summary:'Lost-response fixture'},expected_version:3,idempotency_key:'paired.replay'};
  await measured('baseline',()=>call(older,'event_log',request));await older.kill();older=new Client(baseline,root);await older.init();older.observe=e=>{older.lastWireBytes=e.wire_bytes;observe(e);};const baselineRetry=await measured('baseline',()=>call(older,'event_log',request,true));assert.equal(baselineRetry.code,'V3_IDEMPOTENCY_SCOPE_MISMATCH');await oldView(a.task);
  const newRequest={...a.scope,kind:'progress',details:{summary:'Lost-response fixture'},request_id:'paired.replay.new'};
  const first=await fresh('task_record',newRequest);await newer.kill();newer=new Client(binary,root);await newer.init();newer.observe=observe;const status=await fresh('operation_status',{task_id:a.task,session_id:a.scope.session_id,request_id:newRequest.request_id});assert(status.ok);const replay=await fresh('task_record',newRequest);assert.deepEqual(first,replay);group.assertions.push('real process restart; repeated request returns original receipt; B0 legacy version normalization rejects replay; readback required; new stable operation succeeds');
 });
 // Large history is a separate explicit fixture milestone, not repeated reads
 // added merely to inflate the denominator. Every event passes real validators.
 for(let i=0;i<historyCount;i++)await call(older,'event_log',{...a.scope,kind:'progress',details:{summary:`History fixture ${i}`},idempotency_key:`history.${i}`});
 const long='中文 long evidence '.repeat(1100);
 const written=await call(older,'event_log',{...a.scope,kind:'progress',details:{summary:long},idempotency_key:'history.long'});
 await scenario('S09','1000 events and complete large evidence expansion',async()=>{
  const legacy=await oldView(a.task,a.node.node_id);const brief=await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,session_id:a.scope.session_id});parity(brief,legacy);
  const id=written.result.event.event_id;let offset=0,text='';
  do{const part=await fresh('task_inspect',{task_id:a.task,entity_kind:'event',entity_id:id,offset,max_bytes:8192});text+=part.data.text;offset=part.data.next_offset;}while(offset!==null);
  assert(JSON.stringify(JSON.parse(text)).includes(long));assert(JSON.stringify(legacy).includes(long));group.assertions.push('entire requested evidence restored; normal brief still includes current blockers');
 });
 await scenario('S10','Memory/config invalidation and projection repair',async()=>{
  await oldView(a.task,a.node.node_id);const before=await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id});
  const entry={entry_id:'memory.paired',kind:'accepted_decision',scope:['fixture/current'],principal_scope:'team',revision:0,status:'active',owner:'team',content:'Keep fixture data isolated',evidence_refs:['test:fixture-policy'],verified_against:[],confidence:90,freshness:'fresh',invalidation:null,supersedes:[],injection_policy:'task_relevant',sensitivity:'internal',updated_at:new Date().toISOString()};
  await bothWrite('memory_write',{...a.scope,action:'create',entry_id:entry.entry_id,expected_revision:0,entry});await oldView(a.task,a.node.node_id);const after=await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,if_revision:before.revision});assert(!after.unchanged);assert(JSON.stringify(after.data.constraints).includes(entry.content));
  await writeFile(join(root,'.vibehub/project-settings.yaml'),'schema_version: 1\nrevision: 1\n');await oldView(a.task);const settings=await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,if_revision:after.revision});assert(!settings.unchanged);
  await bothWrite('projection_rebuild',{project_id:project});parity(await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id}),await oldView(a.task,a.node.node_id));group.assertions.push('Memory/config changes invalidate revision; rebuild preserves domain truth');
 });
 await scenario('S11','Known IDs direct and out-of-scope rejection',async()=>{
  const legacy=await oldView(a.task,a.node.node_id);const finding=await fresh('task_inspect',{task_id:a.task,entity_kind:'finding',entity_id:'finding.paired',include:['evidence']});assert.equal(finding.data.finding_id,'finding.paired');assert(JSON.stringify(legacy).includes('finding.paired'));
  await fresh('task_inspect',{task_id:a.task,entity_kind:'criterion',entity_id:a.node.criterion_ids[0],include:['evidence']});await fresh('task_inspect',{task_id:a.task,entity_kind:'evidence',entity_id:'test:negative-fixture'});
  await oldView(task);const denied=await fresh('task_inspect',{task_id:task,entity_kind:'finding',entity_id:'finding.paired'},true);assert.equal(denied.code,'ENTITY_NOT_FOUND');group.assertions.push('three known entity kinds reachable directly; wrong task rejected');
 });
 await scenario('S12','One complete work context vs fragmented legacy resource reads',async()=>{
  const legacy=await oldView(a.task,a.node.node_id);const brief=await fresh('workspace_context',{session_id:a.scope.session_id});parity(brief,legacy);
  assert(brief.data.criteria.length&&brief.data.scope.length&&brief.data.constraints&&brief.data.blockers.length&&brief.next_step.kind);group.assertions.push('single response contains goal/scope/criteria/constraints/blockers/next step; baseline conservatively charged one bundle, not multiple resources');
 });
 await scenario('S13','Server filters, associations and exhausted budget',async()=>{
  const legacy=await oldView(a.task,a.node.node_id);const failed=await fresh('task_inspect',{task_id:a.task,section:'criteria',state:'failed'});assert.equal(failed.data.items.length,1);assert(JSON.stringify(legacy).includes(failed.data.items[0].criterion_id));
  const events=await fresh('task_inspect',{task_id:a.task,section:'timeline',state:'failed'});assert.equal(events.data.items.length,1);
  await fresh('task_brief',{task_id:a.task,node_id:a.node.node_id,max_bytes:1024},true);group.assertions.push('state/session filtering returns the matching facts; insufficient necessary-context budget rejects explicitly');
 });
 await scenario('S14','Explicit diagnostic snapshot and tool-only complete fallback',async()=>{
  const legacy=await oldView(a.task);
  const manifest=await fresh('diagnostic_export',{task_id:a.task});let offset=0,text='';
  do{const part=await fresh('diagnostic_read',{export_id:manifest.export_id,offset,max_bytes:65536});text+=part.text;offset=part.next_offset;}while(offset!==null);
  assert.equal(createHash('sha256').update(text).digest('hex'),manifest.sha256);const snapshot=JSON.parse(text);const oldEvents=legacy.result.task_timeline.events.filter(e=>!e.summary_key.startsWith('memory.'));assert(oldEvents.every(e=>snapshot.events.some(x=>x.event_id===e.timeline_event_id)),JSON.stringify(oldEvents.filter(e=>!snapshot.events.some(x=>x.event_id===e.timeline_event_id)).map(e=>({event_id:e.event_id,summary_key:e.summary_key}))));const authoritative=(await readFile(join(root,'.vibehub/v3/projects',project,'events.jsonl'),'utf8')).trim().split('\n').map(JSON.parse).filter(e=>e.task_id===a.task&&!e.event_type.startsWith('memory.'));assert.deepEqual(snapshot.events.map(e=>e.event_id).sort(),authoritative.map(e=>e.event_id).sort());group.baseline_diagnostic_complete=oldEvents.length===snapshot.events.length;group.diagnostic_event_counts={new_complete:snapshot.events.length,baseline_window:oldEvents.length};assert(snapshot.exclusions.includes('credential_fields'));await fresh('diagnostic_read',{export_id:manifest.export_id,action:'delete'});group.assertions.push('all diagnostic chunks charged; every baseline-window event included; complete export hash/explicit deletion; concurrent snapshot rejection in export-race harness');
 });
 await Promise.all(pendingTokens);
 const sum=which=>reports.reduce((acc,r)=>{for(const k of ['calls','result_bytes','wire_bytes','reference_tokens','error_calls'])acc[k]=(acc[k]??0)+r[which][k];return acc;},{});
 const totals={baseline:sum('baseline'),new:sum('new')};const reduction=1-totals.new.result_bytes/totals.baseline.result_bytes;
 for(const r of reports)r.output_reduction=1-r.new.result_bytes/r.baseline.result_bytes;
 const catalogStats={};for(const [mode,catalog] of Object.entries(catalogs)){const text=JSON.stringify(catalog);catalogStats[mode]={tools:catalog.tools.length,bytes:Buffer.byteLength(text),reference_tokens:await countTokens(text)};}
 const decisionCheckpoints=reports.filter(r=>['S01','S03','S07','S12'].includes(r.id)).flatMap(r=>r.new.tools.filter(t=>['workspace_context','task_brief'].includes(t)).map((tool,i)=>({scenario:r.id,checkpoint:i+1,baseline:1,new:1,tool,semantic_assertions:r.assertions})));
 const interactionMetrics={decision_round_trips:decisionCheckpoints,recovery_round_trips:{S06:{baseline:[1,1,1,1],new:[1,1,1,1,1],note:'enum correction, binding refresh, version refresh, missing argument correction; new cursor refresh adds one independently injected case'}},invalid_calls:{baseline:0,new:0,definition:'Unexpected parameter/binding/order failures; every intentional negative call is asserted and listed separately'},controlled_error_calls:{baseline:totals.baseline.error_calls,new:totals.new.error_calls},detail_and_diagnostic_calls:{baseline:reports.filter(r=>['S09','S11','S13','S14'].includes(r.id)).reduce((n,r)=>n+r.baseline.calls,0),new:reports.filter(r=>['S09','S11','S13','S14'].includes(r.id)).reduce((n,r)=>n+r.new.calls,0)},note:'Calls are protocol calls, not model reasoning turns. Explicit large-body/diagnostic expansion is separate from obtaining current-work context.'};
 const report={interaction_metrics:interactionMetrics,history_events:historyCount,catalogs:catalogStats,status:reduction>=.7?'byte_gate_passed':'byte_gate_failed',baseline_commit:'24547e3a0c1b0d6eb606d04aaff1324f32752150',baseline_sha256:createHash('sha256').update(await readFile(baseline)).digest('hex'),current_sha256:createHash('sha256').update(await readFile(binary)).digest('hex'),tokenizer:{package:'tiktoken',version:'0.12.0',encoding:'o200k_base',scope:'exact normalized tool result JSON; reference tokenizer, not host model-visible token attribution'},totals,output_reduction:reduction,scenarios:reports,limitations:['Deterministic actual tool workflow, not an LLM success-rate evaluation','Same unchanged typed-write receipt charged equally to both lanes','Read scenarios use the real B0 binary against the same authoritative fixture','S14 charges every new export byte but B0 task_view only exposes a 200-event window; aggregate savings are a conservative byte lower bound, not equal export capability','Large-history/setup writes are excluded from both lanes; each measured workflow call is counted','Host final model-visible formatting is measured separately']};
 console.log(JSON.stringify(report,null,2));
}finally{await newer?.kill();await older?.kill();tokenizer.stdin.end();await rm(root,{recursive:true,force:true});}
