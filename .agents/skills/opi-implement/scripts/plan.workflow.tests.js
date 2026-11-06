const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const test = require('node:test')
const { runWorkflow, failureCases } = require('./verify-workflows.tests.js')

failureCases('plan')

test('plan: domain context sent to reviewers resolves to a maintained file', async () => {
  const { calls } = await runWorkflow('plan', { count: 0 })
  const lenses = calls.filter(({ config }) => config.phase === 'Lens audit')
  assert.equal(lenses.length, 5)
  for (const { prompt } of lenses) {
    const match = prompt.match(/^Read (docs\/\S+) and the applicable /m)
    assert.ok(match, 'reviewer needs a domain context source')
    assert.ok(fs.statSync(path.resolve(__dirname, '../../../..', match[1])).isFile(), match[1])
  }
})

test('plan: verifier batches remain bounded and preserve all findings', async () => {
  const { result, parallelSizes } = await runWorkflow('plan', { count: 12 })
  assert.deepEqual(parallelSizes, [5, 5, 5, 2])
  assert.equal(result.graph_findings.length, 12)
  assert.equal(result.verdict, 'GRAPH_REVISION_REQUIRED')
  assert.deepEqual(result.resource_summary, {
    lens_agents: 5, verify_agents: 12, synthesis_agents: 1, total_agents: 18, max_parallel_agents: 5,
  })
})
