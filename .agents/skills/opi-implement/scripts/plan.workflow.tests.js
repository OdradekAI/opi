const assert = require('node:assert/strict')
const test = require('node:test')
const { runWorkflow, failureCases } = require('./verify-workflows.tests.js')

failureCases('plan')

test('plan: verifier batches remain bounded and preserve all findings', async () => {
  const { result, parallelSizes } = await runWorkflow('plan', { count: 12 })
  assert.deepEqual(parallelSizes, [5, 5, 5, 2])
  assert.equal(result.graph_findings.length, 12)
  assert.equal(result.verdict, 'GRAPH_REVISION_REQUIRED')
  assert.deepEqual(result.resource_summary, {
    lens_agents: 5, verify_agents: 12, synthesis_agents: 1, total_agents: 18, max_parallel_agents: 5,
  })
})
