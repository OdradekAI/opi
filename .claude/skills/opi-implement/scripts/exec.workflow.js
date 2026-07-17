export const meta = {
  name: 'exec-verify',
  description: 'Exec-stage adversarial multi-lens verification of an opi-implement task implementation + evidence (Phase D, deep path)',
  phases: [
    { title: 'Lens audit' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

const task = args.task
const sourcePath = args.sourceDesignPath
const commit = args.commit

const FINDINGS_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        required: ['lens', 'task_id', 'criterion_id', 'field', 'problem', 'severity', 'suggested_fix', 'source_citation', 'confidence'],
        properties: {
          lens: { type: 'string' },
          task_id: { type: 'string' },
          criterion_id: { type: 'null' },
          field: { type: 'string' },
          problem: { type: 'string' },
          severity: { enum: ['high', 'medium', 'low'] },
          suggested_fix: { type: 'string' },
          source_citation: { type: 'string', pattern: '(§|#)' },
          confidence: { enum: ['high', 'medium', 'low'] },
        },
      },
    },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['task_id', 'accepted', 'reason'],
  properties: {
    task_id: { type: 'string' },
    accepted: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'must_fix', 'flagged_for_human', 'rejected'],
  properties: {
    summary: { type: 'string' },
    must_fix: { type: 'array', items: { type: 'object', additionalProperties: true } },
    flagged_for_human: { type: 'array', items: { type: 'object', additionalProperties: true } },
    rejected: { type: 'array', items: { type: 'object', additionalProperties: true } },
  },
}

const LENSES = [
  { key: 'implementation-matches-dod', charter: 'L-D1: every observable assertion in the task definition_of_done is actually implemented (no stubs/TODOs/placeholders passing a real assertion). Inspect the files changed at the commit.' },
  { key: 'tests-non-vacuous', charter: 'L-D2: the task tests assert meaningful behavior (not tautological / always-pass / over-mocked).' },
  { key: 'production-call-site-proven', charter: 'L-D3: runtime/CLI/session/provider claims have a real production call site exercised by a test, not tests-of-helpers.' },
  { key: 'evidence-truthfulness', charter: 'L-D4: the Opi-* commit footers + verification evidence match reality (commands ran, outputs preserved).' },
  { key: 'non-goal-leak', charter: 'L-D5: the implementation does not drift into a phase Non-Goal (token-trigger: npm, marketplace, OAuth, telemetry, sandboxing, web-UI parity, pi session compatibility, workflow tools, MCP core, plan mode core, sub-agent core).' },
  { key: 'workspace-deps-honored', charter: 'L-D6: internal deps go through [workspace.dependencies]; no bare path deps in any changed Cargo.toml.' },
]

phase('Lens audit')
const lensResults = await parallel(LENSES.map((l) => () =>
  agent(
    'You are an exec-verify lens auditing one opi-implement task implementation + its evidence.\n' +
    'Task object (DoD, evidence, acceptance_scenarios, task_owned_paths):\n' + JSON.stringify(task) + '\n' +
    'HEAD commit being verified: ' + commit + '\n' +
    'Run `git show --stat ' + commit + '` and read each changed file to inspect the actual implementation.\n' +
    'Phase design doc (for Non-Goals + DoD context): ' + sourcePath + '\n' +
    'Apply lens ' + l.key + ': ' + l.charter + '\n' +
    'Hard rules: cite the source section (use § or #) for every finding; verify the cited heading appears verbatim; ' +
    'emit one finding per real problem. If the lens finds nothing real, return findings: [].',
    { label: 'lens:' + l.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA }
  )
))
const allFindings = lensResults.filter(Boolean).flatMap((r) => r.findings)

const foldable = allFindings.filter((f) =>
  f.severity === 'high' || (f.severity === 'medium' && f.confidence === 'high'))
const flagged = allFindings.filter((f) => !foldable.includes(f))

phase('Verify')
const verdicts = await parallel(foldable.map((f) => () =>
  agent(
    'Adversarially verify this exec-verify finding. Try to REJECT it.\n' +
    'Task: ' + f.task_id + '  Commit: ' + commit + '\n' +
    'Problem: ' + f.problem + '\n  Proposed fix: ' + f.suggested_fix + '\n' +
    'Read the changed files at ' + commit + ' and decide. ACCEPT only if the problem is real AND the fix is correct. ' +
    'Default to accepted=false if the implementation already satisfies the concern or the fix is wrong.',
    { label: 'verify:' + f.lens, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((v) => ({ finding: f, verdict: v }))
    .catch(() => ({ finding: f, verdict: { task_id: f.task_id, accepted: false, reason: 'verify-agent-error' } }))
))
const mustFix = verdicts.filter(Boolean).filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter(Boolean).filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await agent(
  'Synthesize the opi-implement exec-verify report.\n' +
  'MUST-FIX (block Phase D pass; route to Phase C):\n' + JSON.stringify(mustFix) + '\n' +
  'Flagged for human:\n' + JSON.stringify(flagged) + '\n' +
  'Rejected:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

return { must_fix: mustFix, flagged_for_human: flagged, rejected, report }
