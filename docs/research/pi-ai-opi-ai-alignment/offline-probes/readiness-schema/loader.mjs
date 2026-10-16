import { pathToFileURL } from 'node:url';
import { resolve as resolvePath } from 'node:path';

// Only redirects the two bare imports in the exact Pi utility and diagnostic
// TypeBox imports. No Pi or TypeBox algorithm is copied or changed.
export async function resolve(specifier, context, nextResolve) {
  if (specifier === 'typebox' || specifier.startsWith('typebox/')) {
    const suffix = specifier === 'typebox' ? 'index.mjs' : `${specifier.slice(8)}/index.mjs`;
    return { url: pathToFileURL(resolvePath(process.env.PROBE_TYPEBOX_ROOT, 'build', suffix)).href, shortCircuit: true };
  }
  return nextResolve(specifier, context);
}
