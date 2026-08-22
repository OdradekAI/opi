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

phase('Lens audit')
const lensResults = await parallel(LENSES.map((l) => () =>
  agent(
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
const allFindings = lensResults.filter(Boolean).flatMap((r) => r.findings)

const foldable = allFindings.filter((f) =>
  f.severity === 'high' || (f.severity === 'medium' && f.confidence === 'high'))
const flagged = allFindings.filter((f) => !foldable.includes(f))

phase('Verify')
const verdicts = await parallel(foldable.map((f) => () =>
  agent(
    'Adversarially verify this phase-exit-verify finding. Try to REJECT it.\n' +
    'Criterion: ' + f.criterion_id + '  Decision: ' + (f.decision_id || 'none') + '  Phase: ' + phaseNum + '\n' +
    'Problem: ' + f.problem + '\n  Proposed fix: ' + f.suggested_fix + '\n' +
    'Read the criterion in the source design doc and the trace entry. ACCEPT only if the criterion is genuinely not-met. ' +
    'Default to accepted=false if the trace already satisfies the lens or the finding is a misread.',
    { label: 'verify:' + f.lens, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((v) => ({ finding: f, verdict: v }))
    .catch(() => ({ finding: f, verdict: { criterion_id: f.criterion_id, accepted: false, reason: 'verify-agent-error' } }))
))
const notMet = verdicts.filter(Boolean).filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter(Boolean).filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await agent(
  'Synthesize the opi-implement phase-exit-verify report.\n' +
  'NOT-MET (the calling agent upserts criteria_trace[criterion_id].status=not-met for each; F.1b then REFUSEs archive):\n' + JSON.stringify(notMet) + '\n' +
  'Flagged for human (do not mutate trace):\n' + JSON.stringify(flagged) + '\n' +
  'Rejected:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

return { not_met: notMet, flagged_for_human: flagged, rejected, report }
