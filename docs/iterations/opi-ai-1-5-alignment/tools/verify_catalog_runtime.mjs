// Load only the generated catalog module, not provider factories or live clients.
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

export function compareRuntimeCatalog(runtime, catalog) {
  const types = { MODELS: 'chat', IMAGE_MODELS: 'image', CLASSIFIER_MODELS: 'classifier' };
  const providers = Object.keys(catalog).sort();
  const errors = [];
  let count = 0;
  for (const [name, type] of Object.entries(types)) {
    const exported = runtime[name];
    if (!exported || !isDeepStrictEqual(Object.keys(exported).sort(), providers)) {
      errors.push(`${name}: provider identities differ`);
      continue;
    }
    for (const provider of providers) {
      const expected = catalog[provider].filter((model) => model.type === type);
      const models = exported[provider];
      const ids = expected.map((model) => model.id).sort();
      if (!models || new Set(ids).size !== ids.length
        || !isDeepStrictEqual(Object.keys(models).sort(), ids)) {
        errors.push(`${name}/${provider}: model identities differ`);
        continue;
      }
      for (const model of expected) {
        if (!isDeepStrictEqual(models[model.id], model)) errors.push(`${name}/${provider}/${model.id}: model value differs`);
      }
      count += expected.length;
    }
  }
  if (count !== Object.values(catalog).reduce((total, models) => total + models.length, 0)) {
    errors.push('catalog includes unrepresented or missing model types');
  }
  return { status: errors.length ? 'fail' : 'pass', providers: providers.length, models: count, errors };
}

async function main(args) {
  if (args.length !== 2) throw new Error('usage: verify_catalog_runtime.mjs <snapshot> <catalog.json>');
  const [snapshot, catalogPath] = args;
  const runtime = await import(pathToFileURL(join(resolve(snapshot), 'packages/ai/dist/models.generated.js')).href);
  const result = compareRuntimeCatalog(runtime, JSON.parse(readFileSync(catalogPath, 'utf8')));
  console.log(JSON.stringify(result));
  if (result.status !== 'pass') process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => { console.error(error.stack); process.exitCode = 1; });
}
