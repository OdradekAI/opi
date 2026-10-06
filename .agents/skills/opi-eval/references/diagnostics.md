# Evaluation diagnostic checklist

Read before freezing capture requirements and when diagnosing a retained
benchmark result. This checklist is self-contained. The runtime returns
artifact references and capability status; the skill does not import Agent
source modules or infer native file layouts.

- [Retention](#retention-by-evidence-role)
- [Validity and gaps](#validity-and-gaps)
- [Comparison](#comparison-and-consumption)
- [Verification](#verification-cases)

## Retention by evidence role

Apply the same requested retention policy to pi and Opi. Preserve partial
diagnostics on failure, cancellation and retries, together with the full
expected inventory. Each observed fact carries its trial/attempt identity,
source artifact reference and measurement scope. Each absent fact carries a
reason such as `unsupported`, `not_collected`, `capture_truncated`,
`privacy_withheld` or `artifact_missing`; these are proposed diagnostic statuses,
not an assertion about fields supported by an existing native format.

| Evidence role | Retain | Meaning and limit |
|---|---|---|
| Protocol and expected inventory | Sanitized effective inputs/origins; subjects, runtime/build/package digests; benchmark revision, task/grader/image identities; model/connection/controls, tools, repetitions, bounds and capture policy; every expected logical slot and protocol digest before dispatch | A protocol digest identifies inputs, not reproducible remote model output. Never include credential values or hash private secret-bearing profiles. |
| Observations and attempts | Durable per-attempt dispatch/settlement records, native result location, failure category, cancelled/unstarted/effect-unknown states, retry links and first-valid selection | Persist incrementally after each settlement. A crash may leave an attempt unresolved; keep it explicit rather than deleting the slot. |
| Native grading | Original native job/result trees, grader inputs/outputs, score and native definition, completion/failure records, integrity decisions and provenance | Shell exit alone is not a score. Preserve valid zero and benchmark-defined valid timeouts. Infrastructure/adapter failures are distinct. |
| Agent trajectory | Task input, ordered user/assistant messages, public output, tool names/arguments/results and errors, turn/call correlation, retry/compaction events where exposed | Retain permitted structured events from the actual client. A CLI console transcript is not proof of a complete trajectory. Exclude private raw thinking and secrets before sealing. |
| System prompt | SHA-256 of the actual effective system-prompt bytes passed to the model at a recorded request/turn; record prompt changes separately | Do not substitute the user prompt, config digest or a reconstructed prompt. Keep hash provenance; preserve prompt text only where privacy policy permits it. |
| Native session | Original session artifact, format/client version and digest, or explicit absence; scope changes across reload/resume if applicable | A session may help reproduce context but does not guarantee resumability. Source-format retention and a derived sanitized transcript are distinct. |
| Usage and timing | Per-trial and per-call input/output/cache/reasoning token facts where exposed, tool/call counts and durations, retry/compaction, trial wall time, time-to-first-token/critical path only if measured | Record units and observation scope. Command wall time is not provider or task latency. Thinking configuration is not observed reasoning-token usage. |
| Cost | Provider-reported or estimated amounts, currency, price source/version and observed usage coverage, including failed attempts | Missing prices or all-zero placeholder pricing means unavailable. Keep estimates separate from billed cost; measured zero requires evidence. |
| Delivered artifacts | Collected files/patches with relative path, size/digest and declared collection scope; exact files supplied to the grader | An artifact allowlist is not a complete final filesystem. If full workspace retention is requested, capability, privacy and size bounds must be resolved before dispatch. |
| Verification and handoff | Original seals, verification exits and reports, two byte-identical offline reconstructions, snapshot/comparison bindings, cleanup status and all retained artifact references | Score evidence and diagnostic derivatives have distinct digests. Retain originals unchanged; unknown privacy status blocks sharing. |

## Validity and gaps

Assess three independent questions:

1. **Score validity:** does verified native grading establish a benchmark-valid
   outcome for the planned subject/task/attempt? The native adapter owns the
   exact score range, completion semantics and identity checks.
2. **Diagnostic completeness:** which requested explanatory artifacts and
   measurements exist, are bound to this trial and are safe to retain/share?
3. **Comparison eligibility:** do both verified observations share the frozen
   comparison controls and identities, with disclosed cross-run limits?

A missing session, prompt digest or cost measurement can limit diagnosis while
leaving a native score valid. Duplicate observations, a changed subject/model,
invalid native result or missing seal may instead defeat score validity or
pairing. Do not transfer the reference experiment's assertion count, score
threshold or private harness into a different benchmark's grading contract.

For a new run, freeze any required diagnostic capabilities before dispatch.
Missing required capability blocks preparation; an optional gap is disclosed.
If required capture is lost during execution, preserve the native result and
report incomplete diagnostics separately. Do not rerun a valid trial solely
to obtain a more convenient diagnostic record without a newly authorized run.
Historical results keep their original scoring and capture contract.

Capture errors and buffer/size limits must be visible. Extra retention is
bounded and privacy-reviewed before execution; it never authorizes private
reasoning capture, arbitrary provider HTTP logging, credential retention or
editing sealed artifacts. A safe derivative links to its original and cannot
replace seal-bound native evidence.

## Comparison and consumption

Freeze a comparison manifest over both original snapshots. Pair by benchmark
and task identity, repetition/seed, requested model/connection and effective
controls, Agent role, tools/environment and native grading path. Product
versions are the intended differing subject input; shared controls must match.
Missing, duplicated, invalid and excluded pairs remain visible.

Keep each run's native aggregate and full denominator. Report paired deltas
with the comparable intersection and its coverage. When any expected pair is
blocked, withhold a **full-corpus paired headline**; an explicitly labelled
partial paired diagnostic may still be shown. Never relabel that subset as a
complete pi/Opi comparison.

For each efficiency metric, use only pairs where both sides have compatible
measurements; give its own denominator and unavailable count. Operational
totals separately account for all observed attempts, including failures and
retries, with coverage rather than a fabricated complete total. Do not sum
multiple valid attempts into one logical trial or select their best score.

Saved pi baselines and new Opi runs are cross-run observations. Report their
timestamps and remote model-version uncertainty; default single repetitions
support no stability, causal or statistical-significance claim. A low score is
data, not automatically an infrastructure defect or retry trigger.

## Verification cases

Runtime/adapter conformance should exercise these cases without paid calls:

- Full inventory survives an interrupted dispatch and a partially written
  result; attempted and unstarted counts remain distinct.
- A valid native zero is completed; a missing score and infrastructure failure
  stay distinct under the native benchmark's rules.
- Failed attempts retain available messages, usage and delivered artifacts.
- A valid score with no native session has a diagnostic gap; a required missing
  prompt-hash capability blocks a new preparation.
- Actual subject/provider/model mismatch rejects the applicable native binding;
  a requested selector cannot stand in for an observed service identity.
- Duplicate/missing comparison sides block the full paired headline; a metric
  absent on one side is excluded only from that metric's paired denominator.
- Unavailable pricing produces no zero-cost claim; failed-attempt consumption
  remains in operational totals with its measurement coverage.
- A truncated or privacy-withheld trajectory is labelled incomplete. Original
  seals and historical scores remain unchanged.
- Exact-client synthetic proofs check first request and tool continuation;
  fixture success is kept separate from real benchmark performance evidence.
