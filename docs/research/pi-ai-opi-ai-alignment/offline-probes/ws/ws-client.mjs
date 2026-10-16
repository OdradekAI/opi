import { createHash } from 'node:crypto';
const [url, mode] = process.argv.slice(2);
const ctorText = String(globalThis.WebSocket);
const result = {
  layer: 'global-constructor-only-not-public-provider',
  runtime: process.versions.bun ? 'bun' : 'node', version: process.version,
  mode, opened: false, messages: [], errors: [],
  constructorSha256: createHash('sha256').update(ctorText).digest('hex'),
};
let socket;
let settled = false;
function finish(outcome) {
  if (settled) return;
  settled = true;
  result.outcome = outcome;
  if (socket) { result.protocol = socket.protocol; result.extensions = socket.extensions; }
  console.log(JSON.stringify(result));
  try { socket?.close(); } catch {}
  setTimeout(() => process.exit(0), 20);
}
const timer = setTimeout(() => finish('harness-timeout'), 1500);
timer.unref?.();
try {
  // Exactly the Pi source call shape for pi-headers. This is a constructor probe,
  // not an import/execution of Pi's public provider and not its Bun proxy wrapper.
  socket = mode === 'pi-headers'
    ? new globalThis.WebSocket(url, { headers: { Authorization: 'Bearer offline-fake-token', 'X-Opi-Offline-Canary': 'pi130-canary' } })
    : new globalThis.WebSocket(url);
  socket.addEventListener('open', () => { result.opened = true; });
  socket.addEventListener('message', event => {
    result.messages.push(typeof event.data === 'string' ? event.data : `binary:${event.data?.byteLength ?? event.data?.size}`);
    finish('message');
  });
  socket.addEventListener('error', event => {
    result.errors.push({ message: event.message ?? null, error: event.error?.message ?? null });
  });
  socket.addEventListener('close', event => {
    result.close = { code: event.code, reason: event.reason, wasClean: event.wasClean };
    finish(result.opened ? 'closed-after-open' : 'rejected-before-open');
  });
} catch (error) {
  result.errors.push({ name: error.name, message: error.message });
  finish('constructor-threw');
}
