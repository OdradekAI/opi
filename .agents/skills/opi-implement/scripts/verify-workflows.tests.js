const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const test = require('node:test')

const source = (name) => fs.readFileSync(path.join(__dirname, name + '.workflow.js'), 'utf8')
const jsonLine = (prompt, prefix) => JSON.parse(prompt.split('\n').find((line) => line.startsWith(prefix)).slice(prefix.length))
const jsonAfter = (prompt, heading) => JSON.parse(prompt.split(heading + '\n')[1].split('\n')[0])

function defaultArgs(count = 1) {
  const task = { id: 'T', start_commit: 'a'.repeat(40), definition_of_done: 'Observable behavior' }
  return {
    draftTasks: Array.from({ length: count }, (_, index) => ({ id: String(index) })),
    task, sourceDesignPath: 'docs/opi-spec.md',
    phase: 20, criteriaTrace: [{ id: 'C', status: 'met' }], phaseTasks: [], independence: 'test',
    review_sha256: 'd'.repeat(64),
    candidate: {
      schema: 'opi-task-review/1', mode: 'worktree', task: structuredClone(task), task_id: 'T',
      start_commit: task.start_commit, review_head: task.start_commit,
      task_sha256: 'b'.repeat(64), dod_sha256: 'c'.repeat(64), candidate_sha256: 'e'.repeat(64),
      source_design_path: 'docs/opi-spec.md', source_sha256: 'f'.repeat(64),
      files: [{ path: 'new-production-path.rs', before: null, after: { sha256: '1'.repeat(64), mode: '100644' }, index: null }],
      verification: { checks: [{ command: 'captured-test-command', exit_code: 0, stdout: '/external/stdout.log', stderr: '/external/stderr.log' }] },
      raw_proofs: { '/external/stdout.log': '2'.repeat(64) },
      proposed_footers: { 'Opi-Task': 'T', 'Opi-DoD-SHA256': 'c'.repeat(64), 'Opi-Verification': 'unit; captured-test-command', 'Opi-Evaluator': 'passed' },
    },
  }
}

async function runWorkflow(name, options = {}) {
  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor
  const execute = new AsyncFunction('args', 'phase', 'parallel', 'agent', source(name).replace('export const meta =', 'const meta ='))
  const count = options.count ?? 1
  const calls = []
  const parallelSizes = []
  let emitted = false
  const args = options.args || defaultArgs(count)
  if (options.mutateArgs) options.mutateArgs(args)
  const agent = async (prompt, config) => {
    calls.push({ prompt, config })
    if (options.onAgent) options.onAgent(prompt, config, args)
    if (options.throwAt === config.phase) throw new Error('injected runtime failure')
    if (options.invalidAt === config.phase) return options.invalidValue ?? null
    if (config.phase === 'Lens audit') {
      const findings = emitted ? [] : Array.from({ length: count }, (_, index) => ({
        lens: config.label.replace(/^(plan|lens):/, ''),
        task_id: name === 'phase-exit' ? null : name === 'exec' ? 'T' : String(index),
        ...(name === 'plan' ? { axis: 'execution-readiness', route: 'GRAPH_REVISION_REQUIRED', blocking: true }
          : { criterion_id: name === 'phase-exit' ? 'C' : null }),
        decision_id: null, field: 'behavior', problem: 'Observable contract violation',
        severity: 'high', suggested_fix: 'A possible correction', source_citation: 'docs/opi-spec.md#Contract', confidence: 'high',
        ...options.findingOverride,
      }))
      emitted = true
      return { findings }
    }
    if (config.phase === 'Verify') return {
      ...(name === 'exec' ? { task_id: options.wrongIdentity ? 'OTHER' : 'T' }
        : name === 'phase-exit' ? { criterion_id: options.wrongIdentity ? 'OTHER' : 'C' } : {}),
      accepted: options.accepted ?? true, reason: options.accepted === false ? 'Not established by current evidence' : 'Current production evidence establishes the defect',
    }
    if (name === 'plan') return {
      summary: 'reviewed', verdict: prompt.match(/Verdict: ([A-Z_]+)/)[1], independence: 'test',
      design_findings: jsonLine(prompt, 'Design findings: '), graph_findings: jsonLine(prompt, 'Graph findings: '),
      flagged_for_human: jsonLine(prompt, 'Non-blocking human flags: '), rejected: jsonLine(prompt, 'Rejected findings: '),
      ...options.reportOverride,
    }
    return {
      summary: 'reviewed',
      ...(name === 'exec' ? { must_fix: jsonAfter(prompt, 'MUST-FIX (block Phase D pass; route to Phase C):') }
        : { not_met: jsonAfter(prompt, 'NOT-MET (the calling agent upserts criteria_trace[criterion_id].status=not-met for each; F.1b then REFUSEs archive):') }),
      flagged_for_human: jsonAfter(prompt, name === 'exec' ? 'Flagged for human:' : 'Flagged for human (do not mutate trace):'),
      rejected: jsonAfter(prompt, 'Rejected:'),
      ...options.reportOverride,
    }
  }
  const result = await execute(options.asString ? JSON.stringify(args) : args, () => {}, async (tasks) => {
    if (options.parallelError) throw new Error('injected parallel failure')
    parallelSizes.push(tasks.length)
    const results = await Promise.all(tasks.map((operation) => operation()))
    if (options.sparseParallelCall === parallelSizes.length) return new Array(tasks.length)
    return options.dropParallelResult ? results.slice(1) : results
  }, agent)
  return { result, calls, parallelSizes }
}

function failureCases(name) {
  for (const stage of ['Lens audit', 'Verify', 'Synthesize']) {
    test(name + ': ' + stage + ' error cannot complete review', async () => {
      await assert.rejects(runWorkflow(name, { throwAt: stage }), /REVIEW-INCOMPLETE/)
    })
    test(name + ': ' + stage + ' missing output cannot complete review', async () => {
      await assert.rejects(runWorkflow(name, { invalidAt: stage }), /REVIEW-INCOMPLETE/)
    })
  }
  test(name + ': malformed verifier output is not a rejection', async () => {
    await assert.rejects(runWorkflow(name, { invalidAt: 'Verify', invalidValue: { accepted: 'false', reason: 'invalid' } }), /REVIEW-INCOMPLETE/)
  })
  test(name + ': missing parallel member cannot look clean', async () => {
    await assert.rejects(runWorkflow(name, { dropParallelResult: true }), /REVIEW-INCOMPLETE/)
  })
  for (const sparseParallelCall of [1, 2]) {
    test(name + ': sparse parallel results cannot complete stage ' + sparseParallelCall, async () => {
      await assert.rejects(runWorkflow(name, { sparseParallelCall }), /REVIEW-INCOMPLETE/)
    })
  }
  test(name + ': sparse finding array is invalid reviewer output', async () => {
    await assert.rejects(runWorkflow(name, { invalidAt: 'Lens audit', invalidValue: { findings: new Array(1) } }), (error) => {
      assert.match(error.message, /REVIEW-INCOMPLETE/)
      assert.match(error.cause.message, /REVIEW-INCOMPLETE: .* returned invalid output/)
      return true
    })
  })
  test(name + ': parallel failure preserves its cause and cannot complete', async () => {
    await assert.rejects(runWorkflow(name, { parallelError: true }), (error) => {
      assert.match(error.message, /REVIEW-INCOMPLETE/)
      assert.equal(error.cause.message, 'injected parallel failure')
      return true
    })
  })
  test(name + ': finding must belong to the reviewed scope', async () => {
    const findingOverride = name === 'phase-exit' ? { criterion_id: 'OTHER' } : { task_id: 'OTHER' }
    await assert.rejects(runWorkflow(name, { findingOverride }), /REVIEW-INCOMPLETE/)
  })
  test(name + ': a valid negative judgment remains rejected', async () => {
    const { result } = await runWorkflow(name, { accepted: false })
    assert.equal(result.rejected.length, 1)
    assert.equal((result.must_fix || result.not_met || result.graph_findings).length, 0)
  })
  test(name + ': confirmed defect remains blocking', async () => {
    const { result } = await runWorkflow(name)
    assert.equal((result.must_fix || result.not_met || result.graph_findings).length, 1)
    assert.equal(result.rejected.length, 0)
  })
  test(name + ': successful empty review completes', async () => {
    const { result } = await runWorkflow(name, { count: 0, asString: true })
    assert.equal((result.must_fix || result.not_met || result.graph_findings).length, 0)
    assert.equal(result.rejected.length, 0)
  })
  test(name + ': synthesis cannot discard a confirmed finding', async () => {
    const reportOverride = name === 'plan' ? { graph_findings: [], verdict: 'READY' }
      : name === 'exec' ? { must_fix: [] } : { not_met: [] }
    await assert.rejects(runWorkflow(name, { reportOverride }), /REVIEW-INCOMPLETE/)
  })
}

module.exports = { runWorkflow, failureCases, defaultArgs }

if (require.main === module) {
  for (const name of ['exec', 'phase-exit']) {
    failureCases(name)
    test(name + ': verifier must answer the same finding identity', async () => {
      await assert.rejects(runWorkflow(name, { wrongIdentity: true }), /REVIEW-INCOMPLETE/)
    })
  }
  for (const [label, mutation] of [
    ['missing candidate', (args) => { delete args.candidate }],
    ['missing receipt hash', (args) => { delete args.review_sha256 }],
    ['stale task content', (args) => { args.task.definition_of_done = 'Changed behavior' }],
    ['different task identity', (args) => { args.candidate.task_id = 'OTHER' }],
    ['different source', (args) => { args.sourceDesignPath = 'docs/other.md' }],
    ['wrong DoD footer', (args) => { args.candidate.proposed_footers['Opi-DoD-SHA256'] = '0'.repeat(64) }],
    ['missing verification', (args) => { delete args.candidate.verification }],
    ['malformed file hash', (args) => { args.candidate.files[0].after.sha256 = 'bad' }],
  ]) {
    test('exec: rejects ' + label + ' before review', async () => {
      await assert.rejects(runWorkflow('exec', { mutateArgs: mutation }), /REVIEW-INCOMPLETE/)
    })
  }
  test('exec: lenses and verifier receive the same complete candidate', async () => {
    const args = defaultArgs()
    const { result, calls } = await runWorkflow('exec', { args })
    assert.equal(result.review_sha256, args.review_sha256)
    assert.equal(result.candidate_sha256, args.candidate.candidate_sha256)
    assert.deepEqual(result.reviewed_input, args.candidate)
    for (const { prompt, config } of calls.filter(({ config }) => config.phase !== 'Synthesize')) {
      assert.ok(prompt.includes(JSON.stringify(args.candidate)), config.label)
      assert.ok(prompt.includes('new-production-path.rs'), config.label)
      assert.ok(prompt.includes('captured-test-command'), config.label)
    }
  })
  test('exec: result exposes the actual reviewed input even with a reused receipt hash', async () => {
    const args = defaultArgs()
    args.candidate.files = []
    const { result } = await runWorkflow('exec', { args, count: 0 })
    assert.deepEqual(result.reviewed_input, args.candidate)
    assert.deepEqual(result.reviewed_input.files, [])
  })
  test('exec: caller mutation during review cannot change the frozen input', async () => {
    const args = defaultArgs()
    const frozen = structuredClone(args.candidate)
    const { result, calls } = await runWorkflow('exec', { args, onAgent: (_prompt, _config, original) => {
      original.candidate.files = []
      original.task.definition_of_done = 'unreviewed caller mutation'
    } })
    assert.deepEqual(result.reviewed_input, frozen)
    for (const { prompt, config } of calls.filter(({ config }) => config.phase !== 'Synthesize')) {
      assert.ok(prompt.includes(JSON.stringify(frozen)), config.label)
      assert.ok(!prompt.includes('unreviewed caller mutation'), config.label)
    }
  })
  test('exec: manual commit review uses the bound commit rather than worktree', async () => {
    const args = defaultArgs()
    args.candidate.mode = 'commit'
    args.candidate.review_head = '9'.repeat(40)
    const { calls } = await runWorkflow('exec', { args })
    assert.match(calls[0].prompt, /committed candidate/)
    assert.ok(calls[0].prompt.includes('9'.repeat(40)))
  })
}
