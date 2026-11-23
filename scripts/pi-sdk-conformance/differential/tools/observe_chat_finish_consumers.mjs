import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { pathToFileURL } from 'node:url';

const [snapshot, receipt] = process.argv.slice(2);
assert.ok(snapshot && receipt && process.argv.length === 4, 'snapshot and paired receipt required');
const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { Harness, LiveDoc, MemoryStorage, ToolResultEntry, defineTool },
  { nativeTimer },
  { allEntries, chatSetup, textOf },
  { addTool },
  { context, documentValues, followCommits },
  { obj, str, int },
  { fauxAssistantMessage, fauxText },
] = await Promise.all([
  from('packages/durable/src/index.ts'),
  from('packages/durable/src/timers.ts'),
  from('packages/durable/test/chat-support.ts'),
  from('packages/durable/test/harness-support.ts'),
  from('packages/durable/test/session-support.ts'),
  from('packages/schema/src/index.ts'),
  from('packages/ai/dist/providers/faux.js'),
]);

async function until(promise, checkpoint) {
  const controller = new AbortController();
  try {
    return await Promise.race([
      promise,
      sleep(5000, undefined, { signal: controller.signal }).then(() => assert.fail('no progress at ' + checkpoint)),
    ]);
  } finally { controller.abort(); }
}

function message(row) {
  const blocks = row.content.map(({ raw, ...block }) => structuredClone(block));
  const result = fauxAssistantMessage(blocks, {
    stopReason: row.stop === 'tool_use' ? 'toolUse' : row.stop,
    timestamp: 1,
  });
  result.usage.input = row.input_tokens;
  result.usage.output = row.output_tokens;
  result.usage.totalTokens = row.input_tokens + row.output_tokens;
  if (row.raw_stop !== null) result.rawStopReason = row.raw_stop;
  if (row.response_id !== null) result.responseId = row.response_id;
  return result;
}

function viewMessage(item) {
  return {
    role: item.role, content: structuredClone(item.content), stop: item.stopReason,
    input_tokens: item.usage?.input, output_tokens: item.usage?.output,
    error_message: item.errorMessage ?? null,
  };
}

async function durable(side, trace) {
  const setup = chatSetup();
  const executed = [];
  const progress = [];
  const reports = [];
  const reached = Promise.withResolvers();
  const release = Promise.withResolvers();
  const reported = Promise.withResolvers();
  const timer = Promise.withResolvers();
  const committed = Promise.withResolvers();
  const boundary = trace.checkpoints.after_usage;
  const limit = boundary.events.length;
  assert.deepEqual(boundary.events.map((event) => event.kind), trace.final.events.slice(0, limit).map((event) => event.kind));
  const projectedEvents = [...boundary.events, ...trace.final.events.slice(limit)];
  let dispatches = 0;
  let completion;
  let replayed;
  let initialSourceFinished = false;
  const interval = 731;

  addTool(setup.registry, defineTool({
    name: 'record', description: 'Records fixture arguments', parameters: obj({ a: str(), n: int() }),
    async execute(args) {
      executed.push(structuredClone(args));
      return { output: [{ type: 'text', text: 'ok' }] };
    },
  }));
  setup.faux.setResponses([(ctx) => {
    replayed = ctx.messages.find((entry) => entry.role === 'assistant')?.content;
    return fauxAssistantMessage([fauxText('settled')]);
  }]);
  const models = new Proxy(setup.models, {
    get(target, property) {
      if (property === 'streamSimple') return (model, ctx, options) => {
        if (++dispatches > 1) return target.streamSimple(model, ctx, options);
        const final = message(trace.final.result);
        return {
          async *[Symbol.asyncIterator]() {
            try {
              for (const [offset, row] of projectedEvents.entries()) {
                const partial = message(row.message);
                if (row.kind === 'done') yield { type: 'done', reason: final.stopReason, message: final };
                else if (row.kind === 'error') yield { type: 'error', reason: final.stopReason, error: final };
                else yield {
                  type: row.kind, contentIndex: row.index, delta: row.delta, content: row.content,
                  toolCall: row.kind === 'toolcall_end' ? partial.content[row.index] : undefined,
                  partial,
                };
                if (offset + 1 === limit) {
                  if (boundary.eof) return;
                  reached.resolve({ kind: 'held' });
                  await release.promise;
                }
              }
            } finally { initialSourceFinished = true; }
          },
          result: async () => final,
        };
      };
      const value = Reflect.get(target, property, target);
      return typeof value === 'function' ? value.bind(target) : value;
    },
  });
  const harness = await Harness.open(new MemoryStorage(), {
    models, registry: setup.registry,
    settings: { progress: { partialIntervalMs: interval } },
    onReport: ({ error }) => { reports.push(String(error)); reported.resolve(error); },
    setTimer(callback, delay, options) {
      if (delay !== interval) return nativeTimer(callback, delay, options);
      let active = true;
      timer.resolve(() => { if (active) callback(); });
      return () => { active = false; };
    },
  }, context);
  try {
    const root = await harness.root(context, { agent: { model: { provider: 'faux', modelId: 'faux-1' } } });
    followCommits(harness, (publication) => {
      for (const change of documentValues(publication)) {
        const partial = change.value?.generation?.message;
        if (change.record.kind === LiveDoc.definition.kind && partial) {
          const value = viewMessage(partial);
          progress.push(value);
          committed.resolve(value);
        }
      }
    });
    harness.resume();
    const submission = await root.submit({ type: 'input', content: 'fixture' }, context);
    const outcome = submission.wait(context).then((result) => { completion = result; return result; });
    const first = await until(Promise.race([
      reached.promise,
      outcome.then((result) => ({ kind: 'completed', result })),
      reported.promise.then((error) => { throw error; }),
    ]), side + ' ' + trace.case + ' usage boundary');
    if (first.kind === 'held') {
      const flush = await until(timer.promise, 'Durable progress timer');
      flush();
      await until(committed.promise, 'Durable held progress publication');
    }
    const beforeEntries = await allEntries(root);
    const before = {
      source_trace_public_eof: boundary.eof,
      projection_generator_finished: initialSourceFinished,
      boundary_winner: first.kind,
      completion: completion ?? null,
      executed: structuredClone(executed), dispatches,
      stored_assistants: beforeEntries.flatMap((entry) => entry.model ?? []).filter((entry) => entry.role === 'assistant').map(viewMessage),
      progress: structuredClone(progress),
    };
    assert.equal(before.boundary_winner, boundary.eof ? 'completed' : 'held');
    assert.equal(before.projection_generator_finished, boundary.eof);
    assert.equal(before.executed.length, boundary.eof ? 1 : 0);
    assert.equal(before.dispatches, boundary.eof ? 2 : 1);
    assert.equal(before.stored_assistants.length, boundary.eof ? 2 : 0);
    if (boundary.eof) assert.equal(before.completion.status, 'done');
    else assert.equal(before.completion, null);
    release.resolve();
    const result = await until(outcome, side + ' ' + trace.case + ' final outcome');
    const entries = await allEntries(root);
    const answers = entries.flatMap((entry) => entry.model ?? []).filter((entry) => entry.role === 'assistant');
    const results = entries.filter((entry) => ToolResultEntry.is(entry));
    const expectedSuccess = trace.final.result.stop === 'tool_use';
    assert.equal(executed.length, expectedSuccess ? 1 : 0);
    assert.equal(dispatches, expectedSuccess ? 2 : 1);
    assert.equal(results.length, expectedSuccess ? 1 : 0);
    if (expectedSuccess) {
      assert.deepEqual(executed, [{ a: 'ok', n: 2 }]);
      assert.equal(result.status, 'done');
      assert.equal(textOf(answers.at(-1)), 'settled');
      assert.deepEqual(answers[0].content, replayed);
    } else {
      assert.equal(result.status, 'unanswered');
      assert.equal(result.reason, 'model_error');
    }
    return {
      side, case: trace.case, after_usage: before,
      final: {
        outcome: result, executed, dispatches, stored_assistants: answers.map(viewMessage),
        tool_results: results.map((entry) => textOf(entry.model[0])), replayed: replayed ?? null,
        answer: expectedSuccess ? textOf(answers.at(-1)) : null,
        projection_generator_finished: initialSourceFinished, reports,
      },
    };
  } finally {
    release.resolve();
    await harness.close(context);
  }
}

const paired = JSON.parse(await readFile(receipt, 'utf8'));
const inputs = { pi: paired.pi_runs[0].trace, opi: paired.opi_runs[0].trace };
const observations = [];
for (const caseName of ['trailer', 'error_after_usage', 'cancel_after_usage']) {
  for (const side of ['pi', 'opi']) {
    const trace = inputs[side].find((item) => item.case === caseName);
    assert.ok(trace, side + ' ' + caseName);
    observations.push(await durable(side, trace));
  }
}
const result = {
  method: 'Recorded SDK event/message projections fed to the frozen real Pi Durable Harness; this is not Pi invoking Rust through a live language bridge.',
  limits: [
    'The source receipts omit diagnostics and source identity/aliasing; the projection uses Faux model identity and preserves recorded content, stop reason, usage, and event sequence.',
    'The cancel trace models a provider-aborted response; the Durable application cancellation token is not cancelled.',
    'The timer is explicitly flushed only at the held checkpoint; this does not measure real-time latency or scheduling.',
  ],
  snapshot, observations,
};
console.log(JSON.stringify(result));
