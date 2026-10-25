# SDK onboarding and parameter admission — P.4 review package

[简体中文](onboarding-admission.zh.md) · [Complete graph and traces](onboarding-admission-graph.md) · [Structured P.2 result](onboarding-admission-review.json)

## Write-only installation receipt

The human explicitly selected `confirm-all` and **write-only**. P.4 is satisfied;
P.5 installed the confirmed graph at `2026-10-25T07:50:28.672233+00:00`.
No task-array or registered-source change was made after confirmation. The
sections below retain the pre-confirmation review record and its input hashes.

| Installed result | Verification |
|---|---|
| Canonical ledger | `.opi-impl-state.json`, SHA-256 `c06564f7501713a5108c26e17473dbb9707ea2d5fdc77fdae86431e714a6d5de` |
| Same reviewed task array | SHA-256 `ab7fcba57e2453cfea5aa397508168be53f0bd080e966885b9c98ae862646168`; 68 tasks, 273 scenarios, 32 passing / 36 failing |
| Preserved execution state | All 32 passing task objects, every task's status/dependencies/execution history and phase-exit history equal the pre-installation baseline |
| Registered sources | Same 16 paths; all CRLF-normalized pins match, zero source drift |
| Host rows | HOST-CFG / HOST-VALUES remain separate, unchanged, unregistered candidates, excluded from this SDK graph and spec_files |
| Scratch cleanup | Verified ignored draft and install candidate removed after successful installation; no ledger tmp or replacement backup remains |

Atomic installation used the Windows ledger guard's `Install` with
`-ExpectedTargetSha256 b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`.
It returned exit 0 and `ledger install ok` with the installed hash above.
`validate-plan.py .opi-impl-state.json` and ledger guard `Validate` on the
installed canonical file both returned exit 0. Structured post-installation
checks verified the identities, retained history, source pins and exclusions.

This write-only step changes the canonical ledger and this bilingual receipt/
shared graph header. Production, tests, parent/registered specifications and
host candidates are untouched by this installation step. Prior source edits
remain uncommitted. HEAD stays `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`;
staged and task commits are none. The unrelated research file retains SHA-256
`aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824` and is excluded.
No task execution, Cargo/runtime test, commit or publication followed. Test
impact: **none**.

## Pre-confirmation review snapshot

P.0 source admission is ready and P.2 returned **READY** for the reviewed draft.
READY authorized presentation at P.4 only. At that handoff the complete graph
had not yet been human-confirmed or installed; production, tests, the parent
specifications, canonical ledger and Git history were unchanged. The receipt
above records the subsequent explicit confirmation and installation.

The user approved the bilingual onboarding/configuration design and explicitly
requested `opi-implement plan`. Approved SDK meaning was first materialized as
shaping output in the existing registered sources, then reviewed at P.0/P.2;
the admission reviewer did not edit or invent source meaning. The approved
proposal is evidence, not an extra registered specification.

## Fixed inputs and reconciliation boundary

| Input | Identity / disposition |
|---|---|
| Branch / HEAD | `main` / `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab` |
| Pre-installation canonical ledger bytes | SHA-256 `b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a` |
| Original P.2 draft | SHA-256 `773f3744b42e58ccfe70ebf786c2d574cc45c989dcda115b1bee9b151a6ea35b` |
| Final draft, after review bookkeeping | `.opi-impl-state.draft.json`, SHA-256 `1da703413227fc72200cca429f1957f417966f4548a38ad8b092df13d852e64a` |
| Identical reviewed task array | SHA-256 `ab7fcba57e2453cfea5aa397508168be53f0bd080e966885b9c98ae862646168`; canonical sorted-key compact JSON, UTF-8, no trailing newline |
| Registered source set | Same 16 paths. Only bilingual `spec-draft` and the sole `acceptance-matrix` have changed contents/pins. The full before/after pin table is in the graph appendix. |
| Runtime graph | Same 68 tasks, 32 passing / 36 failing; all task IDs, titles, dependencies, runtime history and all 32 passing task objects preserved |
| New acceptance | 7 ONB criteria, 22 added scenarios in 22 failing owners; total scenarios 251 → 273 |

P.2 consumed the original task array. Afterwards only root `verify_runs`
bookkeeping was appended to the ignored draft; the task-array identity and
registered-source inputs did not change. This does not edit the reviewed graph.
At that handoff the draft confirmation timestamp was null. The canonical
confirmation and old source pins remained untouched until the P.4 human gate.

## P.0 source admission

| Admission question | Governing source / answer |
|---|---|
| Reviewed and registered | Existing Phase 20 registry, exactly its 16 files; approved SDK design materialized in `spec-draft.md` / `.zh.md` and the unique matrix |
| Problem / solution / scope | Existing SDK source plus “SDK onboarding and parameter acceptance”: ordinary SDK first use, manual closure and exact adopted request semantics |
| Success and exit | ONB-01–07 map to existing V groups and owners; retain 57 active AIR / 54 SDK V groups, independent F.1a and host-only exclusion |
| Provenance / Rust differences | Inward Pi 1.3 SDK protocol source; Pi 1.4 README is usability comparison only. Rust types/runtime/dependency and existing RDF rules remain. No new SDK protocol delta. |
| Placement | Existing SDK owners and assurance/document owners. Product parsers/value execution remain separate Reference Product candidates. |
| Domain and authority | Parent AUTH/PLACE/PHASE clauses and CONTEXT unchanged; no new User Policy, Package Trust or Agent Core authority |
| Public test seam | Real Models/native/simple/direct requests, same-source external Cargo consumer, native/platform proof and documentary handoff, each at its existing owner |
| Source-return rule | No missing product meaning was repaired inside graph initialization. The human-approved shaping amendment precedes P.0. No arbitrary proposal or recursive linked file enters spec_files. |

## Row-level acceptance changes

The [full graph](onboarding-admission-graph.md) renders every task's tier,
owned paths, commit type, dependencies/topological order, evaluator flag,
scenario/call-site counts, substrate flag, forbidden scope and inference notes,
all scenario ownership/commands and six-answer traces. It also includes exact
before/after DoDs and new scenarios for each changed row. The shared appendix
quotes canonical field text and has bilingual headings; it needs no duplicate
translated data copy and is not a parallel progress ledger.

| Owner | Added acceptance / exact field effect |
|---|---|
| 20.16 | Codex native options/frozen transport at its original protocol owner; DoD + ONB-03 scenario + sourced note, no new dependency |
| 20.34 | Profile/model/call defaults through real requests, auth endpoint before compat, false/zero/omission and exchange/attempt accounting; DoD + ONB-05 + note |
| 20.35 | Native/simple source/type/unit/default/precedence inventory and shared-mechanism request capture; DoD + ONB-03 + note, append existing `request_enrichment.rs` to behavioral tests |
| 20.36–20.49 | Each protocol/typed operation's exact parameter, omission, failure and real wire proof; one ONB-03 scenario, DoD supplement and note per owner |
| 20.51 | Header omission/set/remove, protected identity and callbacks per variant; DoD + ONB-04 + note; preserve limited current-adapter closure and later consumers |
| 20.56 | Bilingual manual organization and real API examples; DoD + ONB-01 + note; own exact future `quick_start.rs`, `quick_start_stream.rs`, `tool_loop.rs` examples, compile here |
| 20.57 | Execute those same examples from a real `.crate` in an ordinary external consumer; DoD + ONB-02 + note; preserve full assembled SDK/package responsibilities |
| 20.58 | README support claims versus actual native/MSRV/features/artifact evidence; ONB-06 + note, original DoD unchanged |
| 20.59 | Final manual/example/evidence references and actual dispositions; DoD + ONB-07 + note; matrix drift P.0/P.2/P.4 and independent F.1a unchanged |

All other fields/rows are retained. No task status is promoted, no auth evidence
is relabeled, no original scenario or note is deleted and no dependency is
added or removed. Existing four standardized notes, eight shared decisions and
six substrate dependency closures remain intact. New requirements use the
same public mechanisms and test surfaces, not new shared decision owners.

## Two host rows: registration planning only

The separate [Reference Product source candidate](../reference-product-configuration/delivery-candidate.md)
has a Chinese counterpart, parent/strategy mapping, problem/scope/placement,
fixed Pi 1.4 config inputs, non-goals, success/exit, platform/risk/rollback,
production seams and six executable scenario groups.

| Source row | Proposed registration | Dependency direction |
|---|---|---|
| HOST-CFG | One future Reference Product vertical slice: default TOML, explicit settings/models JSON bundle, direct Pi fields, one resolver, old-field rejection, authority-preserving config-to-request proof | Consumes admitted SDK factories/options/callbacks |
| HOST-VALUES | A second future vertical slice: literal/env/escaping/command values, actual auth/headers, per-request resolution, source timeout/cancel/process-tree cleanup | Depends on HOST-CFG and reusable admitted source/process mechanisms |

These rows receive no formal Phase/task number now. They are not `20.60/61`,
not current executable tasks and not extra `spec_files`. Future work selects
their Phase and explicitly registers the source before a separate P.0/P.2/P.4.
No host parser/file/policy becomes a standalone SDK prerequisite. No parent
revision or current skill-registry change is needed for this planning record.

Pi 1.4 field vocabulary is confined to that host candidate. SDK protocols stay
Pi 1.3; client identity, whitespace-tool-result and other 1.4 behavioral changes
require their own source review. Old TOML spelling gets no compatibility reader;
actual Opi-only capabilities retain typed ownership rather than disappearing.

## P.2 independent adversarial result

One fresh-context reviewer sequentially ran the five charters covering all ten
design/execution lenses, the full original graph, minimum-change answers,
shared decisions and substrate closure. Independence is
`fresh-context-same-family`, not cross-model. No Workflow runtime entry point is
available in this session; the documented single-reviewer fallback was used.
`plan.workflow.js` was not executed and `wf_ref` is null.

Result: **READY**, no surviving design/graph finding or human flag. Three
candidate blockers were rejected after counterchecking:

1. Missing host tasks in the SDK graph: rejected because the source explicitly
   excludes them and requires independent registration.
2. 20.35 depending on unfinished adapters: rejected because it proves only the
   shared mechanism; protocol owners and 20.34 retain assembled proof.
3. 20.56 only compiling examples: rejected because dependent 20.57 owns the
   actual packaged same-source public execution.

The [structured result](onboarding-admission-review.json) preserves the findings,
citations, rejection reasons and resource/identity record. No finding was
auto-folded into the graph. The reviewer changed no files, ran no Cargo/provider
call and did not confirm or install the graph.

## Verification performed for this admission

| Command / check | Actual result |
|---|---|
| `python .claude/skills/opi-implement/scripts/validate-plan.py .opi-impl-state.draft.json` | Exit 0; `plan validation ok tasks=68` |
| `& .claude/skills/opi-implement/scripts/ledger-guard.ps1 -Command Validate -Path .opi-impl-state.draft.json` | Exit 0; strict UTF-8/v2/size/sensitive-data validation. Validate only; no Install. |
| Supplemental structured Python check | Unique IDs, complete dependency references, no cycles, tiers, behavioral-test ownership, scenario commands/sites and ONB coverage passed. Existing path annotations in 20.13/14 were interpreted as annotations, not path suffixes. |
| Bilingual/identity check | Seven SDK criteria and six host criteria match; five host comparison hashes, local links and whitespace passed. All 32 passing objects and all runtime/dependency fields are equal to canonical. |
| `python scripts/opi-doc-check.py` | Exit 0; `opi documentation contracts: PASS` |
| `git diff --check` plus new Markdown whitespace check | Exit 0; no whitespace failures |
| Cargo/runtime/package/live | Not executed: this delivery changes only approved source/plan/evidence metadata, not implementation or tests |

Pre-installation worktree-only inventory: committed for that step **none**; staged **none**;
tracked unstaged are exactly the three registered SDK source files. New
admission evidence and bilingual host candidate remain untracked. The four
prior approved proposal documents remain carried-in, untouched and untracked.
The unrelated research file remains untracked and excluded, SHA-256
`aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824`.
The ignored draft was retained to resume P.4 and was removed after the confirmed
installation above. Test impact: **none**.

## P.4 choice history and write boundary

The concrete [complete graph](onboarding-admission-graph.md) and its row changes
are ready for human review. Options from the owning initializer are
`confirm-all`, `edit-task <id>`, `apply-rule`, `export-draft`, `import-draft` or
`abort`. Any graph edit invalidates READY and repeats P.2 before confirmation.

The earlier “agree with the design” authorized materialization and planning;
it did not confirm the then newly displayed complete graph. The workflow's P.4
rule is: **“REFUSE to proceed until whole graph is confirmed.”** The canonical
ledger stayed unchanged with expected source drift while awaiting that gate.
The later explicit `confirm-all` and write-only instruction satisfied it.

The write-only boundary has now completed as recorded above: confirmation,
identity recheck, optimistic-concurrency installation and scratch removal.
No task implementation, staging, commit or publication follows automatically.
