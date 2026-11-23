// Frozen Pi's final parser, real tool-dispatch seam and disk session manager.
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [snapshot, rustInput, sessionDir] = process.argv.slice(2);
assert.ok(snapshot && rustInput && sessionDir && process.argv.length === 5);
const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ parseToolCallArguments }, { runToolCall }, { SessionManager }, { fauxAssistantMessage }] = await Promise.all([
  from('packages/ai/src/utils/json-parse.ts'),
  from('packages/agent/src/agent-loop.ts'),
  from('packages/coding-agent/src/core/session-manager.ts'),
  from('packages/ai/dist/providers/faux.js'),
]);
const bits = (n) => { const b = Buffer.alloc(8); b.writeDoubleBE(n); return b.toString('hex'); };
const input = JSON.parse(await readFile(rustInput, 'utf8'));
assert.equal(input.final_numbers.length, 10);
const final_numbers = input.final_numbers.map(({ raw, bits: rustBits, serialized }) => {
  const parsed = parseToolCallArguments(raw);
  assert.equal(parsed.argumentsError, undefined);
  assert.equal(bits(parsed.arguments.n), rustBits);
  // A real JavaScript downstream consumer of the Rust serialization.
  assert.equal(bits(JSON.parse(serialized).n), rustBits);
  return { raw, bits: bits(parsed.arguments.n), rust_serialization_bits: bits(JSON.parse(serialized).n) };
});
const raw = `{"n":1${'0'.repeat(64)},"password":"numeric-canary"}`;
const parsed = parseToolCallArguments(raw);
assert.equal(bits(parsed.arguments.n), '4d384f03e93ff9f5');
const rows = [];
let executions = 0;
const tool = {
  name: 'record_number', label: 'Record number', description: 'Local numeric fixture',
  parameters: { type: 'object', properties: { n: { type: 'number', minimum: 1e64, maximum: 1e64 }, password: { type: 'string' } }, required: ['n'], additionalProperties: false },
  async execute(id, args) { executions++; rows.push({ stage: 'execute', id, bits: bits(args.n) }); return { content: [], details: {} }; },
};
const call = { type: 'toolCall', id: 'initial', name: tool.name, ...parsed };
const assistant = fauxAssistantMessage([call], { stopReason: 'toolUse', timestamp: 1 });
async function dispatch(toolCall, messages) {
  const result = await runToolCall(toolCall, {
    tools: [tool], assistantMessage: assistant, context: { messages, tools: [tool] },
    beforeToolCall: ({ args }) => { rows.push({ stage: 'hook', id: toolCall.id, bits: bits(args.n) }); },
  });
  assert.equal(result.isError, false);
}
await dispatch(call, []);
await mkdir(sessionDir, { recursive: false });
const session = SessionManager.create(resolve(sessionDir), resolve(sessionDir));
session.appendMessage(assistant);
const file = session.getSessionFile();
const before = await readFile(file);
const restored = SessionManager.open(file, resolve(sessionDir)).buildSessionContext().messages;
assert.deepEqual(await readFile(file), before);
const storedCall = restored.flatMap(m => m.content ?? []).find(c => c.type === 'toolCall');
assert.equal(bits(storedCall.arguments.n), '4d384f03e93ff9f5');
assert.equal(executions, 1, 'restoring history must not execute it');
await dispatch({ ...storedCall, id: 'after-resume' }, restored);
assert.equal(executions, 2);
assert.ok(rows.every(row => row.bits === '4d384f03e93ff9f5'));
console.log(JSON.stringify({ final_numbers, agent: rows, session: {
  file, before_bytes: before.length, restored_bits: bits(storedCall.arguments.n),
  executions_before_resume: 1, executions_after_next_call: executions,
}, scope: 'Frozen source runToolCall and real SessionManager disk recovery; no whole Pi Agent loop or policy equivalence claimed.' }));
