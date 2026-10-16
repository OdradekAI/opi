// Historical payload executed without --no-install. DO NOT execute this file.
// It created an uncontrolled Bun auto-resolution cache at the workspace root.
try {
  const m = await import('file:///D:/Luiz/Odradek/opi/.repo/pi-1.3.0/packages/ai/src/api/openai-codex-responses.ts');
  console.log(JSON.stringify({ imported: true, exports: Object.keys(m) }));
} catch (e) {
  console.log(JSON.stringify({ imported: false, error: e.message, code: e.code, stack: e.stack }));
  process.exitCode = 1;
}
