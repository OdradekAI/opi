import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { compareProviderGroups, inspectDependencies, inspectDist } from './inspect_acceptance_materials.mjs';

function write(path, value) {
  mkdirSync(join(path, '..'), { recursive: true });
  writeFileSync(path, typeof value === 'string' ? value : JSON.stringify(value));
}

function fixture(run) {
  const root = mkdtempSync(join(tmpdir(), 'opi-materials-'));
  try { run(root); } finally { rmSync(root, { recursive: true, force: true }); }
}

test('inspector CLI rejects missing inputs with a nonzero exit', () => {
  const script = fileURLToPath(new URL('./inspect_acceptance_materials.mjs', import.meta.url));
  const result = spawnSync(process.execPath, [script], { encoding: 'utf8' });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /usage: inspect_acceptance_materials/);
});

test('nested AI resolution wins over a plausible root package', () => fixture((root) => {
  write(join(root, 'packages/ai/dist/index.js'), '');
  write(join(root, 'packages/ai/package.json'), { dependencies: { probe: '2.0.0' } });
  for (const [location, version] of [['node_modules/probe', '1.0.0'], ['packages/ai/node_modules/probe', '2.0.0']]) {
    write(join(root, location, 'package.json'), { name: 'probe', version, main: 'index.js' });
    write(join(root, location, 'index.js'), 'export {};');
  }
  const record = { version: '2.0.0', integrity: 'synthetic' };
  const lockPath = 'packages/ai/node_modules/probe';
  write(join(root, 'package-lock.json'), { packages: { [lockPath]: record } });
  const baseline = { upstream: { direct_dependencies: { probe: { declared: '2.0.0', lock_records: { [lockPath]: record } } } } };
  const first = inspectDependencies(root, baseline)[0];
  assert.equal(first.status, 'matched-frozen-direct-record');
  assert.equal(first.directory, lockPath);
  write(join(root, lockPath, 'package.json'), { name: 'probe', version: '3.0.0', main: 'index.js' });
  assert.equal(inspectDependencies(root, baseline)[0].status, 'mismatch');
  rmSync(join(root, lockPath, 'index.js'));
  assert.equal(inspectDependencies(root, baseline)[0].status, 'blocked');
}));

test('changed costs, missing identities and extras are not normalized away', () => {
  const expected = { chat: { 'chat:m': { type: 'chat', id: 'm', cost: { input: 0.035 } } } };
  const changed = structuredClone(expected);
  changed.chat['chat:m'].cost.input = 0.006;
  assert.deepEqual(compareProviderGroups(expected, changed).changed, ['chat/chat:m']);
  assert.deepEqual(compareProviderGroups(expected, {}).missing, ['chat/chat:m']);
  assert.deepEqual(compareProviderGroups({}, expected).extra, ['chat/chat:m']);
  assert.deepEqual(compareProviderGroups(expected, structuredClone(expected)).changed, []);
});

test('dist completeness and embedded-source consistency are separate checks', () => fixture((root) => {
  const source = 'export const value = 1;\n';
  write(join(root, 'packages/ai/src/index.ts'), source);
  for (const suffix of ['.js', '.d.ts', '.d.ts.map']) write(join(root, `packages/ai/dist/index${suffix}`), '');
  const mapPath = join(root, 'packages/ai/dist/index.js.map');
  write(mapPath, { sources: ['../src/index.ts'], sourcesContent: [source] });
  assert.equal(inspectDist(root, 'ai').status, 'complete-source-maps-match');
  write(mapPath, { sources: ['../src/index.ts'], sourcesContent: ['different source'] });
  assert.equal(inspectDist(root, 'ai').status, 'blocked');
  rmSync(join(root, 'packages/ai/dist/index.d.ts'));
  assert.ok(inspectDist(root, 'ai').errors.some((error) => error.includes('missing')));
}));
