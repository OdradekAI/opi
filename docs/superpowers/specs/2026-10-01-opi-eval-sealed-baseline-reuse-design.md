# Opi-eval sealed baseline reuse — Companion support (design draft)

Status: **DRAFT — unregistered.** Per AGENTS.md this file authorizes no
delivery. It awaits human review and registration; admission then flows
through `opi-implement plan`. The operator-level interim policy is already
live in the skill at
`.agents/skills/opi-eval/references/sealed-baseline.md`; this draft promotes
that manual disclosure into checked Companion behavior.

## Motivation

A sealed pi run is expensive to repeat and its comparability dimensions are
already digest-pinned and recorded (pi subject identity from the static lock,
corpus/task/image digests, connection and model controls, environment and
Companion build identity). Re-running pi for every Opi candidate trial spends
live budget without changing the baseline evidence. The Companion should
verify those identities itself and emit cross-run comparison pairs, instead
of relying on the operator's manual pairing in the human report.

## Non-goals

- The six-trial acceptance schema keeps same-run pairing; no reuse there.
- Fidelity cases and cross-benchmark aggregation are untouched.
- Sealed baseline bundles are never edited, regraded or re-sealed by reuse.
- No provider/model substitution: a reused baseline binds the exact frozen
  connection.

## Contract sketch

### Experiment declaration

Additive optional table on the experiment (single-subject candidate = Opi;
absent table keeps today's behavior):

```toml
[baseline_reuse]
root = "<absolute registered evidence-root path>"
manifest_sha256 = "<digest where a manifest exists>"
companion_sha256 = "<baseline run's Companion executable digest>"
```

`validate` resolves the root against `docs/eval/evidence-roots.jsonl`,
requires a consumable status, and rejects unregistered roots by name.

### Plan-time identity verification

`plan` preflight compares, per candidate task: baseline pi bundle digest and
subject identity, per-family material digest (corpus revision, task package,
image digests), connection plus all effective `model_controls`, recorded
environment identity, and Companion executable digest equal to
`baseline_reuse.companion_sha256`. Any mismatch or unknown marks that task
`baseline-incomparable` in the plan preview. Default is fail-closed: the plan
rejects unless the operator explicitly accepts the listed incomparable tasks
for non-paired execution. Run-plan schema gains the reuse binding and bumps;
existing plans/materials regenerate per the usual regeneration rules.

### Report

Normalized report schema bumps with a cross-run pairing section: candidate
trial to baseline trial with both bundle digests, the baseline root/manifest,
a per-dimension match table, and `comparable | incomparable` status mirroring
the fidelity vocabulary. Repetition control pairs stay same-run only; no
averaging across runs.

### Verify and regrade

Unchanged. Seals and regrading operate per run; the baseline root is
read-only input.

## Failure semantics

Fail-closed throughout: missing/unregistered root, failed seal verification
or unknown identity dimension yields a named rejection or an explicit
`incomparable` marking, never a substituted fresh pi number.

## Test sketch

- Matched tuple fixture: plan emits paired preview; report contains cross-run
  pairs with provenance.
- Mismatched material digest or Companion digest: task marked
  `baseline-incomparable`; plan rejects without explicit acceptance.
- Registry-less root: `validate` rejects with the named reason.
- Config without the table: behavior unchanged (regression guard).

## Open questions

- Partial incomparability granularity: per task vs per family at plan and
  report time.
- Whether `report` re-checks baseline seals or binds to the plan's frozen
  verification (draft prefers plan-bound).
- Whether v1 needs a recorded equivalence-statement mechanism for changed
  Companions, or ships strict same-digest only (skill policy keeps the
  statement path meanwhile).
