import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
assert.ok(snapshot && fixturePath && process.argv.length === 4, "snapshot and fixture required");
const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels, createProvider }, chat] = await Promise.all([
  from("packages/ai/dist/models.js"), from("packages/ai/dist/api/openai-completions.js"),
]);
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
const frame = (value) => `data: ${JSON.stringify(value)}\n\n`;
const view = (message) => ({
  content: structuredClone(message.content), stop: message.stopReason.replace("toolUse", "tool_use"),
  raw_stop: message.rawStopReason ?? null, response_id: message.responseId ?? null,
  input_tokens: message.usage.input, output_tokens: message.usage.output,
});
const eventView = (event) => event.type === "error" ? {kind: "error", message: view(event.error)} : ({
  kind: event.type, index: event.contentIndex ?? null, delta: event.delta ?? null,
  content: event.content ?? null, message: view(event.partial ?? event.message),
});
const until = async (promise) => {
  const timer = new AbortController();
  try { return await Promise.race([promise, sleep(5000, null, {signal: timer.signal}).then(() => assert.fail("probe stalled"))]); }
  finally { timer.abort(); }
};

async function observe(test) {
  const connected = Promise.withResolvers();
  let requestBody;
  let peerClosed = false;
  const server = createServer(async (request, response) => {
    let body = "";
    for await (const part of request) body += part;
    assert.equal(request.method, "POST");
    assert.equal(request.url, "/v1/chat/completions");
    assert.equal(request.headers.authorization, "Bearer fixture-key");
    requestBody = JSON.parse(body);
    response.writeHead(200, {"content-type":"text/event-stream"});
    response.on("close", () => { peerClosed = true; });
    connected.resolve(response);
  });
  await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
  try {
    const model = {id:"gpt-4o",name:"Fixture",api:"openai-completions",provider:"openai",
      baseUrl:`http://127.0.0.1:${server.address().port}/v1`,reasoning:false,input:["text"],
      contextWindow:128000,maxTokens:4096,cost:{input:0,output:0,cacheRead:0,cacheWrite:0}};
    const models = createModels();
    models.setProvider(createProvider({id:"openai",models:[model],api:{stream:chat.stream,streamSimple:chat.streamSimple},
      auth:{apiKey:{name:"Fixture",resolve:async()=>({auth:{apiKey:"fixture-key"}})}}}));
    const gates = new Map(["after_finish","after_usage"].map((name)=>[name,{reached:Promise.withResolvers(),release:Promise.withResolvers()}]));
    const abort = new AbortController();
    const call = models.streamSimple(model, {
      messages:[{role:"user",content:"fixture",timestamp:1}],
      tools:[{name:"record",description:"Records fixture arguments",parameters:{type:"object",properties:{a:{type:"string"},n:{type:"integer"}},required:["a","n"]}}],
    }, {apiKey:"fixture-key",maxTokens:4096,maxRetries:0,signal:abort.signal,
      async onProviderStreamEvent(raw) {
        if (raw.checkpoint) { const gate=gates.get(raw.checkpoint); gate.reached.resolve(); await gate.release.promise; }
      },
    });
    const state = {events:[],result:null,eof:false};
    const collect = (async()=>{
      for await (const event of call) state.events.push(eventView(event));
      state.result=view(await call.result()); state.eof=true;
    })();
    const response = await until(connected.promise);
    const finish=structuredClone(fixture.finish);
    if(test.usage==="inline") finish.usage=fixture.usage.usage;
    response.write(fixture.prefix.map(frame).join("")+frame(finish));
    const checkpoints={};
    for(const name of ["after_finish","after_usage"]) {
      if(name==="after_usage" && test.usage==="trailer") response.write(frame(fixture.usage));
      response.write(frame({choices:[],checkpoint:name}));
      const gate=gates.get(name);
      await until(Promise.race([gate.reached.promise,collect]));
      await new Promise(setImmediate);
      checkpoints[name]={...structuredClone(state),peer_closed:peerClosed};
      gate.release.resolve();
    }
    if(test.tail==="done") response.write("data: [DONE]\n\n");
    else if(test.tail==="error") response.write(frame(fixture.error));
    else if(test.tail==="cancel") abort.abort();
    await sleep(200);
    checkpoints.held_http_eof={...structuredClone(state),peer_closed:peerClosed};
    response.end();
    await until(collect);
    return {case:test.id,request:requestBody,checkpoints,final:structuredClone(state)};
  } finally {
    server.closeAllConnections();
    await new Promise((closed)=>server.close(closed));
  }
}
const traces=[];
for(const test of fixture.cases) traces.push(await observe(test));
console.log(JSON.stringify(traces));
