import { build } from "esbuild";
import assert from "node:assert/strict";
const entry=`
import assert from "node:assert/strict";
import {loadV3ProductionViews} from "./src/services/v3ProductionViews.ts";
let revision="r1",loads=0,changeDuringLoad=false,sections=[];
globalThis.nativeInvoke=async(name,args)=>{
 if(name==="v3_read_view_revision")return revision;
 if(name==="v3_load_view_sections"){sections.push(args.sections);return Object.fromEntries(args.sections.map(panel=>[panel,{revision}]));}
 assert.equal(name,"v3_load_view_bundle");loads++;
 if(changeDuringLoad)revision="r3";
 return {project_overview:{project_id:args.expectedProjectId},project_structure:{marker:"structure"},agent_results:{marker:"results"},task_timeline:{marker:"timeline"},plan_graph:{marker:"plan"},node_brief:{marker:"brief"}};
};
const a=await loadV3ProductionViews("/fixture","task.a","project.a");
assert.equal(a.projectOverview.project_id,"project.a");assert.deepEqual([a.projectStructure.marker,a.agentResults.marker,a.taskTimeline.marker,a.planGraph.marker,a.nodeBrief.marker],["structure","results","timeline","plan","brief"]);
const same=await loadV3ProductionViews("/fixture","task.a","project.a",{background:true});
assert.equal(a,same);assert.equal(loads,1);
revision="r2";
const changed=await loadV3ProductionViews("/fixture","task.a","project.a",{background:true});
assert.notEqual(changed,a);assert.equal(loads,2);
await loadV3ProductionViews("/fixture","task.a","project.a");assert.equal(loads,3);
await loadV3ProductionViews("/fixture","task.b","project.a",{background:true});assert.equal(loads,4);
changeDuringLoad=true;revision="race";
await loadV3ProductionViews("/fixture","task.a","project.a");
changeDuringLoad=false;
await loadV3ProductionViews("/fixture","task.a","project.a",{background:true});assert.equal(loads,6,"racing bundle must not be cached under newer facts");
revision="r4";
const visible=await loadV3ProductionViews("/fixture","task.a","project.a",{background:true,panels:["project_overview","node_brief","task_timeline"]});
assert.deepEqual(sections,[["project_overview","node_brief","task_timeline"]]);
const unchanged=await loadV3ProductionViews("/fixture","task.a","project.a",{background:true,panels:["task_timeline"]});assert.equal(visible,unchanged);
const switched=await loadV3ProductionViews("/fixture","task.a","project.a",{background:true,panels:["project_overview","node_brief","plan_graph"]});
assert.deepEqual(sections[1],["plan_graph"]);assert.equal(switched.taskTimeline,visible.taskTimeline);
assert.notEqual(switched.planGraph,visible.planGraph);assert.equal(loads,6);
`;
const result=await build({stdin:{contents:entry,resolveDir:process.cwd(),sourcefile:"native-cache.test.ts"},bundle:true,write:false,format:"esm",platform:"node",plugins:[{name:"native-mock",setup(build){build.onResolve({filter:/^@tauri-apps\/api\/core$/},()=>({path:"mock",namespace:"native"}));build.onLoad({filter:/.*/,namespace:"native"},()=>({contents:"export const invoke=(...args)=>globalThis.nativeInvoke(...args);",loader:"js"}));}}]});
assert.equal(result.outputFiles.length,1);
await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString("base64")}`);
console.log("Agent native cache: unchanged identity, invalidation, foreground refresh, task switch and racing dependencies passed");
