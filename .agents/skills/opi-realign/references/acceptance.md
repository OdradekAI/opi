# Reproduction acceptance and recommendation cards

Derive expected behavior from the supplied pi snapshot, not this document's
examples, a previous report, or an external acceptance document's revision.
This reference defines the method and engineering proof obligations; snapshot-
specific constants, provider counts, test counts and resource budgets are
rediscovered for each invocation. Proposing a test is not executing it.

## One executable contract per observation

Each acceptance record names:

`ID | source contract | pi entry | Opi entry | precondition | input/action |
expected observations | comparison rule | negative cases | command/fixture |
result/evidence | platform/material identity`

Observations include parameters/defaults, request bytes, callback/event order,
result, error stage, state changes, external effects and cleanup as applicable.
Cover normal, boundary and failure paths; stateful contracts also cover
cancellation, concurrency, stale results and commit/recovery ordering. A feature
name, test filename, empty checklist or final text alone is not acceptance.

Separate **snapshot-equivalence requirements** from **engineering/safety
requirements**. If they conflict, preserve both results and the proposed
intentional difference; changing an expected value cannot make both pass.
Not-applicable needs snapshot/scope evidence. Unexecuted, blocked and deliberate
differences never count as strict-reproduction passes.

## Comparison rules

Freeze the rule before evaluating the candidate, including any tolerated
nondeterminism. Use the same controlled upstream responses and schedules for
both implementations; do not compare live model prose word for word.

| Observation | Required comparison |
|---|---|
| Requests | URL, method, effective headers, payload, route, attempts and order; exact bytes for signing/protocol-sensitive fields. |
| Data | Per-field missing/null/default/unknown policy, numeric and character units, order and round-trip behavior; no blanket field deletion or array sorting. |
| Events/state | Event-time and later partial reads, indices/deltas/terminal results, callback timing, aliases/input mutation, stores/caches and side effects. Use partial orders only where concurrency permits them. |
| IDs/time | Controlled exact values or named correspondence preserving identity, uniqueness, units and ordering; do not strip all IDs/times. |
| Errors | Stage, channel, category, stable diagnostic content, retained partial/usage, and effects after failure. |
| Resources/performance | Same workload and observation boundary, cleanup/retention conditions and predefined budgets; measurements cannot compensate for behavioral mismatch. |

Give comparators negative controls: drop an event/usage, alter a tool ID or
header, swap null and missing, reorder a system update. Each applicable control
must fail. Keep the two execution drivers independent of the semantic code
being compared. Pending behavior needs a bounded observation assertion, not a
hung test process. Preserve first failures; retries are diagnostic, not erasure.

## Proof layers

Select the union required by the claimed behavior, not by test-file counts:

1. **Inventory and types/pure semantics:** public entry mapping, compile-pass/fail
   cases, normalization, schema, costs, data and finite state transitions.
2. **Module contracts:** actual Models/auth/store/stream/tool responsibilities
   with controlled external dependencies and fault/cancellation injection.
3. **Wire and differential:** real request construction/parsing against local
   HTTP/WS/process peers; identical fixtures drive pi and Rust public entries.
4. **Consumer and product:** real SDK/CLI/extension paths, multi-turn tools,
   model switching, persistence/recovery and cleanup; do not mock away the path
   being accepted.
5. **Distribution and robustness:** extracted release package in an external
   consumer, platform/runtime/features, property/fuzz/race/regression cases,
   secrets, resource retention and paired performance where claimed.
6. **Live service validity:** separately authorized, credential/budget-bounded
   evidence only for current live-service claims. It replaces none of the above.

Existing matching evidence may be retained after provenance/applicability
checks. Record actual commands, exits, executed/skipped/blocked counts, features,
fixture/result hashes and raw evidence locations. Zero tests, an omitted feature,
a stale filter, or an unrelated private test cannot close a public contract.

Default probes use synthetic secrets, controlled clocks/barriers, local peers,
isolated scratch and no external network. Never activate live tests because
credentials happen to exist. Missing dependencies or required execution
authority mean blocked proof, not missing functionality or permission to install.

## Independent acceptance gates

| Gate | Pass obligation |
|---|---|
| Scope | All discovered in-scope public capabilities, operations and host/data contracts mapped; exclusions evidenced; no unresolved baseline conflicts. |
| Behavior | Every applicable contract passes its defined observations; no unresolved or intentionally changed behavior counted as strict equivalence. |
| Structure | Responsibility/dependency/ownership review from `language-porting.md`, real interface consumers, explicit Rust types/errors/lifetimes, and preserved capability coverage. |
| Verification | Reachable, executed tests at the claimed interface, meaningful negative controls, source-bound results and reproducible drivers/fixtures. |
| Safety/resources | Authority before effects, secret/account isolation, explicit cancellation/cleanup, bounded work/retention and measured budgets without silently shrinking accepted inputs. |
| Delivery | Real package and external caller, documented platform/runtime/feature matrix, examples, dependency/data provenance and repeatable build/verification instructions. |

Do not average gate results. Separate current evidence from proposed checks.
A complete audit may report failed or blocked gates; it is not a release signoff.
Resource, performance and execution-volume thresholds need a stated basis:
snapshot contract, measured baseline, or explicit proposed engineering budget.
Where no defensible numeric value exists, specify the measurement and decision
needed rather than inventing a universal threshold or claiming acceptance ready.

## Conditional gap-closing card

For each retained difference or structural finding, include:

- **Finding IDs and target behavior:** the snapshot facts and observable outcome
  to reproduce; do not describe roadmap permission as a technical prerequisite.
- **Current path and proposed change:** concrete owners/callers, reusable parts,
  smallest coherent mechanism and migration; distinguish verified facts from
  design hypotheses. For Opi-only behavior, assess interference and preservation
  rather than recommending deletion simply to mirror pi.
- **Architecture case:** responsibility/dependency/lifetime impact, alternatives,
  native/runtime/feature/security costs, and why retaining or changing the
  present structure helps. Cite the evidence, not an Opi placement policy.
- **Acceptance:** linked executable records, failure cases, proof layers, gates,
  platform/resource conditions and test impact (`add/update/delete/retain/none`).
- **Dependencies and uncertainty:** technical ordering, unresolved decisions,
  required probes and rollback/migration risks. A design-only horizon requires
  design validation, not a fabricated current pi oracle.

Recommend next technical steps by proven dependency or risk-reduction reasons,
not product priority. The user can use these cards to revise specifications and
plan development later; this skill neither performs nor admits that work.
