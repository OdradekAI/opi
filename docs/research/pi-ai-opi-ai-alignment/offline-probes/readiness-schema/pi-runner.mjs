import { readFileSync, realpathSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { Compile } from 'typebox/compile';

const { validateToolArguments } = await import(pathToFileURL(resolve(process.env.PROBE_PI_ROOT, 'packages/ai/src/utils/validation.ts')).href);
const source = readFileSync(process.argv[2], 'utf8');
const cases = source.trimEnd().split('\n').filter(Boolean).map(line => JSON.parse(line));
const selected = process.argv[3] ? cases.filter(c => c.id === process.argv[3]) : cases.filter(c => !c.containment);
console.log(JSON.stringify({ kind:'runtime', execPath:process.execPath, realExecPath:realpathSync(process.execPath), version:process.version, versions:process.versions, execArgv:process.execArgv, importedPi:'packages/ai/src/utils/validation.ts', cases:selected.length }));
for (const c of selected) {
  const input = structuredClone(c.arguments);
  const raw = JSON.stringify(input);
  const schema = structuredClone(c.schema);
  const result = {kind:'case', id:c.id, input_js:input, direct_typebox:{}, public_pi:{}};
  try {
    const validator = Compile(schema);
    result.direct_typebox.compile='ok';
    try {
      result.direct_typebox.check = validator.Check(input);
      result.direct_typebox.errors = validator.Errors(input);
    } catch(e) { result.direct_typebox.check_exception={name:e.name,message:e.message}; }
  } catch(e) { result.direct_typebox.compile_exception={name:e.name,message:e.message}; }
  try {
    const normalized = validateToolArguments({name:'probe',description:'offline',parameters:schema}, {type:'toolCall',id:'probe',name:'probe',arguments:input, ...(Object.hasOwn(c,'argumentsError')?{argumentsError:c.argumentsError}:{})});
    result.public_pi={outcome:'returned',normalized};
    try { result.public_pi.returned_value_passes_typebox = Compile(schema).Check(normalized); }
    catch(e) { result.public_pi.returned_check_exception={name:e.name,message:e.message}; }
  } catch(e) { result.public_pi={outcome:'threw',exception:{name:e.name,message:e.message}}; }
  result.public_pi.raw_unchanged = raw === JSON.stringify(input);
  console.log(JSON.stringify(result));
}
