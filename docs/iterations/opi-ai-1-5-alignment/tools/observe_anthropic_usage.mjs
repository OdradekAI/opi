import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setImmediate as turn } from 'node:timers/promises';

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) process.exit(2);
const load = (p) => import(pathToFileURL(resolve(snapshot, p)).href);
const [{createModels,createProvider}, api, {InMemoryTelemetryContext}] = await Promise.all([
  load('packages/ai/dist/models.js'), load('packages/ai/dist/api/anthropic-messages.js'),
  load('packages/telemetry/dist/index.js'),
]);
const cases = JSON.parse(await readFile(fixturePath, 'utf8'));
function fixture(c, ending) {
  const frames = [
    {type:'message_start',message:{id:'usage-response',model:'usage-model',usage:{input_tokens:11,output_tokens:7,cache_read_input_tokens:5,cache_creation_input_tokens:3}}},
    {type:'content_block_start',index:0,content_block:{type:'text',text:''}},
    {type:'content_block_delta',index:0,delta:{type:'text_delta',text:'A'}},
    {type:'content_block_delta',index:0,delta:{type:'text_delta',text:'B'}},
    {type:'content_block_stop',index:0},
  ];
  if(c.after_stop) frames.push({type:'message_delta',delta:{stop_reason:'end_turn'},usage:{}});
  frames.push({type:'message_delta',delta:c.after_stop?{}:{stop_reason:'end_turn'},...('usage' in c?{usage:c.usage}:{})});
  frames.push(ending==='wire-error'?{type:'error',error:{type:'overloaded_error',message:'fixture failure'}}:{type:'message_stop'});
  return frames.map(f=>`event: ${f.type}\ndata: ${JSON.stringify(f)}\n\n`).join('');
}
function view(m) { return {text:m.content.filter(b=>b.type==='text').map(b=>b.text).join(''),
  usage:[m.usage.input,m.usage.output,m.usage.cacheRead,m.usage.cacheWrite],cost:m.usage.cost.total,
  stop:m.stopReason,raw_stop:m.rawStopReason??null,response_id:m.responseId??null}; }
async function observe(c,route,ending) {
  let requests=0;
  const server=createServer(async(req,res)=>{for await(const chunk of req){}; requests++;
    res.writeHead(200,{'content-type':'text/event-stream'});res.end(fixture(c,ending));});
  await new Promise(r=>server.listen(0,'127.0.0.1',r));
  try {
    const model={id:'usage-model',name:'Usage fixture',provider:'anthropic',api:'anthropic-messages',
      baseUrl:`http://127.0.0.1:${server.address().port}`,reasoning:false,input:['text'],contextWindow:100000,maxTokens:4096,
      cost:{input:1,output:2,cacheRead:3,cacheWrite:4}};
    let sourceCall;
    const mappedApi={stream:(...args)=>(sourceCall=api.stream(...args)),streamSimple:(...args)=>(sourceCall=api.streamSimple(...args))};
    const models=createModels();
    models.setProvider(createProvider({id:'anthropic',models:[model],api:mappedApi,
      auth:{apiKey:{name:'fixture',resolve:async()=>({auth:{apiKey:'fixture-key'}})}}}));
    const memory=new InMemoryTelemetryContext();
    const telemetry=c.observer_fault?{metrics:memory.metrics,startSpan:(options,callback)=>memory.startSpan(options,span=>callback(new Proxy(span,{
      get(target,key){if(key==='addEvent')return (name,attributes)=>{if(name==='pi.ai.first_chunk')throw new Error('fixture observer');return target.addEvent(name,attributes);};
        const value=Reflect.get(target,key);return typeof value==='function'?value.bind(target):value;}
    })))}:memory;
    const firstSeen=Promise.withResolvers();
    const options={apiKey:'fixture-key',maxTokens:4096,maxRetries:0,telemetry,
      async onProviderStreamEvent(frame){
        if(frame.delta?.text==='B') await firstSeen.promise;
        if(ending==='callback-error' && frame.type==='message_stop') throw new Error('fixture callback');
      }};
    const call=route==='direct'?mappedApi.stream(model,{messages:[]},options):models.streamSimple(model,{messages:[]},options);
    let held;const events=[];
    for await(const e of call){
      if(e.type==='text_delta') {events.push({kind:e.type,delta:e.delta,message:structuredClone(view(e.partial))});
        if(!held){held=e.partial;firstSeen.resolve();}}
      else if(e.type==='text_end'||e.type==='done'||e.type==='error') events.push({kind:e.type,message:structuredClone(view(e.partial??e.message??e.error))});
    }
    const result=await call.result();firstSeen.resolve();const sourceResult=await sourceCall.result();await turn();
    assert.equal(requests,1);
    return {id:c.id,route,ending,events,result:view(result),delivery:result.stopReason==='error'?'error':'ok',held:held?view(held):null,
      source_result:view(sourceResult),spans:memory.getSpans().map(s=>({name:s.name,ended:s.settled,attributes:s.attributes,
        events:s.events.map(e=>({name:e.name,attributes:e.attributes}))})),
      metrics:memory.getMetrics().map(m=>({name:m.name,value:m.value,attributes:m.attributes})),requests};
  } finally {server.closeAllConnections();await new Promise(r=>server.close(r));}
}
const rows=[];
const watchdog=setTimeout(()=>{throw new Error('Anthropic usage probe timed out');},30000);
try {
  for(const c of cases)for(const ending of ['success','wire-error','callback-error'])for(const route of ['simple','direct'])
    rows.push(await observe(c,route,ending));
  console.log(JSON.stringify({cases:rows}));
} finally {clearTimeout(watchdog);}
