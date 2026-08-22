const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

async function runWorkflow() {
  const workflowPath = path.join(__dirname, 'plan.workflow.js')
  const workflowSource = fs.readFileSync(workflowPath, 'utf8')
  assert.match(workflowSource, /shared decision identity/)
  assert.match(workflowSource, /replace-don't-layer/)
  const source = workflowSource
    .replace('export const meta =', 'const meta =')
  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor
  const execute = new AsyncFunction('args', 'phase', 'parallel', 'agent', source)

  const parallelSizes = []
  let verifierCalls = 0
  const parallel = async (tasks) => {
    parallelSizes.push(tasks.length)
    return Promise.all(tasks.map((task) => task()))
  }
  const agent = async (_prompt, options) => {
    if (options.label.startsWith('plan:')) {
      const findings = options.label === 'plan:design-lineage-placement'
        ? Array.from({ length: 12 }, (_, index) => ({
          axis: 'design-readiness',
          lens: 'design-lineage-placement',
          task_id: String(index),
          decision_id: null,
          field: 'test',
          problem: 'test finding',
          severity: 'high',
          suggested_fix: 'fix it',
          source_citation: 'docs/opi-spec.md#Test',
          confidence: 'high',
          route: 'GRAPH_REVISION_REQUIRED',
          blocking: true,
        }))
        : []
      return { findings }
    }
    if (options.label.startsWith('verify:')) {
      verifierCalls += 1
      if (verifierCalls === 1) return null
      return { accepted: true, reason: 'supported' }
    }
    return {
      summary: 'test report',
      verdict: 'GRAPH_REVISION_REQUIRED',
      independence: 'test',
      design_findings: [],
      graph_findings: [],
      flagged_for_human: [],
      rejected: [],
    }
  }

  const result = await execute(
    {
      draftTasks: [],
      sourceDesignPath: 'docs/opi-spec.md',
      phase: 'test',
      independence: 'test',
    },
    () => {},
    parallel,
    agent,
  )

  assert.deepEqual(parallelSizes, [5, 5, 5, 2])
  assert.equal(verifierCalls, 12)
  assert.equal(result.design_findings.length, 11)
  assert.equal(result.rejected.length, 1)
  assert.equal(result.rejected[0].reason, 'verify-agent-error')
  assert.deepEqual(result.resource_summary, {
    lens_agents: 5,
    verify_agents: 12,
    synthesis_agents: 1,
    total_agents: 18,
    max_parallel_agents: 5,
  })
}

runWorkflow()
  .then(() => process.stdout.write('plan.workflow.js tests: PASS\n'))
  .catch((error) => {
    process.stderr.write(`${error.stack}\n`)
    process.exitCode = 1
  })
