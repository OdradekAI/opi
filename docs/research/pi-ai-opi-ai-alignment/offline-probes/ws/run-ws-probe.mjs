import net from 'node:net';
import { createHash } from 'node:crypto';
import { deflateRawSync, constants } from 'node:zlib';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const rustMode = process.argv[2] === '--rust';
const executables = process.argv.slice(rustMode ? 3 : 2);
if (!executables.length) throw new Error('Pass exact local Node/Bun executable paths; this harness never installs anything');
const TEXT = 'offline-wire-canary';
function frame(data, opcode = 1, compressed = false) {
  const body = Buffer.isBuffer(data) ? data : Buffer.from(data);
  let header;
  if (body.length < 126) header = Buffer.from([0x80 | (compressed ? 0x40 : 0) | opcode, body.length]);
  else if (body.length <= 65535) { header = Buffer.alloc(4); header[0] = 0x80 | (compressed ? 0x40 : 0) | opcode; header[1] = 126; header.writeUInt16BE(body.length, 2); }
  else { header = Buffer.alloc(10); header[0] = 0x80 | (compressed ? 0x40 : 0) | opcode; header[1] = 127; header.writeBigUInt64BE(BigInt(body.length), 2); }
  return Buffer.concat([header, body]);
}
const cases = [
  { id: 'baseline-101-tail', note: '101 and first text frame written together' },
  { id: 'connection-token-list', connection: 'keep-alive, Upgrade' },
  { id: 'headers-total-124', extraCount: 121 },
  { id: 'headers-total-125', extraCount: 122 },
  { id: 'headers-total-140', extraCount: 137 },
  { id: 'header-value-16000', padding: 16000 },
  { id: 'header-value-20000', padding: 20000 },
  { id: 'header-value-66000', padding: 66000 },
  { id: 'tail-text-70000', textLength: 70000 },
  { id: 'invalid-accept', invalidAccept: true },
  { id: 'unsolicited-extension', extension: 'x-unrequested' },
  { id: 'unsolicited-subprotocol', subprotocol: 'unrequested' },
  { id: 'pmd-response-plain-frame', extension: 'permessage-deflate' },
  { id: 'pmd-response-compressed-frame', extension: 'permessage-deflate', compressed: true },
  { id: 'rsv1-without-extension', compressed: true },
];
function response(c, requestHeaders) {
  const key = requestHeaders['sec-websocket-key'];
  const accept = c.invalidAccept ? 'invalid-accept' : createHash('sha1').update(key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
  const headers = ['HTTP/1.1 101 Switching Protocols', 'Upgrade: websocket', `Connection: ${c.connection ?? 'Upgrade'}`, `Sec-WebSocket-Accept: ${accept}`];
  for (let i = 0; i < (c.extraCount ?? 0); i++) headers.push(`X-Extra-${i}: x`);
  if (c.padding) headers.push(`X-Padding: ${'x'.repeat(c.padding)}`);
  if (c.extension) headers.push(`Sec-WebSocket-Extensions: ${c.extension}`);
  if (c.subprotocol) headers.push(`Sec-WebSocket-Protocol: ${c.subprotocol}`);
  let body = Buffer.from(c.textLength ? 'x'.repeat(c.textLength) : TEXT);
  if (c.compressed) body = deflateRawSync(body, { flush: constants.Z_SYNC_FLUSH, finishFlush: constants.Z_SYNC_FLUSH }).subarray(0, -4);
  const head = Buffer.from(headers.join('\r\n') + '\r\n\r\n');
  return { bytes: Buffer.concat([head, frame(body, 1, c.compressed)]), headerBytes: head.length, headerCount: headers.length - 1 };
}
const results = [];
for (const executable of executables) {
  const executableHash = createHash('sha256').update(fs.readFileSync(executable)).digest('hex');
  for (const mode of rustMode ? ['rust-native'] : ['pi-headers', 'standard']) for (const c of cases) {
    const wire = { requests: [], responses: [], tcpWriteCalls: 0 };
    const sockets = new Set();
    const server = net.createServer(socket => {
      sockets.add(socket); socket.on('error', () => {}); socket.on('close', () => sockets.delete(socket));
      let request = Buffer.alloc(0), sent = false;
      socket.on('data', bytes => {
        if (sent) return;
        request = Buffer.concat([request, bytes]);
        const end = request.indexOf('\r\n\r\n');
        if (end < 0) return;
        sent = true;
        const requestText = request.subarray(0, end + 4).toString();
        const requestHeaders = {};
        for (const line of requestText.split('\r\n').slice(1)) { const p = line.indexOf(':'); if (p > 0) requestHeaders[line.slice(0,p).toLowerCase()] = line.slice(p+1).trim(); }
        wire.requests.push({ text: requestText, sha256: createHash('sha256').update(request.subarray(0,end+4)).digest('hex'), headerCanary: requestHeaders['x-opi-offline-canary'] ?? null, authorization: requestHeaders.authorization ?? null, pmdOffer: requestHeaders['sec-websocket-extensions'] ?? null });
        const r = response(c, requestHeaders);
        wire.responses.push({ headerBytes: r.headerBytes, headerCount: r.headerCount, bytes: r.bytes.length, sha256: createHash('sha256').update(r.bytes).digest('hex') });
        socket.write(r.bytes); wire.tcpWriteCalls++;
        setTimeout(() => socket.destroy(), 700).unref();
      });
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const url = `ws://127.0.0.1:${server.address().port}/offline/${c.id}`;
    const runtimeArgs = /bun\.exe$/i.test(executable) ? ['--no-install'] : [];
    const childArgs = rustMode ? [url] : [...runtimeArgs, path.join(here, 'ws-client.mjs'), url, mode];
    const child = spawn(executable, childArgs, { stdio: ['ignore', 'pipe', 'pipe'], env: { SystemRoot: process.env.SystemRoot, PATH: path.dirname(executable), TEMP: process.env.TEMP } });
    let stdout = '', stderr = '';
    child.stdout.on('data', x => stdout += x); child.stderr.on('data', x => stderr += x);
    const containment = setTimeout(() => child.kill(), 4000);
    const exit = await new Promise(resolve => child.on('close', (code, signal) => resolve({ code, signal })));
    clearTimeout(containment);
    for (const socket of sockets) socket.destroy();
    await new Promise(resolve => server.close(resolve));
    let client; try { client = JSON.parse(stdout.trim()); } catch { client = { parseError: true, stdout }; }
    const classification = exit.code === 0 ? client.outcome : client.messages?.length ? 'message-delivered-then-process-failed' : 'process-failed-no-result';
    const row = { case: c, executable, executableSha256: executableHash, command: [executable, ...childArgs], mode, classification, client, wire, exit, stderr };
    results.push(row);
    console.log(JSON.stringify({ executable: path.basename(path.dirname(executable)), case: c.id, mode, outcome: client.outcome, opened: client.opened, messageLengths: client.messages?.map(x=>x.length), canary: wire.requests[0]?.headerCanary, pmdOffer: wire.requests[0]?.pmdOffer }));
  }
}
fs.writeFileSync(path.join(here, rustMode ? 'ws-rust-loopback-results.json' : 'ws-node-bun-results.json'), JSON.stringify({ scope: rustMode ? 'direct-TTW-client_async-not-opi-provider' : 'global-constructor-probes-not-public-provider', isolation: 'Only 127.0.0.1; scrubbed child env; fake credential; no npm packages', note: '1500ms client / 4000ms process timeouts are experiment containment, not SDK limits. A server write does not determine client read boundaries.', results }, null, 2) + '\n');
// Fixed synthetic handshake bytes permit deterministic supplied-reader testing.
const synthetic = [];
for (const c of cases) {
  const r = response(c, { 'sec-websocket-key': 'dGhlIHNhbXBsZSBub25jZQ==' });
  synthetic.push({ id: c.id, hex: r.bytes.toString('hex'), headerBytes: r.headerBytes, headerCount: r.headerCount, chunk: 1_000_000 });
}
synthetic.push({ ...synthetic[0], id: 'baseline-one-byte-read', chunk: 1 });
fs.writeFileSync(path.join(here, 'synthetic-cases.json'), JSON.stringify(synthetic, null, 2) + '\n');
