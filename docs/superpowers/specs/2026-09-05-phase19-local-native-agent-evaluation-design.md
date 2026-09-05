# Phase 19: Local Native Agent Evaluation

[简体中文](2026-09-05-phase19-local-native-agent-evaluation-design.zh.md)

## 1. Authority and outcome

This delivery specification materializes the human-reviewed local evaluation
direction. The English document owns Phase meaning; its Chinese counterpart
preserves the same identifiers and requirements. Source registration is owned
by the reviewed registry in
[`opi-implement`](../../../.agents/skills/opi-implement/SKILL.md). Registration
does not admit a task graph, start implementation, authorize provider calls, or
record completion. Only an explicitly invoked `opi-implement plan` may admit
the source and install a reviewed graph in `.opi-impl-state.json`.

The normative parent is [`docs/opi-spec.md`](../../opi-spec.md), and domain
language remains in [`docs/CONTEXT.md`](../../CONTEXT.md). This Phase derives
from **STRAT-002 — Establish independent cross-Agent Eval**. Its end-to-end
owner is the Evaluation product owner.

The outcome is an independent `opi-eval` local workflow: a user configures a
provider, selects Opi, pi, or both, runs pinned Terminal-Bench 2.1,
Terminal-Bench 3.0, or Datacurve DeepSWE v1.1 tasks, and obtains native outcomes
that trace to those Agents' actual task artifacts. Task execution, grading,
evidence, and reports stay on the user's computer. Model inference may use the
user's remote API. The acceptance platform is Windows x86_64 with WSL2 Linux
and a local Docker Linux engine.

| Parent clauses | Phase responsibility |
|---|---|
| `GOAL-004`, `PRIN-001`–`PRIN-003`, `PLACE-001`–`PLACE-004` | Keep the Companion independent of Opi product crates and Minimal Runtime activation. |
| `PRIN-004`–`PRIN-005`, `CTRL-002`–`CTRL-003` | Fail closed on invalid identity or evidence; retain effective controls and redact secrets. |
| `CTRL-004` | Grade the selected Agent's real artifacts with the admitted native grader. |
| `CTRL-005` | Freeze comparisons and retain selected/full coverage, missing pairs, exclusions, and failure classifications. |
| `CTRL-006` | Preserve immutable bundles, offline report reconstruction, and fresh identities for new execution. |
| `CTRL-007`, `GATE-001`–`GATE-002` | Keep native runners and task materials external; demonstrate Opi/pi and Harbor/Pier integration without prematurely stabilizing a public SDK. |
| `PHASE-001`–`PHASE-006` | Deliver this finite scope with explicit acceptance, risk, rollback, and platform boundaries. |

## 2. Problem, evidence, and priority

The existing conformance foundation has reusable experiment, lifecycle,
process-supervision, sealing, and reporting mechanisms. Its native producer
runs Opi/pi separately from the task job; the grading path does not consume
their output, and the pinned runners default to oracle when no Agent is
selected. A native reward therefore does not establish an Opi/pi score.
The current local entry also depends on fixture-oriented inputs and fixed
scripted-provider configuration. Correct score attribution precedes broader
experiments and convenience work.

The following are **non-normative evidence**, not substitute authority:

- [Phase18 execution-gap investigation](../../research/2026-09-05-opi-eval-phase18-execution-gap.zh.md): source-level dataflow and historical log evidence, without a new dynamic reproduction or full historical artifact audit.
- [Windows feasibility investigation](../../research/2026-09-05-local-agent-benchmark-windows-feasibility.zh.md): pinned task semantics and observed host limitations; installation is not proof that the engine or tasks run.
- [Zhipu endpoint investigation](../../research/2026-09-05-zhipu-eval-provider-endpoints.zh.md): endpoint documentation, with model access and tool-loop compatibility still requiring execution evidence.
- [Frozen Phase18 specification](../../snapshots/phase18/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md): historical delivery scope, not an active source for this Phase.
- [Current Companion README](../../../crates/opi-eval/README.md), [benchmark profiles](../../../crates/opi-eval/profiles/benchmarks/), and [external lock](../../../crates/opi-eval/external-locks/static/linux-x86_64.json): current implementation and material identities, not proof of the proposed behavior.

Phase19 corrects and completes the independent Eval route; it does not advance
to Learning or Promotion. Historical snapshots and receipts remain unchanged.

## 3. Scope and placement

The selected approach extends the existing Rust Companion CLI and experiment
configuration, calling pinned external Harbor/Pier runners with real Opi/pi
integrations. The Companion owns selection, configuration projection,
supervision, evidence binding, and reporting. The runner owns task environment
creation, official artifact collection, and native verifier execution. Each
Agent retains its own provider client, tools, and native configuration.

This is an Independent Companion placement: pi-only evaluation has complete
value without an Opi executable; failure cannot change ordinary Opi behavior;
and removing the Companion removes evaluation policy without moving it into
Agent Core. Rust types and explicit ownership protect local contracts, while
external processes preserve cross-language reuse. Reuse the existing adapter
and lifecycle boundaries before introducing a new trait, crate, or protocol.
Material acquisition and runner integration do not embed datasets, grader
implementations, container images, or a new sandbox engine in the crate.

Hand-running multiple upstream tools and manually matching their outputs does
not deliver the agreed local entry. Implementing a second container runner
duplicates upstream task semantics without a demonstrated need. Publication,
repository extraction, and stable third-party APIs are separate decisions.

Non-goals:

- A native PowerShell execution path for these Linux task sets, remote workers,
  hosted scheduling, a GUI, or leaderboard submission.
- Completion of every resource-intensive TB3 task on this PC; full resource
  preflight is required, while real acceptance uses an explicit feasible subset.
- A performance season, statistically significant superiority claim, or
  requirement to beat pi or obtain a positive benchmark score.
- A new Agent Core evidence/provider seam, Eval-specific LLM client, additional
  Agent or benchmark, Learning, Promotion, or automatic activation of results.
- Eighteen mandatory trials across all three API protocols; only the first
  selected connection requires real-provider acceptance in this Phase.

## 4. Local workflow and configuration

The built program and its declared integration materials operate from an
independent evaluation directory. A repository checkout may be used to build
them, but runtime does not require repository fixtures, a GitHub run/artifact
identifier, workflow dispatch, upload, or the project skill named `opi-eval`.
Public source and image downloads remain ordinary material acquisition.

| User operation | Required observable behavior |
|---|---|
| Diagnose and validate | Report Linux engine reachability, architecture, available resources, missing materials, and invalid configuration without starting a model. |
| Prepare materials | Resolve pinned task packages, runner dependencies, Agent executables, and images into a local cache; emit verified material identities and actionable failures. No model calls. |
| Preview an experiment | Show subjects, connection, task list and selection rule, repetitions, total trial count, concurrency, bounds, and resource failures before execution. |
| Run | Execute the frozen plan through selected native integrations; show task, Agent, lifecycle stage, settlement, and score/error, and save a structured local result. |
| Recover or replace | Classify durable completion, cancellation, failure, and effect uncertainty. Replacement uses new trial identities and preserves prior evidence and paired-group rules. |
| Verify, recompute, or regrade | Distinguish byte verification, report reconstruction, and actual native regrading. None invokes an Agent or provider. |

Use the existing TOML experiment entry and CLI where their semantics remain
accurate. The admitted graph determines exact field and command spelling,
updates help/examples/callers together, and gives each operation a concrete
public test seam. A byte-verification operation cannot retain documentation
that calls it actual native regrading. Live execution is explicit in the
selected configuration and run operation; credential presence never selects it.

The configuration carries these semantic inputs:

| Object | Required contents |
|---|---|
| Subjects | One or more selected Agent implementations, executable/runtime identity, isolated native configuration, and any declared comparison edges. |
| Connection | Provider identity, protocol, base URL, exact model ID, and local credential reference. Protocol names do not rename the provider. |
| Common controls | Effective reasoning, sampling, context/output limits and their origins; required unsupported controls prevent a strict comparison. |
| Task selection | Corpus source/revision, complete task inventory, explicit full/subset selection, task material identities, repetitions and seeds where applicable. |
| Execution bounds | Positive bounded concurrency, planned trial count, supervision limits, and user-authorized usage/spend limits with their enforcement capabilities. |
| Storage and capture | Local cache and result locations, evidence-capture policy, and private credential inputs separated from exportable artifacts. |

Credentials come from user-selected environment references or private local
files. Resolve them only for the selected Agent process, without reading or
rewriting user-global Agent configuration. The grader receives neither provider
credentials nor unrelated host files. Secret configuration is excluded from
task handoff; in a shared environment, remove temporary credential material
before verifier execution and sanitize its process environment. Do not mount
the host Docker socket or broad host directories into an Agent task to achieve
tool access.

The first real comparison uses the user-selected connection below. The other
two entries are optional independently named connections. All three projections
receive deterministic request-path, authentication, and tool-loop fixture
coverage for both Agents; this does not claim real-service qualification for
the optional connections.

| Protocol | Base URL | Native API | First real acceptance |
|---|---|---|---|
| Anthropic Messages | `https://open.bigmodel.cn/api/anthropic` | `anthropic-messages` | Zhipu `glm-5.3-flash`, Opi and pi |
| OpenAI Chat Completions | `https://open.bigmodel.cn/api/coding/paas/v4` | `openai-completions` | Optional |
| OpenAI Responses | `https://open.bigmodel.cn/api/v1` | `openai-responses` | Optional |

No implicit model, endpoint, protocol, or billing-product fallback is allowed.
Native Agent model selectors may differ syntactically; their actual request
identity and effective controls establish comparability. Missing service-model
identity remains unknown with its reason; it cannot be fabricated from config.
Unsupported access or controls fail at the owning boundary. A deliberate
configuration change creates a new experiment identity. Monetary estimates,
provider-reported usage, and billed amounts remain distinct; unknown prices
cannot support a guaranteed currency cap. A requested bound that cannot be
enforced is rejected or explicitly revised before execution, never ignored.

## 5. Native execution and material contract

An upstream job explicitly selects the Opi/pi integration and binds its actual
Agent, task, and trial identity. That integration installs/invokes the selected
Agent in the task environment, so its file and shell tools operate there. The
same upstream trial supplies the official handoff and native grading result.
An independent host Agent followed by a second unspecified runner job is not a
valid integration. Missing, oracle, or mismatched actual Agent identity rejects
the score even when every subprocess exits successfully.

| Task family | Handoff and grader boundary |
|---|---|
| TB2.1 | Honor the selected task's declared lifecycle; for shared-verifier tasks, grade that Agent's resulting task environment after credential cleanup. |
| TB3 | Honor per-task verifier isolation and collect declarations; for separate-verifier tasks, copy only the declared Agent artifacts into the verifier environment. |
| DeepSWE v1.1 | Honor the task's base revision and collect hook; collect the Agent's committed patch and apply it in the pristine, separate verifier under its declared network policy. Uncommitted work is not silently substituted for the patch. |

The family rows are integration obligations, not overrides for every task's
configuration. Parse each selected task and preserve its network, CPU, memory,
GPU, disk, Agent timeout, and verifier timeout requirements. Provider access
uses the runner's supported Agent-side inference route; it does not widen task
or grader network access. Unsupported lifecycle or resource declarations cause
an explicit rejection, not an approximate task run labelled native-equivalent.

Reuse the current pinned corpus and Harbor/Pier identities as the starting
material set. Generalize task selection beyond the three current fixed smoke
profiles. Every selected task gets a verified file/digest inventory, instructions,
environment, collection and native-grader identity. Resolve mutable image tags
to immutable material identities before the trial plan freezes. An upstream
revision change requires new integrity admission and configuration identity.
Local material verification replaces a mandatory GitHub upload-receipt
prerequisite; it retains byte/provenance verification and records local origin
truthfully. It does not manufacture GitHub provenance for local materials.

Full preflight enumerates the pinned corpus and reports per-task requirements
against resources visible inside WSL/Docker, including unknown observations.
Insufficient GPU type, memory, disk, missing images, or unsupported task
semantics cannot silently shrink the requested set. A user-selected feasible
subset freezes explicit task IDs and selection reasons. Full planning does not
require downloading or starting every task image merely to report requirements.

Oracle execution is an explicitly selected, separately identified diagnostic.
It never supplies an Agent result, fills a missing pair, or changes the scored
denominator. For each family, a controlled correct/incorrect artifact witness
passes through the same production handoff and real native grader, proving
that grading responds to the submitted artifact. Such witnesses remain
conformance evidence, separate from the six real-provider trials.

## 6. Evidence, failure, and reporting

Extend existing immutable bundles only where the working integrations require
it. Each score binds subject/executable identity, effective connection and
controls, task and runner revisions, actual upstream job/trial/Agent identity,
environment and resource identity, Agent trajectory, delivered artifact or
patch digest, verifier input identity, raw native output, and its interpretation.
Preserve native artifacts when normalization cannot represent their semantics.
Do not impose a new canonical trace format beyond the demonstrated shared seam.

| Outcome | Reporting treatment |
|---|---|
| Valid native success or Agent failure | Preserve the native metric and benchmark-defined outcome, including zero scores and valid task timeouts. |
| Infrastructure, provider access, adapter, identity, or invalid-grading failure | Preserve the reason and attempted/planned coverage; do not score it as Agent success or failure. |
| Cancelled, unstarted, or effect-unknown | Preserve the lifecycle status and missing coverage; do not invent an outcome or silently rerun. |

Each benchmark report shows its metric definition and denominator alongside
corpus size, selected tasks, planned and attempted trials, valid graded trials,
Agent failures, invalid/infrastructure failures, and unexecuted items. A subset
rate is explicitly labelled subset; an incomplete full experiment cannot claim
a complete full score. Task success and test-case pass fraction are different
metrics. Results from the three benchmarks are not averaged into one headline.
Paired comparisons use the same task/repetition and frozen controls, expose
missing pairs and differences, and never match different protocol groups.
Single-subject reports require no artificial comparison edge or absent Agent.

Offline byte verification checks original seals. Report reconstruction consumes
verified native results and regenerates deterministic output without network
or model access. Actual regrading consumes preserved real Agent artifacts with
the pinned native grader in a fresh environment, preserving the task's verifier
rules; it emits a new derived grading record referencing the source bundle,
without rewriting the original score. Missing required artifacts, cached
environment materials, or an offline equivalent permitted by the verifier for
a required external service rejects offline regrading. Byte checking alone
does not satisfy this requirement.

## 7. Requirements and acceptance scenarios

Every normative obligation below has an owner and an observable verification
route. The acceptance identifiers are the source criteria for graph extraction.
The operations above describe required behavior, not commands already shipped.

| ID | Requirement | Owner | Mechanical verification |
|---|---|---|---|
| P19-AC-001 | The CLI **MUST** complete validation, material preparation, execution, and reporting from an independent local directory without repository fixtures or GitHub execution metadata. | Evaluation product owner | Assembled CLI acceptance with declared external materials and a local output tree. |
| P19-AC-002 | Native results **MUST** bind the actual selected Agent and its delivered artifacts; missing/mismatched identities and oracle results **MUST NOT** become Agent scores. | Native integration owner | Inject missing/wrong Agent, substituted artifact, stale job, and oracle output through the production result reader; each is rejected. |
| P19-AC-003 | TB2.1 **MUST** grade the real trial environment under the task's native lifecycle. | TB2.1 integration owner | Correct/incorrect native witness plus both real Agent traces, tool effects, and graded artifacts. |
| P19-AC-004 | TB3 **MUST** preserve declared collection and separate-verifier semantics where required. | TB3 integration owner | Native witness changes only a declared artifact; unrelated files do not enter the verifier; both real Agents traverse that handoff. |
| P19-AC-005 | DeepSWE **MUST** grade the collected committed patch in the declared pristine verifier. | DeepSWE integration owner | Correct/incorrect patch witnesses, base/patch/verifier identity checks, and both real Agents' patch-to-result traces. |
| P19-AC-006 | Provider projection **MUST** consume the selected protocol, endpoint, exact model, credential reference, and effective controls for each Agent. | Agent integration owner | Both native Agent clients against local fake servers for all three protocols; missing credentials/control mismatch rejected; first connection proven by real tool-loop evidence. |
| P19-AC-007 | Resource preflight and selection **MUST** preserve per-task requirements and explicit full/subset scope. | Material and selection owner | Enumerate the full pinned inventories without models; reject an unmet TB3 resource requirement; run a frozen feasible subset without changing limits or scope. |
| P19-AC-008 | Reports **MUST** retain native metric meanings, failure classes, denominators, pair coverage, single-subject results, and unknown telemetry reasons. | Report owner | Public report-path fixtures covering valid zero, partial/full/subset, missing pairs, provider failure, and protocol mismatch. |
| P19-AC-009 | Verification, recomputation, and native regrading **MUST** be distinct, offline-capable operations preserving immutable source evidence. | Evidence owner | Tamper rejection, deterministic reconstruction, fresh native regrade of saved artifacts, missing-cache rejection, and zero Agent/provider invocations. |
| P19-AC-010 | Credentials, supervision, cancellation, and recovery **MUST** respect local authority and preserve effects and evidence. | Lifecycle and privacy owner | Canary credentials absent from grader/export; supervised cancellation cleans only trial-owned processes/resources; recovery retains unknown effects and replaces with new identities. |
| P19-AC-011 | Default tests and CI **MUST** remain deterministic and credential-free; local real acceptance **MUST** be explicit and separate. | Test and Evaluation product owners | Populated dummy credential environment does not enable live tests; local native checks can run without dispatch; real acceptance report is independently identified. |
| P19-AC-012 | Phase exit **MUST** include at least six real Agent trials: one feasible task per benchmark, each run once by Opi and pi with the selected first connection, followed by native grading. | Evaluation product owner and local operator | Frozen run manifest, redacted provider/tool evidence, six attributable native results, bundles, and reproducible local reports. |
| P19-AC-013 | The Companion **MUST** preserve independent placement and deliberately record changed 0.x contracts. | Companion owner | Cargo dependency/package review; pi-only acceptance without Opi; unchanged Minimal Runtime; synchronized help, schemas, fixtures, bilingual README, and Unreleased entries for actual changes. |

The six trials prove the product loop, not statistical significance. A real
Agent may fail the task and receive zero; that can satisfy the execution
criterion. An authentication failure, missing tool loop, invalid artifact
handoff, or infrastructure failure cannot substitute for one of the six.
Failed attempts remain in the evidence even when replacement trials are needed.

## 8. Verification and local acceptance handoff

Graph admission assigns each criterion to a scenario-owning vertical slice,
with exact CLI/test commands and real production call sites. Reuse the current
`assembled_run`, `native_driver`, Agent/benchmark conformance, bundle, report,
and authority test surfaces where they exercise the claimed path. New tests
require a missing behavioral case rather than a Phase-specific copy of an
existing assertion. No test calls a paid provider or becomes live because a
credential happens to exist.

Native artifact witnesses run locally with pinned external materials and no
live model. CI may run an explicitly bounded native conformance smoke, but is
not the product's runtime or source of user scores. Existing GitHub workflow
entry points may remain as thin regression callers of the same local path;
they cannot own a second materialization or scoring policy.

Real acceptance is a separately authorized local operator run of the Companion,
outside the `opi-implement` harness, which forbids provider calls and reading
real auth files. The harness may verify supplied redacted evidence offline;
this specification does not waive its restrictions. Until actual evidence is
present and verified, `P19-AC-012` remains not met and Phase exit is unavailable.
Pure deterministic development can proceed without an API key.

Before that run, the local operator freezes the actual tasks, Agent builds,
runner/material digests, model controls, concurrency, usage/time budget, and
credential references. Candidate tasks already researched are
`openssl-selfsigned-cert`, `batched-eval-parity`, and `abs-module-cache-flags`;
these are candidates until local resource/image checks and the operator's
selection freeze them. The selected account's model access and billing-product
eligibility remain explicit inputs; an endpoint alone does not prove them.
No credentials or spending are authorized by source registration.

The local acceptance handoff preserves commands, exit codes, redacted logs,
requested/effective configuration, material and bundle digests, per-criterion
results, and report paths. Dynamic claims need preserved artifacts; source
inference and old workflow success do not close them. Runtime evidence stays
outside this specification, using the project workflow's evidence/snapshot
locations and private local storage where necessary.

Phase exit also requires the repository's verification union, run once for the
final relevant state:

```sh
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Use the repository's external Cargo cache and platform wrappers. The WSL/Linux
product path needs actual local acceptance; Windows/macOS compilation or hosted
Linux success cannot substitute for it. Docs-only source shaping uses only the
documentation checks. These commands are future verification requirements,
not claims that they were run while writing this document.

## 9. Risk thresholds, rollback, and compatibility

Any single occurrence of these conditions blocks the affected score and Phase
exit: oracle/other-Agent attribution, substituted or unbound verifier input,
tampered accepted evidence, silent model/protocol/resource fallback, an invalid
trial counted as Agent success/failure, a subset relabelled full, secret exposure
to a grader/export, or unbounded/orphaned execution after claimed cancellation.
The tolerated count is zero. A missing required trial or unverifiable required
measurement blocks its corresponding acceptance or claim; no score compensates.

On a budget, authority, provenance, or privacy breach, stop new affected trials,
cancel owned execution through supervision, preserve observed outcomes and
private diagnostic evidence, and withhold invalid reports. Resource cleanup
is limited to identified trial-owned containers/processes and temporary
credentials; retained bundles, user directories, and shared caches are not
cleanup targets. Monetary bounds retain the enforcement limits in Section 4.

Rollback disables the affected optional local execution path or restores a
previous verified Companion build. If the earlier build has the known grading
attribution gap, it is usable only for accurately labelled conformance and
byte verification, not renewed Agent-score publication. Do not keep competing
old/new grading paths active or silently switch between them. Reopening real
execution requires a corrected identity and explicit run scope.

Existing Opi/pi configuration, credentials, sessions, and ordinary runtime
behavior remain untouched. Preserve original bundles and reports byte-for-byte;
new readers identify legacy evidence as legacy/conformance or unsupported,
never upgrade its score claims by relabelling. Changed experiment, material,
bundle, report, or CLI contracts are deliberately scoped 0.x changes with
updated schemas/callers/docs and Unreleased history. No automatic migration or
compatibility facade is required.

## 10. Delivery decomposition and admission handoff

These are source outcomes, not installed tasks or a second progress ledger.
Derive task IDs and necessary splits during `opi-implement plan`; preserve the
source IDs in the graph. Each split needs its own observable completion and
must remain in the dependency closure of its scenario owner.

| Source ID | Delivery outcome and owner | Prerequisites | Acceptance / completion evidence |
|---|---|---|---|
| P19-D01 | Selected-Agent native trial and TB2.1 score binding; native integration owner | Existing Companion foundation | `AC-002`, `AC-003`: CLI-to-native correct/incorrect witness and wrong/oracle identity rejection. |
| P19-D02 | User connection projection through both real Agent clients; Agent integration owner | D01's resolved trial identity | `AC-006`, credential part of `AC-010`: three protocol fake-server loops with isolated native config; selected real route remains for D08. |
| P19-D03 | TB3 declared artifact handoff; TB3 integration owner | D01 | `AC-004`: attributable native grading across the required environment boundary. |
| P19-D04 | DeepSWE committed patch handoff; DeepSWE integration owner | D01 | `AC-005`: base/patch/pristine-verifier provenance and native positive/negative witness. |
| P19-D05 | Independent local preparation, full preflight, and explicit multi-task selection; local workflow owner | D01–D04 configuration/task/material contracts | Preparation/selection portion of `AC-001`, and `AC-007`: verified local materials and frozen selection without fixtures or GitHub receipt dependencies. |
| P19-D06 | Native outcome reports and offline artifact operations; evidence/report owner | D01–D04 | `AC-008`, `AC-009`: honest coverage, single-subject/paired reports and real offline regrading. |
| P19-D07 | Bounded recovery and CI/product entry alignment; lifecycle/product owner | D02, D05, D06 | `AC-010`, `AC-011`, `AC-013`: cancellation/credential/independence checks, locally reproducible deterministic gates and synchronized user documentation. |
| P19-D08 | Local acceptance evidence verification and Phase exit; Evaluation product owner | D02–D07 plus separately authorized operator evidence | `AC-012` and all prior criteria: six real trials, native outcomes and offline reconstruction verified; required workspace gates pass. |

D01–D07 close their stated deterministic/native wiring evidence. D08 owns the
assembled real workflow in `AC-001`, the real-provider portions of
`AC-003`–`AC-006`, and `AC-012`; an earlier component's passing witness does
not close those live product scenarios.

The source can be admitted without credentials or a purchased provider plan;
those are runtime inputs, not missing product design. Exact controls and task
selection are frozen before the authorized run, not guessed by implementation.
Any discovery requiring a new core seam, different native semantics, broader
authority, or weakened acceptance returns to human shaping under `PHASE-006`.
Learning, larger performance studies, other protocols' real qualification,
publication, and native Windows execution remain separate follow-up scope.
