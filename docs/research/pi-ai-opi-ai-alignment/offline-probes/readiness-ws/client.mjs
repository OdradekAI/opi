import {readFileSync} from 'node:fs';
let ws;const result={opened:false,messages:[],errors:[]};let done=false;
function finish(){if(done)return;done=true;console.log(JSON.stringify(result));try{ws?.close()}catch{}setTimeout(()=>process.exit(0),10)}
try{ws=new WebSocket(process.argv[2],{headers:{Authorization:'Bearer offline-fake-token','X-Opi-Offline-Canary':'pi130-canary'}});ws.addEventListener('open',()=>{result.opened=true;ws.send(readFileSync(new URL('./outbound.txt',import.meta.url),'utf8'))});ws.addEventListener('message',e=>result.messages.push(String(e.data)));ws.addEventListener('error',e=>result.errors.push(e.message??e.error?.message??'error'));ws.addEventListener('close',finish)}catch(e){result.errors.push(String(e));finish()}
setTimeout(finish,1400);
