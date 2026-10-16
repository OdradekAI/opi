// Synthetic records exercise original Pi helpers; they are not a Pi catalog artifact.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join, relative, isAbsolute, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
const [pi, scratch] = process.argv.slice(2).map(value => resolve(value));
const inside = (root, path) => {
  const rel = relative(root, path);
  return rel === '' || (rel !== '..' && !rel.startsWith('..' + sep) && !isAbsolute(rel));
};
assert(scratch && !inside(resolve(pi, '../..'), scratch), 'scratch must be outside the source workspace');
const md = await import(pathToFileURL(join(pi, 'packages/ai/scripts/model-data.ts')));
const { hydrateModelCatalog } = await import(pathToFileURL(join(pi, 'packages/ai/scripts/hydrate-model-catalog.ts')));
const flat = await import(pathToFileURL(join(pi, 'packages/ai/src/model-catalog.ts')));
const root = join(scratch, 'synthetic-package');
const providersDir = join(root, 'src/providers');
mkdirSync(providersDir, { recursive: true });
writeFileSync(join(root, 'src/models.generated.ts'), readFileSync(join(pi, 'packages/ai/src/models.generated.ts')));
const ids = md.readModelDataProviderIds(root);
const catalog = Object.fromEntries(ids.map(provider => [provider, ['chat', 'image', 'classifier'].map(type => ({
  type, id: 'synthetic-shared-id', api: 'fixture-' + type, provider,
  name: 'Artificial offline record', baseUrl: 'http://127.0.0.1:1', input: ['text'],
  ...(type === 'image' ? { output: ['image'] } : { contextWindow: 8192 }),
  ...(type === 'chat' ? { reasoning: false, maxTokens: 1024 } : {}),
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
}))]));
const catalogPath = join(scratch, 'synthetic-models.all.json');
const save = () => writeFileSync(catalogPath, JSON.stringify(catalog));
save();
hydrateModelCatalog(root, catalogPath);
const data = join(providersDir, 'data');
const structure = Object.fromEntries(ids.map(id => [id, md.groupProviderModelData(id, catalog[id]).structure]));
const cases = [];
const positive = name => { md.validateModelDataDirectory(structure, data); cases.push({name, result:'accepted'}); };
const negative = (name, body) => {
  let error;
  try { body(); } catch (e) { error = String(e.message); }
  assert(error, name + ' must reject');
  cases.push({name, result:'rejected', error});
};
positive('126-synthetic-records-42-providers-three-operation-identities');
const first = ids[0];
const file = join(data, first + '.json');
const bytes = readFileSync(file);
const groups = JSON.parse(bytes);
for (const [type, fn] of [['chat',flat.flattenChatModelCatalog], ['image',flat.flattenImageModelCatalog], ['classifier',flat.flattenClassifierModelCatalog]]) {
  const result = fn(first, groups);
  assert.deepEqual(Object.keys(result), ['synthetic-shared-id']);
  assert.equal(result['synthetic-shared-id'].type, type);
}
cases.push({name:'operation-qualified-flatten-same-id',result:'accepted'});
writeFileSync(file, Buffer.concat([bytes, Buffer.from(' ')]));
negative('tampered-bytes-reject-hash', () => md.validateModelDataDirectory(structure, data));
writeFileSync(file, bytes);
const manifest = join(data, '.manifest.json');
const manifestBytes = readFileSync(manifest);
const value = JSON.parse(manifestBytes);
value.schemaVersion = 5;
writeFileSync(manifest, JSON.stringify(value));
negative('wrong-manifest-schema-version', () => md.validateModelDataDirectory(structure, data));
writeFileSync(manifest, manifestBytes);
const omitted = catalog[first];
delete catalog[first]; save();
negative('missing-provider-no-installation', () => hydrateModelCatalog(root, catalogPath));
catalog[first] = omitted;
catalog[first].push({...omitted[0]}); save();
negative('duplicate-operation-id', () => hydrateModelCatalog(root, catalogPath));
catalog[first].pop(); save();
hydrateModelCatalog(root, catalogPath);
assert.deepEqual(readFileSync(manifest), manifestBytes);
assert.deepEqual(readFileSync(file), bytes);
positive('deterministic-rehydration-same-bytes');
console.log(JSON.stringify({scope:'original Pi helpers, synthetic data only; no upstream artifact or remote support proof', schemaVersion:md.MODEL_DATA_SCHEMA_VERSION,providers:ids.length,records:ids.length*3,catalog_sha256:createHash('sha256').update(readFileSync(catalogPath)).digest('hex'),manifest_sha256:createHash('sha256').update(manifestBytes).digest('hex'),cases}));
