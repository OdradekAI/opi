import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { compareRuntimeCatalog } from './verify_catalog_runtime.mjs';

const model = { type: 'chat', id: 'm', api: 'wire', provider: 'probe', cost: { input: 0.035 } };
const catalog = { probe: [model] };
const runtime = () => ({ MODELS: { probe: { m: structuredClone(model) } }, IMAGE_MODELS: { probe: {} }, CLASSIFIER_MODELS: { probe: {} } });

test('runtime comparison retains values, identities and unsupported types', () => {
  assert.equal(compareRuntimeCatalog(runtime(), catalog).status, 'pass');
  const changed = runtime();
  changed.MODELS.probe.m.cost.input = 0.006;
  assert.match(compareRuntimeCatalog(changed, catalog).errors[0], /model value differs/);
  delete changed.MODELS.probe.m;
  assert.match(compareRuntimeCatalog(changed, catalog).errors[0], /model identities differ/);
  changed.MODELS.extra = {};
  assert.match(compareRuntimeCatalog(changed, catalog).errors[0], /provider identities differ/);
  assert.equal(compareRuntimeCatalog(runtime(), { probe: [{ ...model, type: 'unknown' }] }).status, 'fail');
  assert.equal(compareRuntimeCatalog({}, catalog).status, 'fail');
});

test('runtime CLI actually loads emitted JS and rejects a wrong model', () => {
  const root = mkdtempSync(join(tmpdir(), 'opi-restored-catalog-'));
  const script = fileURLToPath(new URL('./verify_catalog_runtime.mjs', import.meta.url));
  try {
    mkdirSync(join(root, 'packages/ai/dist'), { recursive: true });
    writeFileSync(join(root, 'package.json'), '{"type":"module"}');
    const catalogPath = join(root, 'catalog.json');
    writeFileSync(catalogPath, JSON.stringify(catalog));
    const module = join(root, 'packages/ai/dist/models.generated.js');
    const write = (value) => writeFileSync(module, Object.entries(value).map(([key, item]) => `export const ${key} = ${JSON.stringify(item)};`).join('\n'));
    write(runtime());
    const command = () => spawnSync(process.execPath, [script, root, catalogPath], { encoding: 'utf8' });
    const matched = command();
    assert.equal(matched.status, 0, matched.stderr);
    assert.equal(JSON.parse(matched.stdout).models, 1);
    const wrong = runtime();
    wrong.MODELS.probe.m.cost.input = 123;
    write(wrong);
    const failed = command();
    assert.equal(failed.status, 1, failed.stderr);
    assert.equal(JSON.parse(failed.stdout).status, 'fail');
    assert.equal(spawnSync(process.execPath, [script], { encoding: 'utf8' }).status, 1);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
