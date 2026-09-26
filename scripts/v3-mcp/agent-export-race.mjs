import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {createHash} from 'node:crypto';
import {Client} from './client.mjs';
const root=await mkdtemp(join(tmpdir(),'vibehub-export-race-'));let reader,writer;const outcomes={coherent_before:0,coherent_after:0,revision_rejected:0};
try {
 await mkdir(join(root,'.vibehub/tasks/task.race'),{recursive:true});
 await writeFile(join(root,'.vibehub/project.yaml'),'schema_version: 3\nproject_id: project.race\nname: race\n');
 await writeFile(join(root,'.vibehub/tasks/task.race/task.yaml'),'task_id: task.race\ntitle: Race fixture\nintent: Verify scoped snapshot coherence\nphase: implement\nphase_status: active\nworkflow_profile: lightweight\nacceptance_criteria: []\n');
 reader=new Client(resolve(process.env.VIBEHUB_TEST_BINARY??(process.platform==='win32'?'target/debug/vibehub.exe':'target/debug/vibehub')),root);writer=new Client(resolve(process.env.VIBEHUB_TEST_BINARY??(process.platform==='win32'?'target/debug/vibehub.exe':'target/debug/vibehub')),root);await Promise.all([reader.init(),writer.init()]);
 const good=async(c,n,a)=>{const r=await c.call(n,a);assert(!r.isError,JSON.stringify(r.structuredContent));return r.structuredContent;};
 const scope={project_id:'project.race',task_id:'task.race',session_id:'session.race',actor:'race-test'};
 const started=await good(writer,'task_start',{...scope,request_id:'start',interaction_id:'interaction.race',working_directory:root,expected_binding_revision:0});scope.binding_revision=started.binding_revision;
 for(let i=0;i<10;i++) {
  const before=await good(reader,'task_inspect',{task_id:scope.task_id,section:'task'});
  const [exported]=await Promise.all([reader.call('diagnostic_export',{task_id:scope.task_id}),good(writer,'task_record',{...scope,request_id:`race.${i}`,kind:'progress',details:{summary:`Observed concurrency fixture iteration ${i}: `+'中'.repeat(2000)}})]);
  const after=await good(reader,'task_inspect',{task_id:scope.task_id,section:'task'});assert.notEqual(before.revision,after.revision);
  if(exported.isError) {assert.equal(exported.structuredContent.code,'READ_REVISION_CHANGED');outcomes.revision_rejected++;continue;}
  const manifest=exported.structuredContent;
  const bytes=await readFile(join(root,manifest.path));const snapshot=JSON.parse(bytes);
  assert.equal(createHash('sha256').update(bytes).digest('hex'),manifest.sha256);
  assert.equal(snapshot.revision,manifest.revision);
  const isBefore=manifest.revision===before.revision;
  assert(isBefore||manifest.revision===after.revision,'mixed snapshot revision');
  assert.equal(snapshot.events.length,2+i+(isBefore?0:1),'event set and snapshot revision agree');
  assert.deepEqual([...snapshot.lifecycle.event_ids].sort(),snapshot.events.map(e=>e.event_id).sort(),'projection and events are from same snapshot');
  outcomes[isBefore?'coherent_before':'coherent_after']++;
  await good(reader,'diagnostic_read',{export_id:manifest.export_id,action:'delete'});
 }
 console.log(JSON.stringify({status:'passed',independent_mcp_processes:2,concurrent_exports:10,outcomes},null,2));
}finally{await Promise.all([reader?.kill(),writer?.kill()]);await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});}
