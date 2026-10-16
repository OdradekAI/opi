import http2 from 'node:http2';
import fs from 'node:fs';
import path from 'node:path';
import {randomInt} from 'node:crypto';
import {spawn} from 'node:child_process';
const [fixtures,native]=process.argv.slice(2);const bytes=x=>fs.readFileSync(path.join(fixtures,x));
for(const identity of ['client-p521','client-rsa-e3','client-rsa1024']) {
 const received=[];
 const server=http2.createSecureServer({key:bytes('server-key.pem'),cert:bytes('server-cert.pem'),ca:bytes('ca.pem'),requestCert:true,rejectUnauthorized:true});
 server.on('tlsClientError',()=>{});server.on('sessionError',()=>{});
 server.on('request',(req,res)=>{received.push({authorized:req.socket.authorized,peer:req.socket.getPeerCertificate()?.subject?.CN,httpVersion:req.httpVersion});res.end('ok')});
 const port=await new Promise((resolve,reject)=>{
  const attempt=()=>server.listen(randomInt(20000,60000),'127.0.0.1',()=>resolve(server.address().port));
  server.on('error',e=>e.code==='EADDRINUSE'?attempt():reject(e));attempt();
 });
 const url=`https://localhost:${port}/`;
 let reference;
 try{reference=await new Promise((resolve)=>{
  let session;try{session=http2.connect(url,{ca:bytes('ca.pem'),cert:bytes(identity+'-cert.pem'),key:bytes(identity+'-key.pem')});}catch(error){resolve({ok:false,stage:'context',message:error.message});return;}
  let done=false;const finish=value=>{if(done)return;done=true;session.destroy();resolve(value)};
  session.on('error',error=>finish({ok:false,stage:'session',message:error.message}));
  session.on('connect',()=>{const request=session.request({':path':'/'});let body='';request.setEncoding('utf8');request.on('data',x=>body+=x);request.on('end',()=>finish({ok:true,body}));request.on('error',error=>finish({ok:false,stage:'request',message:error.message}));request.end();});
 });}catch(error){reference={ok:false,message:error.message}}
 const nativeResult=await new Promise(resolve=>{
  const child=spawn(native,['h2','GET',url,'',path.join(fixtures,'ca.der'),path.join(fixtures,identity+'-cert.der'),path.join(fixtures,identity+'-key.der')],{windowsHide:true,env:{SystemRoot:process.env.SystemRoot,PATH:process.env.PATH,TEMP:fixtures,TMP:fixtures}});
  let stdout='',stderr='';child.stdout.on('data',x=>stdout+=x);child.stderr.on('data',x=>stderr+=x);child.on('exit',exit=>resolve({exit,stdout,stderr}));
 });
 server.close();console.log(JSON.stringify({identity,reference,native:nativeResult,received}));
}
