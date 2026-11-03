export const meta = {
  name: 'phase-exit-verify',
  description: 'Phase-exit-stage adversarial multi-lens audit of F.1a criteria trace (Phase F)',
  phases: [
    { title: 'Lens audit' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

// The Workflow runtime may hand `args` as a JSON string rather than a parsed
// object; normalize before reading so lens prompts get the bound trace/source/phase.
const _args = typeof args === 'string' ? JSON.parse(args) : args
const trace = _args.criteriaTrace    // F.1a's criteria_trace[] array
const phaseTasks = _args.phaseTasks
const sourcePath = _args.sourceDesignPath
const phaseNum = _args.phase         // renamed from `phase` to avoid shadowing the `phase()` progress hook

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
          task_id: { type: 'null' },
          criterion_id: { type: 'string' },
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
  required: ['criterion_id', 'accepted', 'reason'],
  properties: {
    criterion_id: { type: 'string' },
    accepted: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'not_met', 'flagged_for_human', 'rejected'],
  properties: {
    summary: { type: 'string' },
    not_met: { type: 'array', items: { type: 'object', additionalProperties: true } },
    flagged_for_human: { type: 'array', items: { type: 'object', additionalProperties: true } },
    rejected: { type: 'array', items: { type: 'object', additionalProperties: true } },
  },
}

const LENSES = [
  { key: 'criterion-traced-to-code', charter: 'L-F1: each criterion in the trace maps to real code that implements it (not just a claim).' },
  { key: 'criterion-traced-to-test', charter: 'L-F2: each criterion has a test that exercises it.' },
  { key: 'non-goals-respected', charter: 'L-F3: no phase Non-Goal was implemented to satisfy a criterion.' },
  { key: 'residuals-exactly-cited', charter: 'L-F4: every deferred-by-updated-design criterion carries an exact current-spec citation; uncited deferrals are not-met.' },
  { key: 'substrate-vs-product-honest', charter: 'L-F5: no product criterion is closed by substrate-only tasks across the phase.' },
  { key: 'shared-decision-closure', charter: 'L-F6: inspect completed phaseTasks and assembled repository code. Every declared shared decision has one owning typed Interface, all production consumers route through it, legacy paths are closed, its closure test exercises the Interface, and every touched test appears truthfully in session_notes gate_results.test_disposition. Map any failure to one declared criterion_id.' },
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

const canonical = (value) => JSON.stringify(value, function (_key, item) {
  return item && typeof item === 'object' && !Array.isArray(item)
    ? Object.fromEntries(Object.keys(item).sort().map((key) => [key, item[key]])) : item
})

phase('Lens audit')
const lensResults = await reviewParallel(LENSES.map((l) => () =>
  reviewAgent(
    'You are a phase-exit-verify lens auditing F.1a criteria trace for phase ' + phaseNum + '.\n' +
    'Phase design doc: ' + sourcePath + '\n' +
    'F.1a criteria trace (input):\n' + JSON.stringify(trace) + '\n' +
    'Completed phase tasks, including shared_decision notes and test_disposition evidence:\n' + JSON.stringify(phaseTasks) + '\n' +
    'Apply lens ' + l.key + ': ' + l.charter + '\n' +
    'For each criterion the lens finds problematic, emit a finding with criterion_id = the criterion id. ' +
    'Use its stable shared decision id for a decision finding and decision_id=null otherwise. ' +
    'Cite the source section (§ or #); verify the heading appears verbatim. ' +
    'If nothing is wrong, return findings: [].',
    { label: 'lens:' + l.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA }
  )
))
const allFindings = lensResults.flatMap((r) => r.findings)
requireReview(Array.isArray(trace) && allFindings.every((finding) => trace.some((entry) => entry.id === finding.criterion_id)), 'lens returned an unknown criterion')

const foldable = allFindings.filter((f) =>
  f.severity === 'high' || (f.severity === 'medium' && f.confidence === 'high'))
const flagged = allFindings.filter((f) => !foldable.includes(f))

phase('Verify')
const verdicts = await reviewParallel(foldable.map((f) => () =>
  reviewAgent(
    'Adversarially verify this phase-exit-verify finding. Try to REJECT it.\n' +
    'Criterion: ' + f.criterion_id + '  Decision: ' + (f.decision_id || 'none') + '  Phase: ' + phaseNum + '\n' +
    'Problem: ' + f.problem + '\n  Proposed fix: ' + f.suggested_fix + '\n' +
    'Read the criterion in the source design doc and the trace entry. ACCEPT only if the criterion is genuinely not-met. ' +
    'Default to accepted=false if the trace already satisfies the lens or the finding is a misread.',
    { label: 'verify:' + f.lens, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((verdict) => {
      requireReview(verdict.criterion_id === f.criterion_id, 'verifier answered a different finding')
      return { finding: f, verdict }
    })
))
const notMet = verdicts.filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await reviewAgent(
  'Synthesize the opi-implement phase-exit-verify report.\n' +
  'NOT-MET (the calling agent upserts criteria_trace[criterion_id].status=not-met for each; F.1b then REFUSEs archive):\n' + JSON.stringify(notMet) + '\n' +
  'Flagged for human (do not mutate trace):\n' + JSON.stringify(flagged) + '\n' +
  'Rejected:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

requireReview(canonical(report.not_met) === canonical(notMet) && canonical(report.flagged_for_human) === canonical(flagged) && canonical(report.rejected) === canonical(rejected), 'synthesis changed the supplied findings')

return { not_met: notMet, flagged_for_human: flagged, rejected, report }
