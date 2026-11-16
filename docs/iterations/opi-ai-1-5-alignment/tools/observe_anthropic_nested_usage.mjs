import { createServer } from 'node:http';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [snapshot] = process.argv.slice(2);
if (!snapshot || process.argv.length !== 3) process.exit(2);
const load = (p) => import(pathToFileURL(resolve(snapshot, p)).href);
const [{ createModels, createProvider }, api] = await Promise.all([
  load('packages/ai/dist/models.js'),
  load('packages/ai/dist/api/anthropic-messages.js'),
]);

function fixture(ending) {
  const frames = [
    { type: 'message_start', message: { id: 'nested-response', model: 'nested-model', usage: {
      input_tokens: 100, output_tokens: 0, cache_read_input_tokens: 0,
      cache_creation_input_tokens: 200,
      cache_creation: { ephemeral_1h_input_tokens: 150 },
      output_tokens_details: { thinking_tokens: 0 },
    } } },
    { type: 'message_delta', delta: { stop_reason: 'end_turn' }, usage: {
      output_tokens: 20,
      cache_creation: { ephemeral_1h_input_tokens: 75 },
      output_tokens_details: { thinking_tokens: 6 },
    } },
    ending === 'wire-error'
      ? { type: 'error', error: { type: 'overloaded_error', message: 'nested fixture' } }
      : { type: 'message_stop' },
  ];
  return frames.map((frame) => `event: ${frame.type}\ndata: ${JSON.stringify(frame)}\n\n`).join('');
}

function view(message) {
  return {
    usage: [message.usage.input, message.usage.output, message.usage.cacheRead,
      message.usage.cacheWrite, message.usage.cacheWrite1h ?? null, message.usage.reasoning ?? null],
    stop: message.stopReason,
    raw_stop: message.rawStopReason ?? null,
    cost: message.usage.cost,
  };
}

async function run(route, ending) {
  const server = createServer(async (_req, res) => {
    res.writeHead(200, { 'content-type': 'text/event-stream' });
    res.end(fixture(ending));
  });
  await new Promise((resolveServer) => server.listen(0, '127.0.0.1', resolveServer));
  try {
    const model = {
      id: 'nested-model', name: 'Nested usage fixture', provider: 'anthropic', api: 'anthropic-messages',
      baseUrl: `http://127.0.0.1:${server.address().port}`, reasoning: true,
      input: ['text'], contextWindow: 100000, maxTokens: 4096,
      cost: { input: 1, output: 2, cacheRead: 3, cacheWrite: 4 },
    };
    let sourceCall;
    const mappedApi = {
      stream: (...args) => (sourceCall = api.stream(...args)),
      streamSimple: (...args) => (sourceCall = api.streamSimple(...args)),
    };
    const models = createModels();
    models.setProvider(createProvider({ id: 'anthropic', models: [model], api: mappedApi,
      auth: { apiKey: { name: 'fixture', resolve: async () => ({ auth: { apiKey: 'fixture-key' } }) } } }));
    const options = { apiKey: 'fixture-key', maxTokens: 4096, maxRetries: 0 };
    const call = route === 'direct'
      ? mappedApi.stream(model, { messages: [] }, options)
      : models.streamSimple(model, { messages: [] }, options);
    for await (const _event of call) {}
    const result = await call.result();
    const source = sourceCall ? await sourceCall.result() : result;
    return { route, ending, result: view(result), source_result: view(source) };
  } finally {
    server.closeAllConnections();
    await new Promise((resolveServer) => server.close(resolveServer));
  }
}

const cases = [];
for (const ending of ['success', 'wire-error']) {
  for (const route of ['simple', 'direct']) cases.push(await run(route, ending));
}
console.log(JSON.stringify({ cases }));
