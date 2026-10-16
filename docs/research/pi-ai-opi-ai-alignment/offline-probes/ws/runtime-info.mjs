import { createHash } from 'node:crypto';
console.log(JSON.stringify({ execPath: process.execPath, version: process.version, versions: process.versions, websocketName: globalThis.WebSocket.name, websocketConstructorSha256: createHash('sha256').update(String(globalThis.WebSocket)).digest('hex') }));
