export const meta = {
  name: 'exec-verify',
  description: 'Exec-stage adversarial multi-lens verification of an opi-implement task implementation + evidence (Phase D, deep path)',
  phases: [
    { title: 'Lens audit' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

// The Workflow runtime may hand `args` as a JSON string rather than a parsed
// object; snapshot the JSON input so asynchronous callers cannot change the
// receipt between its prompt serialization and the returned review binding.
const _args = JSON.parse(typeof args === 'string' ? args : JSON.stringify(args))
const task = _args.task
const sourcePath = _args.sourceDesignPath
const candidate = _args.candidate
const reviewSha256 = _args.review_sha256

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
        required: ['lens', 'task_id', 'criterion_id', 'decision_id', 'field', 'problem', 'severity', 'suggested_fix', 'source_citation', 'confidence'],
        properties: {
          lens: { type: 'string' },
          task_id: { type: 'string' },
          criterion_id: { type: 'null' },
          decision_id: { type: ['string', 'null'] },
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
  { key: 'implementation-matches-dod', charter: 'L-D1: every observable assertion in the task definition_of_done is actually implemented (no stubs/TODOs/placeholders passing a real assertion). Inspect all bound candidate files, including additions and deletions.' },
  { key: 'tests-non-vacuous', charter: 'L-D2: the task tests assert meaningful behavior (not tautological / always-pass / over-mocked).' },
  { key: 'production-call-site-proven', charter: 'L-D3: runtime/CLI/session/provider claims have a real production call site exercised by a test, not tests-of-helpers.' },
  { key: 'evidence-truthfulness', charter: 'L-D4: the Opi-* commit footers + verification evidence match reality (commands ran, outputs preserved).' },
  { key: 'non-goal-leak', charter: 'L-D5: the implementation does not drift into a phase Non-Goal (token-trigger: npm, marketplace, OAuth, telemetry, sandboxing, web-UI parity, pi session compatibility, workflow tools, MCP core, plan mode core, sub-agent core).' },
  { key: 'workspace-deps-honored', charter: 'L-D6: internal deps go through [workspace.dependencies]; no bare path deps in any changed Cargo.toml.' },
  { key: 'decision-locality-test-stewardship', charter: 'L-D7: for every task shared_decision, search its owner, production consumers, legacy paths, and relevant unchanged consumer code. One typed Interface must own the decision; all consumers must route through it; listed legacy paths must be closed. Read relevant test bodies and reject layered equivalent tests, superseded-Interface tests, or an incomplete or false session_notes gate_results.test_disposition.' },
]

// The runtime's schema support is not a completion signal: required outputs
// are checked again before any verdict or empty blocking list is returned.
function requireReview(condition, detail) {
  if (!condition) throw new Error('REVIEW-INCOMPLETE: ' + detail)
}
function matchesSchema(value, schema) {
  const kind = value === null ? 'null' : Array.isArray(value) ? 'array' : typeof value
  const types = Array.isArray(schema.type) ? schema.type : [schema.type]
  if (schema.type && !types.includes(kind)) return false
  if (schema.enum && !schema.enum.includes(value)) return false
  if (schema.pattern && !new RegExp(schema.pattern).test(value)) return false
  if (kind === 'array' && schema.items) return Array.from(value).every((item) => matchesSchema(item, schema.items))
  if (kind === 'object' && schema.properties) {
    if ((schema.required || []).some((key) => !Object.hasOwn(value, key))) return false
    if (schema.additionalProperties === false && Object.keys(value).some((key) => !Object.hasOwn(schema.properties, key))) return false
    return Object.entries(schema.properties).every(([key, field]) => !Object.hasOwn(value, key) || matchesSchema(value[key], field))
  }
  return true
}
async function reviewAgent(prompt, options) {
  let result
  try { result = await agent(prompt, options) }
  catch (cause) { throw new Error('REVIEW-INCOMPLETE: ' + options.label + ' execution failed', { cause }) }
  requireReview(matchesSchema(result, options.schema), options.label + ' returned invalid output')
  if (Object.hasOwn(result, 'accepted')) requireReview(result.reason.trim().length > 0, options.label + ' returned no judgment reason')
  if (Object.hasOwn(result, 'findings')) {
    const lens = options.label.replace(/^(plan|lens):/, '')
    requireReview(result.findings.every((finding) => finding.lens === lens), options.label + ' returned a different lens')
  }
  return result
}
async function reviewParallel(operations) {
  let results
  try { results = await parallel(operations) }
  catch (cause) { throw new Error('REVIEW-INCOMPLETE: parallel execution failed', { cause }) }
  requireReview(Array.isArray(results) && results.length === operations.length && Array.from(results).every((result) => result !== null && result !== undefined), 'missing parallel result')
  return results
}

// task_completion.py owns byte/digest checks before and after this workflow.
// Here the caller's task/source must agree with the frozen review receipt.
const sha256 = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value)
const commitId = (value) => typeof value === 'string' && /^[a-f0-9]{40}$/.test(value)
const canonical = (value) => JSON.stringify(value, function (_key, item) {
  return item && typeof item === 'object' && !Array.isArray(item)
    ? Object.fromEntries(Object.keys(item).sort().map((key) => [key, item[key]])) : item
})
requireReview(candidate && candidate.schema === 'opi-task-review/1', 'missing task review receipt')
requireReview(sha256(reviewSha256) && sha256(candidate.candidate_sha256), 'missing receipt identity')
requireReview(['worktree', 'commit'].includes(candidate.mode), 'unknown candidate mode')
requireReview(task && candidate.task && canonical(task) === canonical(candidate.task), 'task differs from review receipt')
requireReview(candidate.task_id === task.id && candidate.start_commit === task.start_commit, 'task identity differs from review receipt')
requireReview(commitId(candidate.start_commit) && commitId(candidate.review_head), 'invalid candidate commit identity')
requireReview(candidate.mode !== 'worktree' || candidate.review_head === candidate.start_commit, 'worktree baseline changed')
requireReview(candidate.source_design_path === sourcePath && sha256(candidate.source_sha256), 'phase source differs from review receipt')
requireReview(sha256(candidate.task_sha256) && sha256(candidate.dod_sha256), 'missing task content identity')
const validState = (state, index = false) => state === null || (state && ['100644', '100755'].includes(state.mode) && (index ? commitId(state.blob) : sha256(state.sha256)))
requireReview(Array.isArray(candidate.files) && candidate.files.every((file) => file && typeof file.path === 'string' && file.path.length > 0 && validState(file.before) && validState(file.after) && validState(file.index, true)), 'invalid candidate file inventory')
requireReview(new Set(candidate.files.map((file) => file.path)).size === candidate.files.length, 'duplicate candidate path')
requireReview(candidate.verification && Array.isArray(candidate.verification.checks) && candidate.raw_proofs && typeof candidate.raw_proofs === 'object' && !Array.isArray(candidate.raw_proofs) && Object.values(candidate.raw_proofs).every(sha256), 'missing bound verification evidence')
const footers = candidate.proposed_footers
requireReview(footers && footers['Opi-Task'] === task.id && footers['Opi-DoD-SHA256'] === candidate.dod_sha256 && typeof footers['Opi-Verification'] === 'string' && typeof footers['Opi-Evaluator'] === 'string', 'planned footers differ from task identity')
const candidateInstructions = candidate.mode === 'worktree'
  ? 'Inspect the bound worktree candidate against start_commit, including staged, unstaged and untracked implementation. Read every listed path and its production consumers; a baseline commit alone is not the candidate.\n'
  : 'Inspect the committed candidate at review_head against start_commit. Read the committed bytes of every listed path; do not substitute dirty worktree content.\n'
const reviewInput = 'Frozen task/source/evidence/footers receipt (review SHA-256 ' + reviewSha256 + '):\n' + JSON.stringify(candidate) + '\n' + candidateInstructions

phase('Lens audit')
const lensResults = await reviewParallel(LENSES.map((l) => () =>
  reviewAgent(
    'You are an exec-verify lens auditing one opi-implement task implementation + its evidence.\n' +
    'Task object (DoD, evidence, acceptance_scenarios, task_owned_paths):\n' + JSON.stringify(task) + '\n' +
    reviewInput +
    'Phase design doc (for Non-Goals + DoD context): ' + sourcePath + '\n' +
    'Apply lens ' + l.key + ': ' + l.charter + '\n' +
    'Hard rules: cite the source section (use § or #) for every finding; verify the cited heading appears verbatim; ' +
    'use the stable shared decision id for a decision finding and decision_id=null otherwise; ' +
    'emit one finding per real problem. If the lens finds nothing real, return findings: [].',
    { label: 'lens:' + l.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA }
  )
))
const allFindings = lensResults.flatMap((r) => r.findings)
requireReview(allFindings.every((finding) => finding.task_id === task.id), 'lens returned a different task')

const foldable = allFindings.filter((f) =>
  f.severity === 'high' || (f.severity === 'medium' && f.confidence === 'high'))
const flagged = allFindings.filter((f) => !foldable.includes(f))

phase('Verify')
const verdicts = await reviewParallel(foldable.map((f) => () =>
  reviewAgent(
    'Adversarially verify this exec-verify finding. Try to REJECT it.\n' +
    'Task: ' + f.task_id + '  Decision: ' + (f.decision_id || 'none') + '\n' + reviewInput +
    'Problem: ' + f.problem + '\n  Proposed fix: ' + f.suggested_fix + '\n' +
    'Decide whether the claimed defect is established by the bound candidate and current production evidence. ' +
    'An incorrect suggested fix does not refute a real defect; assess the claim separately from the proposed repair. ' +
    'Default to accepted=false if the concern is already satisfied or the claim remains uncertain.',
    { label: 'verify:' + f.lens, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((verdict) => {
      requireReview(verdict.task_id === f.task_id, 'verifier answered a different finding')
      return { finding: f, verdict }
    })
))
const mustFix = verdicts.filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await reviewAgent(
  'Synthesize the opi-implement exec-verify report.\n' +
  'MUST-FIX (block Phase D pass; route to Phase C):\n' + JSON.stringify(mustFix) + '\n' +
  'Flagged for human:\n' + JSON.stringify(flagged) + '\n' +
  'Rejected:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

requireReview(canonical(report.must_fix) === canonical(mustFix) && canonical(report.flagged_for_human) === canonical(flagged) && canonical(report.rejected) === canonical(rejected), 'synthesis changed the supplied findings')

return { review_sha256: reviewSha256, candidate_sha256: candidate.candidate_sha256, reviewed_input: candidate, must_fix: mustFix, flagged_for_human: flagged, rejected, report }
