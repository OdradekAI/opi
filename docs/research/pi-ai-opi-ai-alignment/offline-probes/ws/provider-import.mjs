import { pathToFileURL } from 'node:url';
if (!process.argv[2]) throw new Error('Pass the pinned Pi snapshot packages/ai/src/api/openai-codex-responses.ts path');
try {
  const m = await import(pathToFileURL(process.argv[2]).href);
  console.log(JSON.stringify({ imported: true, exports: Object.keys(m) }));
} catch (e) {
  console.log(JSON.stringify({ imported: false, error: e.message, code: e.code, stack: e.stack }));
  process.exitCode = 1;
}
