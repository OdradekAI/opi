import http from 'node:http';
import http2 from 'node:http2';
import https from 'node:https';
import fs from 'node:fs';
import path from 'node:path';
import { createHash, createPublicKey, randomInt } from 'node:crypto';
import { createRequire } from 'node:module';
import { spawn } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const [npmRoot, fixtures, native] = process.argv.slice(2);
const requireNpm = createRequire(path.join(npmRoot, 'package.json'));
const nodeFetch = (await import(pathToFileURL(path.join(npmRoot, 'node_modules/node-fetch/src/index.js')))).default;
const gaxios = requireNpm('gaxios');
const metadata = requireNpm('gcp-metadata');
const gtoken = requireNpm('google-auth-library/build/src/gtoken/jwsSign.js');
const jws = requireNpm('jws');
const headers = { authorization: 'Bearer SYNTHETIC-SECRET', cookie: 'synthetic-cookie=1', 'x-opi-canary': 'synthetic-public-canary' };
const emit = x => console.log(JSON.stringify(x));
let requests = [];
const sockets = new Set();
const createHttp = () => {
  const server = http.createServer(async (req, res) => {
    let body = ''; for await (const chunk of req) body += chunk;
    const observation = { method: req.method, path: req.url, headers: req.headers, body, httpVersion: req.httpVersion };
    requests.push(observation);
    const url = new URL(req.url, 'http://127.0.0.1');
    if (url.pathname === '/redirect') { res.writeHead(Number(url.searchParams.get('status')), {location:url.searchParams.get('to')}); res.end(); }
    else if (url.pathname === '/loop' && Number(url.searchParams.get('n')) > 0) {
      res.writeHead(302,{location:`/loop?n=${Number(url.searchParams.get('n'))-1}`});res.end();
    } else if (url.pathname.startsWith('/computeMetadata/')) {
      if (url.searchParams.has('delay')) await new Promise(r=>setTimeout(r,4000));
      res.writeHead(200,{'metadata-flavor': url.searchParams.has('bad') ? 'Wrong' : 'Google'});
      res.end(JSON.stringify({access_token:'SYNTHETIC-METADATA-TOKEN',expires_in:3600,token_type:'Bearer'}));
    } else { res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify(observation)); }
  });
  server.on('connection', s=>{sockets.add(s);s.on('close',()=>sockets.delete(s));});
  return server;
};
// Fixture ports avoid the fetch standard's blocked service-port range.
// This is experiment containment, not an SDK routing policy.
const listen = server => new Promise((resolve,reject)=>{
  const attempt=()=>server.listen(randomInt(20000,60000),'127.0.0.1',()=>resolve(server.address().port));
  server.on('error',error=>error.code==='EADDRINUSE'?attempt():reject(error));attempt();
});
const one = createHttp(), two = createHttp();
const port1=await listen(one),port2=await listen(two);
const base1=`http://127.0.0.1:${port1}`,base2=`http://127.0.0.1:${port2}`;

async function nativeRun(args) {
  return new Promise(resolve=>{
    const child=spawn(native,args,{env:{SystemRoot:process.env.SystemRoot,PATH:process.env.PATH,TEMP:fixtures,TMP:fixtures},windowsHide:true});
    let out='',err='';child.stdout.on('data',b=>out+=b);child.stderr.on('data',b=>err+=b);
    const timer=setTimeout(()=>child.kill(),15000);
    child.on('exit',(code,signal)=>{clearTimeout(timer);let result;try{result=JSON.parse(out.trim())}catch{result={parse_failure:true}}
      resolve({exit:code,signal,stdout:out,stderr:err,result});});
  });
}
for (const status of [301,302,303,307,308]) for(const method of ['GET','POST','PUT']) {
  for(const backend of ['global-fetch','node-fetch','gaxios','reqwest-default','reqwest-owner-leg']) {
    requests=[];
    const url=`${base1}/redirect?status=${status}&to=${encodeURIComponent(base2+'/final')}`;
    let outcome;
    try {
      const body=method==='GET'?undefined:'synthetic-body';
      if(backend==='global-fetch'||backend==='node-fetch') {
        const response=await (backend==='global-fetch'?fetch:nodeFetch)(url,{method,headers,body,signal:AbortSignal.timeout(8000)});
        outcome={ok:true,status:response.status,body:await response.text()};
      } else if(backend==='gaxios') {
        const response=await gaxios.request({url,method,headers,data:body,retry:false,timeout:8000});
        outcome={ok:true,status:response.status,data:response.data};
      } else outcome=await nativeRun([backend==='reqwest-default'?'native-default-redirect':'owner-leg',method,url,body??'']);
    }catch(error){outcome={ok:false,name:error.name,message:error.message};}
    emit({kind:'redirect',id:`${status}-${method}-${backend}`,status,method,backend,outcome,requests});
  }
}
for(const n of [11,20,21]) for(const backend of ['global-fetch','node-fetch','gaxios','reqwest-default']) {
  requests=[];const url=`${base1}/loop?n=${n}`;let outcome;
  try {
    if(backend==='gaxios') {const response=await gaxios.request({url,retry:false,timeout:8000});outcome={ok:true,status:response.status};}
    else if(backend==='reqwest-default') outcome=await nativeRun(['native-default-redirect','GET',url,'']);
    else {const response=await(backend==='global-fetch'?fetch:nodeFetch)(url,{signal:AbortSignal.timeout(8000)});await response.text();outcome={ok:true,status:response.status};}
  }catch(error){outcome={ok:false,name:error.name,message:error.message};}
  emit({kind:'redirect-loop',id:`${n}-${backend}`,n,backend,outcome,requests});
}

for(const testcase of JSON.parse(fs.readFileSync(path.join(fixtures,'rsa-cases.json')))) {
  const key=fs.readFileSync(path.join(fixtures,testcase.id+'.pem'),'utf8');
  let reference;
  try {
    const token=gtoken.getJwsSign({iss:'synthetic@example.invalid',scope:'synthetic-scope',key,additionalClaims:{iat:1,exp:3601}});
    const [header,payload,signature]=token.split('.');
    reference={accepted:true,header:JSON.parse(Buffer.from(header,'base64url')),payload:JSON.parse(Buffer.from(payload,'base64url')),
      signature_bytes:Buffer.from(signature,'base64url').length,verified:jws.verify(token,'RS256',createPublicKey(key)),
      signature_sha256:createHash('sha256').update(Buffer.from(signature,'base64url')).digest('hex')};
  }catch(error){reference={accepted:false,name:error.name,message:error.message};}
  emit({kind:'rsa',id:testcase.id,...testcase,reference,native:await nativeRun(['rsa',testcase.format,path.join(fixtures,testcase.id+'.der')])});
}

metadata.setGCPResidency(false);emit({kind:'metadata-timeout',residency:false,value:metadata.requestTimeout()});
metadata.setGCPResidency(true);emit({kind:'metadata-timeout',residency:true,value:metadata.requestTimeout()});
metadata.setGCPResidency(false);process.env.GCE_METADATA_HOST=base1;
for(const kind of ['normal','wrong-flavor','delayed']) {
  requests=[];const before=performance.now();let outcome;
  try {outcome={ok:true,value:await metadata.instance({property:'service-accounts/default/token',params:kind==='wrong-flavor'?{bad:1}:kind==='delayed'?{delay:1}:{}})};}
  catch(error){outcome={ok:false,name:error.name,message:error.message,code:error.code};}
  emit({kind:'metadata-request',id:kind,outcome,elapsed_ms:performance.now()-before,requests});
}

const bytes = name=>fs.readFileSync(path.join(fixtures,name));
const tlsServers=[];
for(const testcase of [
  {id:'h2',nativeProfile:'h2'}, {id:'h1',nativeProfile:'h1'},
  {id:'no-trust',nativeProfile:'h2',noTrust:true},
  {id:'wrong-host',nativeProfile:'h2',wrongHost:true},
  {id:'expired',nativeProfile:'h2',expired:true},
  {id:'mtls-rsa',nativeProfile:'h2',client:'client-rsa'},
  {id:'mtls-ec',nativeProfile:'h2',client:'client-ec'},
  {id:'mtls-ed25519',nativeProfile:'h2',client:'client-ed25519'},
  {id:'mtls-missing',nativeProfile:'h2',requireClient:true}
]) {
  const identity=testcase.expired?'expired':'server';let tlsRequests=[];
  const server=http2.createSecureServer({key:bytes(identity+'-key.pem'),cert:bytes(identity+'-cert.pem'),ca:bytes('ca.pem'),
    allowHTTP1:true,requestCert:!!testcase.client||!!testcase.requireClient,rejectUnauthorized:true});
  server.on('sessionError',()=>{});server.on('tlsClientError',()=>{});
  server.on('request',(req,res)=>{const value={httpVersion:req.httpVersion,authorized:req.socket.authorized,
    peer_cn:req.socket.getPeerCertificate()?.subject?.CN??null,alpn:req.socket.alpnProtocol};tlsRequests.push(value);res.end(JSON.stringify(value));});
  const port=await listen(server);tlsServers.push(server);
  const url=`https://${testcase.wrongHost?'127.0.0.1':'localhost'}:${port}/`;
  const args=[testcase.nativeProfile,'GET',url,''];
  if(!testcase.noTrust) args.push(path.join(fixtures,'ca.der'));
  if(testcase.client) args.push(path.join(fixtures,testcase.client+'-cert.der'),path.join(fixtures,testcase.client+'-key.der'));
  const outcome=await nativeRun(args);
  emit({kind:'tls',id:testcase.id,outcome,requests:tlsRequests});
}
for(const s of tlsServers){s.closeAllConnections?.();s.close();}
for(const socket of sockets) socket.destroy();one.close();two.close();
emit({kind:'finish',node:process.version,versions:process.versions});
