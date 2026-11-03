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
const VERIFY_BATCH_SIZE = 5

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
          'axis', 'lens', 'task_id', 'decision_id', 'field', 'problem', 'severity',
          'suggested_fix', 'source_citation', 'confidence', 'route', 'blocking',
        ],
        properties: {
          axis: { enum: ['design-readiness', 'execution-readiness'] },
          lens: { type: 'string' },
          task_id: { type: ['string', 'null'] },
          decision_id: { type: ['string', 'null'] },
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
    charter: 'Check pi design lineage, justified Rust-native divergence, evidence provenance, plugin-first placement, and whether any proposed core work is only the smallest missing extension seam. Require inference_notes fields "reuse_search", "placement", "surface_necessity", and "simplification_ceiling" with an observable revisit_when and simplification_trigger. A trigger of none needs no extra evidence. When a task claims unused, duplicate, or superseded code, or proposes delete, merge, replace, or dependency substitution, require production_consumers, nonproduction_consumers, net_deletion, and residual_glue in the existing note reasons.',
  },
  {
    key: 'design-domain-seams',
    axis: 'design-readiness',
    charter: 'Check domain vocabulary, deep module interfaces, explicit public acceptance/test seams, shared decision identity, exactly one owner, typed representation, consumer agreement, temporary-path closure, problem/solution/out-of-scope completeness, contradictions, and unstated decisions.',
  },
  {
    key: 'execution-coverage-slices',
    axis: 'execution-readiness',
    charter: 'Check criterion coverage, demonstrable vertical slices, substrate/product honesty, acceptance scenarios, and production call sites. Every substrate task must be contained by a later scenario owner through the transitive depends_on closure.',
  },
  {
    key: 'execution-dependencies-ownership',
    axis: 'execution-readiness',
    charter: 'Check real blocking edges, cycles, expand-contract justification, crate/tier ownership, task-owned paths, and cross-task sequencing.',
  },
  {
    key: 'execution-verification-scope',
    axis: 'execution-readiness',
    charter: "Check observable DoDs, agreed behavioral seams, proportional verification tiers/addenda, forbidden-scope guards, non-goal leakage, whether every runtime claim has both a production caller and behavioral verification, and whether planned tests follow replace-don't-layer at the owning Interface.",
  },
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
const lensResults = await reviewParallel(LENSES.map((lens) => () =>
  reviewAgent(
    'You are an adversarial opi-implement plan-admission reviewer.\n' +
    'Read the registered source design at ' + sourcePath + ' in full.\n' +
    'Review phase ' + activePhase + ' using axis ' + lens.axis + ' and lens ' + lens.key + '.\n' +
    'Charter: ' + lens.charter + '\n' +
    'Read docs/CONTEXT.md and the applicable AGENTS.md/CLAUDE.md rules.\n' +
    'Never edit the source or draft. Never invent product scope. Emit one finding per falsifiable problem.\n' +
    'Use RESEARCH_REQUIRED only for missing facts/evidence, DESIGN_DECISION_REQUIRED only for an unsettled product/architecture/domain/seam decision, and GRAPH_REVISION_REQUIRED only when the reviewed source is sufficient but the task graph is defective.\n' +
    'Set blocking=false for observations that do not prevent source or graph admission. Cite an exact source heading for every finding and verify it exists.\n' +
    'Use the stable shared decision id when the finding concerns a declared shared_decision; otherwise set decision_id=null.\n' +
    'Do not invoke opi-implement, this review workflow, or spawn additional agents.\n' +
    'Draft task graph JSON:\n' + JSON.stringify(draft),
    { label: 'plan:' + lens.key, phase: 'Lens audit', schema: FINDINGS_SCHEMA },
  )
))
const allFindings = lensResults.flatMap((result) => result.findings)
requireReview(Array.isArray(draft) && allFindings.every((finding) => finding.task_id === null || draft.some((task) => task.id === finding.task_id)), 'lens returned an unknown task')
const blocking = allFindings.filter((finding) => finding.blocking)
const nonBlocking = allFindings.filter((finding) => !finding.blocking)

phase('Verify')
const verified = []
for (let offset = 0; offset < blocking.length; offset += VERIFY_BATCH_SIZE) {
  const batch = blocking.slice(offset, offset + VERIFY_BATCH_SIZE)
  const batchResults = await reviewParallel(batch.map((finding) => () =>
    reviewAgent(
      'Try to REJECT this proposed plan-admission finding.\n' +
      'Source design: ' + sourcePath + '\n' +
      'Original draft graph: ' + JSON.stringify(draft) + '\n' +
      'Finding: ' + JSON.stringify(finding) + '\n' +
      'Reject it if the citation does not support the claim, the issue is already satisfied, the route is wrong, or the finding invents scope. Default to accepted=false when uncertain.\n' +
      'Do not propose or apply edits. Do not invoke another reviewer or spawn agents.',
      { label: 'verify:' + finding.lens + ':' + (finding.task_id || 'source'), phase: 'Verify', schema: VERDICT_SCHEMA },
    )
      .then((verdict) => ({ finding, verdict }))
  ))
  verified.push(...batchResults)
}

const resourceSummary = {
  lens_agents: LENSES.length,
  verify_agents: blocking.length,
  synthesis_agents: 1,
  total_agents: LENSES.length + blocking.length + 1,
  max_parallel_agents: Math.max(
    LENSES.length,
    Math.min(VERIFY_BATCH_SIZE, blocking.length),
    1,
  ),
}

const surviving = verified
  .filter((item) => item.verdict.accepted)
  .map((item) => item.finding)
const rejected = verified
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
const report = await reviewAgent(
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

requireReview(report.verdict === verdict && report.independence === independence && canonical(report.design_findings) === canonical(designFindings) && canonical(report.graph_findings) === canonical(graphFindings) && canonical(report.flagged_for_human) === canonical(nonBlocking) && canonical(report.rejected) === canonical(rejected), 'synthesis changed the supplied verdict or findings')

return {
  verdict,
  design_findings: designFindings,
  graph_findings: graphFindings,
  flagged_for_human: nonBlocking,
  rejected,
  resource_summary: resourceSummary,
  report,
}
