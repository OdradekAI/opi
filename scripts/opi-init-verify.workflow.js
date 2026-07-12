export const meta = {
  name: 'opi-init-verify',
  description: 'Adversarial multi-lens verification of an opi-implement init task-graph draft',
  phases: [
    { title: 'Lens audit' },
    { title: 'Fold' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

const draft = args.draftTasks
const sourcePath = args.sourceDesignPath

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
        required: ['lens', 'task_id', 'field', 'problem', 'severity', 'suggested_fix', 'source_citation', 'confidence'],
        properties: {
          lens: { type: 'string' },
          task_id: { type: 'string' },
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
  required: ['task_id', 'field', 'accepted', 'reason'],
  properties: {
    task_id: { type: 'string' },
    field: { type: 'string' },
    accepted: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['summary', 'folded', 'flagged_for_human', 'rejected'],
  properties: {
    summary: { type: 'string' },
    folded: { type: 'array', items: { type: 'object', additionalProperties: true } },
    flagged_for_human: { type: 'array', items: { type: 'object', additionalProperties: true } },
    rejected: { type: 'array', items: { type: 'object', additionalProperties: true } },
  },
}

const LENSES = [
  { key: 'dod-precision', charter: 'L1 DoD precision: detect vague verbs and missing observable assertions; suggest concrete command/API/artifact/call-site/runtime/diagnostics/error expansions.' },
  { key: 'tier-boundary', charter: 'L2 tier/crate-boundary: enforce the opi-ai/opi-agent/opi-coding-agent ownership invariant and task_owned_paths containment.' },
  { key: 'forbidden-scope', charter: 'L3 forbidden-scope: ensure every Non-Goal is a forbidden_scope inference_note and no task risks implementing a non-goal.' },
  { key: 'coverage', charter: 'L4 coverage: every Goal/SC/workflow has an owning task with acceptance_scenarios + production_call_sites; audit composite-row splits.' },
  { key: 'dependency-sequencing', charter: 'L5 dependency/sequencing: depends_on matches Sequencing; no cycles; extract defer/split/residual with re-trigger conditions.' },
  { key: 'substrate-product', charter: 'L6 substrate-vs-product: substrate_only correctness; no product scenario closed by substrate-only evidence.' },
]

phase('Lens audit')
const lensResults = await parallel(LENSES.map((l) => () =>
  agent(
    'You are an init-verify lens auditing an opi-implement task-graph draft.\n' +
    'Read the source phase design doc at ' + sourcePath + ' in full.\n' +
    'Apply lens ' + l.key + ': ' + l.charter + '\n' +
    'Hard rules: never propose implementing a Non-Goal; never invent tasks or scope beyond the source; ' +
    'emit one finding per problem; cite the source section (use § or #) for every finding and verify the cited heading appears verbatim in the source.\n' +
    'Draft task graph JSON:\n' + JSON.stringify(draft),
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
    'Adversarially verify this proposed init-verify correction. Try to REJECT it.\n' +
    'Source design doc: ' + sourcePath + '\n' +
    'Task: ' + f.task_id + '  Field: ' + f.field + '\n' +
    'Problem: ' + f.problem + '\n' +
    'Proposed fix: ' + f.suggested_fix + '\n' +
    'Citation: ' + f.source_citation + '\n' +
    'REJECT if the fix contradicts the source, implements a Non-Goal, invents scope beyond the source, ' +
    'or requires task-graph surgery (adding/removing/restructuring tasks). Default to accepted=false if uncertain.',
    { label: 'verify:' + f.task_id + ':' + f.field, phase: 'Verify', schema: VERDICT_SCHEMA }
  )
    .then((v) => ({ finding: f, verdict: v }))
    .catch(() => ({ finding: f, verdict: { task_id: f.task_id, field: f.field, accepted: false, reason: 'verify-agent-error' } }))
))
const confirmed = verdicts.filter(Boolean).filter((v) => v.verdict.accepted).map((v) => v.finding)
const rejected = verdicts.filter(Boolean).filter((v) => !v.verdict.accepted)
  .map((v) => ({ finding: v.finding, reason: v.verdict.reason }))

phase('Synthesize')
const report = await agent(
  'Synthesize the opi-implement init-verify report.\n' +
  'Confirmed folds (apply to draft with inference_notes provenance):\n' + JSON.stringify(confirmed) + '\n' +
  'Flagged for human review (not auto-applied):\n' + JSON.stringify(flagged) + '\n' +
  'Rejected by adversarial verify:\n' + JSON.stringify(rejected) + '\n' +
  'Write a concise summary plus the three lists.',
  { label: 'synthesize', phase: 'Synthesize', schema: REPORT_SCHEMA }
)

return { confirmed_folds: confirmed, flagged_for_human: flagged, rejected, report }
