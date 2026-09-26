import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {Client} from './client.mjs';
const root=await mkdtemp(join(tmpdir(),'vibehub-state-matrix-'));let c;const evidence=[];
try {
 await mkdir(join(root,'.vibehub/tasks'),{recursive:true});
 await writeFile(join(root,'.vibehub/project.yaml'),'schema_version: 3\nproject_id: project.matrix\nname: matrix\n');
 c=new Client(resolve(process.env.VIBEHUB_TEST_BINARY??(process.platform==='win32'?'target/debug/vibehub.exe':'target/debug/vibehub')),root);await c.init();
 const call=async(name,args)=>{const r=await c.call(name,args);assert(!r.isError,`${name}: ${JSON.stringify(r.structuredContent)}`);return r.structuredContent;};
 const empty=await call('workspace_context',{});assert.equal(empty.data.candidates.length,0);
 for(const profile of ['lightweight','standard','full']) {
  const initial=profile==='lightweight'?[]:[{title:'Current work',goal:'Verify scoped current work',scope:['fixture/current'],criteria:[1],depends_on:[]},{title:'Other work',goal:'Verify unrelated blocking does not steal context',scope:['fixture/other'],criteria:[2],depends_on:[]}];
  const created=(await call('task_create',{project_id:'project.matrix',title:`Matrix ${profile}`,intent:'中文意图：验证真实作用域与策略',acceptance_criteria:profile==='lightweight'?['Atomic fixture check']:['Current fixture check','Other fixture check'],workflow_profile:profile,initial_plan:initial})).result;
  const task=created.task_id;const plan=await call('task_inspect',{task_id:task,section:'plan'});
  const node=plan.data.items.find(n=>n.goal==='Verify scoped current work');
  const other=plan.data.items.find(n=>n.goal==='Verify unrelated blocking does not steal context');
  const scope={project_id:'project.matrix',task_id:task,session_id:`session.${profile}`,actor:'matrix'};
  const start=await call('task_start',{...scope,request_id:`start.${profile}`,interaction_id:`interaction.${profile}`,working_directory:root,expected_binding_revision:0,...(node?{node_id:node.node_id}:{})});
  scope.binding_revision=start.binding_revision;
  const brief=await call('workspace_context',{session_id:scope.session_id});
  const legacy=(await call('task_view',{task_id:task,...(node?{node_id:node.node_id}:{})})).result;
  assert.equal(brief.data.execution_policy.effective_profile,profile);
  assert.equal(brief.data.execution_policy.planning_required,legacy.node_brief.execution_policy.planning_required);
  assert.equal(brief.data.goal,legacy.node_brief.goal);
  assert.equal(brief.data.state,legacy.node_brief.task_state);
  assert.equal(brief.data.binding.binding_revision,legacy.project_overview.session_bindings[scope.session_id].binding_revision);
  for(const criterion of brief.data.criteria) {
   const old=legacy.node_brief.criteria.find(c=>c.criterion_id===criterion.criterion_id);
   assert(old);assert.equal(old.status,criterion.state);
  }
  assert.equal(brief.next_step.kind,'work_required');
  assert(brief.data.constraints&&brief.data.criteria.length===1&&Buffer.byteLength(JSON.stringify(brief))<16384);
  if(other) {
   const invalid=await c.call('plan_node_state_set',{...scope,node_id:other.node_id,state:'invalid-fixture-state'});assert(invalid.isError);assert(invalid.structuredContent.field_errors.some(e=>e.field==='state'&&e.allowed_values.includes('blocked')));
   await call('plan_node_state_set',{...scope,node_id:other.node_id,state:'blocked'});
   await call('finding_manage',{...scope,action:'open',finding_id:`finding.other.${profile}`,node_id:other.node_id,severity:'medium',evidence_refs:['test:other-fixture-blocker'],details:{summary:'Isolated historical node blocker'}});
   const scoped=await call('workspace_context',{session_id:scope.session_id});
   assert(!scoped.data.blockers.some(b=>b.entity_id===`finding.other.${profile}`));
   assert.equal(scoped.next_step.kind,'work_required');
   await call('finding_manage',{...scope,action:'open',finding_id:`finding.current.${profile}`,node_id:node.node_id,severity:'medium',evidence_refs:['test:current-fixture-blocker'],details:{summary:'Controlled local fixture finding'}});
   const withFinding=await call('workspace_context',{session_id:scope.session_id});
   assert(withFinding.data.blockers.some(b=>b.entity_id===`finding.current.${profile}`));
   const details=await call('task_inspect',{task_id:task,section:'blockers',node_id:node.node_id,limit:1});
   assert.equal(details.data.items[0].entity_id,`finding.current.${profile}`);
   const direct=await call('task_inspect',{task_id:task,entity_kind:'finding',entity_id:`finding.current.${profile}`,include:['evidence']});
   assert.equal(direct.data.finding_id,`finding.current.${profile}`);
   if(profile==='standard') {
    for(let i=0;i<26;i++)await call('finding_manage',{...scope,action:'open',finding_id:`finding.paged.${i}`,node_id:node.node_id,severity:'medium',evidence_refs:['test:page-fixture'],details:{summary:'Controlled pagination fixture finding'}});
    const ids=[];let cursor;
    do {const page=await call('task_inspect',{task_id:task,section:'blockers',node_id:node.node_id,...(cursor?{cursor}:{})});ids.push(...page.data.items.map(x=>x.entity_id));cursor=page.data.next_cursor;}while(cursor);
    assert.equal(ids.length,27);assert.equal(new Set(ids).size,27);
    const completeContext=await call('workspace_context',{session_id:scope.session_id});assert.equal(completeContext.data.blockers.length,27);assert(Buffer.byteLength(JSON.stringify(completeContext))<=16384);
   }
   const updatedLegacy=(await call('task_view',{task_id:task})).result;
   assert.equal(withFinding.data.state,updatedLegacy.node_brief.task_state);
   assert(updatedLegacy.node_brief.blocker_details.some(b=>JSON.stringify(b).includes(`finding.current.${profile}`)),'legacy contains same open finding fact');
  }
  evidence.push({profile,goal_policy_binding_state_parity:true,one_read_context:true,scoped_blockers:!!other,result_bytes:Buffer.byteLength(JSON.stringify(brief)),legacy_result_bytes:Buffer.byteLength(JSON.stringify({ok:true,result:legacy})),output_reduction:1-Buffer.byteLength(JSON.stringify(brief))/Buffer.byteLength(JSON.stringify({ok:true,result:legacy}))});
 }
 const candidates=await call('workspace_context',{});assert.equal(candidates.data.candidates.length,3);assert.equal(candidates.next_step.kind,'input_required');
 // Match the known lightweight Task explicitly; candidate order is intentionally not identity.
 const bound=await call('workspace_context',{session_id:'session.lightweight'});
 const foreign=candidates.data.candidates.find(t=>t.task_id!==bound.scope.task_id).task_id;
 const mismatch=await c.call('task_brief',{task_id:foreign,session_id:'session.lightweight'});assert.equal(mismatch.structuredContent.code,'V3_SCOPE_MISMATCH');
 console.log(JSON.stringify({status:'passed',checks:['empty workspace','Chinese intent','three policy profiles','single-read necessary context','legacy semantic parity','unrelated blockers stay scoped','direct finding and associations','more than one default page of blockers with complete initial context','multiple candidates never bind','Session/Task mismatch'],cases:evidence},null,2));
}finally{if(c)await c.kill();if (process.env.VIBEHUB_TEST_RETAIN_TEMP !== "1") await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});}
