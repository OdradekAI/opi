// Actual pinned-source observation; run with <pi-snapshot> <output.json>.
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { registerHooks } from 'node:module';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

if (process.argv.length !== 4) throw new Error('Usage: node observe_ai_telemetry.mjs <pi-snapshot> <output.json>');
const root = resolve(process.argv[2]).replaceAll('\\', '/') + '/';
const sourceUrl = (path) => pathToFileURL(root + path).href;
const loaded = new Set();
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === '@earendil-works/pi-telemetry') {
      return { url: sourceUrl('packages/telemetry/src/index.ts'), shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url.startsWith(pathToFileURL(root).href)) loaded.add(fileURLToPath(url));
    return nextLoad(url, context);
  },
});
const { lazyStream } = await import(sourceUrl('packages/ai/src/api/lazy.ts'));
const { observeModelRequest } = await import(sourceUrl('packages/ai/src/telemetry.ts'));
const { InMemoryTelemetryContext } = await import(sourceUrl('packages/telemetry/src/index.ts'));
const checkpoint = () => new Promise((done) => setImmediate(done));
const deferred = () => {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
};
const model = { id: 'm', provider: 'probe', api: 'probe' };
const message = (stopReason = 'stop') => ({
  role: 'assistant', api: 'probe', provider: 'probe', model: 'm',
  content: [{ type: 'text', text: 'A😀' }],
  usage: { input: 1, output: 2, cacheRead: 3, cacheWrite: 4, totalTokens: 10,
    cost: { input: 0.01, output: 0.02, cacheRead: 0.03, cacheWrite: 0.04, total: 0.1 } },
  stopReason, timestamp: Date.now(), responseId: 'response-1',
  ...(stopReason === 'aborted' ? { errorMessage: 'cancelled' } : {}),
});
const cases = [];
const metrics = (memory) => memory.getMetrics().map(({ name, value, attributes }) => ({ name, value, attributes }));
const rejections = [];
process.on('unhandledRejection', (error) => rejections.push(String(error)));
const watchdog = setTimeout(() => { throw new Error('probe watchdog'); }, 5000);

try {
  {
    const memory = new InMemoryTelemetryContext();
    const terminalGate = deferred();
    const resultGate = deferred();
    const reachedTerminal = deferred();
    const reachedResult = deferred();
    const final = message();
    const trace = [];
    let setup = 0;
    const source = {
      async *[Symbol.asyncIterator]() {
        yield { type: 'start', partial: message() };
        yield { type: 'text_end', contentIndex: 0, content: 'A😀', partial: final };
        yield { type: 'toolcall_delta', contentIndex: 1, delta: '😀', partial: final };
        yield { type: 'toolcall_end', contentIndex: 1, toolCall: { name: 'sample' }, partial: final };
        yield { type: 'done', reason: 'stop', message: final };
        reachedTerminal.resolve();
        await terminalGate.promise;
      },
      async result() {
        reachedResult.resolve();
        await resultGate.promise;
        return final;
      },
    };
    const outer = lazyStream(model, () => { setup++; return Promise.resolve(source); },
      observeModelRequest(model, { telemetry: memory }, false));
    trace.push({ step: 'immediate-return-no-outer-poll', setup, spans: memory.getSpans() });
    assert.equal(setup, 1);
    assert.equal(memory.getSpans().length, 1);
    assert.equal(memory.getSpans()[0].settled, false);
    await reachedTerminal.promise;
    const result = await outer.result();
    trace.push({ step: 'terminal-result-before-source-eof', sameResult: result === final,
      spans: memory.getSpans(), metrics: metrics(memory) });
    assert.equal(result, final);
    assert.equal(memory.getSpans()[0].settled, false);
    assert.equal(metrics(memory).filter((row) => row.name === 'pi.ai.request.duration').length, 0);
    terminalGate.resolve();
    await reachedResult.promise;
    trace.push({ step: 'source-eof-awaiting-source-result', spans: memory.getSpans() });
    assert.equal(memory.getSpans()[0].settled, false);
    resultGate.resolve();
    await checkpoint();
    const spans = memory.getSpans();
    trace.push({ step: 'source-result-settled', spans, metrics: metrics(memory) });
    assert.equal(spans[0].settled, true);
    assert.deepEqual(spans[0].events.filter((event) => event.name === 'pi.ai.block')
      .map((event) => event.attributes.chars), [3, 2]);
    assert.equal(metrics(memory).filter((row) => row.name === 'pi.ai.tokens').length, 4);
    cases.push({ id: 'ai-terminal-result-precedes-source-and-telemetry-settlement', trace });
  }
  {
    const memory = new InMemoryTelemetryContext();
    const controller = new AbortController();
    const aborted = deferred();
    controller.signal.addEventListener('abort', () => aborted.resolve(), { once: true });
    let sourceReturn = 0;
    let finished = false;
    const final = message('aborted');
    const source = {
      [Symbol.asyncIterator]() {
        const inner = (async function* () {
          yield { type: 'start', partial: message() };
          await aborted.promise;
          yield { type: 'error', reason: 'aborted', error: final };
          finished = true;
        })();
        return { next: (...params) => inner.next(...params),
          return: (...params) => { sourceReturn++; return inner.return(...params); } };
      },
    };
    const outer = lazyStream(model, async () => source,
      observeModelRequest(model, { telemetry: memory }, false));
    const iterator = outer[Symbol.asyncIterator]();
    assert.equal((await iterator.next()).value.type, 'start');
    await iterator.return();
    await checkpoint();
    const trace = [{ step: 'outer-return', sourceReturn, signalAborted: controller.signal.aborted,
      finished, spans: memory.getSpans() }];
    assert.equal(sourceReturn, 0);
    assert.equal(finished, false);
    assert.equal(controller.signal.aborted, false);
    controller.abort();
    const result = await outer.result();
    await checkpoint();
    trace.push({ step: 'explicit-abort-without-consumer', sourceReturn, finished,
      resultStop: result.stopReason, spans: memory.getSpans(), metrics: metrics(memory) });
    assert.equal(sourceReturn, 0);
    assert.equal(finished, true);
    assert.equal(result.stopReason, 'aborted');
    assert.equal(memory.getSpans()[0].settled, true);
    assert.deepEqual(memory.getSpans()[0].status, { status: 'ok' });
    cases.push({ id: 'ai-outer-return-and-explicit-cooperative-abort', trace });
  }
  for (const target of ['pi.ai.first_chunk.duration', 'pi.ai.request.duration']) {
    const memory = new InMemoryTelemetryContext();
    const adapter = {
      metrics: {
        counter: (name, options) => memory.counter(name, options),
        histogram: (name, options) => name === target
          ? { record() { throw new Error(`injected-${target}`); } }
          : memory.histogram(name, options),
      },
      startSpan: (options, callback) => memory.startSpan(options, callback),
    };
    const final = message();
    const source = { async *[Symbol.asyncIterator]() {
      yield { type: 'done', reason: 'stop', message: final };
    } };
    const outer = lazyStream(model, async () => source,
      observeModelRequest(model, { telemetry: adapter }, false));
    const result = await outer.result();
    await checkpoint();
    await checkpoint();
    const delivered = [];
    for await (const event of outer) delivered.push(event.type);
    const spans = memory.getSpans();
    assert.equal(result, final);
    assert.deepEqual(delivered, ['done']);
    if (target === 'pi.ai.request.duration') {
      assert.equal(spans[0].settled, false);
      assert.equal(spans[0].attributes.stop, 'stop');
    } else {
      assert.equal(spans[0].settled, true);
      assert.equal(spans[0].attributes.stop, 'error');
      assert.equal(spans[0].status.status, 'error');
    }
    cases.push({ id: `ai-record-throw-${target}`, trace: [{ step: 'bounded-checkpoint',
      resultStop: result.stopReason, sameResult: result === final, delivered, spans,
      metrics: metrics(memory), unhandledRejections: [...rejections] }] });
  }
  {
    const memory = new InMemoryTelemetryContext();
    let setup = 0;
    const adapter = {
      metrics: { histogram() { throw new Error('factory-marker'); } },
      startSpan: (options, callback) => memory.startSpan(options, callback),
    };
    let thrown;
    try {
      lazyStream(model, async () => { setup++; return {}; },
        observeModelRequest(model, { telemetry: adapter }, false));
    } catch (error) { thrown = error.message; }
    assert.equal(thrown, 'factory-marker');
    assert.equal(setup, 0);
    assert.equal(memory.getSpans().length, 0);
    cases.push({ id: 'ai-instrument-factory-throw-before-setup',
      trace: [{ thrown, setup, spans: memory.getSpans() }] });
  }
  const report = {
    node: process.version, platform: process.platform, arch: process.arch,
    command: [process.execPath, ...process.argv.slice(1)],
    driverSha256: createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex'),
    scope: 'Actual pinned lazy.ts, telemetry.ts and pi-telemetry source through one recorded package-name resolver; no Models/provider/package or Rust parity claim.',
    limits: ['Source import hook maps exactly one package specifier to its pinned source index; no modified or copied implementation.',
      'Cooperative AbortController test uses an authored source iterable, not a provider transport.',
      'Pending is asserted at event-loop checkpoints; unresolved span is not a whole-process leak proof.'],
    sources: [...loaded].sort().map((path) => ({ path, sha256: createHash('sha256').update(readFileSync(path)).digest('hex') })),
    sourceAssertions: 'passed', cases,
  };
  writeFileSync(resolve(process.argv[3]), JSON.stringify(report, null, 2) + '\n');
  process.stdout.write(`Recorded ${cases.length} source probes; all assertions passed.\n`);
} finally { clearTimeout(watchdog); }
