#!/usr/bin/env node
// Read-only observations of the actual frozen TS files, not a semantic reimplementation.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';

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
const resultBeforeEof = 'models-result-before-source-eof';
const terminalAndSpanBeforeEof = 'models-terminal-and-span-before-source-eof';
const resultAttributeWriteFailure = 'models-result-attribute-write-failure';
const resultAttributeWriteError = 'controlled-result-attribute-write-failure';
const firstChunkWriteFailure = 'models-first-chunk-event-write-failure';
const firstChunkWriteError = 'controlled-first-chunk-event-write-failure';
const histogramCreationFailure = 'models-request-duration-histogram-creation-failure';
const histogramCreationError = 'controlled-request-duration-histogram-creation-failure';
const requestSpanStartFailure = 'models-request-span-start-failure';
const requestSpanStartError = 'controlled-request-span-start-failure';
const firstChunkDurationFailure = 'models-first-chunk-duration-record-failure';
const firstChunkDurationError = 'controlled-first-chunk-duration-record-failure';
const firstChunkDurationMetric = 'pi.ai.first_chunk.duration';
const requestDurationFailure = 'models-request-duration-record-failure';
const requestDurationError = 'controlled-request-duration-record-failure';
const requestDurationMetric = 'pi.ai.request.duration';
const deferredWait0 = 'models-deferred-wait0-pending-then-ready';
const deferredResultBeforeEof = 'models-deferred-result-before-source-eof';
const deferredReadyBeforeEof = 'models-deferred-ready-before-source-eof';
const deferredLocalCancelAfterDone = 'models-deferred-local-cancel-after-done';
const deferredLocalCancelBeforeDone = 'models-deferred-local-cancel-before-done';
const deferredRemoteCancel = 'models-deferred-remote-cancel-then-fetch';
const deferredAuthSetupFailure = 'models-deferred-auth-setup-failure';
const deferredPositiveWait = 'models-deferred-positive-wait';
const deferredHandleExpiry = 'models-deferred-handle-expiry';
const deferredHandleRouteMismatch = 'models-deferred-handle-route-mismatch';
const deferredUnknownJobId = 'models-deferred-unknown-job-id';
const deferredStreamSourceCleanup = 'models-deferred-stream-source-cleanup';
const deferredHandleCases = [deferredHandleExpiry, deferredHandleRouteMismatch, deferredUnknownJobId];
const modelsLifecycleCases = [
  'models-no-consumption', 'models-stop-consumption', resultBeforeEof, terminalAndSpanBeforeEof, resultAttributeWriteFailure,
  firstChunkWriteFailure, histogramCreationFailure, requestSpanStartFailure, firstChunkDurationFailure, requestDurationFailure,
  deferredWait0, deferredResultBeforeEof, deferredReadyBeforeEof, deferredLocalCancelAfterDone, deferredLocalCancelBeforeDone,
  deferredRemoteCancel, deferredAuthSetupFailure, deferredPositiveWait, ...deferredHandleCases, deferredStreamSourceCleanup,
];
const modelsSetupCases = ['models-missing-provider', 'models-auth-setup-failure'];
const probeKey = 'in-memory-probe-key';
const probeModel = (provider) => ({
  id: 'mock-model', name: 'Lifecycle probe', api: 'anthropic-messages', provider,
  baseUrl: 'https://example.invalid', reasoning: false, input: ['text'],
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 4096, maxTokens: 512,
});

async function probeModels() {
  // This is the artifact behind the package's public ./models export.
  const { createModels } = await import(pathToFileURL(join(snapshot, 'packages/ai/dist/models.js')).href);
  return createModels();
}

async function probeRecorder() {
  const { InMemoryTelemetryContext } = await import(pathToFileURL(join(snapshot, 'packages/telemetry/dist/index.js')).href);
  return new InMemoryTelemetryContext();
}

function requestSpans(recorder, includeDetails = false) {
  return recorder.getSpans().filter((span) => span.name === 'pi.ai.request').map((span) => ({
    name: span.name, provider: span.attributes.provider, model: span.attributes.model,
    settled: span.settled, stop: span.attributes.stop ?? null,
    ...(includeDetails ? { events: span.events.map((event) => event.name), status: span.status } : {}),
  }));
}

function terminalSnapshot(item, final) {
  assert.equal(item.done, false);
  assert.equal(item.value.type, 'done');
  assert.equal(item.value.reason, 'stop');
  const terminal = item.value.message;
  assert.equal(terminal, final);
  return { kind: item.value.type, stop: terminal.stopReason, provider: terminal.provider, model: terminal.model };
}

async function waitAtGateOrAbort(gate, signal) {
  if (signal.aborted) return;
  const aborted = deferred();
  const onAbort = () => aborted.resolve();
  signal.addEventListener('abort', onAbort, { once: true });
  try {
    await Promise.race([gate, aborted.promise]);
  } finally {
    signal.removeEventListener('abort', onAbort);
  }
}

function gatedSource(partial, final, eofGate, getAbortSignal) {
  const gate = deferred(), reached = deferred(), finished = deferred();
  const returned = deferred();
  const counters = { iterators: 0, sourceReturn: 0, sourceTerminal: 0 };
  const cleanup = { returnCompleted: 0, sourceFinally: 0, resultCalls: 0 };
  let sourceEof = false;
  let result = final, abortedTerminals = 0;
  const source = {
    [Symbol.asyncIterator]() {
      counters.iterators++;
      const inner = (async function* () {
        try {
          if (partial) yield { type: 'start', partial };
          reached.resolve();
          if (getAbortSignal) {
            const signal = getAbortSignal();
            assert.ok(signal, 'the cancellable provider receives a signal');
            await waitAtGateOrAbort(gate.promise, signal);
            if (signal.aborted) {
              // Match Faux's provider-side error terminal on a cancelled stream.
              result = { ...partial, stopReason: 'aborted', errorMessage: 'Request was aborted' };
              abortedTerminals++;
              yield { type: 'error', reason: 'aborted', error: result };
              return;
            }
          } else {
            await gate.promise;
          }
          counters.sourceTerminal++;
          yield { type: 'done', reason: 'stop', message: final };
          if (eofGate) {
            eofGate.reached.resolve();
            await eofGate.release.promise;
          }
        } finally {
          cleanup.sourceFinally++;
        }
      })();
      return {
        async next() {
          const item = await inner.next();
          if (item.done) {
            sourceEof = true;
            finished.resolve();
          }
          return item;
        },
        async return(...params) {
          counters.sourceReturn++;
          const item = await inner.return(...params);
          if (item.done) cleanup.returnCompleted++;
          returned.resolve();
          return item;
        },
      };
    },
    result: async () => { cleanup.resultCalls++; return result; },
  };
  return { source, counters, cleanup, gate, reached, finished, returned,
    get sourceEof() { return sourceEof; }, get abortedTerminals() { return abortedTerminals; } };
}

function controlledModel(collection, eofGate, emitStart = true) {
  const model = probeModel('lifecycle-differential');
  const final = { ...message(), api: model.api, provider: model.provider, model: model.id };
  const partial = emitStart ? { ...final, content: [], stopReason: 'pending' } : null;
  const gated = gatedSource(partial, final, eofGate);
  const { source, counters, cleanup } = gated;
  let dispatchCalls = 0;
  const dispatch = (requestModel) => {
    assert.equal(requestModel.provider, model.provider);
    assert.equal(requestModel.id, model.id);
    dispatchCalls++;
    return source;
  };
  collection.setProvider({
    id: model.provider, name: 'Lifecycle probe', getModels: () => [model], stream: dispatch, streamSimple: dispatch,
    auth: { apiKey: { name: 'Controlled API key resolver', resolve: async ({ credential }) => {
      assert.equal(credential?.key, probeKey);
      return { auth: { apiKey: probeKey } };
    } } },
  });
  const sourceSnapshot = (includeCleanup = false) => ({
    dispatch_calls: dispatchCalls, source_started: counters.iterators > 0,
    terminal_count: counters.sourceTerminal, source_eof: gated.sourceEof,
    source_iterators: counters.iterators, source_return_calls: counters.sourceReturn,
    ...(includeCleanup ? {
      source_return_completions: cleanup.returnCompleted,
      source_finally_calls: cleanup.sourceFinally,
      source_result_calls: cleanup.resultCalls,
    } : {}),
  });
  const start = (telemetry) => {
    const options = { apiKey: probeKey };
    if (telemetry) options.telemetry = telemetry;
    return collection.streamSimple(model, { messages: [] }, options);
  };
  return { start, final, sourceSnapshot, gate: gated.gate, reached: gated.reached, finished: gated.finished, returned: gated.returned };
}

function controlledModelCall(collection, record, eofGate, telemetry) {
  const { start, ...fixture } = controlledModel(collection, eofGate);
  const observe = (step, detail) => record(step, { observed: fixture.sourceSnapshot(), ...detail });
  const call = start(telemetry);
  observe('constructed');
  return { call, observe, ...fixture };
}

async function observeModelsCompletion(caseId, record) {
  const collection = await probeModels();
  const eofGate = caseId === resultBeforeEof ? { reached: deferred(), release: deferred() } : undefined;
  const { call, final, observe, gate, reached, finished } = controlledModelCall(collection, record, eofGate);
  await reached.promise;
  observe('source-at-gate');
  if (caseId === 'models-stop-consumption') {
    const iterator = call[Symbol.asyncIterator]();
    const first = await iterator.next();
    assert.equal(first.value.type, 'start');
    assert.equal((await iterator.return()).done, true);
    observe('consumer-stopped-after-start');
  }
  const resultPromise = eofGate ? call.result() : undefined;
  const watch = resultPromise && track(resultPromise);
  if (watch) {
    await checkpoint();
    assert.equal(watch.state, 'pending');
  }
  observe('before-source-release', watch && { result_state: watch.state });
  gate.resolve();
  if (eofGate) {
    await eofGate.reached.promise;
    // Give the result's promise handlers a checkpoint while EOF is still held.
    await checkpoint();
    assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
    const ready = watch.state === 'fulfilled';
    if (ready) assert.equal(watch.result, final);
    observe('result-before-source-eof', {
      result_state: ready ? 'ready' : 'pending',
      result: ready ? { stop: watch.result.stopReason, provider: watch.result.provider, model: watch.result.model } : null,
    });
    eofGate.release.resolve();
  }
  await finished.promise;
  observe('source-released');
  const result = await (resultPromise ?? call.result());
  assert.equal(result, final);
  record('result-observed', {
    stop: result.stopReason, provider: result.provider, model: result.model, matches_source: result === final,
  });
}

async function observeModelsTerminalAndSpan(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    // Keep the actual parent and callback. The delegated child's promise
    // resolves after the recorder has settled the request span.
    let requestSpanFinished;
    const startSpan = parent.startSpan.bind(parent);
    parent.startSpan = (options, callback) => {
      const completion = startSpan(options, callback);
      if (options.name === 'pi.ai.request') requestSpanFinished = completion;
      return completion;
    };
    const eofGate = { reached: deferred(), release: deferred() };
    const { call, final, observe, gate, reached, finished } = controlledModelCall(collection, record, eofGate, parent);
    await reached.promise;
    observe('source-at-gate');
    assert.ok(requestSpanFinished, 'request span started');
    const iterator = call[Symbol.asyncIterator]();
    const first = await iterator.next();
    assert.equal(first.done, false);
    assert.equal(first.value.type, 'start');
    const next = iterator.next();
    const watch = track(next);
    await checkpoint();
    assert.equal(watch.state, 'pending');
    observe('before-source-release', { public_start: first.value.type, next_state: 'pending', request_spans: requestSpans(recorder) });
    gate.resolve();

    await eofGate.reached.promise;
    await checkpoint();
    assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
    const ready = watch.state === 'fulfilled';
    observe('terminal-and-span-before-source-eof', {
      next_state: ready ? 'ready' : 'pending', terminal: ready ? terminalSnapshot(watch.result, final) : null,
      request_spans: requestSpans(recorder),
    });
    eofGate.release.resolve();
    await finished.promise;
    observe('source-released');
    const terminal = await next;
    const done = terminalSnapshot(terminal, final);
    const outerEof = await iterator.next();
    assert.equal(outerEof.done, true);
    await requestSpanFinished;
    record('public-stream-completed', { terminal: done, outer_eof: outerEof.done, request_spans: requestSpans(recorder) });
    const result = await call.result();
    record('result-observed', {
      stop: result.stopReason, provider: result.provider, model: result.model,
      matches_source: result === final, matches_terminal: result === terminal.value.message,
    });
  });
}

async function observeHostRejections(observe, describe = (reason) => ({ message: String(reason) })) {
  const hostErrors = [];
  const unhandled = (reason, promise) => hostErrors.push({ channel: 'unhandledRejection', ...describe(reason, promise) });
  process.on('unhandledRejection', unhandled);
  try {
    await observe(hostErrors);
  } finally {
    process.off('unhandledRejection', unhandled);
  }
}

function faultHistogramRecord(recorder, target, marker, sample) {
  const attempts = [];
  const histogram = recorder.histogram.bind(recorder);
  recorder.histogram = (name, definition) => {
    const instrument = histogram(name, definition);
    if (name !== target) return instrument;
    return { record(...args) {
      attempts.push({ name, kind: definition.kind, unit: definition.unit, ...sample(args[1]) });
      if (attempts.length === 1) throw new Error(marker);
      instrument.record(...args);
    } };
  };
  return attempts;
}

async function observeResultAfterRelease({ call, final, observe, gate, reached, finished }, recorder, record, faultReached) {
  await reached.promise;
  observe('source-at-gate');
  const iterator = call[Symbol.asyncIterator]();
  const first = await iterator.next();
  assert.equal(first.done, false);
  assert.equal(first.value.type, 'start');
  observe('before-source-release', { public_start: first.value.type, request_spans: requestSpans(recorder) });
  gate.resolve();
  await finished.promise;
  observe('source-released');
  await faultReached.promise;
  const terminal = await iterator.next();
  const done = terminalSnapshot(terminal, final);
  const outerEof = await iterator.next();
  assert.equal(outerEof.done, true);
  record('public-stream-completed', { terminal: done, outer_eof: outerEof.done });
  const result = await call.result();
  record('result-observed', {
    stop: result.stopReason, provider: result.provider, model: result.model,
    matches_source: result === final, matches_terminal: result === terminal.value.message,
  });
}

async function observeModelsResultAttributeFailure(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      const faultReached = deferred();
      const writes = [];
      let sampleSource;
      const startSpan = parent.startSpan.bind(parent);
      parent.startSpan = (options, callback) => {
        if (options.name !== 'pi.ai.request') return startSpan(options, callback);
        return startSpan(options, (span) => {
          const setAttributes = span.setAttributes.bind(span);
          span.setAttributes = (attributes) => {
            if (typeof attributes.stop === 'string' && Object.hasOwn(attributes, 'usage.input')) {
              writes.push({ observed: sampleSource(), stop: attributes.stop });
              if (writes.length === 1) {
                faultReached.resolve();
                throw new Error(resultAttributeWriteError);
              }
            }
            return setAttributes(attributes);
          };
          return callback(span);
        });
      };
      const fixture = controlledModelCall(collection, record, undefined, parent);
      sampleSource = fixture.sourceSnapshot;
      await observeResultAfterRelease(fixture, recorder, record, faultReached);
      // Observe settlement and the host channel after the injected throw and
      // the forwarding continuation have had an event-loop checkpoint.
      await checkpoint();
      record('result-attribute-write-failure-observed', {
        fault_delivery: 'synchronous-throw', fault_marker: resultAttributeWriteError, write_attempts: writes,
        request_spans: requestSpans(recorder), host_errors: hostErrors,
      });
    });
  });
}

async function observeTerminalGate({ call, final, sourceSnapshot, gate, reached, returned }, recorder, record) {
  const sampleSource = () => sourceSnapshot(true);
  const snapshotTerminal = (item) => {
    if (!item.done && item.value.type === 'error') {
      assert.equal(item.value.reason, 'error');
      const error = item.value.error;
      return { kind: 'error', stop: error.stopReason, provider: error.provider, model: error.model, error_message: error.errorMessage };
    }
    return { ...terminalSnapshot(item, final), error_message: null };
  };
  const iterator = call[Symbol.asyncIterator]();
  const first = await iterator.next();
  assert.equal(first.done, false);
  assert.equal(first.value.type, 'start');
  // A forwarding loop that closes its iterator after Start never enters
  // the provider's terminal gate. Either signal permits the held sample.
  await Promise.race([reached.promise, returned.promise]);
  const next = iterator.next();
  const watch = track(next);
  await checkpoint();
  assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
  const ready = watch.state === 'fulfilled';
  record('terminal-gate-held', {
    observed: sampleSource(), public_start: first.value.type, next_state: ready ? 'ready' : 'pending',
    terminal: ready ? snapshotTerminal(watch.result) : null, request_spans: requestSpans(recorder),
  });
  gate.resolve();
  const item = await next;
  const terminal = snapshotTerminal(item);
  const eof = await iterator.next();
  assert.equal(eof.done, true);
  record('public-stream-completed', { terminal, outer_eof: eof.done, observed: sampleSource() });
  const result = await call.result();
  record('result-observed', {
    delivery: 'fulfilled-message', stop: result.stopReason, provider: result.provider, model: result.model,
    matches_terminal: result === (item.value.type === 'error' ? item.value.error : item.value.message),
  });
}

async function observeModelsFirstChunkFailure(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      const writes = [];
      let sampleSource;
      const startSpan = parent.startSpan.bind(parent);
      parent.startSpan = (options, callback) => {
        if (options.name !== 'pi.ai.request') return startSpan(options, callback);
        return startSpan(options, (span) => {
          const addEvent = span.addEvent.bind(span);
          span.addEvent = (name, attributes) => {
            if (name === 'pi.ai.first_chunk') {
              writes.push({ event: name, observed: sampleSource() });
              if (writes.length === 1) throw new Error(firstChunkWriteError);
            }
            return addEvent(name, attributes);
          };
          return callback(span);
        });
      };
      const constructed = (step, data) => record(step, { ...data, request_spans: requestSpans(recorder) });
      const fixture = controlledModelCall(collection, constructed, undefined, parent);
      sampleSource = () => fixture.sourceSnapshot(true);
      await observeTerminalGate(fixture, recorder, record);
      await checkpoint();
      record('first-chunk-write-failure-observed', {
        fault_delivery: 'synchronous-throw', fault_marker: firstChunkWriteError, write_attempts: writes,
        request_spans: requestSpans(recorder), host_errors: hostErrors,
      });
    });
  });
}

async function observeModelsHistogramCreationFailure(record) {
  const collection = await probeModels();
  // Pi caches instruments by metrics adapter identity. This recorder is cold.
  const recorder = await probeRecorder();
  const { start, final, sourceSnapshot } = controlledModel(collection);
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      assert.equal(parent.metrics, recorder);
      const injected = new Error(histogramCreationError);
      const attempts = [];
      const histogram = recorder.histogram.bind(recorder);
      recorder.histogram = (name, definition) => {
        if (name === 'pi.ai.request.duration') {
          attempts.push({
            name, kind: definition.kind, unit: definition.unit,
            observed: sourceSnapshot(), request_spans: requestSpans(recorder),
          });
          if (attempts.length === 1) throw injected;
        }
        return histogram(name, definition);
      };
      let call, errorMessage;
      try {
        call = start(parent);
      } catch (error) {
        assert.equal(error, injected);
        errorMessage = error.message;
      }
      assert.equal(call, undefined);
      assert.equal(errorMessage, histogramCreationError);
      record('call-boundary', {
        provider: final.provider, model: final.model, delivery: 'synchronous-throw', error_message: errorMessage,
        instrument_attempt_count: attempts.length, observed: sourceSnapshot(), request_spans: requestSpans(recorder),
      });
      await checkpoint();
      record('histogram-creation-failure-observed', {
        fault_delivery: 'synchronous-throw', fault_marker: injected.message, instrument_attempts: attempts,
        observed: sourceSnapshot(), request_spans: requestSpans(recorder), host_errors: hostErrors,
      });
    });
  });
}

async function observeModelsRequestSpanStartFailure(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  const fixture = controlledModel(collection);
  const injected = new Error(requestSpanStartError);
  let rejectedStart;
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      assert.equal(parent.metrics, recorder);
      const instruments = [];
      for (const kind of ['histogram', 'counter']) {
        const create = recorder[kind].bind(recorder);
        recorder[kind] = (name, definition) => {
          const instrument = create(name, definition);
          instruments.push({ name, kind, unit: definition.unit });
          return instrument;
        };
      }
      const attempts = [];
      const startSpan = parent.startSpan.bind(parent);
      parent.startSpan = (options, callback) => {
        if (options.name === 'pi.ai.request') {
          attempts.push({
            name: options.name, provider: options.attributes.provider, model: options.attributes.model,
            created_instruments: [...instruments], observed: fixture.sourceSnapshot(),
            request_spans: requestSpans(recorder),
          });
          if (attempts.length === 1) {
            rejectedStart = Promise.reject(injected);
            return rejectedStart;
          }
        }
        return startSpan(options, callback);
      };
      const call = fixture.start(parent);
      record('constructed', {
        call_delivery: 'returned-handle', span_start_attempt_count: attempts.length,
        observed: fixture.sourceSnapshot(), request_spans: requestSpans(recorder),
      });
      await observeTerminalGate({ ...fixture, call }, recorder, record);
      await checkpoint();
      record('request-span-start-failure-observed', {
        fault_delivery: 'rejected-promise', fault_marker: injected.message, span_start_attempts: attempts,
        request_spans: requestSpans(recorder), host_errors: hostErrors,
      });
    });
  }, (reason, promise) => ({
    message: String(reason), error_name: reason instanceof Error ? reason.name : null,
    injected_start_rejection: promise === rejectedStart && reason === injected,
  }));
}

async function observeModelsFirstChunkDurationFailure(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  const eofGate = { reached: deferred(), release: deferred() };
  const fixture = controlledModel(collection, eofGate, false);
  const source = () => fixture.sourceSnapshot(true);
  const spans = () => requestSpans(recorder, true);
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      assert.equal(parent.metrics, recorder);
      const attempts = faultHistogramRecord(recorder, firstChunkDurationMetric, firstChunkDurationError,
        () => ({ observed: source(), request_spans: spans() }));
      const call = fixture.start(parent);
      record('constructed', { observed: source(), request_spans: spans() });
      await fixture.reached.promise;
      const iterator = call[Symbol.asyncIterator]();
      const next = iterator.next();
      const watch = track(next);
      await checkpoint();
      assert.equal(watch.state, 'pending');
      record('terminal-gate-held', { observed: source(), next_state: watch.state, request_spans: spans() });
      fixture.gate.resolve();
      const eofReached = await Promise.race([
        eofGate.reached.promise.then(() => true), fixture.returned.promise.then(() => false),
      ]);
      await checkpoint();
      assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
      const ready = watch.state === 'fulfilled';
      record('source-exit-checkpoint', {
        observed: source(), eof_gate_reached: eofReached, next_state: ready ? 'ready' : 'pending',
        terminal: ready ? terminalSnapshot(watch.result, fixture.final) : null, request_spans: spans(),
      });
      eofGate.release.resolve();
      if (eofReached) await fixture.finished.promise;
      const item = await next;
      const terminal = terminalSnapshot(item, fixture.final);
      const outerEof = await iterator.next();
      assert.equal(outerEof.done, true);
      record('public-stream-completed', { terminal, outer_eof: outerEof.done, observed: source() });
      const result = await call.result();
      assert.equal(result, fixture.final);
      record('result-observed', {
        delivery: 'fulfilled-message', stop: result.stopReason, provider: result.provider, model: result.model,
        matches_terminal: result === item.value.message,
      });
      await checkpoint();
      const durations = recorder.getMetrics()
        .filter(({ name }) => name === firstChunkDurationMetric || name === requestDurationMetric)
        .map(({ name, attributes }) => ({ name, stop: attributes.stop ?? null }));
      record('first-chunk-duration-record-failure-observed', {
        fault_delivery: 'synchronous-throw', fault_marker: firstChunkDurationError, record_attempts: attempts,
        request_spans: spans(), duration_measurements: durations, host_errors: hostErrors,
      });
    });
  });
}

async function observeModelsRequestDurationFailure(record) {
  const collection = await probeModels();
  const recorder = await probeRecorder();
  await observeHostRejections(async (hostErrors) => {
    await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
      assert.equal(parent.metrics, recorder);
      const faultReached = deferred();
      const attempts = faultHistogramRecord(recorder, requestDurationMetric, requestDurationError, (attributes) => {
        faultReached.resolve();
        const span = recorder.getSpans().find(({ name }) => name === 'pi.ai.request');
        return {
          observed: fixture.sourceSnapshot(true), request_spans: requestSpans(recorder, true),
          record_stop: attributes.stop ?? null, usage_input: span?.attributes['usage.input'] ?? null,
        };
      });
      const fixture = controlledModelCall(collection, record, undefined, parent);
      await observeResultAfterRelease(fixture, recorder, record, faultReached);
      await checkpoint();
      record('request-duration-record-failure-observed', {
        fault_delivery: 'synchronous-throw', fault_marker: requestDurationError, record_attempts: attempts,
        request_spans: requestSpans(recorder, true), metric_names: recorder.getMetrics().map(({ name }) => name),
        host_errors: hostErrors,
      });
    });
  });
}

async function observeModelsSetup(caseId, record) {
  const collection = await probeModels();
  const authFailure = caseId === 'models-auth-setup-failure';
  const provider = authFailure ? 'lifecycle-differential' : 'missing-provider';
  const model = probeModel(provider);
  const counters = { auth_calls: 0, dispatch_calls: 0 };
  const reached = deferred(), release = deferred();
  const marker = 'controlled-auth-setup-failure';
  if (authFailure) {
    const dispatch = () => { counters.dispatch_calls++; throw new Error('dispatch before successful auth'); };
    collection.setProvider({
      id: provider, name: 'Lifecycle probe', getModels: () => [model], stream: dispatch, streamSimple: dispatch,
      auth: { apiKey: { name: 'Controlled API key resolver', resolve: async ({ credential }) => {
        assert.equal(credential?.key, probeKey);
        counters.auth_calls++;
        reached.resolve();
        await release.promise;
        throw new Error(marker);
      } } },
    });
  }
  // The explicit fixture key selects the resolver without a preceding store await.
  const stream = collection.streamSimple(model, { messages: [] }, authFailure ? { apiKey: probeKey } : undefined);
  record('constructed', {
    phase: authFailure ? 'authentication' : 'route-lookup', provider, model: model.id, ...counters,
  });
  if (authFailure) {
    await reached.promise;
    assert.equal(counters.auth_calls, 1);
    assert.equal(counters.dispatch_calls, 0);
    record('auth-at-gate', counters);
    release.resolve();
  }
  const iterator = stream[Symbol.asyncIterator]();
  const first = await iterator.next();
  assert.equal(first.done, false);
  assert.equal(first.value.type, 'error');
  const failure = first.value.error;
  assert.equal(failure.stopReason, 'error');
  assert.equal(failure.provider, provider);
  assert.equal(failure.model, model.id);
  assert.ok(failure.errorMessage.includes(authFailure ? marker : 'Unknown provider: missing-provider'));
  record('failure-event', {
    delivery: 'error-event', stop: failure.stopReason, error_message: failure.errorMessage,
    message_provider: failure.provider, message_model: failure.model,
  });
  const eof = await iterator.next();
  assert.equal(eof.done, true);
  record('event-eof', { done: eof.done, ...counters });
  const result = await stream.result();
  assert.equal(result, failure);
  record('result-observed', { delivery: 'fulfilled-message', stop: result.stopReason, matches_event: result === failure });
}

function deferredModel(collection, { eofGate, abortAtTerminalGate = false, beforeAuthSuccess, beforeFetchEvents } = {}) {
  const model = { ...probeModel('lifecycle-differential'), api: 'scripted-deferred' };
  const handle = { provider: model.provider, modelId: model.id, api: model.api, id: 'deferred-1', pollAfterMs: 25 };
  const base = { ...message('ready'), api: model.api, provider: model.provider, model: model.id };
  const pending = { ...base, content: [], stopReason: 'deferred', deferred: handle,
    usage: { ...base.usage, input: 0, output: 0, totalTokens: 0 } };
  const ready = { ...base, usage: { input: 2, output: 3, cacheRead: 4, cacheWrite: 5, totalTokens: 14,
    cost: { ...base.usage.cost, input: 0.25, total: 0.25 } } };
  const dispatches = [];
  let forwardedSignal, remoteCancelCalls = 0;
  const gated = (eofGate || abortAtTerminalGate) && gatedSource(
    { ...ready, ...(abortAtTerminalGate && { content: [] }), stopReason: 'pending' }, ready, eofGate,
    abortAtTerminalGate ? () => forwardedSignal : undefined,
  );
  const unexpectedChat = () => assert.fail('fetch must use the deferred dispatch');
  collection.setProvider({
    id: model.provider, name: 'Lifecycle probe', getModels: () => [model],
    stream: unexpectedChat, streamSimple: unexpectedChat,
    auth: { apiKey: { name: 'Controlled API key resolver', resolve: async ({ credential }) => {
      assert.equal(credential?.key, probeKey);
      if (beforeAuthSuccess) await beforeAuthSuccess();
      return { auth: { apiKey: probeKey } };
    } } },
    fetchDeferred(requestModel, receivedHandle, options) {
      assert.deepEqual([requestModel.provider, requestModel.id, requestModel.api], [model.provider, model.id, model.api]);
      dispatches.push({ handle: structuredClone(receivedHandle), wait_ms: options.wait ?? null });
      assert.equal(remoteCancelCalls, 0, 'the scripted job is unavailable after a remote cancel');
      const attempt = dispatches.length;
      if (gated && attempt === 1) {
        forwardedSignal = options.signal;
        return gated.source;
      }
      const result = attempt === 1 ? pending : ready;
      return {
        async *[Symbol.asyncIterator]() {
          if (beforeFetchEvents) await beforeFetchEvents(attempt);
          yield { type: 'start', partial: result === pending ? result : { ...base, content: [], stopReason: 'pending' } };
          yield { type: 'done', reason: result.stopReason, message: result };
        },
        result: async () => result,
      };
    },
    async cancelDeferred() { remoteCancelCalls++; },
  });
  const fetch = (telemetry, signal, wait = 0) => collection.fetchDeferred(model, handle, { wait, apiKey: probeKey, telemetry, signal });
  const stream = (telemetry) => collection.streamDeferred(model, handle, { wait: 0, apiKey: probeKey, telemetry });
  return { dispatches, fetch, stream, handle, ready, gated,
    get forwardedSignal() { return forwardedSignal; },
    get remoteCancelCalls() { return remoteCancelCalls; },
  };
}

function deferredResultSnapshot(result) {
  return { stop: result.stopReason, provider: result.provider, model: result.model,
    content: result.content, deferred: result.deferred ?? null,
    usage: { input: result.usage.input, output: result.usage.output, cache_read: result.usage.cacheRead,
      cache_write: result.usage.cacheWrite, cost: result.usage.cost.total } };
}

function deferredPollSnapshot(recorder, parentId, spanOffset, metricOffset) {
  const spans = recorder.getSpans().slice(spanOffset);
  const measurements = recorder.getMetrics().slice(metricOffset).map((measurement) => {
    const duration = measurement.kind === 'histogram';
    assert.ok(!duration || (Number.isFinite(measurement.value) && measurement.value >= 0));
    return { name: measurement.name, kind: measurement.kind, unit: measurement.unit,
      attributes: measurement.attributes, value: duration ? null : measurement.value,
      context: measurement.context === null ? 'none' :
        isDeepStrictEqual(measurement.context, spans[0]?.context) ? 'poll' : 'other' };
  });
  return {
    spans: spans.map((span) => ({
      name: span.name, attributes: span.attributes, status: span.status, parent_is_caller: span.parentId === parentId,
      settled: span.settled, events: span.events.map((event) => event.name),
    })),
    measurements,
  };
}

async function observeDeferredWait0(record) {
  const { dispatches, fetch } = deferredModel(await probeModels());
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    for (const name of ['pending', 'ready']) {
      const dispatchOffset = dispatches.length;
      const spanOffset = recorder.getSpans().length;
      const metricOffset = recorder.getMetrics().length;
      const result = await fetch(parent);
      const settledAtResult = recorder.getSpans().slice(spanOffset).map((span) => span.settled);
      const metricNamesAtResult = recorder.getMetrics().slice(metricOffset).map((measurement) => measurement.name);
      await checkpoint();

      record(name, {
        dispatches: dispatches.slice(dispatchOffset),
        result: deferredResultSnapshot(result),
        settled_at_result: settledAtResult, metric_names_at_result: metricNamesAtResult,
        ...deferredPollSnapshot(recorder, parentId, spanOffset, metricOffset),
      });
    }
  });
}

function deferredGatedSourceSnapshot(dispatches, gated) {
  return {
    dispatch_calls: dispatches.length, source_started: gated.counters.iterators > 0,
    terminal_count: gated.counters.sourceTerminal, source_eof: gated.sourceEof,
    source_iterators: gated.counters.iterators, source_return_calls: gated.counters.sourceReturn,
    source_return_completions: gated.cleanup.returnCompleted, source_finally_calls: gated.cleanup.sourceFinally,
    source_result_calls: gated.cleanup.resultCalls,
  };
}

async function observeGatedDeferredFetch(record, cancelAt) {
  const beforeDone = cancelAt === 'before-done';
  const eofGate = beforeDone ? undefined : { reached: deferred(), release: deferred() };
  const fixture = deferredModel(await probeModels(), { eofGate, abortAtTerminalGate: beforeDone });
  const { dispatches, fetch, ready, gated } = fixture;
  const controller = cancelAt ? new AbortController() : null;
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    const observe = (step, result, details = {}) => record(step, {
      observed: {
        ...deferredGatedSourceSnapshot(dispatches, gated),
        ...(beforeDone && { source_aborted_terminals: gated.abortedTerminals }),
      },
      dispatches, result: result ? deferredResultSnapshot(result) : null,
      ...deferredPollSnapshot(recorder, parentId, 1, 0),
      ...(controller && {
        local_cancelled: controller.signal.aborted,
        backend_cancelled: fixture.forwardedSignal?.aborted ?? null,
        signal_forwarded: fixture.forwardedSignal === controller.signal,
        remote_cancel_calls: fixture.remoteCancelCalls,
      }),
      ...details,
    });
    const resultPromise = fetch(parent, controller?.signal);
    const watch = track(resultPromise);
    await gated.reached.promise;
    await checkpoint();
    assert.equal(watch.state, 'pending');
    observe('terminal-gate-held', null);
    if (beforeDone) {
      controller.abort();
      const result = await resultPromise;
      const failure = { result_delivery: 'fulfilled-message', error_message: result.errorMessage };
      assert.equal(result.stopReason, 'aborted');
      observe('local-cancel-observed', result, failure);

      await gated.finished.promise;
      await checkpoint();
      observe('source-released', result, failure);
    } else {
      gated.gate.resolve();
      await eofGate.reached.promise;
      await checkpoint();
      assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
      if (watch.state === 'fulfilled') assert.equal(watch.result, ready);
      observe('eof-gate-held', watch.result);
      if (controller) {
        controller.abort();
        await checkpoint();
        assert.ok(watch.state === 'pending' || watch.state === 'fulfilled', watch.error);
        observe('local-cancel-observed', watch.result);
      }
      eofGate.release.resolve();

      await gated.finished.promise;
      const result = await resultPromise;
      assert.equal(result, ready);
      await checkpoint();
      observe('source-released', result);
    }
    if (controller) {
      const offset = dispatches.length;
      const refetched = await fetch();
      record('handle-refetched', {
        dispatches: dispatches.slice(offset), result: deferredResultSnapshot(refetched),
        remote_cancel_calls: fixture.remoteCancelCalls,
      });
    }
  });
}

async function observeDeferredStreamSourceCleanup(record, stopAfter) {
  const eofGate = { reached: deferred(), release: deferred() };
  const eofGateState = track(eofGate.reached.promise);
  const prefix = stopAfter ? `stop-after-${stopAfter}-` : '';
  let authCalls = 0;
  const fixture = deferredModel(await probeModels(), { eofGate, beforeAuthSuccess: () => { authCalls++; } });
  const { dispatches, stream, ready, gated } = fixture;
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    const observe = (step, delivery) => record(prefix + step, {
      observed: deferredGatedSourceSnapshot(dispatches, gated),
      eof_gate_reached: eofGateState.state === 'fulfilled',
      ...(stopAfter && { auth_calls: authCalls }),
      dispatches, remote_cancel_calls: fixture.remoteCancelCalls, delivery,
      ...deferredPollSnapshot(recorder, parentId, 1, 0),
    });
    const deferredStream = stream(parent);
    const iterator = deferredStream[Symbol.asyncIterator]();
    const start = await iterator.next();
    assert.equal(start.done, false);
    assert.equal(start.value.type, 'start');
    await gated.reached.promise;
    observe('terminal-gate-held', { kind: start.value.type });

    gated.gate.resolve();
    const terminal = await iterator.next();
    terminalSnapshot(terminal, ready);
    await eofGate.reached.promise;
    observe('done-observed', { kind: terminal.value.type, reason: terminal.value.reason,
      message: deferredResultSnapshot(terminal.value.message) });
    if (stopAfter !== 'done') {
      assert.equal((await iterator.next()).done, true);
      observe('public-eof-observed', { kind: 'eof' });
    }
    if (stopAfter) {
      assert.equal((await iterator.return()).done, true);
      observe('consumer-stopped', { kind: 'stopped' });
    }

    eofGate.release.resolve();
    await gated.finished.promise;
    if (stopAfter) {
      await checkpoint();
      observe('after-eof-release', { kind: 'stopped' });
      return;
    }
    const result = await deferredStream.result();
    await checkpoint();
    observe('source-released', { kind: 'result', message: deferredResultSnapshot(result) });
  });
}

async function observeDeferredRemoteCancel(record) {
  const { fauxProvider, fauxAssistantMessage } = await import(
    pathToFileURL(join(snapshot, 'packages/ai/dist/providers/faux.js')).href
  );
  const faux = fauxProvider({
    provider: 'lifecycle-differential', api: 'scripted-deferred', models: [{ id: 'mock-model' }],
    deferred: { pendingFetches: 1, pollAfterMs: 25 },
  });
  const model = faux.getModel();
  const models = await probeModels();
  const dispatches = [];
  const cancelContexts = [];
  models.setProvider({
    ...faux.provider,
    fetchDeferred(requestModel, handle, options) {
      dispatches.push({ handle: structuredClone(handle), wait_ms: options.wait ?? null });
      return faux.provider.fetchDeferred(requestModel, handle, options);
    },
    cancelDeferred(requestModel, handle, options) {
      cancelContexts.push(options.telemetry);
      return faux.provider.cancelDeferred(requestModel, handle, options);
    },
  });
  faux.setResponses([fauxAssistantMessage('ready')]);
  const submission = await models.completeSimple(model, { messages: [] }, { deferred: true, apiKey: probeKey });
  assert.equal(submission.stopReason, 'deferred');
  const handle = structuredClone(submission.deferred);
  assert.match(handle.id, /^deferred:/);
  // Preserve the actual handle at dispatch. Canonicalize its random ID in evidence only.
  const normalizeHandle = (received) => {
    assert.deepEqual(received, handle);
    return { ...received, id: 'deferred-1' };
  };
  const normalizeError = (message) => message.replace(handle.id, 'deferred-1');

  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    let spanOffset = 1, metricOffset = 0;
    const observe = (step, fields) => {
      const { spans, measurements } = deferredPollSnapshot(recorder, parentId, spanOffset, metricOffset);
      spanOffset += spans.length;
      metricOffset += measurements.length;
      for (const span of spans) {
        if (span.status.error) {
          const error = span.status.error;
          span.status = { ...span.status, error: { ...error, message: normalizeError(error.message) } };
        }
      }
      record(step, {
        dispatches: dispatches.splice(0).map((entry) => ({ ...entry, handle: normalizeHandle(entry.handle) })),
        fetch_count: faux.state.deferredFetchCount,
        cancelled_handles: faux.state.cancelledDeferred.map(normalizeHandle),
        spans, measurements, ...fields,
      });
    };
    const fetch = () => models.fetchDeferred(model, handle, { wait: 0, apiKey: probeKey, telemetry: parent });
    const pending = await fetch();
    await checkpoint();
    observe('pending', { result: deferredResultSnapshot({ ...pending, deferred: normalizeHandle(pending.deferred) }) });

    const cancelled = await models.cancelDeferred(model, handle, { apiKey: probeKey, telemetry: parent });
    assert.equal(cancelled, undefined);
    await checkpoint();
    observe('cancelled', { result_delivery: 'ok', telemetry_forwarded: cancelContexts.length === 1 && cancelContexts[0] === parent });

    const failure = await fetch();
    assert.equal(failure.errorMessage, `Faux deferred response was cancelled: ${handle.id}`);
    await checkpoint();
    observe('refetch-failed', {
      result: deferredResultSnapshot(failure), result_delivery: 'fulfilled-message',
      error_message: normalizeError(failure.errorMessage),
    });
  });
  assert.equal(faux.state.callCount, 1);
}

async function observeDeferredAuthSetupFailure(record) {
  const reached = deferred(), release = deferred();
  const marker = 'controlled-auth-setup-failure';
  let authCalls = 0;
  const fixture = deferredModel(await probeModels(), {
    async beforeAuthSuccess() {
      authCalls++;
      reached.resolve();
      await release.promise;
      throw new Error(marker);
    },
  });
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    const observe = (step, fields) => record(step, {
      auth_calls: authCalls, dispatches: fixture.dispatches, remote_cancel_calls: fixture.remoteCancelCalls,
      ...deferredPollSnapshot(recorder, parentId, 1, 0), ...fields,
    });
    const fetch = fixture.fetch(parent);
    const watch = track(fetch);
    await reached.promise;
    await checkpoint();
    assert.equal(watch.state, 'pending');
    observe('auth-at-gate', { handle: fixture.handle, wait_ms: 0, result_state: watch.state });
    release.resolve();

    const failure = await fetch;
    assert.ok(failure.errorMessage.includes(marker));
    await checkpoint();
    observe('fetch-failed', {
      result: deferredResultSnapshot(failure), result_delivery: 'fulfilled-message', error_message: failure.errorMessage,
    });
  });
}

async function observeDeferredPositiveWait(record) {
  const waitMs = 50;
  const attempts = ['deferred', 'ready'].map((outcome) => ({ outcome, reached: deferred(), release: deferred() }));
  const fixture = deferredModel(await probeModels(), {
    async beforeFetchEvents(attempt) {
      const { reached, release } = attempts[attempt - 1];
      reached.resolve();
      await release.promise;
    },
  });
  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    for (const { outcome, reached, release } of attempts) {
      const dispatchOffset = fixture.dispatches.length;
      const spanOffset = recorder.getSpans().length;
      const metricOffset = recorder.getMetrics().length;
      const observe = (name, fields) => record(name, {
        dispatches: fixture.dispatches.slice(dispatchOffset), remote_cancel_calls: fixture.remoteCancelCalls,
        ...deferredPollSnapshot(recorder, parentId, spanOffset, metricOffset), ...fields,
      });
      const fetch = fixture.fetch(parent, undefined, waitMs);
      const watch = track(fetch);
      await reached.promise;
      await checkpoint();
      assert.equal(watch.state, 'pending');
      observe(`${outcome}-outcome-held`, { result_state: watch.state, result: null });
      release.resolve();

      const result = await fetch;
      await checkpoint();
      observe(outcome, { result: deferredResultSnapshot(result) });
    }
  });
  assert.equal(fixture.dispatches.length, attempts.length);
}

async function observeDeferredHandleChecks(caseId, record) {
  const { fauxProvider, fauxAssistantMessage } = await import(
    pathToFileURL(join(snapshot, 'packages/ai/dist/providers/faux.js')).href
  );
  const faux = fauxProvider({
    provider: 'lifecycle-differential', api: 'scripted-deferred', models: [{ id: 'mock-model' }],
    deferred: { pendingFetches: 0, pollAfterMs: 25 },
  });
  const model = faux.getModel();
  const models = await probeModels();
  const apiKey = faux.provider.auth.apiKey;
  let authCalls = 0;
  const dispatches = [];
  models.setProvider({
    ...faux.provider,
    auth: { ...faux.provider.auth, apiKey: { ...apiKey, async resolve(input) {
      assert.equal(input.credential?.key, probeKey);
      authCalls++;
      return apiKey.resolve(input);
    } } },
    fetchDeferred(requestModel, receivedHandle, options) {
      assert.deepEqual([requestModel.provider, requestModel.id, requestModel.api], [model.provider, model.id, model.api]);
      dispatches.push({ handle: structuredClone(receivedHandle), wait_ms: options.wait ?? null });
      return faux.provider.fetchDeferred(requestModel, receivedHandle, options);
    },
  });
  faux.setResponses([fauxAssistantMessage('ready')]);
  const submission = await models.completeSimple(model, { messages: [] }, { deferred: true, apiKey: probeKey });
  assert.equal(submission.stopReason, 'deferred');
  const original = submission.deferred;
  assert.match(original.id, /^deferred:/);
  assert.equal(original.expiresAt, undefined);
  let attempts;
  switch (caseId) {
    case deferredHandleExpiry:
      attempts = ['before-expiry', 'at-expiry', 'without-sdk-clock'].map((name) => [
        name, { ...original, expiresAt: 1_700_000_000_000 },
      ]);
      break;
    case deferredHandleRouteMismatch:
      attempts = [
        ['provider-mismatch', { ...original, provider: 'other-provider' }],
        ['model-mismatch', { ...original, modelId: 'other-model' }],
        ['api-mismatch', { ...original, api: 'other-api' }],
        ['original-handle-refetch', original],
      ];
      break;
    case deferredUnknownJobId:
      attempts = [
        ['unknown-job-id', { ...original, id: 'deferred-missing' }],
        ['original-handle-refetch', original],
      ];
      break;
    default:
      assert.fail(`Unexpected handle check: ${caseId}`);
  }
  // Only the ID generated by Faux needs normalization in the trace.
  const normalizeHandle = (handle) => ({ ...handle, id: handle.id === original.id ? 'deferred-1' : handle.id });
  const normalizeError = (message) => message.replace(original.id, 'deferred-1');

  const recorder = await probeRecorder();
  await recorder.startSpan({ name: 'lifecycle-probe' }, async (parent) => {
    const parentId = recorder.getSpans()[0].id;
    for (const [name, handle] of attempts) {
      const authOffset = authCalls;
      const fetchOffset = faux.state.deferredFetchCount;
      const spanOffset = recorder.getSpans().length;
      const metricOffset = recorder.getMetrics().length;
      const result = await models.fetchDeferred(model, handle, { wait: 0, apiKey: probeKey, telemetry: parent });
      await checkpoint();
      const recordedDispatches = dispatches.splice(0).map((entry) => {
        assert.deepEqual(entry.handle, handle);
        return { ...entry, handle: normalizeHandle(entry.handle) };
      });
      const poll = deferredPollSnapshot(recorder, parentId, spanOffset, metricOffset);
      for (const span of poll.spans) {
        if (span.status.error) {
          const error = span.status.error;
          span.status = { ...span.status, error: { ...error, message: normalizeError(error.message) } };
        }
      }
      record(name, {
        request_handle: normalizeHandle(handle), sdk_now_ms: null, auth_calls: authCalls - authOffset,
        fetch_count: faux.state.deferredFetchCount - fetchOffset, dispatches: recordedDispatches,
        remote_cancel_calls: faux.state.cancelledDeferred.length,
        result: deferredResultSnapshot(result), result_delivery: 'fulfilled-message',
        error_message: result.errorMessage === undefined ? null : normalizeError(result.errorMessage),
        ...poll,
      });
    }
  });
  assert.equal(faux.state.callCount, 1);
}

async function observe(caseId) {
  const trace = [];
  const record = (step, data) => trace.push({ step, ...structuredClone(data) });
  if (modelsLifecycleCases.includes(caseId)) {
    if (caseId === terminalAndSpanBeforeEof) await observeModelsTerminalAndSpan(record);
    else if (caseId === resultAttributeWriteFailure) await observeModelsResultAttributeFailure(record);
    else if (caseId === firstChunkWriteFailure) await observeModelsFirstChunkFailure(record);
    else if (caseId === histogramCreationFailure) await observeModelsHistogramCreationFailure(record);
    else if (caseId === requestSpanStartFailure) await observeModelsRequestSpanStartFailure(record);
    else if (caseId === firstChunkDurationFailure) await observeModelsFirstChunkDurationFailure(record);
    else if (caseId === requestDurationFailure) await observeModelsRequestDurationFailure(record);
    else if (caseId === deferredWait0) await observeDeferredWait0(record);
    else if (caseId === deferredResultBeforeEof || caseId === deferredReadyBeforeEof) await observeGatedDeferredFetch(record);
    else if (caseId === deferredLocalCancelAfterDone) await observeGatedDeferredFetch(record, 'after-done');
    else if (caseId === deferredLocalCancelBeforeDone) await observeGatedDeferredFetch(record, 'before-done');
    else if (caseId === deferredRemoteCancel) await observeDeferredRemoteCancel(record);
    else if (caseId === deferredAuthSetupFailure) await observeDeferredAuthSetupFailure(record);
    else if (caseId === deferredPositiveWait) await observeDeferredPositiveWait(record);
    else if (caseId === deferredStreamSourceCleanup) {
      for (const stopAfter of [null, 'done', 'public-eof']) await observeDeferredStreamSourceCleanup(record, stopAfter);
    }
    else if (deferredHandleCases.includes(caseId)) await observeDeferredHandleChecks(caseId, record);
    else await observeModelsCompletion(caseId, record);
    return { id: caseId, status: 'observed', sourceAssertions: 'passed', trace };
  }
  if (modelsSetupCases.includes(caseId)) {
    await observeModelsSetup(caseId, record);
    return { id: caseId, status: 'observed', sourceAssertions: 'passed', trace };
  }
  const { EventStream } = await import(pathToFileURL(join(snapshot, sourcePaths[0])).href);
  const { lazyStream } = await import(pathToFileURL(join(snapshot, sourcePaths[1])).href);

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
    const final = message();
    const { source, counters, gate, reached, finished } = gatedSource(message(''), final);
    let setup = 0;
    const observations = () => ({ setup, ...counters });
    const outer = lazyStream(model, () => { setup++; return Promise.resolve(source); });
    record('immediately-after-call', { counters: observations() }); assert.equal(setup, 1);
    await reached.promise;
    record('source-at-gate-without-outer-poll', { counters: observations() });
    if (caseId === 'lazy-stop-consumption') {
      const iterator = outer[Symbol.asyncIterator]();
      const first = await iterator.next(); assert.equal(first.value.type, 'start');
      const stopped = await iterator.return();
      record('outer-iterator-return', { stopped, counters: observations() });
    }
    gate.resolve(); await finished.promise; await checkpoint();
    const result = await outer.result();
    record('source-completed-after-gate', { counters: observations(), resultStop: result.stopReason,
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
  if (!cases.includes(caseId) && !modelsLifecycleCases.includes(caseId) && !modelsSetupCases.includes(caseId)) {
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
