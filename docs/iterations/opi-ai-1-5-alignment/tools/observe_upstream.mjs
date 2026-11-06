#!/usr/bin/env node
// Read-only observations of the actual frozen TS files, not a semantic reimplementation.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const args = process.argv.slice(2);
const value = (name) => {
  const index = args.indexOf(name);
  if (index < 0 || !args[index + 1]) throw new Error(`Required argument: ${name}`);
  return args[index + 1];
};
const snapshot = resolve(value('--snapshot'));
const sourcePaths = ['packages/ai/src/utils/event-stream.ts', 'packages/ai/src/api/lazy.ts'];
const checkpoint = () => new Promise((done) => setImmediate(done));
const deferred = () => {
  let resolveValue;
  const promise = new Promise((done) => { resolveValue = done; });
  return { promise, resolve: resolveValue };
};
const track = (promise) => {
  const state = { state: 'pending' };
  promise.then((result) => Object.assign(state, { state: 'fulfilled', result }),
    (error) => Object.assign(state, { state: 'rejected', error: String(error) }));
  return state;
};
const message = (text = 'complete') => ({
  role: 'assistant', content: [{ type: 'text', text }], api: 'probe', provider: 'probe', model: 'm',
  usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
  stopReason: 'stop', timestamp: 0,
});
const model = { id: 'm', api: 'probe', provider: 'probe' };
const cases = [
  'event-stream-extract-throw', 'event-stream-terminal-waiters', 'event-stream-reference',
  'lazy-no-consumption', 'lazy-stop-consumption', 'lazy-observer-terminal-throw',
  'lazy-observer-result-throw', 'lazy-observer-repeat-throw', 'lazy-setup-failure-phases',
];

async function observe(caseId) {
  const { EventStream } = await import(pathToFileURL(join(snapshot, sourcePaths[0])).href);
  const { lazyStream } = await import(pathToFileURL(join(snapshot, sourcePaths[1])).href);
  const trace = [];
  const record = (step, data) => trace.push({ step, ...structuredClone(data) });

  if (caseId === 'event-stream-extract-throw') {
    const stream = new EventStream(() => true, () => { throw new Error('extract-marker'); });
    const first = track(stream[Symbol.asyncIterator]().next());
    const second = track(stream[Symbol.asyncIterator]().next());
    const result = track(stream.result());
    await checkpoint();
    record('two-waiters-registered', { first, second, result });
    let thrown;
    try { stream.push('terminal'); } catch (error) { thrown = error.message; }
    await checkpoint();
    const newIterator = await stream[Symbol.asyncIterator]().next();
    record('after-terminal-push', { thrown, first, second, result, newIterator });
    assert.equal(thrown, 'extract-marker');
    assert.equal(first.state, 'pending'); assert.equal(second.state, 'pending');
    assert.equal(result.state, 'pending'); assert.equal(newIterator.done, true);
    stream.end(); await checkpoint();
    record('after-end-without-result', { first, second, result });
    assert.equal(first.result.done, true); assert.equal(second.result.done, true);
    assert.equal(result.state, 'pending');
    stream.end('late-result'); await checkpoint();
    record('after-end-with-result', { result });
    assert.equal(result.result, 'late-result');
  } else if (caseId === 'event-stream-terminal-waiters') {
    const stream = new EventStream((event) => event === 'terminal', (event) => event);
    const first = track(stream[Symbol.asyncIterator]().next());
    const second = track(stream[Symbol.asyncIterator]().next());
    const result = track(stream.result());
    await checkpoint(); stream.push('terminal'); await checkpoint();
    record('after-terminal-push', { first, second, result });
    assert.deepEqual(first.result, { value: 'terminal', done: false });
    assert.equal(second.state, 'pending'); assert.equal(result.result, 'terminal');
    stream.end(); await checkpoint();
    record('after-explicit-end', { second }); assert.equal(second.result.done, true);
  } else if (caseId === 'event-stream-reference') {
    const stream = new EventStream(() => false, (event) => event);
    const block = { text: 'A' }; const content = [block]; const partial = { content };
    const event = { partial }; stream.push(event);
    const received = (await stream[Symbol.asyncIterator]().next()).value;
    const retainedBlock = received.partial.content[0];
    record('immediate-read', { text: retainedBlock.text,
      sameEvent: received === event, samePartial: received.partial === partial,
      sameArray: received.partial.content === content, sameBlock: retainedBlock === block });
    block.text += 'B'; content.push({ text: 'C' });
    record('delayed-read', { text: retainedBlock.text, content: received.partial.content });
    assert.equal(received, event); assert.equal(retainedBlock.text, 'AB');
    assert.equal(received.partial.content.length, 2); stream.end();
  } else if (caseId === 'lazy-no-consumption' || caseId === 'lazy-stop-consumption') {
    const gate = deferred(); const reached = deferred(); const finished = deferred();
    const counters = { setup: 0, iterators: 0, sourceReturn: 0, sourceTerminal: 0 };
    const final = message();
    const source = {
      [Symbol.asyncIterator]() {
        counters.iterators++;
        const inner = (async function* () {
          yield { type: 'start', partial: message('') };
          reached.resolve(); await gate.promise;
          counters.sourceTerminal++;
          yield { type: 'done', reason: 'stop', message: final };
          finished.resolve();
        })();
        return {
          next: (...params) => inner.next(...params),
          return: (...params) => { counters.sourceReturn++; return inner.return(...params); },
        };
      },
      result: async () => final,
    };
    const outer = lazyStream(model, () => { counters.setup++; return Promise.resolve(source); });
    record('immediately-after-call', { counters }); assert.equal(counters.setup, 1);
    await reached.promise;
    record('source-at-gate-without-outer-poll', { counters });
    if (caseId === 'lazy-stop-consumption') {
      const iterator = outer[Symbol.asyncIterator]();
      const first = await iterator.next(); assert.equal(first.value.type, 'start');
      const stopped = await iterator.return();
      record('outer-iterator-return', { stopped, counters });
    }
    gate.resolve(); await finished.promise; await checkpoint();
    const result = await outer.result();
    record('source-completed-after-gate', { counters, resultStop: result.stopReason,
      sameResult: result === final });
    assert.equal(counters.iterators, 1); assert.equal(counters.sourceReturn, 0);
    assert.equal(result, final); assert.equal(counters.sourceTerminal, 1);
  } else if (caseId.startsWith('lazy-observer-')) {
    const observed = []; const rejections = [];
    const rejected = (reason) => rejections.push(String(reason));
    process.on('unhandledRejection', rejected);
    const final = message();
    const source = { async *[Symbol.asyncIterator]() {
      if (caseId === 'lazy-observer-repeat-throw') yield { type: 'start', partial: message('') };
      yield { type: 'done', reason: 'stop', message: final };
    }, result: async () => final };
    const outer = lazyStream(model, async () => source, {
      onEvent(event) {
        observed.push({ channel: 'event', type: event.type, reason: event.reason ?? null });
        if (caseId === 'lazy-observer-repeat-throw' ||
            (caseId === 'lazy-observer-terminal-throw' && event.type === 'done')) {
          throw new Error('observer-event-marker');
        }
      },
      onResult(result) {
        observed.push({ channel: 'result', stop: result?.stopReason ?? null,
          error: result?.errorMessage ?? null });
        if (caseId === 'lazy-observer-result-throw') throw new Error('observer-result-marker');
      },
    });
    const result = await outer.result(); await checkpoint(); await checkpoint();
    const delivered = []; for await (const event of outer) delivered.push(event.type);
    record('settled-and-drained', { delivered, resultStop: result.stopReason,
      resultError: result.errorMessage ?? null, sameResult: result === final, observed, rejections });
    if (caseId === 'lazy-observer-terminal-throw') {
      assert.equal(result, final); assert.deepEqual(delivered, ['done']);
      assert.deepEqual(observed.map((row) => row.type ?? row.stop), ['done', 'error', 'error']);
      assert.equal(rejections.length, 0);
    } else if (caseId === 'lazy-observer-result-throw') {
      assert.equal(result, final); assert.deepEqual(delivered, ['done']);
      assert.deepEqual(observed.map((row) => row.type ?? row.stop), ['done', 'stop', 'error']);
      assert.equal(rejections.length, 0);
    } else {
      assert.deepEqual(delivered, ['start', 'error']); assert.equal(result.stopReason, 'error');
      assert.equal(observed.filter((row) => row.channel === 'result').length, 0);
      assert.equal(rejections.length, 1); assert.match(rejections[0], /observer-event-marker/);
    }
    process.off('unhandledRejection', rejected);
  } else if (caseId === 'lazy-setup-failure-phases') {
    let synchronousError;
    try { lazyStream(model, () => { throw new Error('synchronous-setup-marker'); }); }
    catch (error) { synchronousError = error.message; }
    const rejected = lazyStream(model, () => Promise.reject(new Error('rejected-setup-marker')));
    const result = await rejected.result(); const delivered = [];
    for await (const event of rejected) delivered.push(event.type);
    record('setup-failures', { synchronousError, rejectedStop: result.stopReason,
      rejectedMessage: result.errorMessage, delivered });
    assert.equal(synchronousError, 'synchronous-setup-marker');
    assert.equal(result.stopReason, 'error'); assert.equal(result.errorMessage, 'rejected-setup-marker');
    assert.deepEqual(delivered, ['error']);
  } else throw new Error(`Unknown case: ${caseId}`);
  return { id: caseId, status: 'observed', sourceAssertions: 'passed', trace };
}

if (args.includes('--case')) {
  const caseId = value('--case');
  if (!cases.includes(caseId)) {
    process.stderr.write(`Unknown case: ${caseId}\n`);
    process.exitCode = 1;
  } else {
    const watchdog = setTimeout(() => {
      process.stderr.write(`Probe did not finish: ${caseId}\n`); process.exit(2);
    }, 5000);
    try { process.stdout.write(JSON.stringify(await observe(caseId))); }
    finally { clearTimeout(watchdog); }
  }
} else {
  const report = {
    schemaVersion: 1,
    scope: 'Raw upstream source observations; no Rust comparison and no package/export verification.',
    limits: [
      'Generic reference case proves EventStream identity forwarding only, not Faux or real adapter partial semantics.',
      'No Models/provider/AI telemetry SDK closure was loaded.',
      'Pending is observed at named event-loop checkpoints, not inferred from a process timeout.',
      'Assertions validate this observed source behavior, not full compatibility or resource-budget acceptance.',
    ],
    node: process.version, platform: process.platform, arch: process.arch,
    driverSha256: createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex'),
    sources: sourcePaths.map((path) => ({ path,
      sha256: createHash('sha256').update(readFileSync(join(snapshot, path))).digest('hex') })),
    command: [process.execPath, fileURLToPath(import.meta.url), ...args],
    cases: [],
  };
  for (const caseId of cases) {
    const childArgs = [fileURLToPath(import.meta.url), '--snapshot', snapshot, '--case', caseId];
    const result = execFileSync(process.execPath, childArgs, {
      encoding: 'utf8', timeout: 10000, maxBuffer: 2 * 1024 * 1024,
      env: { SystemRoot: process.env.SystemRoot ?? '', TEMP: process.env.TEMP ?? '', TZ: 'UTC' },
    });
    report.cases.push({ ...JSON.parse(result), exitCode: 0, command: [process.execPath, ...childArgs] });
  }
  writeFileSync(resolve(value('--output')), JSON.stringify(report, null, 2) + '\n', 'utf8');
  process.stdout.write(`Recorded ${report.cases.length} upstream source cases; all source assertions passed.\n`);
}
