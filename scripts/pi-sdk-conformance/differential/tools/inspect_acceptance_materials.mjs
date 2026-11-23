// Read-only inspection. Does not install, hydrate, build, or contact providers.
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, realpathSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

const json = (path) => JSON.parse(readFileSync(path, 'utf8'));
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const posix = (path) => path.replaceAll('\\', '/');

export function inspectDependencies(snapshot, baseline) {
  const importer = join(snapshot, 'packages/ai/dist/index.js');
  const declarations = json(join(snapshot, 'packages/ai/package.json')).dependencies;
  const locked = json(join(snapshot, 'package-lock.json')).packages;
  return Object.entries(declarations).map(([name, declared]) => {
    try {
      const entry = fileURLToPath(import.meta.resolve(name, pathToFileURL(importer).href));
      if (!statSync(entry).isFile()) throw new Error('resolved entry is not a file');
      let parent = dirname(importer);
      let directory;
      while (true) {
        const candidate = join(parent, 'node_modules', name);
        if (existsSync(join(candidate, 'package.json'))) {
          directory = candidate;
          break;
        }
        if (dirname(parent) === parent) throw new Error('no installed package manifest');
        parent = dirname(parent);
      }
      const manifest = join(directory, 'package.json');
      const installed = json(manifest);
      const real = realpathSync(directory);
      if (!resolve(entry).startsWith(`${real}\\`) && !resolve(entry).startsWith(`${real}/`)) {
        throw new Error('entry resolution disagrees with installed package location');
      }
      const lockPath = posix(relative(snapshot, directory));
      const frozen = baseline.upstream.direct_dependencies[name];
      const record = locked[lockPath];
      const matches = installed.name === name && installed.version === declared
        && declared === frozen.declared && record !== undefined
        && isDeepStrictEqual(record, frozen.lock_records[lockPath]);
      return {
        name, declared, installed: installed.version, entry, directory: posix(relative(snapshot, directory)),
        real_directory: real, manifest_sha256: hash(readFileSync(manifest)),
        entry_sha256: hash(readFileSync(entry)), lock_record: record,
        status: matches ? 'matched-frozen-direct-record' : 'mismatch',
      };
    } catch (error) {
      return { name, declared, status: 'blocked', error: error.message };
    }
  });
}

export function compareProviderGroups(expected, actual) {
  const flatten = (groups) => new Map(Object.entries(groups).flatMap(([api, models]) =>
    Object.entries(models).map(([id, value]) => [`${api}/${id}`, value])));
  const left = flatten(expected);
  const right = flatten(actual);
  return {
    missing: [...left.keys()].filter((id) => !right.has(id)),
    extra: [...right.keys()].filter((id) => !left.has(id)),
    changed: [...left.keys()].filter((id) => right.has(id) && !isDeepStrictEqual(left.get(id), right.get(id))),
    expected_models: left.size,
    actual_models: right.size,
  };
}

export async function inspectCatalog(snapshot, catalogPath) {
  const packageRoot = join(snapshot, 'packages/ai');
  // Import only pure grouping/validation helpers, never the generator/hydrator.
  const helpers = await import(pathToFileURL(join(packageRoot, 'scripts/model-data.ts')).href);
  const catalog = json(catalogPath);
  const source = join(packageRoot, 'src/providers/data');
  const dist = join(packageRoot, 'dist/providers/data');
  const internal = {};
  const structure = helpers.readModelDataStructure(packageRoot);
  for (const [name, path] of Object.entries({ source, dist })) {
    try { helpers.validateModelDataDirectory(structure, path); internal[name] = 'pass'; }
    catch (error) { internal[name] = error.message; }
  }
  const actualProviders = readdirSync(source).filter((name) => name.endsWith('.json') && name !== '.manifest.json')
    .map((name) => name.slice(0, -5));
  const providers = [...new Set([...Object.keys(catalog), ...actualProviders])].sort();
  const comparisons = providers.map((provider) => {
    const expected = helpers.groupProviderModelData(provider, catalog[provider] ?? []).groups;
    const path = join(source, `${provider}.json`);
    const actual = existsSync(path) ? json(path) : {};
    return { provider, ...compareProviderGroups(expected, actual) };
  });
  const mismatches = comparisons.filter((row) => row.missing.length || row.extra.length || row.changed.length);
  const files = ['.manifest.json', ...actualProviders.map((provider) => `${provider}.json`)].sort();
  const distMismatch = files.filter((name) => !existsSync(join(dist, name))
    || !readFileSync(join(source, name)).equals(readFileSync(join(dist, name))));
  return {
    status: mismatches.length ? 'pinned-model-mismatch' : 'pinned-model-values-match',
    internal_validation: internal,
    pinned_models: comparisons.reduce((sum, row) => sum + row.expected_models, 0),
    actual_models: comparisons.reduce((sum, row) => sum + row.actual_models, 0),
    providers: comparisons.length,
    mismatches,
    source_dist_byte_mismatches: distMismatch,
  };
}

function sourceFiles(root) {
  return readdirSync(root, { withFileTypes: true }).flatMap((entry) => {
    const path = join(root, entry.name);
    return entry.isDirectory() ? sourceFiles(path) : [path];
  });
}

export function inspectDist(snapshot, name) {
  const root = join(snapshot, 'packages', name);
  const sourceRoot = join(root, 'src');
  const errors = [];
  let checked = 0;
  for (const source of sourceFiles(sourceRoot).filter((path) => path.endsWith('.ts') && !path.endsWith('.d.ts'))) {
    const stem = join(root, 'dist', relative(sourceRoot, source)).slice(0, -3);
    const outputs = ['.js', '.js.map', '.d.ts', '.d.ts.map'].map((extension) => stem + extension);
    for (const output of outputs) if (!existsSync(output)) errors.push(`missing ${posix(relative(snapshot, output))}`);
    if (!existsSync(`${stem}.js.map`)) continue;
    const map = json(`${stem}.js.map`);
    if (map.sources?.length !== 1 || map.sourcesContent?.length !== 1
      || map.sourcesContent[0] !== readFileSync(source, 'utf8')
      || resolve(dirname(`${stem}.js.map`), map.sourceRoot ?? '', map.sources[0]) !== resolve(source)) {
      errors.push(`source map mismatch: ${posix(relative(snapshot, source))}`);
    }
    checked++;
  }
  if (!checked) errors.push('no source/output pairs');
  return { package: name, status: errors.length ? 'blocked' : 'complete-source-maps-match', checked_sources: checked, errors,
    limit: 'Observed files and embedded-source agreement do not prove reproducible emitted JS or npm package consumption.' };
}

async function main(args) {
  if (args.length !== 3) throw new Error('usage: inspect_acceptance_materials.mjs <snapshot> <baseline.json> <catalog.json>');
  const [snapshotArg, baselinePath, catalogPath] = args;
  const snapshot = resolve(snapshotArg);
  const baseline = json(baselinePath);
  const result = { node: process.version, platform: process.platform, arch: process.arch };
  result.dependencies = inspectDependencies(snapshot, baseline);
  result.dist = ['ai', 'schema', 'telemetry'].map((name) => inspectDist(snapshot, name));
  result.catalog = await inspectCatalog(snapshot, catalogPath);
  console.log(JSON.stringify(result));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => { console.error(error.stack); process.exitCode = 1; });
}
