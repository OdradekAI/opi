const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const read = (name) => fs.readFileSync(path.join(__dirname, name), 'utf8')
const exec = read('exec.workflow.js')
const phaseExit = read('phase-exit.workflow.js')

assert.match(exec, /decision_id/)
assert.match(exec, /decision-locality-test-stewardship/)
assert.match(exec, /unchanged consumer/)
assert.match(phaseExit, /decision_id/)
assert.match(phaseExit, /const phaseTasks = _args\.phaseTasks/)
assert.match(phaseExit, /shared-decision-closure/)
assert.match(phaseExit, /test_disposition/)

process.stdout.write('verify workflow contract tests: PASS\n')
