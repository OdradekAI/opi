import net from 'node:net';
import { randomInt } from 'node:crypto';
import { spawn } from 'node:child_process';
const native=process.argv[2];
for(const count of [99,100,101,125,200]) {
 let received=0;
 const server=net.createServer(socket=>socket.once('data',()=>{
  received++;
  const lines=['HTTP/1.1 200 OK','Content-Length: 2','Connection: close'];
  for(let i=0;i<count-2;i++) lines.push(`X-Synthetic-${i}: v`);
  socket.end(lines.join('\r\n')+'\r\n\r\nok');
 }));
 const port=await new Promise((resolve,reject)=>{
  const attempt=()=>server.listen(randomInt(20000,60000),'127.0.0.1',()=>resolve(server.address().port));
  server.on('error',e=>e.code==='EADDRINUSE'?attempt():reject(e));attempt();
 });
 const url=`http://127.0.0.1:${port}/`;let reference;
 try{const response=await fetch(url);reference={ok:true,status:response.status,body:await response.text(),header_count:[...response.headers].length}}
 catch(error){reference={ok:false,name:error.name,message:error.message}}
 const nativeResult=await new Promise(resolve=>{
  const child=spawn(native,['owner-leg','GET',url,''],{windowsHide:true,env:{SystemRoot:process.env.SystemRoot,PATH:process.env.PATH,TEMP:process.env.TEMP,TMP:process.env.TMP}});
  let stdout='',stderr='';child.stdout.on('data',b=>stdout+=b);child.stderr.on('data',b=>stderr+=b);
  const timer=setTimeout(()=>child.kill(),10000);
  child.on('exit',(exit,signal)=>{clearTimeout(timer);let value;try{value=JSON.parse(stdout)}catch{value={parse_failure:true}}
   resolve({exit,signal,stdout,stderr,value});});
 });
 server.close();console.log(JSON.stringify({count,reference,native:nativeResult,received,runtime:process.versions.bun?'Bun':'Node',version:process.versions.bun??process.version}));
}
