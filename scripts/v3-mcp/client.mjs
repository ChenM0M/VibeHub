import { spawn } from "node:child_process";
export class Client {
  constructor(binary, root, env = {}) {
    this.pending = new Map(); this.id = 0; this.buffer = "";
    this.child = spawn(binary, ["mcp-stdio", root], {stdio:["pipe","pipe","pipe"],env:{...process.env,...env}});
    this.closed = new Promise(resolve=>this.child.once("close",code=>{for(const p of this.pending.values())p.reject(new Error(`server closed ${code}`));this.pending.clear();resolve(code);}));
    this.child.stderr.resume();
    this.child.stdout.setEncoding("utf8");
    this.child.stdout.on("data",chunk=>{this.buffer+=chunk;for(;;){const n=this.buffer.indexOf("\n");if(n<0)break;const line=this.buffer.slice(0,n);this.buffer=this.buffer.slice(n+1);if(!line.trim())continue;const value=JSON.parse(line);const p=this.pending.get(value.id);if(p){this.pending.delete(value.id);value.error?p.reject(new Error(JSON.stringify(value.error))):p.resolve(value.result,Buffer.byteLength(line)+1);}}});
  }
  request(method,params={}) {
    const id=++this.id;
    return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{this.pending.delete(id);reject(new Error(`${method} timeout`));},30000);this.pending.set(id,{resolve:(v,wire_bytes)=>{clearTimeout(timer);this.observe?.({method,params,result:v,wire_bytes});resolve(v)},reject:e=>{clearTimeout(timer);reject(e)}});this.child.stdin.write(JSON.stringify({jsonrpc:"2.0",id,method,params})+"\n");});
  }
  async init(){const r=await this.request("initialize",{protocolVersion:"2025-11-25",capabilities:{},clientInfo:{name:"agent-native-recovery",version:"1"}});this.child.stdin.write(JSON.stringify({jsonrpc:"2.0",method:"notifications/initialized"})+"\n");return r;}
  async call(name,args){return this.request("tools/call",{name,arguments:args});}
  async close(){this.child.stdin.end();return this.closed;}
  async kill(){this.child.kill("SIGKILL");return this.closed;}
}
