export const meta = {
  name: 'plan-admission-review',
  description: 'Non-mutating design- and execution-readiness review of an opi-implement draft graph',
  phases: [
    { title: 'Lens audit' },
    { title: 'Verify' },
    { title: 'Synthesize' },
  ],
}

// The Workflow runtime may hand `args` as a JSON string rather than a parsed
// object; normalize before reading so every reviewer gets the same source/draft.
const _args = typeof args === 'string' ? JSON.parse(args) : args
const draft = _args.draftTasks
const sourcePath = _args.sourceDesignPath
const activePhase = _args.phase
const independence = _args.independence || 'unknown'

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
        required: [
          'axis', 'lens', 'task_id', 'field', 'problem', 'severity',
          'suggested_fix', 'source_citation', 'confidence', 'route', 'blocking',
        ],
        properties: {
          axis: { enum: ['design-readiness', 'execution-readiness'] },
          lens: { type: 'string' },
          task_id: { type: ['string', 'null'] },
          field: { type: 'string' },
          problem: { type: 'string' },
          severity: { enum: ['high', 'medium', 'low'] },
          suggested_fix: { type: 'string' },
          source_citation: { type: 'string', pattern: '(§|#)' },
          confidence: { enum: ['high', 'medium', 'low'] },
          route: {
            enum: [
              'RESEARCH_REQUIRED',
              'DESIGN_DECISION_REQUIRED',
              'GRAPH_REVISION_REQUIRED',
            ],
          },
          blocking: { type: 'boolean' },
        },
      },
    },
  },
}

const VERDICT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['accepted', 'reason'],
  properties: {
    accepted: { type: 'boolean' },
    reason: { type: 'string' },
  },
}

const REPORT_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: [
    'summary', 'verdict', 'independence', 'design_findings', 'graph_findings',
    'flagged_for_human', 'rejected',
  ],
  properties: {
    summary: { type: 'string' },
    verdict: {
      enum: [
        'READY',
        'RESEARCH_REQUIRED',
        'DESIGN_DECISION_REQUIRED',
        'GRAPH_REVISION_REQUIRED',
      ],
    },
    independence: { type: 'string' },
    design_findings: { type: 'array', items: { type: 'object', additionalProperties: true } },
    graph_findings: { type: 'array', items: { type: 'object', additionalProperties: true } },
    flagged_for_human: { type: 'array', items: { type: 'object', additionalProperties: true } },
    rejected: { type: 'array', items: { type: 'object', additionalProperties: true } },
  },
}

const LENSES = [
  {
    key: 'design-lineage-placement',
    axis: 'design-readiness',
    charter: 'Check pi design lineage, justified Rust-native divergence, evidence provenance, plugin-first placement, and whether any proposed core work is only the smallest missing extension seam.',
  },
  {
    key: 'design-domain-seams',
    axis: 'design-readiness',
    charter: 'Check domain vocabulary, deep module interfaces, explicit public acceptance/test seams, problem/solution/out-of-scope completeness, contradictions, and unstated decisions.',
  },
  {
    key: 'execution-coverage-slices',
    axis: 'execution-readiness',
    charter: 'Check criterion coverage, demonstrable vertical slices, substrate/product honesty, acceptance scenarios, and production call sites.',
  },
  {
    key: 'execution-dependencies-ownership',
    axis: 'execution-readiness',
    charter: 'Check real blocking edges, cycles, expand-contract justification, crate/tier ownership, task-owned paths, and cross-task sequencing.',
  },
  {
    key: 'execution-verification-scope',
    axis: 'execution-readiness',
    charter: 'Check observable DoDs, agreed behavioral seams, proportional verification tiers/addenda, forbidden-scope guards, and non-goal leakage.',
  },
]

phase('Lens audit')
const lensResults = await parallel(LENSES.map((lens) => () =>
  agent(
    'You are an adversarial opi-implement plan-admission reviewer.\n' +
    'Read the registered source design at ' + sourcePath + ' in full.\n' +
    'Review phase ' + activePhase + ' using axis ' + lens.axis + ' and lens ' + lens.key + '.\n' +
    'Charter: ' + lens.charter + '\n' +
    'Read docs/CONTEXT.md and the applicable AGENTS.md/CLAUDE.md rules.\n' +
    'Never edit the source or draft. Never invent product scope. Emit one finding per falsifiable problem.\n' +
    'Use RESEARCH_REQUIRED only for missing facts/evidence, DESIGN_DECISION_REQUIRED only for an unsettled product/architecture/domain/seam decision, and GRAPH_REVISION_REQUIRED only when the reviewed source is sufficient but the task graph is defective.\n' +
    'Set blocking=false for observations that do not prevent source or graph admission. Cite an exact source heading for every finding and verify it exists.\n' +
    'Do not invoke opi-implement, this review workflow, or spawn additional agents.\n' +
    'Draft task graph JSON:\n' + JSON.stringify(draft),
    { label: 'plan:' + lens.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA },
  )
))
const allFindings = lensResults.filter(Boolean).flatMap((result) => result.findings)
const blocking = allFindings.filter((finding) => finding.blocking)
const nonBlocking = allFindings.filter((finding) => !finding.blocking)

phase('Verify')
const verified = await parallel(blocking.map((finding) => () =>
  agent(
    'Try to REJECT this proposed plan-admission finding.\n' +
    'Source design: ' + sourcePath + '\n' +
    'Original draft graph: ' + JSON.stringify(draft) + '\n' +
    'Finding: ' + JSON.stringify(finding) + '\n' +
    'Reject it if the citation does not support the claim, the issue is already satisfied, the route is wrong, or the finding invents scope. Default to accepted=false when uncertain.\n' +
    'Do not propose or apply edits. Do not invoke another reviewer or spawn agents.',
    { label: 'verify:' + finding.lens + ':' + (finding.task_id || 'source'), phase: 'Verify', schema: VERDICT_SCHEMA },
  )
    .then((verdict) => ({ finding, verdict }))
    .catch(() => ({ finding, verdict: { accepted: false, reason: 'verify-agent-error' } }))
))

const surviving = verified
  .filter(Boolean)
  .filter((item) => item.verdict.accepted)
  .map((item) => item.finding)
const rejected = verified
  .filter(Boolean)
  .filter((item) => !item.verdict.accepted)
  .map((item) => ({ finding: item.finding, reason: item.verdict.reason }))

const designFindings = surviving.filter((finding) => finding.axis === 'design-readiness')
const graphFindings = surviving.filter((finding) => finding.axis === 'execution-readiness')

let verdict = 'READY'
if (surviving.some((finding) => finding.route === 'RESEARCH_REQUIRED')) {
  verdict = 'RESEARCH_REQUIRED'
} else if (surviving.some((finding) => finding.route === 'DESIGN_DECISION_REQUIRED')) {
  verdict = 'DESIGN_DECISION_REQUIRED'
} else if (surviving.some((finding) => finding.route === 'GRAPH_REVISION_REQUIRED')) {
  verdict = 'GRAPH_REVISION_REQUIRED'
}

phase('Synthesize')
const report = await agent(
  'Write the bounded opi-implement plan-admission report without changing the deterministic verdict.\n' +
  'Verdict: ' + verdict + '\n' +
  'Independence: ' + independence + '\n' +
  'Design findings: ' + JSON.stringify(designFindings) + '\n' +
  'Graph findings: ' + JSON.stringify(graphFindings) + '\n' +
  'Non-blocking human flags: ' + JSON.stringify(nonBlocking) + '\n' +
  'Rejected findings: ' + JSON.stringify(rejected) + '\n' +
  'Return the supplied lists without reranking or applying them.',
  { label: 'plan-admission-report', phase: 'Synthesize', schema: REPORT_SCHEMA },
)

return {
  verdict,
  design_findings: designFindings,
  graph_findings: graphFindings,
  flagged_for_human: nonBlocking,
  rejected,
  report,
}
