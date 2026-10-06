# Eval operator decoupling proposal

[简体中文](operator-decoupling.zh.md)

**Status: design revision only, 2026-10-06.** The checkout-bound operator remains
implemented. Runtime packaging, source relocation, new profile/handoff fields
and expanded capture are proposed work. This document records project-specific
migration ownership; the skill's [operator contract](../../.agents/skills/opi-eval/references/operator.md)
and [diagnostic checklist](../../.agents/skills/opi-eval/references/diagnostics.md)
carry the reusable workflow. The completed
[earlier implementation plan](../superpowers/plans/2026-10-04-opi-eval-operator.md)
remains historical evidence, not the target architecture for this revision.

## Evidence and authority

The diagnostic requirements come from the supplied local pi 1.0.2 evals analysis
at `docs/research/pi-1.0.2/packages/evals.md`, especially
sections 5, 7, 9 and 11: plan all expected tasks before execution, preserve
partial failures, measure actual prompt/usage, save native results and session
artifacts, and keep missing telemetry separate from zero. Section 9 distinguishes
score validity from session availability and performance statistics from all
observed operational consumption. These are design evidence, not an instruction
to reuse pi's private harness or change a benchmark's native scoring rules.
The research migration is separate work; its path is a provenance record,
not a required file dependency of this proposal or the self-contained checklist.

The current runtime/template admits pi 1.0.0. Reading 1.0.2 research neither
updates that pin nor proves a 1.0.2 adapter. Preserve each acquired version's
source/package/executable identity and original snapshots. No new paid run,
pin update, source repair or public packaging is authorized by this proposal.

[The normative specification](../opi-spec.md) PRIN-003 and CTRL-004 through
CTRL-007 keep policy outside Agent Core, native grading authoritative, evidence
immutable and public packaging provisional. Keep the existing unpublished
Independent Companion and Python/Rust process boundary. A local runtime bundle
does not create a new crate, dependency on Agent Core, public adapter ABI or
release workflow. Benchmark datasets, images and native grader implementations
remain separately acquired pinned material; the runtime packages integration
code and acquisition identities, not those external implementations.

## Coupling inventory and file ownership

| Current dependency | Migration owner and target |
|---|---|
| `.agents/skills/opi-eval/scripts/operator_flow.py` uses `Path(__file__).resolve().parents[4]` and implicit `REPO` | Move orchestration to `crates/opi-eval/operator/`; bind runtime package and subject source independently from the profile |
| Skill-local `evaluate.py`, `operator_config.py`, `operator_process.py`, `operator_evidence.py`, `operator_flow.py` | Move all five together into the runtime source tree; package them as one versioned executable closure |
| Skill-local `config.example.toml` supplies defaults | Move the sole template into `crates/opi-eval/operator/`; initialize an explicit private profile from the installed runtime; remove the skill copy after callers migrate |
| Skill operator directly opens `external-locks/static/linux-x86_64.json`, benchmark profiles, integration/producer/proof scripts | Runtime package owns their resource locations and digest inventory; package required producers/helpers as well as the entry point |
| Pi-only preparation still inventories/freezes the Opi checkout and builds the Companion/helper; the Opi executable is built only for candidates | Resolve/freeze the supplied evaluation runtime separately; Opi adapter owns current-tree Cargo build and source inventory; pi adapter owns admitted package acquisition/build |
| Operator invokes `scripts/opi-cargo-cache.py` | Opi build adapter owns cache coordination. Pass its shared target/lease binding through build context; retain the repository helper behind that adapter when needed, rather than calling it from the skill |
| Implicit `INDEX = REPO / "docs/eval/evidence-roots.jsonl"` and execution append | Resolve an external `storage.index`; explicit project reporting may register the returned snapshot in `docs/eval/evidence-roots.jsonl` after execution |
| `native-run.md`, `evidence-root.md`, `local-acceptance.md`, fidelity references/scripts and `SKILL.md` include crate/script/project path assumptions | Move implementation recipes/resource locations to runtime documentation and project policy. Keep portable operation/diagnostic rules in the skill; preserve acceptance, fidelity, privacy, repair and explicit-invocation boundaries |
| `scripts/test_opi_eval_operator.py` and its fixture import skill-local modules | Retain the tests and redirect imports/CLI to the runtime source; add relocation/package/diagnostic cases, rather than deleting existing 44 operator contracts |
| Prepared execution command points to the live skill script | Bind execution and offline verification to the frozen runtime entry point and compatible contract, so later skill/source changes cannot replace reviewed bytes |

The move must include the native executable, capture helper, static locks,
benchmark profiles, integrations, producer/acquisition scripts, control proof,
runtime dependencies and compatible contract documentation. A missing entry in
this closure is a preparation refusal. Do not replace source-tree coupling with
an installation that still imports files from the checkout.

Project-specific source repairs, implementation ledger ownership, bilingual
conclusions and acceptance policy remain caller context. Separating that context
from benchmark execution must not drop the original workflow gates. A caller
may supply project reporting destinations, but their absence cannot prevent
pi-only preparation or offline comparison.

## Configuration and operation contracts

Keep all editable selections in one private TOML. Retain model, endpoint, one
credential source, thinking level/required, retry limits, concurrency, full-batch
deadline, benchmark selection and workspace/cache. Introduce a versioned profile
with `runtime.package`, `subjects.opi.source`, `storage.index` and
`diagnostics.required` as defined by the operator proposal. The current `/1`
parser rejects these keys; do not edit a real profile to use them yet.

Use explicit profile input, normalize paths against its location, and freeze
sanitized effective values/origins. Resolve credentials only for the authorized
live subject launch. The runtime checks requested diagnostic capabilities
before dispatch; optional unavailable measurements remain named gaps.

Retain `init`, `resolve`, baseline/candidate preparation, frozen execution and
offline `compare` as the small process interface. Package metadata declares
operation/handoff/profile versions, supported pi pins/families/controls,
enforceable limits and capture capabilities; it is descriptive, not an arbitrary
command/plugin registry. No new command syntax is promised before implementation
and generated-help/subprocess checks.

Version affected durable contracts when adding the diagnostic dimensions
`score_validity`, `diagnostic_completeness` and `comparison_eligibility`.
Keep readable `/1` snapshots and native seals unchanged. Explicitly translate
an old private profile into a fresh profile only on request; do not copy secrets
into reports or silently add required diagnostics. Import discovery references
without rewriting old evidence; older unmeasured facts are unknown.

## Execution and retained results

1. Resolve the runtime and profile -> verify: package contents, capabilities,
   path separation, credential-free exports and requested admission bounds.
2. Prepare a pinned pi package or build current Opi -> verify: separate runtime
   and subject identities, relevant dirty inputs, build-before/after equality,
   frozen executable/material/plan and exact-client control proof.
3. Freeze the full expected inventory and selected baseline -> verify: all
   logical slots, diagnostic requirements, maximum attempts, original deadline,
   eligible baseline identity or explicit missing/incomparable status.
4. Run precisely the authorized plan -> verify: bounded incremental attempt
   records, first-valid settlement, failed-attempt consumption and supported
   cancellation/cleanup; no automatic pi call or retry of valid zero.
5. Reconstruct from originals and return artifact references -> verify: seals,
   deterministic reports, independent diagnostic status, native/paired metric
   denominators, immutable comparison manifest and external index registration.

Each operation retains its frozen runtime/subject packages, sanitized inputs,
protocol/expected slots, observations/attempts, original native runs, diagnostic
artifacts, verification outputs, snapshot and machine handoff outside source.
Collection is bounded; missing session, effective prompt hash, per-call timing
or final-workspace coverage is declared until real adapter capture proves it.
Console output and `full` capture do not supply those facts by themselves.

Pi baselines are keyed by admitted version and the actual benchmark/model/
control/environment/grader identities, then selected deterministically before
candidate dispatch. Full benchmark refreshes on a pi version update are explicit
operations, never a side effect of an Opi candidate. Keep the runtime fixed
across candidate source changes; changed evaluation/grading paths require
reviewed equivalence or an incomparable comparison. Preserve the old runtime
alongside historical evidence where offline re-verification needs it.

## Migration order and acceptance

All stages below are planned, not completed runtime work:

- [ ] **Package the existing execution closure.** Move operator modules/template,
  bind runtime resources locally, and retain current offline contracts.
  Verify: package relocation, missing/changed resource refusal, unsupported
  contract refusal and CLI help/negative subprocess checks.
- [ ] **Remove implicit source/index dependencies.** Add the versioned explicit
  profile; separate runtime resolution from the real pi/Opi subject build paths.
  Verify: pi-only preparation without an Opi checkout; two relocated dirty Opi
  trees with identical runtime identity and distinct subject identities;
  failed builds never reuse old executables; shared Cargo cache leases settle.
- [ ] **Bind frozen execution and migrate discovery.** Preserve original evidence
  and import existing index references. Verify: source/skill removal does not
  break an already prepared synthetic operation; offline comparisons run with
  no credentials/network and a read-only source tree; old seals are unchanged.
- [ ] **Add adapter-backed diagnostic capabilities.** Implement only facts
  actually exposed by the selected clients and native runners. Verify: all
  [diagnostic cases](../../.agents/skills/opi-eval/references/diagnostics.md#verification-cases),
  required-capability preflight, failed partial captures, privacy and size bounds,
  actual first-request/tool-continuation proofs and native grader conformance.
- [ ] **Finish skill/caller migration.** Remove runtime modules/template and
  checkout recipes from the skill after runtime/caller checks pass; update both
  skill manuals and current crate README counterparts. Verify: a relocated
  skill reads only its own resources and returned versioned references; no
  product internals are imported; original acceptance/fidelity/repair gates and
  parent remaining-budget supervision still hold.

Execution requires separately confirmed runtime change scope. No ledger write,
paid benchmark, release or commit is part of this design revision. Select exact
checks from the actual changed surfaces when implementing: script behavior and
subprocess/package negatives; affected native integration/proof tests; crate
tests/clippy if Rust changes; documentation and whitespace checks. Fixtures
prove infrastructure behavior, not pi/Opi benchmark performance.

## Revision verification

This revision changes documentation only. Test impact: **none**; existing runtime
tests are retained unchanged. Documentation impact: **add** the diagnostic
reference and bilingual migration proposal; **update** operator/skill/manual
guidance; **delete** none. Private profiles, benchmark results and seals remain
unchanged. Run `python scripts/opi-doc-check.py` and `git diff --check`;
record actual outcomes in the user handoff. No Cargo/runtime checks are claimed
for this unimplemented migration.

The generic skill-creator validator rejects the pre-existing
`disable-model-invocation` frontmatter key because its whitelist omits it.
Preserve that explicit-invocation rule; the repository documentation checker
accepts it. This is a validator compatibility limit, not a new runtime defect.
